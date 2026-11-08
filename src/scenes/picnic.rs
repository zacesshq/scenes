//! A picnic on a hill, painted in solid pixels: a sky with a still sun and clouds drifting
//! across it, hills fading into the distance, a grassy hill with a shady tree and a scatter of
//! wild flowers, a checked blanket with a wicker basket and kites flying overhead on long
//! strings, their tails rippling behind them.
//!
//! The sky, clouds, hills, grass, tree and the kite strings are tints of the background held
//! below the non-text floor, so they read as background. The kites, the blanket's checks and
//! the basket keep the theme's own drawing colours, so they stand out as shapes and keep their
//! outline with colour off.

use std::f32::consts::{PI, TAU};

use super::cycle;
use super::paint::{self, Cloud, Wave, stronger};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "A picnic blanket on a grassy hill, with kites flying overhead and clouds drifting by";

pub const CAPTIONS: &[&str] = &[
    "Clean build. Pack a lunch.",
    "Everything passed. Go fly a kite.",
    "Clear skies over the whole project.",
    "Nothing to fix but the sandwiches.",
    "A good day for a green build.",
];

/// How fast a kite drifts about on the wind, in radians a second at the standard speed.
pub(crate) const KITE_DRIFT: f32 = 0.35;
/// How fast the ripple runs down a kite's tail, in radians a second at the standard pace: one
/// ripple every four seconds or so.
pub(crate) const TAIL_RATE: f32 = 1.5;

#[derive(Clone, Debug, PartialEq)]
struct Kite {
    /// The middle of the kite's drift, in pixels.
    home: (f32, f32),
    reach: (f32, f32),
    rates: (f32, f32),
    phase: f32,
    colours: usize,
}

const SKY_STEPS: usize = 10;
const GRASS_STEPS: usize = 6;
const KITE_PAIRS: usize = 3;

/// The hillside's colours in one theme, every one derived from the theme's own.
/// Some are kept only so the tests can check the bands against each other.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct PicnicColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub sun: Rgb,
    pub halo: Rgb,
    pub cloud: Rgb,
    pub cloud_shade: Rgb,
    pub far_hill: Rgb,
    pub mid_hill: Rgb,
    pub grass_top: Rgb,
    pub grass_foot: Rgb,
    pub canopy: Rgb,
    pub canopy_lit: Rgb,
    pub trunk: Rgb,
    pub petals: [Rgb; 2],
    pub string: Rgb,
    pub check: Rgb,
    pub check_dark: Rgb,
    pub check_light: Rgb,
    pub basket: Rgb,
    pub weave: Rgb,
    pub apple: Rgb,
    pub kites: [(Rgb, Rgb); KITE_PAIRS],
    sky: Ramp,
    grass: Ramp,
}

impl PicnicColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let green = theme.green();
        let hazy = mix(green, theme.info, 0.45);
        let bark = mix(theme.yellow(), theme.red(), 0.5);
        let (sky_top, sky_low, sun, halo) = if theme.dark {
            (
                shades.tint(theme.info, 0.25, 1.5),
                shades.tint(theme.info, 0.55, 2.3),
                stronger(theme, theme.yellow(), 0.45),
                shades.tint(theme.warning, 0.6, 2.6),
            )
        } else {
            (
                shades.tint(theme.info, 0.45, 1.95),
                shades.tint(theme.info, 0.15, 1.2),
                theme.background,
                shades.tint(theme.warning, 0.12, 1.15),
            )
        };
        // on a dark theme the sunlit crest is the brightest grass; on a light one the foot of
        // the hill is the darkest
        let (far_hill, mid_hill, grass_top, grass_foot, canopy, canopy_lit, trunk) = if theme.dark {
            (
                shades.tint(hazy, 0.4, 1.75),
                shades.tint(green, 0.35, 1.4),
                shades.tint(green, 0.8, 2.7),
                shades.tint(green, 0.55, 2.0),
                shades.tint(green, 0.3, 1.25),
                shades.tint(green, 0.5, 1.7),
                shades.tint(bark, 0.4, 1.5),
            )
        } else {
            (
                shades.tint(hazy, 0.3, 1.45),
                shades.tint(green, 0.6, 2.15),
                shades.tint(green, 0.4, 1.65),
                shades.tint(green, 0.65, 2.45),
                shades.tint(green, 0.9, 2.95),
                shades.tint(green, 0.6, 2.4),
                shades.tint(bark, 0.7, 2.6),
            )
        };
        let (cloud, cloud_shade) = paint::cloud_tones(theme, 2.2);
        let string = if theme.dark {
            shades.tint(theme.bright(), 0.5, 2.5)
        } else {
            shades.tint(theme.muted, 0.5, 2.3)
        };
        let check = theme.red();
        let basket = theme.yellow();
        Self {
            sky_top,
            sky_low,
            sun,
            halo,
            cloud,
            cloud_shade,
            far_hill,
            mid_hill,
            grass_top,
            grass_foot,
            canopy,
            canopy_lit,
            trunk,
            petals: [
                shades.tint(theme.yellow(), 0.9, 2.9),
                shades.tint(theme.magenta(), 0.9, 2.9),
            ],
            string,
            check,
            check_dark: stronger(theme, check, 0.35),
            check_light: if theme.dark {
                shades.tint(check, 0.45, 2.2)
            } else {
                shades.tint(check, 0.18, 1.35)
            },
            basket,
            weave: stronger(theme, basket, 0.35),
            apple: theme.green(),
            kites: [
                (theme.red(), theme.yellow()),
                (theme.magenta(), theme.cyan()),
                (theme.bright_blue(), theme.yellow()),
            ],
            sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
            grass: Ramp::new(grass_top, grass_foot, GRASS_STEPS),
        }
    }
}

pub struct Picnic {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    kites: Vec<Kite>,
    clouds: Vec<Cloud>,
    far_line: Wave,
    mid_line: Wave,
}

impl Picnic {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut picnic = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            kites: Vec::new(),
            clouds: Vec::new(),
            far_line: Wave::new(rng, 15.0),
            mid_line: Wave::new(rng, 10.0),
        };
        picnic.resize(width, height, rng);
        picnic
    }

    fn tall(&self) -> f32 {
        (self.height * 2) as f32
    }

    fn scale(&self) -> f32 {
        (self.tall() / 48.0).clamp(0.5, 3.0)
    }

    /// The pixel row where the sky meets the distant hills.
    fn horizon(&self) -> f32 {
        (self.tall() * 0.58).round().max(1.0)
    }

    /// The top of the near hill in a column: a broad dome with its crest right of centre.
    fn hill(&self, x: f32) -> f32 {
        let h = self.tall();
        let w = (self.width as f32).max(1.0);
        let u = (x - w * 0.62) / w;
        h * 0.62 + h * 0.55 * u * u
    }

    /// The blanket's back left corner and its size in pixels: width, depth and the slant of
    /// each row.
    fn blanket(&self) -> Option<(f32, f32, f32, f32)> {
        let h = self.tall();
        let w = self.width as f32;
        if w < 20.0 || h < 16.0 {
            return None;
        }
        let wide = (w * 0.22).max(8.0);
        let deep = (h * 0.1).max(3.0).round();
        let x = w * 0.55 - wide / 2.0;
        let y = self.hill(w * 0.55) + h * 0.1;
        Some((x, y.round(), wide.round(), deep))
    }

    /// Where every kite string is held: the front right corner of the blanket.
    fn anchor(&self) -> (f32, f32) {
        match self.blanket() {
            Some((x, y, wide, deep)) => (x + wide + deep * 0.8, y + deep - 1.0),
            None => (self.width as f32 * 0.6, self.tall()),
        }
    }

    fn kite_at(&self, kite: &Kite) -> (f32, f32) {
        let speed = self.motion.speed();
        let ax = cycle(self.t, kite.rates.0 * speed) + kite.phase;
        let ay = cycle(self.t, kite.rates.1 * speed) + kite.phase * 1.3;
        (
            kite.home.0 + kite.reach.0 * ax.sin(),
            kite.home.1 + kite.reach.1 * ay.sin(),
        )
    }

    /// The kite's half width and half height.
    fn kite_size(&self) -> (f32, f32) {
        let s = self.scale();
        ((2.6 * s).max(2.0), (3.6 * s).max(2.5))
    }

    fn paint_land(&self, canvas: &mut Canvas, c: &PicnicColours) {
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let h = self.tall();
        let horizon = self.horizon();
        for x in 0..wide {
            let xf = x as f32;
            let far = horizon - (2.0 + h * 0.08) * (0.5 + 0.5 * self.far_line.at(xf));
            let mid = horizon - (1.0 + h * 0.04) * (0.5 + 0.5 * self.mid_line.at(xf));
            for y in far.round() as i32..tall {
                canvas.set(x, y, c.far_hill);
            }
            for y in mid.round() as i32..tall {
                canvas.set(x, y, c.mid_hill);
            }
            // the near hill, lit at its crest and deepening down its slope
            let top = self.hill(xf).round() as i32;
            let span = (tall - top).max(1) as f32;
            for y in top..tall {
                let t = (y - top) as f32 / span;
                let n = paint::hash(x, y, 37);
                let colour = match n % 40 {
                    0 if t > 0.15 => c.petals[0],
                    1 if t > 0.15 => c.petals[1],
                    2..=6 => c.grass.at(t + 0.25),
                    _ => c.grass.at(t),
                };
                canvas.set(x, y, colour);
            }
        }

        // a broad tree to the left, its canopy lit from the right where the sun is
        if wide >= 24 && h >= 16.0 {
            let tx = wide as f32 * 0.16;
            let ground = self.hill(tx);
            let r = (h * 0.11).max(2.5);
            let trunk_top = ground - r * 1.4;
            let half = (r * 0.18).max(0.6);
            for y in trunk_top.round() as i32..=ground.round() as i32 + 1 {
                for x in (tx - half).round() as i32..=(tx + half).round() as i32 {
                    canvas.set(x, y, c.trunk);
                }
            }
            let cy = trunk_top - r * 0.5;
            for (dx, dy, k) in [
                (-0.7, 0.2, 0.75),
                (0.7, 0.25, 0.75),
                (0.0, -0.35, 0.85),
                (0.0, 0.3, 0.8),
            ] {
                paint::ellipse(canvas, tx + dx * r, cy + dy * r, r * k, r * k, c.canopy);
            }
            for (dx, dy, k) in [(0.45, -0.35, 0.5), (0.85, 0.05, 0.4)] {
                paint::ellipse(canvas, tx + dx * r, cy + dy * r, r * k, r * k, c.canopy_lit);
            }
        }
    }

    fn paint_blanket(&self, canvas: &mut Canvas, c: &PicnicColours) {
        let Some((x0, y0, wide, deep)) = self.blanket() else {
            return;
        };
        let cell = (wide / 9.0).round().max(1.0) as i32;
        for j in 0..deep as i32 {
            // each row further forward sits a little to the right, so the blanket lies flat on
            // the slope in perspective
            let start = (x0 + j as f32 * 0.8).round() as i32;
            for i in 0..wide as i32 {
                let (a, b) = ((i / cell) % 2 == 1, (j / cell) % 2 == 1);
                let colour = match (a, b) {
                    (true, true) => c.check_dark,
                    (false, false) => c.check_light,
                    _ => c.check,
                };
                canvas.set(start + i, y0 as i32 + j, colour);
            }
        }
        // the basket on the left of the blanket, its handle arching over the lid
        let bw = (wide * 0.3).round().max(4.0);
        let bh = (deep * 0.9).round().max(3.0);
        let bx = (x0 + deep * 0.4 + 1.0).round() as i32;
        let by = (y0 + deep * 0.6).round() as i32;
        for y in 0..bh as i32 {
            for x in 0..bw as i32 {
                let colour = if y == 0 || (x + y) % 2 == 0 {
                    c.weave
                } else {
                    c.basket
                };
                canvas.set(bx + x, by - y, colour);
            }
        }
        let (hx, hy) = (bx as f32 + bw / 2.0, (by - bh as i32) as f32 + 0.5);
        let (ra, rb) = (bw / 2.0 - 0.5, (bh * 0.7).max(1.5));
        for i in 0..=16 {
            let a = PI * i as f32 / 16.0;
            canvas.set(
                (hx + ra * a.cos() - 0.5).round() as i32,
                (hy - rb * a.sin()).round() as i32,
                c.basket,
            );
        }
        // two apples beside the basket
        let ax = bx as f32 + bw + 2.0;
        let ay = by as f32 - 0.5;
        paint::ellipse(canvas, ax, ay, 1.0, 1.0, c.apple);
        paint::ellipse(canvas, ax + 2.5, ay + 0.5, 1.0, 1.0, c.apple);
    }

    fn paint_kite(&self, canvas: &mut Canvas, c: &PicnicColours, kite: &Kite) {
        let (kx, ky) = self.kite_at(kite);
        let (a, b) = self.kite_size();
        let (one, two) = c.kites[kite.colours % KITE_PAIRS];
        let (anchor_x, anchor_y) = self.anchor();

        // the string leaves from the middle of the kite behind it, sagging a little on its
        // way down to the blanket
        let (sx, sy) = (kx, ky);
        let sag = ((anchor_x - sx).abs() + (anchor_y - sy).abs()) * 0.08;
        let segments = 24;
        let mut last = (sx, sy);
        for i in 1..=segments {
            let t = i as f32 / segments as f32;
            let p = (
                sx + (anchor_x - sx) * t,
                sy + (anchor_y - sy) * t + sag * (PI * t).sin(),
            );
            paint::line(canvas, (last.0, last.1), (p.0, p.1), 1, c.string);
            last = p;
        }

        // the tail streams away from the string and ripples, the ripple running from the kite
        // to the tip
        let ripple = cycle(self.t, TAIL_RATE * self.motion.pace());
        let swing = if self.motion.calm { 0.8 } else { 1.4 };
        let length = (b * 2.6).round() as i32;
        let away = if anchor_x > kx { -1.0 } else { 1.0 };
        for i in 0..length {
            let along = i as f32 / length.max(1) as f32;
            let x = kx - 0.5
                + away * along * b
                + (i as f32 * 0.5 - ripple + kite.phase).sin() * swing * along;
            let y = ky + b + i as f32 * 0.9;
            canvas.set(x.round() as i32, y.round() as i32, c.string);
            if i % 3 == 2 {
                let bow = if (i / 3) % 2 == 0 { one } else { two };
                canvas.set(x.round() as i32 - 1, y.round() as i32, bow);
                canvas.set(x.round() as i32 + 1, y.round() as i32, bow);
            }
        }

        // a diamond in four panels, opposite panels the same colour
        let (x0, x1) = ((kx - a).floor() as i32, (kx + a).ceil() as i32);
        let (y0, y1) = ((ky - b).floor() as i32, (ky + b).ceil() as i32);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = x as f32 + 0.5 - kx;
                let dy = y as f32 + 0.5 - ky;
                if dx.abs() / a + dy.abs() / b > 1.0 {
                    continue;
                }
                let colour = if (dx < 0.0) == (dy < 0.0) { one } else { two };
                canvas.set(x, y, colour);
            }
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &PicnicColours) {
        let horizon = self.horizon().round() as i32;
        let wide = canvas.width() as i32;
        let h = self.tall();
        paint::gradient(canvas, &c.sky, 0, horizon);
        // a still sun high on the right: it never changes, so it can never flash
        if horizon >= 10 && wide >= 20 {
            let (sx, sy) = (wide as f32 * 0.86, h * 0.13);
            let r = (h * 0.06).clamp(1.5, 5.0);
            paint::ellipse(canvas, sx, sy, r + 2.0, r + 2.0, c.halo);
            paint::ellipse(canvas, sx, sy, r, r, c.sun);
        }
        for cloud in &self.clouds {
            cloud.paint(canvas, c.cloud, c.cloud_shade);
        }
        self.paint_land(canvas, c);
        self.paint_blanket(canvas, c);
        for kite in &self.kites {
            self.paint_kite(canvas, c, kite);
        }
    }
}

impl Scene for Picnic {
    fn name(&self) -> &'static str {
        "picnic"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let h = self.tall();
        let wide = self.width as f32;

        self.kites.clear();
        if h >= 16.0 && wide >= 20.0 {
            let count = self.motion.scaled((self.width / 30).clamp(1, 3)).min(4);
            for i in 0..count {
                let spread = (i as f32 + 0.5) / count as f32;
                self.kites.push(Kite {
                    home: (
                        wide * (0.2 + 0.6 * spread) + rng.range(-3.0, 3.0),
                        h * rng.range(0.16, 0.3),
                    ),
                    reach: (rng.range(2.0, 4.0), rng.range(1.0, 2.0)),
                    rates: (
                        KITE_DRIFT * rng.range(0.8, 1.2),
                        KITE_DRIFT * rng.range(1.2, 1.7),
                    ),
                    phase: rng.range(0.0, TAU),
                    colours: i % KITE_PAIRS,
                });
            }
        }

        self.clouds.clear();
        if h >= 16.0 {
            let count = (self.width / 35).clamp(1, 4);
            for _ in 0..count {
                self.clouds.push(Cloud::random(
                    rng,
                    wide,
                    (h * 0.1, h * 0.4),
                    (8.0, 18.0),
                    self.motion.speed(),
                ));
            }
        }
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
        let wide = self.width as f32;
        for cloud in &mut self.clouds {
            cloud.drift(dt, wide);
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = PicnicColours::new(theme);
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
        assert_apart, assert_imagery, assert_shapes, check_determinism, check_pixel_scene,
        check_scene, pixels,
    };
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("picnic");
        check_determinism("picnic");
    }

    #[test]
    fn passes_the_pixel_checks_in_every_theme() {
        check_pixel_scene("picnic", 6.0);
    }

    #[test]
    fn the_colours_hold_in_every_theme() {
        for theme in theme::all() {
            let c = PicnicColours::new(theme);
            assert_imagery(
                theme,
                &[
                    ("sky top", c.sky_top),
                    ("sky low", c.sky_low),
                    ("halo", c.halo),
                    ("cloud", c.cloud),
                    ("cloud shade", c.cloud_shade),
                    ("far hill", c.far_hill),
                    ("middle hill", c.mid_hill),
                    ("grass at the crest", c.grass_top),
                    ("grass at the foot", c.grass_foot),
                    ("canopy", c.canopy),
                    ("lit canopy", c.canopy_lit),
                    ("trunk", c.trunk),
                    ("yellow flowers", c.petals[0]),
                    ("pink flowers", c.petals[1]),
                    ("string", c.string),
                    ("light check", c.check_light),
                ],
            );
            let mut shapes = vec![
                ("check", c.check),
                ("dark check", c.check_dark),
                ("basket", c.basket),
                ("apple", c.apple),
            ];
            for &(one, two) in &c.kites {
                shapes.push(("kite", one));
                shapes.push(("kite", two));
            }
            assert_shapes(theme, &shapes, &[c.sky_top, c.sky_low, c.grass_top]);
            assert_apart(
                theme,
                &[
                    ("the sky and the far hill", c.sky_low, c.far_hill),
                    ("the far and middle hills", c.far_hill, c.mid_hill),
                    ("the middle hill and the grass", c.mid_hill, c.grass_top),
                    ("the cloud and the sky", c.cloud, c.sky_top),
                    ("the canopy and the grass", c.canopy, c.grass_top),
                    ("the checks", c.check, c.check_light),
                ],
            );
            for &(one, two) in &c.kites {
                assert!(
                    distance(one, two) >= 0.08,
                    "a kite is one colour in {}",
                    theme.name
                );
            }
            assert!(distance(c.basket, c.weave) >= 0.03, "{}", theme.name);
        }
    }

    #[test]
    fn kites_stay_in_the_sky_on_their_strings() {
        let mut rng = Rng::new(31);
        let mut picnic = Picnic::new(100, 30, Motion::LIVELY, &mut rng);
        assert!(picnic.kites.len() >= 2);
        let (_, b) = picnic.kite_size();
        let (_, anchor_y) = picnic.anchor();
        for _ in 0..(120 * 20) {
            picnic.step(0.05, &mut rng);
            for kite in &picnic.kites {
                let (x, y) = picnic.kite_at(kite);
                assert!(y - b >= 0.0, "a kite left the top at {y}");
                assert!(y + b < picnic.horizon(), "a kite sank to {y}");
                assert!(y < anchor_y);
                assert!(x > 0.0 && x < 100.0);
            }
        }
        let theme = theme::default_theme();
        let c = PicnicColours::new(theme);
        let mut frame = Frame::new(100, 30);
        picnic.draw(&mut frame, theme);
        let all = pixels(&frame);
        for (what, colour) in [
            ("check", c.check),
            ("basket", c.basket),
            ("string", c.string),
            ("kite", c.kites[0].0),
        ] {
            assert!(all.contains(&colour), "no {what}");
        }
    }
}
