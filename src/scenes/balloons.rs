//! Balloons: hot air balloons at three distances lifting slowly from behind rolling hills,
//! painted in solid pixels. Each envelope is shaded and striped in gores, with ropes down to a
//! small basket. Each balloon rises from behind the ridge it belongs to, so the hills give the
//! sky its depth. Soft clouds drift slower still across a dawn sky on a dark theme and a day sky
//! on a light one.
//!
//! Every colour comes from the active theme. The sky, the clouds and the hills are tints of the
//! background held below the non-text floor, so they read as background; the balloons keep the
//! theme's own drawing colours, so they stand out as shapes and keep their outline when colour
//! is off.

use std::f32::consts::{FRAC_PI_2, TAU};

use super::{count_for, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "Striped hot air balloons lifting slowly from behind rolling hills under soft drifting clouds, at dawn or by day in light themes";

pub const CAPTIONS: &[&str] = &[
    "Lift off. Everything passed.",
    "Rising gently on a clean build.",
    "Nothing is holding this build down.",
    "Clear skies and light winds.",
    "All ropes cast off, all checks green.",
];

/// Steps in the sky ramp.
const SKY_STEPS: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Size {
    Small,
    Medium,
    Large,
}

impl Size {
    /// The envelope's radius in pixels.
    fn radius(self) -> f32 {
        match self {
            Size::Small => 2.2,
            Size::Medium => 3.6,
            Size::Large => 5.6,
        }
    }

    /// The whole balloon's width and height in pixels, basket included.
    fn extent(self) -> (f32, f32) {
        let r = self.radius();
        (2.0 * r + 1.0, r * 2.9 + 3.0)
    }

    /// How many gores the envelope is striped in.
    fn gores(self) -> usize {
        match self {
            Size::Small => 3,
            Size::Medium => 5,
            Size::Large => 7,
        }
    }

    /// The hill a balloon of this size rises from behind: the far, middle or near ridge.
    fn layer(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug)]
struct Balloon {
    /// The left edge, in pixels.
    x: f32,
    /// The top of the envelope, in pixels.
    y: f32,
    drift: f32,
    /// Pixels a second.
    rise: f32,
    size: Size,
    colour: usize,
}

#[derive(Clone, Debug)]
struct Cloud {
    x: f32,
    /// The flat base of the cloud, in pixels.
    y: f32,
    /// Puffs as offset along the base, radius.
    puffs: Vec<(f32, f32)>,
}

/// The colours of the balloons' sky in one theme, every one derived from the theme's own. Some
/// are kept only so the tests can check the bands against each other.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct SkyColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub cloud: Rgb,
    pub cloud_shade: Rgb,
    /// The far, middle and near hills.
    pub hills: [Rgb; 3],
    /// The lit crest of each hill.
    pub crests: [Rgb; 3],
    pub hedge: Rgb,
    pub rope: Rgb,
    pub basket: Rgb,
    /// Each balloon as its main colour and the colour of every other gore.
    pub envelopes: [(Rgb, Rgb); 5],
    sky: Ramp,
}

impl SkyColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        // a dawn sky on a dark theme, deep at the top and warm at the horizon; a clear day sky
        // on a light theme, paling towards the horizon
        let (sky_top, sky_low, cloud, cloud_shade) = if theme.dark {
            (
                shades.tint(theme.info, 0.16, 1.3),
                shades.tint(theme.warning, 0.38, 2.0),
                shades.tint(theme.foreground, 0.4, 2.5),
                shades.tint(theme.info, 0.3, 1.6),
            )
        } else {
            (
                shades.tint(theme.info, 0.42, 2.1),
                shades.tint(theme.info, 0.1, 1.2),
                theme.background,
                shades.tint(theme.info, 0.25, 1.55),
            )
        };
        // the hills fade towards the sky with distance and gain colour as they come nearer
        let (hills, crests) = if theme.dark {
            (
                [
                    shades.tint(theme.blue(), 0.32, 1.55),
                    shades.tint(theme.green(), 0.3, 1.8),
                    shades.tint(theme.green(), 0.55, 2.6),
                ],
                [
                    shades.tint(theme.blue(), 0.45, 1.9),
                    shades.tint(theme.green(), 0.45, 2.25),
                    shades.tint(theme.bright_green(), 0.75, 2.95),
                ],
            )
        } else {
            (
                [
                    shades.tint(theme.blue(), 0.35, 1.75),
                    shades.tint(theme.green(), 0.32, 1.75),
                    shades.tint(theme.green(), 0.8, 2.9),
                ],
                [
                    shades.tint(theme.blue(), 0.15, 1.25),
                    shades.tint(theme.green(), 0.18, 1.35),
                    shades.tint(theme.green(), 0.55, 2.4),
                ],
            )
        };
        let hedge = if theme.dark {
            shades.tint(theme.green(), 0.15, 1.25)
        } else {
            mix(hills[2], theme.foreground, 0.3)
        };
        Self {
            sky_top,
            sky_low,
            cloud,
            cloud_shade,
            hills,
            crests,
            hedge,
            rope: theme.muted,
            basket: theme.yellow(),
            envelopes: [
                (theme.red(), theme.bright_yellow()),
                (theme.bright_yellow(), theme.red()),
                (theme.bright_cyan(), theme.foreground),
                (theme.magenta(), theme.bright_yellow()),
                (theme.bright_blue(), theme.foreground),
            ],
            sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
        }
    }
}

pub struct Balloons {
    width: usize,
    height: usize,
    motion: Motion,
    balloons: Vec<Balloon>,
    clouds: Vec<Cloud>,
    /// The top pixel row of each hill per column: far, middle and near.
    ridges: [Vec<i32>; 3],
}

impl Balloons {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut sky = Self {
            width: 1,
            height: 1,
            motion,
            balloons: Vec::new(),
            clouds: Vec::new(),
            ridges: [Vec::new(), Vec::new(), Vec::new()],
        };
        sky.resize(width, height, rng);
        sky
    }

    fn sizes(&self) -> &'static [Size] {
        if self.height >= 24 && self.width >= 40 {
            &[
                Size::Small,
                Size::Small,
                Size::Medium,
                Size::Medium,
                Size::Large,
            ]
        } else if self.height >= 12 {
            &[Size::Small, Size::Small, Size::Medium]
        } else {
            &[Size::Small]
        }
    }

    /// The lowest pixel row of a hill, so a balloon can be launched from behind it.
    fn ridge_low(&self, layer: usize) -> i32 {
        self.ridges[layer].iter().copied().max().unwrap_or(0)
    }

    fn launch(&self, rng: &mut Rng, anywhere: bool) -> Balloon {
        let size = *rng.pick(self.sizes());
        let (wide, tall) = size.extent();
        let rise = match size {
            Size::Small => rng.range(0.3, 0.5),
            Size::Medium => rng.range(0.5, 0.8),
            Size::Large => rng.range(0.7, 1.0),
        } * self.motion.speed();
        let low = self.ridge_low(size.layer()) as f32;
        // a few tries for a column clear of the other balloons of the same size, so two never
        // sit on top of each other
        let mut x = rng.range(0.0, (self.width as f32 - wide).max(1.0));
        for _ in 0..4 {
            let clear = self
                .balloons
                .iter()
                .all(|b| b.size != size || (b.x - x).abs() > wide + 1.0);
            if clear {
                break;
            }
            x = rng.range(0.0, (self.width as f32 - wide).max(1.0));
        }
        Balloon {
            x,
            y: if anywhere {
                rng.range(-tall, low)
            } else {
                low + rng.range(0.0, 6.0)
            },
            drift: rng.range(-0.35, 0.35) * self.motion.speed(),
            rise,
            size,
            colour: rng.below(5),
        }
    }

    fn paint_cloud(canvas: &mut Canvas, cloud: &Cloud, c: &SkyColours) {
        for &(dx, r) in &cloud.puffs {
            let (cx, cy) = (cloud.x + dx, cloud.y - r * 0.6);
            let reach = r.ceil() as i32 + 1;
            for py in (cy - r).floor() as i32..=(cloud.y as i32) {
                for px in (cx as i32 - reach)..=(cx as i32 + reach) {
                    let (ox, oy) = (px as f32 + 0.5 - cx, py as f32 + 0.5 - cy);
                    if ox * ox + oy * oy > r * r {
                        continue;
                    }
                    // lit from above, shaded along the flat base
                    let low = py as f32 + 0.5 > cloud.y - 1.2;
                    canvas.set(px, py, if low { c.cloud_shade } else { c.cloud });
                }
            }
        }
    }

    fn paint_balloon(canvas: &mut Canvas, b: &Balloon, c: &SkyColours) {
        let (main, gore) = c.envelopes[b.colour % c.envelopes.len()];
        let r = b.size.radius();
        let (wide, _) = b.size.extent();
        let cx = b.x + wide / 2.0;
        let cy = b.y + r;
        // the envelope: a round crown that narrows to the mouth like a teardrop
        let mouth_y = cy + r * 1.55;
        let mouth = (r * 0.32).max(0.6);
        let shade = mix(main, c.sky_top, 0.35);
        let shade_gore = mix(gore, c.sky_top, 0.35);
        let gores = b.size.gores();
        for py in b.y.floor() as i32..=mouth_y.ceil() as i32 {
            let y = py as f32 + 0.5;
            let half = if y <= cy {
                (r * r - (y - cy).powi(2)).max(0.0).sqrt()
            } else {
                let t = ((y - cy) / (mouth_y - cy)).clamp(0.0, 1.0);
                // the lower half curves in rather than running straight to the mouth
                mouth + (r - mouth) * (1.0 - t * t)
            };
            if half <= 0.2 {
                continue;
            }
            for px in (cx - half - 1.0).floor() as i32..=(cx + half + 1.0).ceil() as i32 {
                let x = px as f32 + 0.5;
                let u = (x - cx) / half;
                if u.abs() > 1.0 {
                    continue;
                }
                // gores are spaced by the angle around the envelope, so they bunch towards the
                // sides as a round balloon's do
                let angle = u.clamp(-1.0, 1.0).asin() / FRAC_PI_2;
                let band = (((angle + 1.0) / 2.0) * gores as f32).floor() as usize;
                let striped = band % 2 == 1;
                let dark = u > 0.45 || y > mouth_y - 1.0;
                let colour = match (striped, dark) {
                    (false, false) => main,
                    (true, false) => gore,
                    (false, true) => shade,
                    (true, true) => shade_gore,
                };
                canvas.set(px, py, colour);
            }
        }
        // the ropes and the basket below the mouth
        let mouth_row = mouth_y.ceil() as i32;
        let basket_top = mouth_row + 1 + i32::from(b.size != Size::Small);
        let (left, right) = ((cx - mouth).round() as i32, (cx + mouth).round() as i32 - 1);
        for y in mouth_row + 1..basket_top {
            canvas.set(left, y, c.rope);
            canvas.set(right.max(left), y, c.rope);
        }
        let basket_wide = (right - left + 1).max(1);
        canvas.fill_rect(left, basket_top, basket_wide, 1, c.basket);
        if b.size == Size::Large {
            canvas.fill_rect(
                left,
                basket_top + 1,
                basket_wide,
                1,
                mix(c.basket, c.sky_top, 0.3),
            );
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &SkyColours) {
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        for y in 0..tall {
            canvas.fill_row(y, c.sky.at(y as f32 / (tall as f32 * 0.85)));
        }
        for cloud in &self.clouds {
            Self::paint_cloud(canvas, cloud, c);
        }
        for layer in 0..3 {
            // balloons rising from behind this hill are drawn before it, so it hides them
            for b in self.balloons.iter().filter(|b| b.size.layer() == layer) {
                Self::paint_balloon(canvas, b, c);
            }
            let ridge = &self.ridges[layer];
            for x in 0..wide {
                let top = ridge[x as usize];
                for y in top..tall {
                    let colour = if y == top {
                        c.crests[layer]
                    } else {
                        c.hills[layer]
                    };
                    canvas.set(x, y, colour);
                }
                // hedgerows run along the nearer hills, with a tree standing up now and then
                if layer > 0 {
                    let h = hash(i64::from(x), layer as i64);
                    for y in top + 2..tall {
                        let row = y - top;
                        let on_hedge = row % (5 + layer as i32 * 2) == 0;
                        if on_hedge && hash(i64::from(x / 6), i64::from(y)) % 3 != 0 {
                            canvas.set(x, y, c.hedge);
                        }
                    }
                    if layer == 1 && h % 23 == 0 && top >= 2 {
                        canvas.fill_rect(x, top - 2, 2, 2, c.hedge);
                    }
                }
            }
        }
    }
}

/// A small deterministic hash, for the fixed details on the hills.
fn hash(a: i64, b: i64) -> u64 {
    let mut z = (a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (b as u64).wrapping_mul(0xD1B5_4A32_D192_ED03);
    z = (z ^ (z >> 29)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^ (z >> 32)
}

impl Scene for Balloons {
    fn name(&self) -> &'static str {
        "balloons"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let h = self.height as f32 * 2.0;
        let bottom = self.height as i32 * 2 - 1;
        let bases = [0.55, 0.72, 0.88];
        let amps = [0.12, 0.08, 0.05];
        for layer in 0..3 {
            let p1 = rng.range(0.0, TAU);
            let p2 = rng.range(0.0, TAU);
            let f = rng.range(0.05, 0.09) * (1.0 + layer as f32 * 0.3);
            self.ridges[layer] = (0..self.width)
                .map(|x| {
                    let x = x as f32;
                    let wave = (x * f + p1).sin() + 0.5 * (x * f * 2.3 + p2).sin();
                    // at least a couple of pixels of rise and fall, so a short screen still has
                    // hills
                    let amp = (amps[layer] * h).max(2.4);
                    (bases[layer] * h + amp * wave).round() as i32
                })
                .map(|y| y.clamp(2.min(bottom), bottom))
                .collect();
        }
        let wanted = self
            .motion
            .scaled(count_for(self.width, self.height, 200, 2, 16));
        self.balloons.clear();
        for _ in 0..wanted {
            let balloon = self.launch(rng, true);
            self.balloons.push(balloon);
        }
        let clouds = count_for(self.width, self.height, 500, 1, 8);
        let sky = (self.ridges[0].iter().copied().min().unwrap_or(4) - 4).max(3);
        self.clouds = (0..clouds)
            .map(|_| {
                let puffs = 3 + rng.below(3);
                let scale = if self.height >= 20 { 1.0 } else { 0.6 };
                Cloud {
                    x: rng.range(-12.0, self.width as f32),
                    y: rng.range(3.0, sky as f32),
                    puffs: (0..puffs)
                        .map(|i| {
                            let middle = 1.0 - (i as f32 / (puffs - 1) as f32 - 0.5).abs();
                            (
                                i as f32 * 3.2 * scale,
                                (1.5 + 2.2 * middle + rng.range(0.0, 0.8)) * scale,
                            )
                        })
                        .collect(),
                }
            })
            .collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        let width = self.width as f32;
        for i in 0..self.balloons.len() {
            let b = &mut self.balloons[i];
            b.y -= b.rise * dt;
            b.x += b.drift * dt;
            let (wide, tall) = b.size.extent();
            if b.y < -tall - 1.0 || b.x < -wide - 1.0 || b.x > width + 1.0 {
                self.balloons[i] = self.launch(rng, false);
            }
        }
        let wind = 0.2 * self.motion.speed();
        for cloud in &mut self.clouds {
            cloud.x = wrap(cloud.x + wind * dt + 20.0, width + 40.0) - 20.0;
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = SkyColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colour::xterm_colour;

    use crate::pixels::{IMAGERY_CEILING, distance};
    use crate::scenes::testing::{
        apart, check_determinism, check_pixel_scene, check_scene, pixels, render,
    };
    use crate::terminal::pixel_256;
    use crate::theme;

    fn drawn(sky: &Balloons, width: usize, height: usize, theme: &ThemeData) -> Frame {
        let mut frame = Frame::new(width, height);
        sky.draw(&mut frame, theme);
        frame
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("balloons");
        check_determinism("balloons");
        check_pixel_scene("balloons", 6.0);
    }

    #[test]
    fn balloons_rise_and_the_hills_stay_put() {
        let mut rng = Rng::new(5);
        let mut sky = Balloons::new(80, 24, Motion::LIVELY, &mut rng);
        let ridges = sky.ridges.clone();
        let before: Vec<f32> = sky.balloons.iter().map(|b| b.y).collect();
        sky.step(1.0, &mut rng);
        for (b, y) in sky.balloons.iter().zip(before) {
            assert!(b.y < y || b.y > 20.0);
        }
        // the fastest balloon climbs well under a row a second, so nothing darts about
        assert!(
            sky.balloons
                .iter()
                .all(|b| b.rise <= 1.0 * Motion::LIVELY.speed())
        );
        assert_eq!(sky.ridges, ridges);
    }

    #[test]
    fn every_size_fits_its_screen_and_reads_as_a_balloon() {
        let theme = theme::default_theme();
        let c = SkyColours::new(theme);
        let mut rng = Rng::new(9);
        let mut big = Balloons::new(100, 30, Motion::FULL, &mut rng);
        assert!(big.sizes().contains(&Size::Large));
        // one large balloon in open sky: striped, shaded and hanging a basket
        big.balloons = vec![Balloon {
            x: 40.0,
            y: 4.0,
            drift: 0.0,
            rise: 0.0,
            size: Size::Large,
            colour: 0,
        }];
        let frame = render(&mut big, 100, 30, 1);
        let all = pixels(&frame);
        let (main, gore) = c.envelopes[0];
        assert!(all.contains(&main) && all.contains(&gore));
        assert!(all.contains(&c.basket) && all.contains(&c.rope));
        let small = Balloons::new(20, 8, Motion::FULL, &mut rng);
        assert_eq!(small.sizes(), &[Size::Small]);
    }

    #[test]
    fn the_colours_stay_apart_and_in_their_bands_in_every_theme() {
        for theme in theme::all() {
            let c = SkyColours::new(theme);
            let name = theme.name;
            apart(
                theme,
                c.sky_top,
                c.sky_low,
                0.03,
                "the sky's top and bottom",
            );
            apart(theme, c.cloud, c.sky_top, 0.03, "the clouds and the sky");
            apart(theme, c.cloud, c.cloud_shade, 0.03, "a cloud and its shade");
            apart(
                theme,
                c.hills[0],
                c.hills[1],
                0.03,
                "the far and middle hills",
            );
            apart(
                theme,
                c.hills[1],
                c.hills[2],
                0.03,
                "the middle and near hills",
            );
            apart(
                theme,
                c.hills[0],
                c.sky_low,
                0.03,
                "the far hills and the sky",
            );
            for (hill, crest) in c.hills.iter().zip(c.crests) {
                apart(theme, *hill, crest, 0.02, "a hill and its crest");
            }
            let mut imagery = vec![
                ("sky top", c.sky_top),
                ("sky low", c.sky_low),
                ("cloud shade", c.cloud_shade),
            ];
            imagery.extend(c.hills.iter().map(|&h| ("hill", h)));
            imagery.extend(c.crests.iter().map(|&h| ("crest", h)));
            for (what, colour) in imagery {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            for (main, gore) in c.envelopes {
                assert!(theme.contrast_of(main) >= theme.floor(), "{name}");
                apart(theme, main, c.sky_top, 0.1, "a balloon and the sky");
                apart(theme, main, c.sky_low, 0.1, "a balloon and the low sky");
                apart(theme, main, c.cloud, 0.1, "a balloon and a cloud");
                apart(theme, main, gore, 0.06, "a balloon's gores");
            }
        }
    }

    #[test]
    fn the_256_colour_fallback_keeps_the_bands_apart() {
        for theme in theme::all() {
            let c = SkyColours::new(theme);
            let near = |colour: Rgb| xterm_colour(pixel_256(colour, theme.background));
            let mut rng = Rng::new(8);
            let mut sky = Balloons::new(80, 24, Motion::FULL, &mut rng);
            for _ in 0..60 {
                sky.step(0.05, &mut rng);
            }
            let frame = drawn(&sky, 80, 24, theme);
            let worst = pixels(&frame)
                .into_iter()
                .map(|colour| distance(colour, near(colour)))
                .fold(0.0_f32, f32::max);
            assert!(worst <= 0.15, "{} is {worst:.3} off", theme.name);
            assert_ne!(near(c.hills[1]), near(c.hills[2]), "{}", theme.name);
            assert_ne!(near(c.hills[0]), near(c.sky_low), "{}", theme.name);
            for (main, _) in c.envelopes {
                assert_ne!(near(main), near(c.sky_top), "{}", theme.name);
            }
        }
    }
}
