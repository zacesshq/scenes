//! Rock pools at low tide, painted in solid pixels: a pale sky, the sea drawn back to a strip
//! along the top with foam at its edge, wet sand and shaded rocks fringed with weed. Pools in
//! the rocks hold water with light shimmering over it. Anemones wave their tentacles in the
//! pools, starfish rest on the bottom and crabs pick their way sideways across the sand.
//!
//! The sky, sea, sand, rocks, weed and pool water are tints of the background held below the
//! non-text floor, so they read as background. The crabs, anemones and starfish keep the
//! theme's own drawing colours, so they stand out as shapes and keep their outline with colour
//! off.

use std::f32::consts::{PI, TAU};

use super::paint::{self, Wave, stronger};
use super::{count_for, cycle};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "Rock pools at low tide with crabs on the sand, anemones and starfish and a gentle shimmer";

pub const CAPTIONS: &[&str] = &[
    "Low tide, no errors.",
    "Every pool clear to the bottom.",
    "The crabs inspected it. All fine.",
    "Calm water, clean build.",
    "Nothing washed up but a passing build.",
];

/// How fast the light shimmers over the pools, in radians a second at the standard pace.
pub(crate) const SHIMMER_RATE: f32 = 0.9;
/// How fast anemone tentacles wave, in radians a second at the standard pace: one wave takes
/// about five seconds.
pub(crate) const WAVE_RATE: f32 = 1.2;
/// How long a crab holds each leg position while it walks, in seconds at the standard pace,
/// so a leg never changes more than two and a half times a second.
pub(crate) const CRAB_STEP: f32 = 0.4;

/// A crab, legs one way, then the other: claws raised, eyes on stalks, a wide shell.
const CRAB: [[&str; 4]; 2] = [
    ["c.e...e.c", "cc.bbb.cc", ".bbbbbbb.", "l.l...l.l"],
    ["c.e...e.c", "cc.bbb.cc", ".bbbbbbb.", ".l.l.l.l."],
];
const CRAB_SMALL: [[&str; 3]; 2] = [["cebec", ".bbb.", "l.l.l"], ["cebec", ".bbb.", ".l.l."]];

/// What lies under each pixel, worked out once per size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ground {
    Sky,
    Sea,
    Foam,
    Sand,
    /// A rock pixel: 0 for a lit face, 1 for the body, 2 for shade.
    Rock(u8),
    Weed,
    /// Pool water, from 0 at the rim to 4 at the deepest.
    Pool(u8),
}

#[derive(Clone, Debug, PartialEq)]
struct Crab {
    x: f32,
    /// The pixel row of the crab's top.
    y: i32,
    /// Pixels per second, negative when walking left.
    speed: f32,
    /// Seconds left walking.
    walk_left: f32,
    rest_left: f32,
    stride: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct Anemone {
    x: f32,
    y: f32,
    colour: usize,
    phase: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct Starfish {
    x: f32,
    y: f32,
    turn: f32,
}

const SKY_STEPS: usize = 6;
const POOL_STEPS: usize = 5;

/// The shore's colours in one theme, every one derived from the theme's own.
/// Some are kept only so the tests can check the bands against each other.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct PoolColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub sea: Rgb,
    pub sea_light: Rgb,
    pub foam: Rgb,
    pub sand: Rgb,
    pub grain: Rgb,
    pub rock: [Rgb; 3],
    pub weed: Rgb,
    pub shallow: Rgb,
    pub deep: Rgb,
    pub glint: Rgb,
    pub crab: Rgb,
    pub claw: Rgb,
    pub eye: Rgb,
    pub anemones: [Rgb; 2],
    pub tips: [Rgb; 2],
    pub starfish: Rgb,
    sky: Ramp,
    pool: Ramp,
}

impl PoolColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let stone = mix(theme.muted, theme.blue(), 0.2);
        let (sky_top, sky_low, sea, sea_light, sand, rock, weed, shallow, deep) = if theme.dark {
            (
                shades.tint(theme.info, 0.2, 1.4),
                shades.tint(theme.info, 0.6, 2.3),
                shades.tint(theme.blue(), 0.5, 1.45),
                shades.tint(theme.cyan(), 0.5, 2.3),
                shades.tint(theme.yellow(), 0.4, 1.75),
                [
                    shades.tint(stone, 0.6, 2.75),
                    shades.tint(stone, 0.45, 2.15),
                    shades.tint(stone, 0.3, 1.5),
                ],
                shades.tint(theme.green(), 0.5, 1.7),
                shades.tint(theme.cyan(), 0.5, 1.75),
                shades.tint(theme.blue(), 0.55, 1.3),
            )
        } else {
            (
                shades.tint(theme.info, 0.4, 1.8),
                shades.tint(theme.info, 0.15, 1.2),
                shades.tint(theme.blue(), 0.45, 2.1),
                shades.tint(theme.cyan(), 0.25, 1.45),
                shades.tint(theme.yellow(), 0.22, 1.3),
                [
                    shades.tint(stone, 0.3, 1.55),
                    shades.tint(stone, 0.45, 2.05),
                    shades.tint(stone, 0.65, 2.7),
                ],
                shades.tint(theme.green(), 0.6, 2.3),
                shades.tint(theme.cyan(), 0.25, 1.4),
                shades.tint(theme.blue(), 0.5, 2.2),
            )
        };
        let (foam, glint) = if theme.dark {
            (
                shades.tint(theme.bright(), 0.6, 2.8),
                shades.tint(theme.bright(), 0.6, 2.9),
            )
        } else {
            (theme.background, theme.background)
        };
        let crab = theme.red();
        let anemones = [theme.magenta(), theme.cyan()];
        Self {
            sky_top,
            sky_low,
            sea,
            sea_light,
            foam,
            sand,
            grain: mix(sand, rock[1], 0.45),
            rock,
            weed,
            shallow,
            deep,
            glint,
            crab,
            claw: stronger(theme, crab, 0.3),
            eye: theme.foreground,
            anemones,
            tips: [
                stronger(theme, anemones[0], 0.4),
                stronger(theme, anemones[1], 0.4),
            ],
            starfish: theme.yellow(),
            sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
            pool: Ramp::new(shallow, deep, POOL_STEPS),
        }
    }
}

pub struct TidePools {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    ground: Vec<Ground>,
    crabs: Vec<Crab>,
    anemones: Vec<Anemone>,
    starfish: Vec<Starfish>,
    shore: Wave,
}

impl TidePools {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut pools = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            ground: Vec::new(),
            crabs: Vec::new(),
            anemones: Vec::new(),
            starfish: Vec::new(),
            shore: Wave::new(rng, 7.0),
        };
        pools.resize(width, height, rng);
        pools
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    fn scale(&self) -> f32 {
        (self.tall() as f32 / 48.0).clamp(0.5, 3.0)
    }

    /// The pixel row where the sky meets the sea.
    fn horizon(&self) -> i32 {
        ((self.tall() as f32 * 0.14).round() as i32).max(1)
    }

    /// The pixel row of the water's edge in a column.
    fn edge(&self, x: i32) -> i32 {
        let base = self.tall() as f32 * 0.27;
        (base + 1.2 * self.shore.at(x as f32)).round() as i32
    }

    fn ground_at(&self, x: i32, y: i32) -> Option<Ground> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.tall() {
            return None;
        }
        Some(self.ground[y as usize * self.width + x as usize])
    }

    /// Whether a crab whose top is at `y` may stand at `x`: every foot on sand or rock.
    fn crab_can_stand(&self, x: i32, y: i32, wide: i32, tall: i32) -> bool {
        (0..wide).all(|dx| {
            matches!(
                self.ground_at(x + dx, y + tall - 1),
                Some(Ground::Sand | Ground::Rock(_) | Ground::Weed) | None
            )
        })
    }

    fn crab_rows(&self) -> &'static [[&'static str; 4]; 2] {
        &CRAB
    }

    fn crab_size(&self) -> (i32, i32) {
        if self.scale() >= 0.8 { (9, 4) } else { (5, 3) }
    }

    /// Works out the ground under every pixel: sky, sea, foam, sand and the rocks with their
    /// pools, placed at random.
    fn lay_out(&mut self, rng: &mut Rng) {
        let (w, h) = (self.width as i32, self.tall());
        let scale = self.scale();
        let mut ground = vec![Ground::Sand; (w * h) as usize];
        let horizon = self.horizon();
        for x in 0..w {
            let edge = self.edge(x);
            for y in 0..h.min(edge + 1) {
                ground[(y * w + x) as usize] = if y < horizon {
                    Ground::Sky
                } else if y >= edge - 1 {
                    Ground::Foam
                } else {
                    Ground::Sea
                };
            }
        }
        let top = self.edge(0) + 3;
        let count = count_for(self.width, self.height, 380, 2, 14);
        let rocks: Vec<(f32, f32, f32, f32)> = (0..count)
            .map(|_| {
                let rx = rng.range(6.0, 15.0) * scale;
                (
                    rng.range(0.0, w as f32),
                    rng.range(top as f32 + rx * 0.3, h as f32 + 2.0),
                    rx,
                    rx * rng.range(0.45, 0.6),
                )
            })
            .collect();
        let bounds = |cx: f32, cy: f32, rx: f32, ry: f32| {
            let xs = ((cx - rx - 2.0) as i32).max(0)..=((cx + rx + 2.0) as i32).min(w - 1);
            let ys = ((cy - ry - 2.0) as i32).max(top)..=((cy + ry + 2.0) as i32).min(h - 1);
            (xs, ys)
        };
        for (i, &(cx, cy, rx, ry)) in rocks.iter().enumerate() {
            let (xs, ys) = bounds(cx, cy, rx, ry);
            for y in ys {
                for x in xs.clone() {
                    let (dx, dy) = ((x as f32 + 0.5 - cx) / rx, (y as f32 + 0.5 - cy) / ry);
                    let wobble = paint::unit(x / 2, y / 2, 17 + i as u32) * 0.16 - 0.08;
                    let field = 1.0 - (dx * dx + dy * dy) + wobble;
                    if field <= 0.0 {
                        continue;
                    }
                    // the light comes from the upper left, so faces turned that way are lit
                    let facing = -(dx + dy) + paint::unit(x, y, 5) * 0.4 - 0.2;
                    ground[(y * w + x) as usize] = Ground::Rock(if field < 0.12 && dy > 0.0 {
                        2
                    } else if facing > 0.45 {
                        0
                    } else if facing < -0.55 {
                        2
                    } else {
                        1
                    });
                }
            }
        }
        // the pools go in once every rock is down, so a later rock never covers one; they only
        // fill where there is rock to hold the water
        self.anemones.clear();
        self.starfish.clear();
        for (i, &(cx, cy, rx, ry)) in rocks.iter().enumerate() {
            if rx < 7.0 * scale.min(1.0) {
                continue;
            }
            let (px, py) = (cx + rng.range(-0.15, 0.15) * rx, cy - ry * 0.05);
            let (prx, pry) = (rx * 0.58, ry * 0.5);
            let (xs, ys) = bounds(px, py, prx, pry);
            for y in ys {
                for x in xs.clone() {
                    let index = (y * w + x) as usize;
                    if !matches!(ground[index], Ground::Rock(_) | Ground::Pool(_)) {
                        continue;
                    }
                    let wobble = paint::unit(x / 2, y / 2, 31 + i as u32) * 0.1 - 0.05;
                    let (qx, qy) = ((x as f32 + 0.5 - px) / prx, (y as f32 + 0.5 - py) / pry);
                    let wet = 1.0 - (qx * qx + qy * qy) + wobble;
                    if wet > 0.0 {
                        let depth = ((wet * 1.6).min(1.0) * (POOL_STEPS - 1) as f32).round();
                        ground[index] = Ground::Pool(depth as u8);
                    } else if wet > -0.22
                        && paint::unit(x, y, 23) < 0.6
                        && matches!(ground[index], Ground::Rock(_))
                    {
                        ground[index] = Ground::Weed;
                    }
                }
            }
            // a small pool holds one anemone; a larger one two, with a starfish between them
            let roomy = prx >= 7.0 * scale;
            for k in 0..(if roomy { 2 } else { 1 }) {
                let side = if k == 0 { -1.0 } else { 1.0 };
                self.anemones.push(Anemone {
                    x: px + side * prx * rng.range(0.45, 0.6),
                    y: py + pry * rng.range(0.1, 0.35),
                    colour: rng.below(2),
                    phase: rng.range(0.0, TAU),
                });
            }
            if roomy && rng.chance(0.7) {
                self.starfish.push(Starfish {
                    x: px + prx * rng.range(-0.1, 0.1),
                    y: py - pry * rng.range(0.0, 0.2),
                    turn: rng.range(0.0, TAU),
                });
            }
        }
        self.ground = ground;
        // anything whose pool a neighbouring rock cut short is left out
        let in_pool = |x: f32, y: f32, ground: &[Ground]| {
            let (x, y) = (x as i32, y as i32);
            x >= 0
                && y >= 0
                && x < w
                && y < h
                && matches!(ground[(y * w + x) as usize], Ground::Pool(_))
        };
        let ground = std::mem::take(&mut self.ground);
        self.anemones.retain(|a| in_pool(a.x, a.y, &ground));
        self.starfish.retain(|s| in_pool(s.x, s.y, &ground));
        self.ground = ground;
    }

    fn paint_anemone(&self, canvas: &mut Canvas, c: &PoolColours, a: &Anemone) {
        let scale = self.scale();
        let body = c.anemones[a.colour % 2];
        let tip = c.tips[a.colour % 2];
        let calm = if self.motion.calm { 0.6 } else { 1.0 };
        let wave = cycle(self.t, WAVE_RATE * self.motion.pace());
        let arms = if scale >= 0.8 { 5 } else { 3 };
        let reach = (2.4 * scale).max(1.8);
        for i in 0..arms {
            // tentacles fan out over the top half, each swaying a little out of step; only
            // their outer part is drawn, so they read as separate strands
            let base = PI + PI * (i as f32 + 0.5) / arms as f32;
            let angle = base + (wave + a.phase + i as f32 * 0.7).sin() * 0.22 * calm;
            let (ux, uy) = (angle.cos(), angle.sin());
            let start = reach * 0.45;
            paint::line(
                canvas,
                (a.x + start * ux, a.y + start * uy),
                (a.x + reach * ux, a.y + reach * uy),
                1,
                body,
            );
            canvas.set(
                (a.x + reach * ux).floor() as i32,
                (a.y + reach * uy).floor() as i32,
                tip,
            );
        }
        paint::ellipse(canvas, a.x, a.y + 0.5, (1.2 * scale).max(1.0), 1.0, body);
    }

    fn paint_starfish(&self, canvas: &mut Canvas, c: &PoolColours, s: &Starfish) {
        let r = (3.0 * self.scale()).max(1.8);
        let (x0, x1) = ((s.x - r - 1.0) as i32, (s.x + r + 1.0) as i32);
        let (y0, y1) = ((s.y - r - 1.0) as i32, (s.y + r + 1.0) as i32);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (dx, dy) = (x as f32 + 0.5 - s.x, y as f32 + 0.5 - s.y);
                let lobe = (1.0 + (5.0 * (dy.atan2(dx) - s.turn)).cos()) / 2.0;
                if (dx * dx + dy * dy).sqrt() <= r * (0.38 + 0.62 * lobe * lobe) {
                    canvas.set(x, y, c.starfish);
                }
            }
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &PoolColours) {
        let (w, h) = (self.width as i32, self.tall());
        if self.ground.len() != (w * h) as usize {
            return;
        }
        let pace = self.motion.pace();
        let phase = cycle(self.t, SHIMMER_RATE * pace);
        let (glint, gleam) = if self.motion.calm {
            (2.35, 1.7)
        } else {
            (2.15, 1.45)
        };
        let horizon = self.horizon().max(1);
        for y in 0..h {
            for x in 0..w {
                let (xf, yf) = (x as f32, y as f32);
                let colour = match self.ground[(y * w + x) as usize] {
                    Ground::Sky => c.sky.at(yf / horizon as f32),
                    Ground::Sea => {
                        let v = (xf * 0.3 + yf * 1.7 - phase).sin()
                            + 0.6 * (xf * 0.13 - yf + phase * 0.7).sin();
                        if v > gleam { c.sea_light } else { c.sea }
                    }
                    Ground::Foam => {
                        // the foam breaks up where it thins, so the edge looks wet, not ruled
                        if paint::unit(x / 2, y, 41) < 0.75 {
                            c.foam
                        } else {
                            c.sea_light
                        }
                    }
                    Ground::Sand => {
                        if paint::hash(x, y, 9) % 6 == 0 {
                            c.grain
                        } else {
                            c.sand
                        }
                    }
                    Ground::Rock(shade) => c.rock[shade.min(2) as usize],
                    Ground::Weed => c.weed,
                    Ground::Pool(depth) => {
                        let v = (xf * 0.6 + yf * 0.9 - phase).sin()
                            + (xf * 0.37 - yf * 0.75 + phase * 0.8).sin()
                            + ((xf + yf) * 0.23 + phase * 0.5).sin();
                        if v > glint {
                            c.glint
                        } else {
                            let lift = if v > gleam { 1.0 } else { 0.0 };
                            let d = (f32::from(depth) - lift).max(0.0);
                            c.pool.at(d / (POOL_STEPS - 1) as f32)
                        }
                    }
                };
                canvas.set(x, y, colour);
            }
        }
        for s in &self.starfish {
            self.paint_starfish(canvas, c, s);
        }
        for a in &self.anemones {
            self.paint_anemone(canvas, c, a);
        }
        let (wide, tall) = self.crab_size();
        for crab in &self.crabs {
            let pose = ((crab.stride / CRAB_STEP) as usize) % 2;
            let palette = [('b', c.crab), ('c', c.claw), ('l', c.crab), ('e', c.eye)];
            let x = crab.x.round() as i32;
            if (wide, tall) == (9, 4) {
                canvas.sprite(x, crab.y, &self.crab_rows()[pose], &palette);
            } else {
                canvas.sprite(x, crab.y, &CRAB_SMALL[pose], &palette);
            }
        }
    }
}

impl Scene for TidePools {
    fn name(&self) -> &'static str {
        "tide-pools"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.lay_out(rng);
        self.crabs.clear();
        let (wide, tall) = self.crab_size();
        let top = self.edge(0) + 3;
        let h = self.tall();
        if h - top < tall + 2 || (self.width as i32) < wide * 2 {
            return;
        }
        let count = self
            .motion
            .scaled(count_for(self.width, self.height, 700, 1, 4));
        for _ in 0..count {
            // a few tries to find somewhere dry to stand
            for _ in 0..30 {
                let x = rng.range(0.0, (self.width as i32 - wide) as f32);
                let y = top + rng.below((h - top - tall).max(1) as usize) as i32;
                if self.crab_can_stand(x as i32, y, wide, tall) {
                    self.crabs.push(Crab {
                        x,
                        y,
                        speed: if rng.chance(0.5) { 1.0 } else { -1.0 } * rng.range(1.8, 3.0),
                        walk_left: rng.range(2.0, 6.0),
                        rest_left: 0.0,
                        stride: 0.0,
                    });
                    break;
                }
            }
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        let pace = self.motion.pace();
        let (wide, tall) = self.crab_size();
        let right_edge = (self.width as i32 - wide) as f32;
        for i in 0..self.crabs.len() {
            let crab = &self.crabs[i];
            if crab.rest_left > 0.0 {
                let crab = &mut self.crabs[i];
                crab.rest_left -= dt;
                if crab.rest_left <= 0.0 {
                    crab.walk_left = rng.range(2.0, 6.0);
                    if rng.chance(0.5) {
                        crab.speed = -crab.speed;
                    }
                }
                continue;
            }
            let next = crab.x + crab.speed * speed * dt;
            let blocked = next < 0.0
                || next > right_edge
                || !self.crab_can_stand(next.round() as i32, crab.y, wide, tall);
            let crab = &mut self.crabs[i];
            if blocked {
                crab.speed = -crab.speed;
            } else {
                crab.x = next;
                crab.stride += dt * pace;
            }
            crab.walk_left -= dt;
            if crab.walk_left <= 0.0 {
                crab.rest_left = rng.range(1.5, 4.0) * if self.motion.calm { 2.0 } else { 1.0 };
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = PoolColours::new(theme);
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
        check_scene("tide-pools");
        check_determinism("tide-pools");
    }

    #[test]
    fn passes_the_pixel_checks_in_every_theme() {
        check_pixel_scene("tide-pools", 6.0);
    }

    #[test]
    fn sprites_are_rectangular() {
        for rows in CRAB
            .iter()
            .map(|r| &r[..])
            .chain(CRAB_SMALL.iter().map(|r| &r[..]))
        {
            assert!(rows.iter().all(|r| r.len() == rows[0].len()), "{rows:?}");
        }
    }

    #[test]
    fn the_colours_hold_in_every_theme() {
        for theme in theme::all() {
            let c = PoolColours::new(theme);
            assert_imagery(
                theme,
                &[
                    ("sky top", c.sky_top),
                    ("sky low", c.sky_low),
                    ("sea", c.sea),
                    ("sea light", c.sea_light),
                    ("foam", c.foam),
                    ("sand", c.sand),
                    ("grain", c.grain),
                    ("lit rock", c.rock[0]),
                    ("rock", c.rock[1]),
                    ("shaded rock", c.rock[2]),
                    ("weed", c.weed),
                    ("shallow pool", c.shallow),
                    ("deep pool", c.deep),
                    ("glint", c.glint),
                ],
            );
            assert_shapes(
                theme,
                &[
                    ("crab", c.crab),
                    ("claw", c.claw),
                    ("anemone", c.anemones[0]),
                    ("second anemone", c.anemones[1]),
                    ("starfish", c.starfish),
                ],
                &[c.sand, c.rock[1], c.shallow, c.deep],
            );
            assert_apart(
                theme,
                &[
                    ("the sky and the sea", c.sky_low, c.sea),
                    ("the sea and the foam", c.sea, c.foam),
                    ("the foam and the sand", c.foam, c.sand),
                    ("the sand and the rock", c.sand, c.rock[1]),
                    ("the rock and its pool", c.rock[1], c.shallow),
                    ("the rock and its weed", c.rock[1], c.weed),
                    ("the pool and its glint", c.deep, c.glint),
                ],
            );
            assert!(distance(c.rock[0], c.rock[2]) >= 0.05, "{}", theme.name);
            assert!(distance(c.shallow, c.deep) >= 0.04, "{}", theme.name);
        }
    }

    #[test]
    fn crabs_keep_out_of_the_pools_and_legs_step_slowly() {
        const { assert!(1.0 / CRAB_STEP <= 2.5) };
        const { assert!(TAU / WAVE_RATE >= 5.0) };
        let mut rng = Rng::new(21);
        let mut pools = TidePools::new(100, 30, Motion::LIVELY, &mut rng);
        assert!(!pools.crabs.is_empty());
        assert!(pools.ground.iter().any(|g| matches!(g, Ground::Pool(_))));
        let (wide, tall) = pools.crab_size();
        let mut walked = 0.0_f32;
        for _ in 0..(90 * 20) {
            let before: Vec<f32> = pools.crabs.iter().map(|c| c.x).collect();
            pools.step(0.05, &mut rng);
            for (crab, x) in pools.crabs.iter().zip(before) {
                walked += (crab.x - x).abs();
                assert!(pools.crab_can_stand(crab.x.round() as i32, crab.y, wide, tall));
            }
        }
        assert!(walked > 20.0, "the crabs barely moved");
        let theme = theme::default_theme();
        let c = PoolColours::new(theme);
        let mut frame = Frame::new(100, 30);
        pools.draw(&mut frame, theme);
        let all = pixels(&frame);
        for (what, colour) in [
            ("crab", c.crab),
            ("rock", c.rock[1]),
            ("pool", c.shallow),
            ("sea", c.sea),
            ("anemone", c.anemones[0]),
        ] {
            assert!(
                all.contains(&colour) || (what == "anemone" && all.contains(&c.anemones[1])),
                "no {what}"
            );
        }
    }
}
