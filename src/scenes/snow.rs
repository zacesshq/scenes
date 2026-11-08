//! Snow: flakes at two depths drift down over far hills and settle into a bank along the bottom,
//! painted in solid pixels.
//!
//! The sky, the hills and their pines are tints of the background held below the non-text floor,
//! so they read as background. The flakes and the settled bank keep the theme's own drawing
//! colours, so they stand out as shapes and keep their outline when colour is off.

use std::f32::consts::TAU;

use super::{count_for, paint};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// How much one flake adds to the bank, in rows.
const FLAKE_DEPTH: f32 = 0.34;
const SKY_STEPS: usize = 8;

#[derive(Clone, Debug)]
struct Flake {
    x: f32,
    y: f32,
    speed: f32,
    phase: f32,
    depth: Depth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Depth {
    Near,
    Middle,
    Far,
}

/// The colours of the snow in one theme, every one derived from the theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct SnowColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub far_hill: Rgb,
    pub near_hill: Rgb,
    pub pine: Rgb,
    pub pine_shade: Rgb,
    pub pine_snow: Rgb,
    pub flake: Rgb,
    pub flake_glow: Rgb,
    pub mid_flake: Rgb,
    pub far_flake: Rgb,
    pub bank: Rgb,
    pub bank_top: Rgb,
    pub bank_shadow: Rgb,
    pub bank_deep: Rgb,
    sky: Ramp,
}

/// Pure white, which on a light theme is the snow itself: it sits a touch above an off-white
/// background, so it stays imagery while reading as fresh snow.
const WHITE: Rgb = Rgb::new(255, 255, 255);

impl SnowColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        if !theme.dark {
            return Self::daylight(theme);
        }
        // a winter night: a deep sky, far hills of snow lit by it, darker near hills and pines
        let sky_top = shades.tint(theme.info, 0.14, 1.3);
        let sky_low = shades.tint(theme.info, 0.36, 1.75);
        let far_hill = shades.tint(theme.info, 0.6, 2.25);
        let near_hill = shades.tint(mix(theme.foreground, theme.info, 0.4), 0.45, 2.75);
        let pine = shades.tint(theme.green(), 0.3, 1.5);
        let pine_shade = shades.tint(theme.green(), 0.16, 1.25);
        let pine_snow = shades.tint(theme.foreground, 0.5, 2.6);
        // snow is white whatever the text colour, warmed a little towards it
        let white = mix(WHITE, theme.foreground, 0.3);
        let flake = white;
        let flake_glow = shades.tint(flake, 0.6, IMAGERY_CEILING - 0.2);
        let mid_flake = shades.held(white, theme.info, 0.3, theme.floor());
        // settled snow is white, cooler and darker where it lies deep
        let bank_top = white;
        let bank = shades.held(white, theme.info, 0.12, theme.floor());
        let bank_shadow = shades.held(
            bank,
            mix(theme.info, sky_top, 0.4),
            0.4,
            IMAGERY_CEILING + 0.6,
        );
        let bank_deep = shades.held(bank, sky_top, 0.5, IMAGERY_CEILING + 0.3);
        Self {
            sky_top,
            sky_low,
            far_hill,
            near_hill,
            pine,
            pine_shade,
            pine_snow,
            flake,
            flake_glow,
            mid_flake,
            far_flake: theme.muted,
            bank,
            bank_top,
            bank_shadow,
            bank_deep,
            sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
        }
    }

    /// A snowy winter day: a pale blue sky deepening overhead, white flakes with a soft cool
    /// edge, a white bank shaded blue underneath and dark pines that keep the outline.
    fn daylight(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let cool = mix(theme.info, theme.grey(), 0.35);
        let sky_top = shades.tint(cool, 0.75, 2.3);
        let sky_low = shades.tint(cool, 0.3, 1.35);
        let far_hill = shades.tint(cool, 0.5, 1.75);
        let near_hill = shades.tint(cool, 0.75, 2.15);
        let pine = shades.held(theme.green(), theme.foreground, 0.25, theme.floor());
        let pine_shade = shades.held(pine, theme.foreground, 0.4, theme.floor());
        let pine_snow = mix(WHITE, theme.background, 0.3);
        let flake = WHITE;
        let flake_glow = shades.tint(cool, 0.3, 1.35);
        let mid_flake = mix(WHITE, sky_low, 0.3);
        let far_flake = mix(WHITE, sky_top, 0.45);
        let bank_top = WHITE;
        let bank = mix(WHITE, theme.background, 0.6);
        let bank_shadow = shades.tint(cool, 0.25, 1.3);
        let bank_deep = shades.tint(cool, 0.45, 1.6);
        Self {
            sky_top,
            sky_low,
            far_hill,
            near_hill,
            pine,
            pine_shade,
            pine_snow,
            flake,
            flake_glow,
            mid_flake,
            far_flake,
            bank,
            bank_top,
            bank_shadow,
            bank_deep,
            sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
        }
    }
}

pub struct Snow {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    flakes: Vec<Flake>,
    /// The depth of settled snow in each column, in rows.
    bank: Vec<f32>,
    /// The two hill lines, as a height in pixels above the bottom for each column.
    far_hills: Vec<i32>,
    near_hills: Vec<i32>,
    /// Pines on the nearer hills, as (column, height in pixels).
    pines: Vec<(usize, i32)>,
}

impl Snow {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut snow = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            flakes: Vec::new(),
            bank: Vec::new(),
            far_hills: Vec::new(),
            near_hills: Vec::new(),
            pines: Vec::new(),
        };
        snow.resize(width, height, rng);
        snow
    }

    fn new_flake(&self, rng: &mut Rng, from_top: bool) -> Flake {
        let roll = rng.range(0.0, 1.0);
        let (depth, speed) = if roll < 0.3 {
            (Depth::Near, rng.range(2.6, 3.8))
        } else if roll < 0.6 {
            (Depth::Middle, rng.range(1.8, 2.5))
        } else {
            (Depth::Far, rng.range(1.0, 1.5))
        };
        Flake {
            x: rng.range(0.0, self.width as f32),
            y: if from_top {
                -rng.range(0.0, 2.0)
            } else {
                rng.range(-1.0, self.height as f32)
            },
            speed: speed * self.motion.speed(),
            phase: rng.range(0.0, TAU),
            depth,
        }
    }

    /// The deepest the bank may get: a third of the screen.
    fn max_depth(&self) -> f32 {
        ((self.height / 3) as f32).max(1.0)
    }

    #[cfg(test)]
    pub(crate) fn bank_depth(&self) -> f32 {
        self.bank.iter().copied().fold(0.0, f32::max)
    }

    /// A rolling line of hills, `peak` pixels at its highest.
    fn hills(&self, rng: &mut Rng, peak: f32) -> Vec<i32> {
        let (l1, l2) = (rng.range(10.0, 22.0), rng.range(4.0, 8.0));
        let (p1, p2) = (rng.range(0.0, TAU), rng.range(0.0, TAU));
        (0..self.width)
            .map(|x| {
                let x = x as f32;
                let h = peak * (0.55 + 0.3 * (x / l1 + p1).sin() + 0.15 * (x / l2 + p2).sin());
                h.round() as i32
            })
            .collect()
    }

    /// The settled depth of a column in pixels, softened against its neighbours so the bank
    /// rolls in drifts rather than standing in steps.
    fn surface(&self, x: usize) -> f32 {
        let at = |i: usize| self.bank[i.min(self.width - 1)];
        let left = at(x.saturating_sub(1));
        let right = at(x + 1);
        (left + 2.0 * at(x) + right) / 2.0
    }

    fn paint(&self, canvas: &mut Canvas, c: &SnowColours) {
        let tall = canvas.height() as i32;
        for y in 0..tall {
            canvas.fill_row(y, c.sky.at(y as f32 / tall.max(1) as f32));
        }

        // the hills, far then near, with pines standing on the nearer line
        for (x, &h) in self.far_hills.iter().enumerate() {
            canvas.fill_rect(x as i32, tall - h, 1, h, c.far_hill);
        }
        for (x, &h) in self.near_hills.iter().enumerate() {
            canvas.fill_rect(x as i32, tall - h, 1, h, c.near_hill);
        }
        // each pine is a narrow triangle in tiers, shaded on its right and with snow lying on
        // the left end of every tier, where the light comes from
        for &(x, height) in &self.pines {
            let Some(&ground) = self.near_hills.get(x) else {
                continue;
            };
            let base = tall - ground;
            let x = x as i32;
            canvas.set(x, base - 1, c.pine_shade);
            // a narrow triangle with a notch where each tier of branches starts
            let tier_rows = (height / 3).max(3);
            for i in 0..height - 1 {
                let notch = i32::from(i > 0 && i % tier_rows == 0);
                let spread = ((i as f32 * 0.42).round() as i32 - notch).max(0);
                let y = base - height + i;
                canvas.fill_rect(x - spread, y, spread + 1, 1, c.pine);
                canvas.fill_rect(x + 1, y, spread, 1, c.pine_shade);
                if notch == 1 && spread >= 1 {
                    canvas.fill_rect(x - spread, y, spread, 1, c.pine_snow);
                }
            }
            canvas.set(x, base - 1 - height, c.pine_snow);
        }

        // the flakes at three depths: a near flake is a soft cross, a middle one a bright point
        // and a far one a faint point
        for flake in &self.flakes {
            let (x, y) = (flake.x.floor() as i32, (flake.y * 2.0).floor() as i32);
            match flake.depth {
                Depth::Near => {
                    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        canvas.set(x + dx, y + dy, c.flake_glow);
                    }
                    canvas.set(x, y, c.flake);
                }
                Depth::Middle => canvas.set(x, y, c.mid_flake),
                Depth::Far => canvas.set(x, y, c.far_flake),
            }
        }

        // the settled bank: a bright crust along its surface, the body of the snow and blue
        // shadow gathering lower down, with a few fixed glints in the crust
        for x in 0..self.width {
            let pixels = self.surface(x).round() as i32;
            if pixels <= 0 {
                continue;
            }
            let top = tall - pixels;
            let xi = x as i32;
            for y in top..tall {
                let into = (y - top) as f32 / pixels.max(1) as f32;
                let colour = if y == top {
                    c.bank_top
                } else if into < 0.45 {
                    c.bank
                } else if into < 0.8 || pixels < 4 {
                    c.bank_shadow
                } else {
                    c.bank_deep
                };
                canvas.set(xi, y, colour);
            }
            if pixels >= 3 && paint::hash(xi, top + 1, 77) % 9 == 0 {
                canvas.set(xi, top + 1, c.bank_top);
            }
        }
    }
}

impl Scene for Snow {
    fn name(&self) -> &'static str {
        "snow"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = count_for(self.width, self.height, 14, 6, 400);
        self.flakes = (0..wanted).map(|_| self.new_flake(rng, false)).collect();
        // settled snow stays where it lies; a wider screen gets bare ground at the new columns
        let max_depth = self.max_depth();
        // snow already lies on the ground when the scene opens, in low rolling drifts
        let old = self.bank.len();
        let (lift, phase) = (rng.range(6.0, 11.0), rng.range(0.0, TAU));
        self.bank.resize(self.width, 0.0);
        for (x, depth) in self.bank.iter_mut().enumerate().skip(old) {
            let roll = 0.5 + 0.5 * (x as f32 / lift + phase).sin();
            *depth = (max_depth * (0.25 + 0.2 * roll)).max(0.5);
        }
        for depth in &mut self.bank {
            *depth = depth.min(max_depth);
        }
        let tall = self.height as f32 * 2.0;
        self.far_hills = self.hills(rng, tall * 0.45);
        self.near_hills = self.hills(rng, tall * 0.28);
        self.pines.clear();
        if self.height >= 8 {
            let mut x = rng.below(6);
            while x < self.width {
                let height = (tall * rng.range(0.08, 0.16)).round() as i32;
                self.pines.push((x, height.max(3)));
                x += 3 + rng.below(9);
            }
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let width = self.width as f32;
        let max_depth = self.max_depth();
        for i in 0..self.flakes.len() {
            let flake = &mut self.flakes[i];
            flake.y += flake.speed * dt;
            let drift =
                (super::cycle(self.t, 0.7 * self.motion.speed()) + flake.phase).sin() * 0.8 + 0.3;
            flake.x = super::wrap(flake.x + drift * self.motion.speed() * dt, width);
            let column = (flake.x as usize).min(self.width - 1);
            let surface = self.height as f32 - 1.0 - self.bank[column];
            if flake.y >= surface {
                if self.bank[column] < max_depth {
                    self.bank[column] += FLAKE_DEPTH;
                }
                // a column that stands more than a row above its neighbour sheds into it
                for neighbour in [column.wrapping_sub(1), column + 1] {
                    if neighbour < self.width && self.bank[column] > self.bank[neighbour] + 1.0 {
                        self.bank[column] -= FLAKE_DEPTH / 2.0;
                        self.bank[neighbour] += FLAKE_DEPTH / 2.0;
                    }
                }
                self.flakes[i] = self.new_flake(rng, true);
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = SnowColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixels::distance;
    use crate::scenes::testing::{
        check_determinism, check_pixel_scene, check_scene, pixels, render,
    };
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("snow");
        check_determinism("snow");
        check_pixel_scene("snow", 6.0);
    }

    #[test]
    fn the_sky_hills_and_snow_stay_distinct_in_every_theme() {
        for theme in theme::all() {
            let c = SnowColours::new(theme);
            let name = theme.name;
            for (a, b, what) in [
                (c.sky_top, c.sky_low, "the sky's top and bottom"),
                (c.far_hill, c.sky_low, "the far hills and the sky"),
                (c.near_hill, c.far_hill, "the near and far hills"),
                (c.pine, c.near_hill, "the pines and their hill"),
            ] {
                assert!(distance(a, b) >= 0.03, "{what} collapse in {name}");
            }
            let mut imagery = vec![
                ("sky top", c.sky_top),
                ("sky low", c.sky_low),
                ("far hill", c.far_hill),
                ("near hill", c.near_hill),
                ("flake glow", c.flake_glow),
            ];
            if theme.dark {
                imagery.push(("pine", c.pine));
                for colour in [c.flake, c.far_flake, c.bank, c.mid_flake] {
                    assert!(theme.contrast_of(colour) >= theme.floor(), "{name}");
                }
                for colour in [c.bank_top, c.bank_shadow, c.bank_deep] {
                    assert!(theme.contrast_of(colour) >= IMAGERY_CEILING, "{name}");
                }
                for colour in [c.flake, c.far_flake, c.bank] {
                    for backdrop in [c.sky_low, c.far_hill, c.near_hill, c.pine] {
                        assert!(distance(colour, backdrop) >= 0.1, "{name}");
                    }
                }
            } else {
                // by day the snow is white imagery, lighter than every part of the sky, and
                // the pines carry the outline
                imagery.extend([
                    ("flake", c.flake),
                    ("bank", c.bank),
                    ("bank shadow", c.bank_shadow),
                ]);
                assert!(theme.contrast_of(c.pine) >= theme.floor(), "{name}");
                for backdrop in [c.sky_top, c.far_hill, c.near_hill] {
                    assert!(distance(c.flake, backdrop) >= 0.05, "{name}");
                }
            }
            for (what, colour) in imagery {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
        }
    }

    #[test]
    fn flakes_fall_and_settle_into_a_bank() {
        let c = SnowColours::new(theme::default_theme());
        let mut rng = Rng::new(12);
        let mut snow = Snow::new(40, 12, Motion::FULL, &mut rng);
        let early = render(&mut snow, 40, 12, 5);
        let all = pixels(&early);
        assert!(all.contains(&c.flake) && all.contains(&c.far_flake) && all.contains(&c.mid_flake));
        for _ in 0..(40 * 20) {
            snow.step(0.05, &mut rng);
        }
        assert!(snow.bank_depth() >= 1.0, "{}", snow.bank_depth());
        let frame = render(&mut snow, 40, 12, 1);
        let bottom: Vec<Rgb> = (0..40)
            .filter_map(|x| frame.get(x, 11).and_then(|cell| cell.pixel_pair()))
            .flat_map(|(a, b)| [a, b])
            .collect();
        assert!(
            bottom.contains(&c.bank)
                || bottom.contains(&c.bank_shadow)
                || bottom.contains(&c.bank_deep)
        );
    }

    #[test]
    fn the_bank_never_rises_above_a_third_of_the_screen() {
        let mut rng = Rng::new(2);
        let mut snow = Snow::new(20, 8, Motion::CALM, &mut rng);
        for _ in 0..(600 * 20) {
            snow.step(0.05, &mut rng);
        }
        assert!(snow.bank_depth() <= snow.max_depth() + FLAKE_DEPTH);
        assert!(snow.bank_depth() >= 1.0);
    }
}
