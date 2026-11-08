//! Blossom: a flowering branch reaching in from the corner of the sky, painted in solid pixels and
//! letting petals go on a spring breeze. The branch tapers as it reaches out and its twigs end in
//! clusters of small flowers. A small bird sits on it, looking one way and then the other now
//! and then. The petals tumble slowly down past far hills and a meadow to a stream that
//! carries them away.
//!
//! Every colour comes from the active theme. The sky, the hills, the meadow and the stream are
//! tints of the background held below the non-text floor, so they read as background; the
//! branch, the flowers, the petals and the bird keep the theme's own drawing colours, so they
//! stand out as shapes and keep their outline when colour is off.

use std::f32::consts::TAU;

use super::{count_for, cycle};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Lab, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "A flowering branch letting petals go on a breeze, down to a stream that carries them off, on a clear day in light themes";

pub const CAPTIONS: &[&str] = &[
    "Everything is in bloom.",
    "A fresh build, freshly green.",
    "Spring cleaning complete.",
    "Not a petal out of place.",
    "The build passed. Let it drift.",
];

/// Seconds a falling petal shows each side before it turns over.
pub const PETAL_TURN: f32 = 0.7;
/// Seconds the bird looks one way before it turns its head.
pub const BIRD_LOOK: f32 = 6.0;

/// A falling petal flat and then edge on as it turns over.
const TURNS: [&[(i32, i32)]; 2] = [&[(0, 0), (1, 0)], &[(0, 0)]];

/// The bird facing right: `b` its body, `w` its wing and tail, `e` its eye and `y` its beak.
const BIRD: [&str; 4] = ["...bb.", "..beby", "wwbbb.", "..bb.."];

/// Steps in the sky ramp.
const SKY_STEPS: usize = 10;

/// A small deterministic hash, for the fixed texture of the meadow and the bank.
fn hash(a: i64, b: i64) -> u64 {
    let mut z = (a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (b as u64).wrapping_mul(0xD1B5_4A32_D192_ED03);
    z = (z ^ (z >> 29)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^ (z >> 32)
}

/// Mirrors a sprite, for the bird looking left.
fn mirrored(rows: &[&str]) -> Vec<String> {
    rows.iter().map(|row| row.chars().rev().collect()).collect()
}

/// The spring colours in one theme, every one derived from the theme's own. Some are kept only
/// so the tests can check the bands against each other.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct SpringColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub hills: Rgb,
    pub meadow: Rgb,
    pub meadow_dark: Rgb,
    pub bank: Rgb,
    pub pebble: Rgb,
    pub water: Rgb,
    pub water_deep: Rgb,
    pub ripple: Rgb,
    pub bark: Rgb,
    pub bark_shade: Rgb,
    pub centre: Rgb,
    /// The petals' colours: deep pink, pink and white.
    pub petals: [Rgb; 3],
    /// The paler pink between a flower's petals, which rounds it off.
    pub blush: [Rgb; 3],
    pub bird: Rgb,
    pub wing: Rgb,
    pub beak: Rgb,
    pub eye: Rgb,
    sky: Ramp,
}

impl SpringColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        // a spring evening with a pink haze on a dark theme, a clear spring day on a light one
        let (sky_top, sky_low, hills) = if theme.dark {
            (
                shades.tint(theme.info, 0.18, 1.35),
                shades.tint(theme.bright_magenta(), 0.45, 2.1),
                shades.tint(theme.blue(), 0.32, 1.35),
            )
        } else {
            (
                shades.tint(theme.info, 0.38, 1.9),
                shades.tint(theme.magenta(), 0.08, 1.15),
                // strong enough to stay apart from the meadow in the 256 colour table on a warm
                // light theme, where every tint leans to the same beige grey
                shades.tint(theme.blue(), 0.45, 2.0),
            )
        };
        let (meadow, meadow_dark, bank, pebble) = if theme.dark {
            (
                shades.tint(theme.green(), 0.5, 2.05),
                shades.tint(theme.green(), 0.22, 1.5),
                shades.tint(theme.green(), 0.55, 2.6),
                shades.tint(theme.grey(), 0.6, 2.8),
            )
        } else {
            (
                shades.tint(theme.green(), 0.3, 1.6),
                shades.tint(theme.green(), 0.5, 2.1),
                shades.tint(theme.green(), 0.8, 2.85),
                shades.tint(theme.grey(), 0.3, 1.45),
            )
        };
        let (water, water_deep) = if theme.dark {
            (
                shades.tint(theme.cyan(), 0.4, 2.2),
                shades.tint(theme.blue(), 0.2, 1.4),
            )
        } else {
            (
                shades.tint(theme.cyan(), 0.3, 1.7),
                shades.tint(theme.blue(), 0.65, 2.7),
            )
        };
        let ripple = if theme.dark {
            mix(water, theme.bright_cyan(), 0.45)
        } else {
            mix(water, theme.background, 0.7)
        };
        let bark = if theme.dark {
            mix(theme.yellow(), theme.red(), 0.45)
        } else {
            theme.yellow()
        };
        let bird = theme.bright_cyan();
        // white blossom only shows up on a dark theme; on a light one the third shade is a
        // deeper red pink instead
        let petals = [
            theme.magenta(),
            theme.bright_magenta(),
            if theme.dark {
                theme.foreground
            } else {
                theme.bright_red()
            },
        ];
        // a light theme's magentas are deep enough to read as dried flowers, so its blossom is
        // a clear mid pink of the same hue, still solid enough to keep its outline
        let petals = if theme.dark {
            petals
        } else {
            petals.map(|p| pink(p, 0.6))
        };
        Self {
            sky_top,
            sky_low,
            hills,
            meadow,
            meadow_dark,
            bank,
            pebble,
            water,
            water_deep,
            ripple,
            bark,
            bark_shade: shades.darker(bark, 0.3),
            // a light theme's yellow is a dark amber that shows as a hole in a pink flower, so
            // its centres are a deeper pink instead
            centre: if theme.dark {
                theme.bright_yellow()
            } else {
                pink(theme.magenta(), 0.45)
            },
            petals,
            blush: if theme.dark {
                petals
            } else {
                petals.map(|p| pink(p, 0.72))
            },
            bird,
            wing: mix(bird, theme.blue(), 0.5),
            beak: theme.yellow(),
            eye: theme.background,
            sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
        }
    }
}

/// A clear pink with the hue of `petal` at OKLab lightness `l`, for blossom on a light theme.
fn pink(petal: Rgb, l: f32) -> Rgb {
    let lab = Lab::from_rgb(petal);
    let chroma = (lab.a * lab.a + lab.b * lab.b).sqrt().max(0.001);
    let scale = chroma.clamp(0.12, 0.17) / chroma;
    Lab {
        l,
        a: lab.a * scale,
        b: lab.b * scale,
    }
    .to_rgb()
}

#[derive(Clone, Debug)]
struct Petal {
    x: f32,
    y: f32,
    fall: f32,
    phase: f32,
    age: f32,
    colour: u8,
    /// Set once the petal has landed on the stream and is floating.
    floating: bool,
}

pub struct Blossom {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// The branch and its twigs as pixels with whether each is on the shaded underside.
    wood: Vec<(i32, i32, bool)>,
    /// Flower pixels with their colour: 0 to 2 a petal colour, 3 a flower's centre and 4 to 6
    /// the paler pink between the petals.
    flowers: Vec<(i32, i32, u8)>,
    petals: Vec<Petal>,
    next_petal: f32,
    /// The far hills' top pixel row in each column.
    hills: Vec<i32>,
    /// Where the bird perches: its left column and top row, in pixels.
    bird: Option<(i32, i32)>,
}

impl Blossom {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut scene = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            wood: Vec::new(),
            flowers: Vec::new(),
            petals: Vec::new(),
            next_petal: 0.0,
            hills: Vec::new(),
            bird: None,
        };
        scene.resize(width, height, rng);
        scene
    }

    /// The first row of the stream. On a short screen the stream is just the bottom row.
    fn stream(&self) -> i32 {
        if self.height >= 10 {
            self.height as i32 - 2
        } else {
            self.height as i32 - 1
        }
    }

    /// The stream's first pixel row.
    fn stream_px(&self) -> i32 {
        self.stream() * 2
    }

    fn turn_period(&self) -> f32 {
        PETAL_TURN / self.motion.pace()
    }

    fn look_period(&self) -> f32 {
        BIRD_LOOK / self.motion.pace()
    }

    /// The branch's top pixel row at a column.
    fn branch_row(x: f32, droop: f32) -> i32 {
        (2.0 + x * droop + (x * 0.12).sin() * 1.6).round() as i32
    }

    /// Adds a cluster of flowers around a twig's tip.
    fn bloom(&mut self, tx: i32, ty: i32, rng: &mut Rng) {
        let count = 1 + rng.below(3);
        for i in 0..count {
            let (fx, fy) = if i == 0 {
                (tx, ty)
            } else {
                (tx + rng.below(5) as i32 - 2, ty + rng.below(5) as i32 - 2)
            };
            let colour = rng.below(3) as u8;
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                self.flowers.push((fx + dx, fy + dy, colour));
            }
            // the gaps between the petals catch a little of the petal colour too, so a flower
            // reads as round
            for (dx, dy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                if rng.chance(0.5) {
                    self.flowers.push((fx + dx, fy + dy, colour + 4));
                }
            }
            self.flowers.push((fx, fy, 3));
        }
    }
}

impl Blossom {
    fn paint(&self, canvas: &mut Canvas, c: &SpringColours) {
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let stream = self.stream_px().min(tall);
        let horizon = self.hills.iter().copied().min().unwrap_or(stream).max(1);
        for y in 0..tall {
            canvas.fill_row(y, c.sky.at(y as f32 / horizon as f32));
        }

        // the far hills, then the meadow down to the bank
        if self.height >= 10 {
            for (x, &top) in self.hills.iter().enumerate() {
                let x = x as i32;
                for y in top..stream {
                    let colour = if y < top + 3 {
                        c.hills
                    } else if hash(i64::from(x / 2), i64::from(y)) % 11 == 0 {
                        c.meadow_dark
                    } else {
                        c.meadow
                    };
                    canvas.set(x, y, colour);
                }
            }
        }
        // the bank with pebbles along the water's edge
        if stream >= 2 {
            for x in 0..wide {
                let h = hash(i64::from(x), 5);
                canvas.set(x, stream - 1, c.bank);
                if self.height >= 10 {
                    canvas.set(x, stream - 2, if h % 3 == 0 { c.bank } else { c.meadow });
                }
                if h % 7 == 0 {
                    canvas.set(x, stream - 1, c.pebble);
                }
            }
        }
        // the stream runs to the right, its ripples drifting with it
        let drift = cycle(self.t, 0.8 * self.motion.speed() / 9.0 * TAU);
        let span = (tall - stream).max(1) as f32;
        for y in stream..tall {
            let depth = (y - stream) as f32 / span;
            let base = mix(c.water, c.water_deep, depth);
            for x in 0..wide {
                let v = (x as f32 / 9.0 * TAU - drift + y as f32 * 1.1).sin()
                    + 0.5 * (x as f32 / 4.0 * TAU - drift * 2.0 + y as f32 * 0.5).sin();
                let colour = if v > 1.0 {
                    c.ripple
                } else if v > 0.4 {
                    mix(base, c.ripple, 0.4)
                } else {
                    base
                };
                canvas.set(x, y, colour);
            }
        }

        for &(x, y, shaded) in &self.wood {
            canvas.set(x, y, if shaded { c.bark_shade } else { c.bark });
        }
        for &(x, y, colour) in &self.flowers {
            let paint = match colour {
                0..=2 => c.petals[colour as usize],
                3 => c.centre,
                _ => c.blush[(colour - 4) as usize % 3],
            };
            canvas.set(x, y, paint);
        }
        if let Some((x, y)) = self.bird {
            let looking_left = (self.t / f64::from(self.look_period())) as u64 % 2 == 0;
            let rows: Vec<String> = if looking_left {
                mirrored(&BIRD)
            } else {
                BIRD.iter().map(|r| r.to_string()).collect()
            };
            let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
            let palette = [('b', c.bird), ('w', c.wing), ('e', c.eye), ('y', c.beak)];
            canvas.sprite(x, y, &rows, &palette);
        }

        let period = self.turn_period();
        for petal in &self.petals {
            let paint = c.petals[petal.colour as usize % 3];
            let (x, y) = (petal.x.round() as i32, petal.y.floor() as i32);
            let shape = if petal.floating || self.motion.calm {
                TURNS[0]
            } else {
                TURNS[((petal.age / period) as usize + petal.colour as usize) % TURNS.len()]
            };
            for &(dx, dy) in shape {
                canvas.set(x + dx, y + dy, paint);
            }
        }
    }
}

impl Scene for Blossom {
    fn name(&self) -> &'static str {
        "blossom"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let reach = ((self.width as f32 * 0.68) as i32).max(4);
        let tall = self.height as f32 * 2.0;
        let stream = self.stream_px();
        // the branch droops more on a tall screen, so it fills the corner either way
        let droop = (tall * 0.3 / reach as f32).clamp(0.05, 0.4);
        self.wood.clear();
        self.flowers.clear();
        let bird_x = (reach as f32 * 0.45) as i32;
        self.bird = if self.width >= 30 && self.height >= 10 {
            Some((bird_x, Self::branch_row(bird_x as f32 + 3.0, droop) - 4))
        } else {
            None
        };

        // the branch tapers from three pixels thick at the corner to one at its tip
        let mut tips = Vec::new();
        let mut next_twig = 4 + rng.below(4) as i32;
        for x in 0..reach {
            let y = Self::branch_row(x as f32, droop);
            let thick = if x < reach / 3 {
                3
            } else if x < reach * 2 / 3 {
                2
            } else {
                1
            };
            for dy in 0..thick {
                self.wood.push((x, y + dy, dy == thick - 1 && thick > 1));
            }
            // fill the step when the branch drops, so the line stays joined
            let next = Self::branch_row(x as f32 + 1.0, droop);
            if next > y + thick {
                for dy in thick..next - y {
                    self.wood.push((x, y + dy, true));
                }
            }
            if x == next_twig && x + 3 < reach {
                next_twig = x + 5 + rng.below(5) as i32;
                if self.bird.is_some_and(|(bx, _)| (x - bx).abs() < 7) {
                    continue;
                }
                // a short twig up or down with flowers at its end, left out where they would
                // not fit above the stream
                let up = y >= 8 && rng.chance(0.5);
                let length = 3 + rng.below(3) as i32;
                let tip = if up {
                    (x + length, y - length)
                } else {
                    (x + length, y + thick + length)
                };
                if tip.1 < 2 || tip.1 > stream - 6 {
                    continue;
                }
                for i in 1..=length {
                    if up {
                        self.wood.push((x + i, y - i, false));
                    } else {
                        self.wood.push((x + i, y + thick - 1 + i, false));
                    }
                }
                tips.push(tip);
            }
            // flowers grow along the branch itself too
            if x > 3 && rng.chance(0.12) && y + thick + 1 < stream - 4 {
                tips.push((x, y - 1));
            }
        }
        let last = Self::branch_row(reach as f32, droop);
        tips.push((reach + 1, last));
        for (tx, ty) in tips {
            if ty >= 2 && ty <= stream - 5 {
                self.bloom(tx, ty, rng);
            }
        }

        let horizon = (tall * 0.6) as i32;
        let p = rng.range(0.0, TAU);
        self.hills = (0..self.width)
            .map(|x| horizon + ((x as f32 * 0.08 + p).sin() * 2.6).round() as i32)
            .collect();
        self.petals.clear();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        let stream = self.stream_px() as f32;
        let tall = self.height as f32 * 2.0;
        let gust = 0.8 + 0.5 * cycle(self.t, 0.09).sin();
        for petal in &mut self.petals {
            petal.age += dt;
            if petal.floating {
                petal.x += 1.0 * speed * dt;
                continue;
            }
            petal.y += petal.fall * dt;
            let sway = (petal.age * 1.6 + petal.phase).sin();
            petal.x += (gust + sway) * speed * dt;
            if petal.y >= stream {
                // a petal settles somewhere on the stream and floats off with it
                let depth = ((tall - stream - 1.0).max(0.0) * (petal.phase / TAU)).floor();
                petal.y = stream + depth;
                petal.floating = true;
            }
        }
        let width = self.width as f32;
        self.petals.retain(|p| p.x < width + 1.0);

        self.next_petal -= dt;
        if self.next_petal <= 0.0 {
            self.next_petal = if self.motion.calm {
                rng.range(1.6, 3.0)
            } else {
                rng.range(0.6, 1.4) / self.motion.busy()
            };
            // a sparse drift, so each petal can be followed on its way down
            let limit = count_for(self.width, self.height, 150, 2, 16);
            if self.petals.len() < limit && !self.flowers.is_empty() {
                let &(x, y, colour) = rng.pick(&self.flowers);
                self.petals.push(Petal {
                    x: x as f32,
                    y: y as f32,
                    fall: rng.range(1.0, 1.8) * speed,
                    phase: rng.range(0.0, TAU),
                    age: 0.0,
                    colour: colour.min(2),
                    floating: false,
                });
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = SpringColours::new(theme);
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

    fn drawn(scene: &Blossom, width: usize, height: usize, theme: &ThemeData) -> Frame {
        let mut frame = Frame::new(width, height);
        scene.draw(&mut frame, theme);
        frame
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("blossom");
        check_determinism("blossom");
        check_pixel_scene("blossom", 6.0);
    }

    #[test]
    fn petals_turn_slowly_and_the_bird_looks_rarely() {
        const { assert!(PETAL_TURN >= 1.0 / 3.0) };
        const { assert!(BIRD_LOOK >= 1.0 / 3.0) };
        for motion in Motion::ALL {
            let mut rng = Rng::new(1);
            let scene = Blossom::new(60, 20, motion, &mut rng);
            assert!(scene.turn_period() >= PETAL_TURN);
            assert!(scene.look_period() >= BIRD_LOOK);
        }
        assert!(BIRD.iter().all(|r| r.len() == BIRD[0].len()));
        assert_eq!(mirrored(&["ab."]), vec![".ba".to_string()]);
    }

    #[test]
    fn petals_fall_to_the_stream_and_float_away() {
        let theme = theme::default_theme();
        let c = SpringColours::new(theme);
        let mut rng = Rng::new(10);
        let mut scene = Blossom::new(80, 24, Motion::FULL, &mut rng);
        for _ in 0..(60 * 20) {
            scene.step(0.05, &mut rng);
        }
        assert!(scene.petals.iter().any(|p| p.floating));
        for petal in &scene.petals {
            assert!(petal.y <= 47.0);
            if petal.floating {
                assert!(petal.y >= scene.stream_px() as f32);
            }
        }
        let frame = render(&mut scene, 80, 24, 1);
        let all = pixels(&frame);
        for colour in [c.centre, c.bird, c.bark, c.ripple] {
            assert!(all.contains(&colour), "{}", colour.hex());
        }
        let small = render(&mut Blossom::new(20, 8, Motion::CALM, &mut rng), 20, 8, 20);
        assert!(pixels(&small).contains(&c.centre));
    }

    #[test]
    fn the_colours_stay_apart_and_in_their_bands_in_every_theme() {
        for theme in theme::all() {
            let c = SpringColours::new(theme);
            let name = theme.name;
            apart(
                theme,
                c.sky_top,
                c.sky_low,
                0.03,
                "the sky's top and bottom",
            );
            apart(theme, c.hills, c.sky_low, 0.03, "the hills and the sky");
            apart(theme, c.meadow, c.hills, 0.03, "the meadow and the hills");
            apart(theme, c.bank, c.meadow, 0.03, "the bank and the meadow");
            apart(theme, c.water, c.bank, 0.03, "the stream and its bank");
            apart(theme, c.water, c.ripple, 0.03, "the stream and its ripples");
            for (what, colour) in [
                ("sky top", c.sky_top),
                ("sky low", c.sky_low),
                ("hills", c.hills),
                ("meadow", c.meadow),
                ("bank", c.bank),
                ("water", c.water),
                ("deep water", c.water_deep),
            ] {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            // a light theme's blossom is a mid pink that keeps its outline, not the text floor
            for colour in c.petals.into_iter().chain([c.centre]) {
                let least = if theme.dark {
                    theme.floor()
                } else {
                    crate::scenes::paint::SOLID
                };
                assert!(theme.contrast_of(colour) >= least, "{name}");
            }
            for colour in [c.bark, c.bird] {
                assert!(theme.contrast_of(colour) >= theme.floor(), "{name}");
            }
            for colour in c.petals.iter().copied().chain([c.bark, c.centre, c.bird]) {
                apart(theme, colour, c.sky_top, 0.1, "a shape and the sky");
                apart(theme, colour, c.sky_low, 0.1, "a shape and the low sky");
                apart(theme, colour, c.water, 0.1, "a shape and the stream");
            }
            apart(
                theme,
                c.petals[2],
                c.petals[0],
                0.08,
                "the palest and deepest petals",
            );
            apart(
                theme,
                c.centre,
                c.petals[0],
                0.08,
                "a flower's centre and its petals",
            );
            apart(theme, c.bird, c.bark, 0.08, "the bird and the branch");
        }
    }

    #[test]
    fn the_256_colour_fallback_keeps_the_bands_apart() {
        for theme in theme::all() {
            let c = SpringColours::new(theme);
            let near = |colour: Rgb| xterm_colour(pixel_256(colour, theme.background));
            let mut rng = Rng::new(8);
            let mut scene = Blossom::new(80, 24, Motion::FULL, &mut rng);
            for _ in 0..60 {
                scene.step(0.05, &mut rng);
            }
            let frame = drawn(&scene, 80, 24, theme);
            let worst = pixels(&frame)
                .into_iter()
                .map(|colour| distance(colour, near(colour)))
                .fold(0.0_f32, f32::max);
            assert!(worst <= 0.15, "{} is {worst:.3} off", theme.name);
            assert_ne!(near(c.hills), near(c.sky_low), "{}", theme.name);
            assert_ne!(near(c.water), near(c.bank), "{}", theme.name);
            assert_ne!(near(c.meadow), near(c.hills), "{}", theme.name);
        }
    }
}
