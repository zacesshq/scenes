//! Storm: a wide stretch of country under a storm. Three banks of cloud roll in at different
//! heights, each with a lit crest and a dark base, curtains of rain hang beneath them and a pale
//! break of light lies along the horizon. Rain slants in the wind in three depths (far, middle
//! and near) over far hills and a field with a hedgerow and a cottage. On a hill nearer by a tree
//! in full leaf bends in the gusts as its crown streams downwind. Puddles in the foreground
//! ripple.
//! Unless motion is reduced a bolt of lightning forks down from the cloud now and then, never
//! sooner than five seconds after the last one and never in the first five seconds, so the change
//! to a storm never opens with a flash.
//!
//! The sky, the cloud, the rain and the land are tints of the background held below the non-text
//! floor; the tree, the cottage window and the bolt are solid, so the storm keeps its shape with
//! colour off.

use std::f32::consts::TAU;

use super::paint::{Blender, ease, noise, shaded, solid, unit};
use super::{count_for, cycle, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// The shortest gap between two bolts, whatever the random numbers say.
pub const MIN_LIGHTNING_GAP: f32 = 5.0;
/// The longest wait for the next bolt.
const LONGEST_WAIT: f32 = 9.0;
/// How long one bolt stays on screen.
pub const FLASH_LENGTH: f32 = 0.12;
/// Seconds of one gust, which bends the tree further and lets it back. A branch drifts across
/// a pixel at a time, so no cell changes faster than a slow sway.
pub const GUST_PERIOD: f32 = 4.5;
/// How long a ripple spreads on a puddle.
const RIPPLE_LENGTH: f32 = 0.9;
/// Pixels a second the wind carries the rain sideways, then the slant it gives a streak.
const WIND: f32 = 9.0;
const SLANT: f32 = 0.45;

pub const SUMMARY: &str = "Slanting rain under rolling banks of storm cloud, a tree in full leaf bending in the wind and lightning, by day in light themes";

pub const CAPTIONS: &[&str] = &[
    "Something broke. The storm will pass.",
    "Rough weather in the build.",
    "Batten down and read the first error.",
];

#[derive(Clone, Debug)]
struct Bolt {
    /// The pixels of the bolt, trunk and fork.
    path: Vec<(i32, i32)>,
    age: f32,
}

/// A raindrop: where it is, how fast it falls and how near it is, from 0 far to 2 near.
#[derive(Clone, Debug)]
struct Drop {
    x: f32,
    y: f32,
    speed: f32,
    depth: u8,
}

/// A clump of the tree's crown: where it sits relative to the top of the trunk, before the wind
/// moves it, with its size.
#[derive(Clone, Debug)]
struct Clump {
    dx: f32,
    dy: f32,
    r: f32,
}

/// The storm's colours in one theme.
struct Colours {
    sky: Ramp,
    /// Each bank of cloud from its dark base to its lit crest, furthest first.
    banks: [Ramp; 3],
    curtain: Rgb,
    break_light: Rgb,
    far_hill: Rgb,
    mid_hill: Rgb,
    hedge: Rgb,
    near_hill: Ramp,
    field: Ramp,
    grass: Rgb,
    puddle: Rgb,
    sheen: Rgb,
    rain: [Rgb; 3],
    wall: Rgb,
    roof: Rgb,
    window: Rgb,
    bark: Rgb,
    bark_lit: Rgb,
    leaves: (Rgb, Rgb, Rgb),
    bolt: Rgb,
    glow: Rgb,
}

impl Colours {
    fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let t = |hue: Rgb, s: f32, ceiling: f64| shades.tint(hue, s, ceiling);
        let grey = mix(theme.muted, theme.info, 0.3);
        let earth = mix(theme.green(), theme.muted, 0.45);
        let rain_hue = mix(theme.info, theme.foreground, 0.4);
        let leaf = solid(theme, theme.green(), shades.darkest(), 0.15);
        let leaves = shaded(theme, leaf);
        // bark is a deep warm brown held just at the solid contrast, so the trunk reads as wood
        // under the crown rather than a pale post
        let wood = mix(theme.warning, theme.red(), 0.35);
        let bark = solid(theme, wood, shades.darkest(), 0.7);
        let bark_lit = solid(theme, wood, shades.darkest(), 0.35);
        if theme.dark {
            // night storm: the cloud is lighter than the sky and brightest along its crests,
            // while a cold break of light under it outlines the land
            Self {
                sky: Ramp::new(t(grey, 0.08, 1.15), t(grey, 0.22, 1.45), 6),
                banks: [
                    Ramp::new(t(grey, 0.14, 1.27), t(grey, 0.34, 1.75), 4),
                    Ramp::new(t(grey, 0.2, 1.38), t(grey, 0.48, 2.15), 4),
                    Ramp::new(t(grey, 0.26, 1.5), t(theme.bright(), 0.5, 2.6), 4),
                ],
                curtain: t(grey, 0.3, 1.6),
                break_light: t(mix(theme.info, theme.yellow(), 0.4), 0.6, 2.7),
                far_hill: t(grey, 0.3, 1.6),
                mid_hill: t(earth, 0.22, 1.42),
                hedge: t(earth, 0.1, 1.17),
                near_hill: Ramp::new(t(earth, 0.1, 1.17), t(earth, 0.05, 1.08), 5),
                field: Ramp::new(t(earth, 0.26, 1.5), t(earth, 0.14, 1.25), 5),
                grass: t(earth, 0.42, 1.95),
                puddle: t(grey, 0.42, 2.0),
                sheen: t(theme.bright(), 0.55, 2.7),
                rain: [
                    t(rain_hue, 0.24, 1.55),
                    t(rain_hue, 0.38, 1.95),
                    t(rain_hue, 0.62, 2.7),
                ],
                wall: t(theme.muted, 0.32, 1.7),
                roof: t(theme.red(), 0.2, 1.38),
                window: theme.yellow(),
                bark,
                bark_lit,
                leaves,
                bolt: theme.bright_yellow(),
                glow: t(theme.bright_yellow(), 0.5, 2.6),
            }
        } else {
            // a daytime storm: a pale sky under darker cloud, heaviest at its base, with a bright
            // break along the horizon and the land greener nearer the eye
            Self {
                sky: Ramp::new(t(grey, 0.36, 1.7), t(grey, 0.16, 1.25), 6),
                banks: [
                    Ramp::new(t(grey, 0.5, 2.05), t(grey, 0.26, 1.45), 4),
                    Ramp::new(t(grey, 0.62, 2.35), t(grey, 0.3, 1.55), 4),
                    Ramp::new(t(grey, 0.78, 2.8), t(grey, 0.36, 1.7), 4),
                ],
                curtain: t(grey, 0.55, 2.2),
                break_light: t(theme.yellow(), 0.06, 1.06),
                far_hill: t(grey, 0.42, 1.85),
                mid_hill: t(earth, 0.55, 2.2),
                hedge: t(earth, 0.8, 2.85),
                near_hill: Ramp::new(t(earth, 0.45, 2.0), t(earth, 0.6, 2.4), 5),
                field: Ramp::new(t(earth, 0.36, 1.7), t(earth, 0.5, 2.1), 5),
                grass: t(earth, 0.72, 2.8),
                puddle: t(grey, 0.14, 1.22),
                sheen: t(theme.info, 0.02, 1.04),
                rain: [
                    t(rain_hue, 0.3, 1.5),
                    t(rain_hue, 0.45, 1.9),
                    t(rain_hue, 0.7, 2.6),
                ],
                wall: t(theme.muted, 0.18, 1.3),
                roof: t(theme.red(), 0.62, 2.6),
                window: t(theme.yellow(), 0.5, 2.0),
                bark,
                bark_lit,
                leaves,
                bolt: theme.bright_yellow(),
                glow: t(theme.bright_yellow(), 0.4, 1.8),
            }
        }
    }
}

pub struct Storm {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    seed: u32,
    drops: Vec<Drop>,
    ripples: Vec<(i32, i32, f32)>,
    crown: Vec<Clump>,
    puddles: Vec<(f32, f32, f32)>,
    tufts: Vec<(i32, i32, i32)>,
    bolt: Option<Bolt>,
    next_bolt: f32,
    /// Seconds since the last bolt appeared. Every strike waits for this to reach the minimum
    /// gap, whatever else resets, so a resize can never bring a bolt early.
    since_strike: f32,
    strikes: u32,
}

impl Storm {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut storm = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            seed: (rng.next_u64() >> 32) as u32,
            drops: Vec::new(),
            ripples: Vec::new(),
            crown: Vec::new(),
            puddles: Vec::new(),
            tufts: Vec::new(),
            bolt: None,
            next_bolt: rng.range(MIN_LIGHTNING_GAP, LONGEST_WAIT),
            since_strike: 0.0,
            strikes: 0,
        };
        storm.resize(width, height, rng);
        storm
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The horizon row, where the far hills sit.
    fn horizon(&self) -> i32 {
        (self.tall() as f32 * 0.6) as i32
    }

    /// The first pixel row of the foreground field.
    fn ground(&self) -> i32 {
        let tall = self.tall();
        (tall - (tall / 5).max(2)).max(1)
    }

    /// The far hills along the horizon at a column.
    fn far_hill(&self, x: i32) -> i32 {
        let x = x as f32;
        let tall = self.tall() as f32;
        let lift = (noise(x / 26.0, 0.5, self.seed ^ 0x1f) * 0.09 + 0.02) * tall;
        self.horizon() - lift.round() as i32
    }

    /// The rolling field in the middle distance at a column.
    fn mid_hill(&self, x: i32) -> i32 {
        let x = x as f32;
        let tall = self.tall() as f32;
        let lift = (0.5 + 0.5 * (x / 30.0 + 1.1).sin()) * tall * 0.05;
        (self.horizon() as f32 + tall * 0.06 - lift).round() as i32
    }

    /// The near hill's top at a column, which the tree stands on.
    fn hill(&self, x: i32) -> i32 {
        let w = self.width as f32;
        let centre = w * 0.72;
        let spread = (w * 0.42).max(6.0);
        let d = (x as f32 - centre) / spread;
        let rise = (self.tall() as f32 * 0.2).max(2.0) * (1.0 - d * d).max(0.0).powf(0.8);
        self.ground() - rise.round() as i32
    }

    /// Where the tree stands, if the screen is big enough for one.
    fn tree_base(&self) -> Option<(i32, i32)> {
        (self.width >= 24 && self.height >= 8).then(|| {
            let x = (self.width as f32 * 0.72) as i32;
            (x, self.hill(x) + 1)
        })
    }

    fn trunk_height(&self) -> f32 {
        (self.tall() as f32 * 0.4).clamp(6.0, 70.0)
    }

    /// How far the wind bends the tree, in radians: a steady lean and a slow gust.
    fn bend(&self) -> f32 {
        let gust = cycle(self.t, TAU / GUST_PERIOD * self.motion.pace()).sin();
        0.22 + 0.07 * gust
    }

    /// The point `s` of the way up the trunk, bent by the wind more towards the top.
    fn trunk_point(&self, foot: (f32, f32), s: f32) -> (f32, f32) {
        let h = self.trunk_height();
        let lean = self.bend() * s * s;
        (
            foot.0 + h * s * lean.sin() * 1.3,
            foot.1 - h * s * lean.cos(),
        )
    }

    fn new_drop(&self, rng: &mut Rng, from_top: bool) -> Drop {
        let roll = rng.unit();
        let depth = if roll < 0.45 {
            0
        } else if roll < 0.8 {
            1
        } else {
            2
        };
        let speed = [16.0, 22.0, 30.0][depth as usize] * rng.range(0.9, 1.1);
        Drop {
            x: rng.range(0.0, self.width as f32),
            y: if from_top {
                -rng.range(0.0, 8.0)
            } else {
                rng.range(-2.0, self.tall() as f32)
            },
            speed: speed * self.motion.speed(),
            depth,
        }
    }

    /// The lowest row a drop of a depth falls to at a column: far rain vanishes into the far
    /// hills, middle rain into the field and near rain reaches the foreground.
    fn rain_floor(&self, depth: u8, x: i32) -> i32 {
        match depth {
            0 => self.far_hill(x),
            1 => self.mid_hill(x).min(self.hill(x)),
            _ => self.tall(),
        }
    }

    fn make_bolt(&self, rng: &mut Rng) -> Option<Bolt> {
        if self.width < 12 || self.height < 6 {
            return None;
        }
        let w = self.width as f32;
        // the bolt stays in the open sky to the left of the tree
        let mut x = rng.range(3.0, (w * 0.5).max(4.0));
        let mut y = (self.tall() as f32 * 0.3).round();
        let length = (self.tall() as f32 * 0.3).clamp(5.0, 40.0) as i32;
        let mut path = Vec::new();
        let mut fork = None;
        for i in 0..length {
            let step = rng.range(-1.2, 1.2);
            let next = (x + step, y + 1.0);
            for (px, py) in [(x, y), next] {
                path.push((px.round() as i32, py.round() as i32));
            }
            (x, y) = next;
            if i == length / 3 {
                fork = Some((x, y, if step >= 0.0 { -1.0 } else { 1.0 }));
            }
        }
        if let Some((mut fx, mut fy, side)) = fork {
            for _ in 0..length / 3 {
                fx += side * rng.range(0.4, 1.3);
                fy += 1.0;
                path.push((fx.round() as i32, fy.round() as i32));
            }
        }
        path.sort_unstable();
        path.dedup();
        Some(Bolt { path, age: 0.0 })
    }

    #[cfg(test)]
    fn flashing(&self) -> bool {
        self.bolt.is_some()
    }

    /// One bank of cloud: a row of great rounded billows along a flat base, drifting with the
    /// wind. Each billow is lit on its crown and its upwind side and darkens to a heavy base,
    /// so the bank reads as solid masses rather than streaks. With `fill` the bank also covers
    /// all the sky above it, as an overcast does.
    #[allow(clippy::too_many_arguments)]
    fn bank(
        &self,
        canvas: &mut Canvas,
        ramp: &Ramp,
        base: f32,
        size: f32,
        drift: f32,
        salt: u32,
        fill: bool,
    ) {
        let wide = canvas.width() as i32;
        let size = size.max(2.0);
        let step = size * 1.1;
        let span = wide as f32 + step * 4.0;
        let count = (span / step).ceil() as i32;
        // each column's best billow: the height of the cloud top there and how lit it is
        let mut tops: Vec<(f32, f32, f32)> = vec![(f32::MAX, 0.0, 0.0); wide.max(1) as usize];
        for i in 0..count {
            let h = unit(i, 7, salt);
            let r = size * (0.5 + 0.9 * h);
            let cx = wrap(i as f32 * step + drift, span) - step * 2.0;
            let cy = base - r * (0.35 + 0.35 * unit(i, 9, salt));
            for x in (cx - r).floor() as i32..=(cx + r).ceil() as i32 {
                if x < 0 || x >= wide {
                    continue;
                }
                let dx = x as f32 + 0.5 - cx;
                if dx.abs() > r {
                    continue;
                }
                let top = cy - (r * r - dx * dx).sqrt();
                let slot = &mut tops[x as usize];
                if top < slot.0 {
                    *slot = (top, cx, r);
                }
            }
        }
        for x in 0..wide {
            let (top, cx, r) = tops[x as usize];
            let top = if fill { -1.0 } else { top };
            if top == f32::MAX {
                continue;
            }
            let side = if r > 0.0 {
                (cx - (x as f32 + 0.5)) / r
            } else {
                0.0
            };
            for y in top.ceil().max(0.0) as i32..(base.round() as i32).min(canvas.height() as i32) {
                let depth = (y as f32 - top) / (base - top).max(1.0);
                // lit on the crown and on the upwind left, heavy and dark along the base
                let crown = (1.0 - depth * 1.6).max(0.0);
                let light = (0.2 + crown * 0.6 + side * 0.25 - (depth - 0.7).max(0.0) * 1.2)
                    .clamp(0.0, 1.0);
                canvas.set(x, y, ramp.at(light));
            }
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        let mut mixer = Blender::new(3);
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let horizon = self.horizon();
        let ground = self.ground();
        let speed = self.motion.speed();

        // the sky, opening into a pale break of light just above the horizon
        for y in 0..horizon {
            let v = y as f32 / horizon.max(1) as f32;
            canvas.fill_row(y, c.sky.at(v));
            let lift = ((v - 0.72) / 0.28).clamp(0.0, 1.0);
            if lift > 0.0 {
                for x in 0..wide {
                    mixer.blend(canvas, x, y, c.break_light, ease(lift) * 0.85);
                }
            }
        }

        // three banks of cloud, the furthest highest and slowest, the nearest lowest and
        // quickest; scene time is wrapped far beyond any visible repeat before it narrows to
        // an f32, so the cloud keeps moving smoothly on a long run
        let drift = (self.t % 100_000.0) as f32 * speed;
        let h = horizon as f32;
        let unit_size = (h / 6.0).max(2.0);
        self.bank(
            canvas,
            &c.banks[0],
            h * 0.34,
            unit_size * 1.3,
            drift * 0.5,
            self.seed,
            true,
        );
        if tall >= 12 {
            self.bank(
                canvas,
                &c.banks[1],
                h * 0.6,
                unit_size * 1.1,
                drift * 0.9 + 31.0,
                self.seed ^ 0x51ed,
                false,
            );
            self.bank(
                canvas,
                &c.banks[2],
                h * 0.8,
                unit_size * 0.7,
                drift * 1.5 + 77.0,
                self.seed ^ 0x2c3,
                false,
            );
        }

        // curtains of rain hanging below the cloud, slanting with the wind
        for y in (h * 0.4) as i32..horizon {
            let v = (y as f32 - h * 0.4) / (h * 0.6);
            for x in 0..wide {
                let along = (x as f32 - y as f32 * SLANT + drift * 0.8) / 16.0;
                let shaft = noise(along, 0.3, self.seed ^ 0x99);
                // one step of veil only, so the curtains add few colours
                if shaft > 0.58 && v < 0.85 {
                    mixer.blend(canvas, x, y, c.curtain, 0.34);
                }
            }
        }

        // a glow inside the cloud where the bolt leaves it
        if let Some(bolt) = &self.bolt {
            if let Some(&(x, y)) = bolt.path.iter().min_by_key(|p| p.1) {
                for dy in -5..=3 {
                    for dx in -9..=9 {
                        let d = ((dx * dx) as f32 / 81.0 + (dy * dy) as f32 / 25.0).sqrt();
                        if d < 1.0 {
                            mixer.blend(canvas, x + dx, y + dy, c.glow, 0.7 * (1.0 - d));
                        }
                    }
                }
            }
        }

        // the far hills, then far rain in front of them
        for x in 0..wide {
            for y in self.far_hill(x).max(0)..tall {
                canvas.set(x, y, c.far_hill);
            }
        }
        self.paint_rain(canvas, c, 0);

        // the field in the middle distance with its hedgerow and a cottage
        for x in 0..wide {
            let top = self.mid_hill(x);
            for y in top.max(0)..tall {
                canvas.set(x, y, c.mid_hill);
            }
            // a hedgerow along the brow, thick and uneven
            let thick = 1 + (noise(x as f32 / 3.0, 0.0, self.seed ^ 0x3) * 2.5) as i32;
            for y in top - thick + 1..=top {
                canvas.set(x, y, c.hedge);
            }
        }
        self.paint_cottage(canvas, c);
        self.paint_rain(canvas, c, 1);

        // the near hill and the foreground field
        for x in 0..wide {
            let top = self.hill(x);
            for y in top.max(0)..ground {
                let v = (y - top) as f32 / (ground - top).max(1) as f32;
                canvas.set(x, y, c.near_hill.at(v));
            }
        }
        let depth = (tall - ground).max(1) as f32;
        for y in ground..tall {
            let v = (y - ground) as f32 / depth;
            canvas.fill_row(y, c.field.at(v));
        }
        // puddles lying in the field, catching the light of the sky
        for &(px, py, half) in &self.puddles {
            let y = py.round() as i32;
            // a flat oval seen from low down, two rows deep
            for (row, width) in [(0, 1.0), (1, 0.7)] {
                let reach = half * width;
                for x in (px - reach).round() as i32..=(px + reach).round() as i32 {
                    canvas.set(x, y + row, c.puddle);
                }
            }
        }
        for &(x, y, age) in &self.ripples {
            let r = (age / RIPPLE_LENGTH * 3.0).round() as i32;
            canvas.set(x - r, y, c.sheen);
            canvas.set(x + r, y, c.sheen);
        }
        // tufts of grass leaning with the wind
        let lean = self.bend() * 4.0;
        for &(x, y, h) in &self.tufts {
            for k in 0..h {
                let dx = (lean * k as f32 / h.max(1) as f32).round() as i32;
                canvas.set(x + dx, y - k, c.grass);
                canvas.set(x + dx + 2, y - k + 1, c.grass);
            }
        }

        self.paint_tree(canvas, c);
        self.paint_rain(canvas, c, 2);

        if let Some(bolt) = &self.bolt {
            for &(x, y) in &bolt.path {
                canvas.set(x, y, c.bolt);
            }
        }
    }

    fn paint_rain(&self, canvas: &mut Canvas, c: &Colours, depth: u8) {
        let length = match depth {
            0 => 2,
            1 => 3,
            _ => 5,
        };
        let colour = c.rain[depth as usize];
        // rain is set rather than mixed, so it adds three colours however much it crosses
        for drop in self.drops.iter().filter(|d| d.depth == depth) {
            for k in 0..length {
                let y = drop.y.floor() as i32 - k;
                let x = (drop.x - SLANT * k as f32).round() as i32;
                if y >= 0 && y < self.rain_floor(depth, x) {
                    canvas.set(x, y, colour);
                }
            }
        }
    }

    fn paint_cottage(&self, canvas: &mut Canvas, c: &Colours) {
        if self.width < 60 || self.height < 16 {
            return;
        }
        let x = (self.width as f32 * 0.2) as i32;
        let base = self.mid_hill(x + 4);
        let s = (self.tall() as f32 / 68.0).clamp(0.8, 2.0);
        let (w, h) = ((10.0 * s) as i32, (5.0 * s) as i32);
        canvas.fill_rect(x, base - h, w, h + 1, c.wall);
        // a pitched roof overhanging the walls with a chimney
        let pitch = (4.0 * s) as i32;
        for k in 0..pitch {
            canvas.fill_rect(x - 1 + k, base - h - 1 - k, w + 2 - 2 * k, 1, c.roof);
        }
        canvas.fill_rect(x + w - 3, base - h - pitch - 1, 2, pitch, c.roof);
        let pane = (2.0 * s) as i32;
        canvas.fill_rect(x + 2, base - h + 1, pane, pane, c.window);
        canvas.fill_rect(x + w - 2 - pane, base - h + 1, pane, pane, c.window);
    }

    fn paint_tree(&self, canvas: &mut Canvas, c: &Colours) {
        let Some((bx, by)) = self.tree_base() else {
            return;
        };
        let foot = (bx as f32, by as f32);
        let h = self.trunk_height();
        let steps = (h * 2.0).ceil().max(1.0) as i32;
        let base_thick = (h / 13.0).clamp(1.0, 4.0);
        // the trunk tapers as it rises, lit along its left side
        for i in 0..=steps {
            let s = i as f32 / steps as f32;
            let (x, y) = self.trunk_point(foot, s);
            let half = base_thick * (1.0 - 0.6 * s) + if s < 0.08 { 1.0 } else { 0.0 };
            let (l, r) = ((x - half).round() as i32, (x + half).round() as i32);
            for px in l..=r.max(l) {
                let colour = if px == l && r > l { c.bark_lit } else { c.bark };
                canvas.set(px, y.round() as i32, colour);
            }
        }
        // limbs reaching up and downwind into the crown
        let top = self.trunk_point(foot, 1.0);
        let bend = self.bend();
        for (at, angle, length) in [(0.55, -0.7, 0.35), (0.7, 0.8, 0.4), (0.85, -0.3, 0.3)] {
            let from = self.trunk_point(foot, at);
            let a = angle + bend * 1.6;
            let to = (from.0 + a.sin() * h * length, from.1 - a.cos() * h * length);
            super::paint::line(canvas, from, to, if h > 20.0 { 2 } else { 1 }, c.bark);
        }
        // the crown: clumps of leaves blown downwind. Each pixel takes its light from the clump
        // that bulges furthest there, lit where that clump faces the upper left and shaded on
        // its lower right, so the crown reads as one rounded mass with lobes
        let (dark, base, light) = c.leaves;
        let gust = bend * h * 0.9;
        let placed: Vec<(f32, f32, f32)> = self
            .crown
            .iter()
            .map(|k| {
                let reach = 1.0 - k.dy;
                (
                    top.0 + k.dx * h + gust * (0.4 + 0.6 * reach),
                    top.1 + k.dy * h * 0.6,
                    k.r * h,
                )
            })
            .collect();
        let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for &(x, y, r) in &placed {
            x0 = x0.min(x - r);
            x1 = x1.max(x + r);
            y0 = y0.min(y - r);
            y1 = y1.max(y + r);
        }
        for py in y0.floor() as i32..=y1.ceil() as i32 {
            for px in x0.floor() as i32..=x1.ceil() as i32 {
                let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
                let mut best: Option<(f32, f32, f32)> = None;
                for &(x, y, r) in &placed {
                    let (nx, ny) = ((fx - x) / r, (fy - y) / r);
                    let f = 1.0 - (nx * nx + ny * ny);
                    if f > 0.0 && best.is_none_or(|b| f > b.0) {
                        best = Some((f, nx, ny));
                    }
                }
                if let Some((f, nx, ny)) = best {
                    let facing = -(nx * 0.6 + ny * 0.8);
                    let colour = if facing > 0.35 && f > 0.15 {
                        light
                    } else if facing < -0.3 || f < 0.12 && facing < 0.2 {
                        dark
                    } else {
                        base
                    };
                    canvas.set(px, py, colour);
                }
            }
        }
    }
}

impl Scene for Storm {
    fn name(&self) -> &'static str {
        "storm"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = self
            .motion
            .scaled(count_for(self.width, self.height, 12, 8, 450));
        let wanted = if self.motion.calm {
            wanted * 2 / 3
        } else {
            wanted
        };
        self.drops = (0..wanted).map(|_| self.new_drop(rng, false)).collect();
        self.ripples.clear();
        if self.bolt.take().is_some() {
            self.next_bolt = self.next_bolt.max(MIN_LIGHTNING_GAP);
        }
        // the crown: a ring of clumps around a fuller middle, a little taller than wide
        let count = 12;
        self.crown = (0..count)
            .map(|i| {
                let a = i as f32 / (count - 1) as f32 * TAU;
                let ring = if i == 0 { 0.0 } else { rng.range(0.24, 0.36) };
                Clump {
                    dx: a.cos() * ring * 1.15,
                    dy: a.sin() * ring * 0.75 - 0.12,
                    r: rng.range(0.16, 0.24),
                }
            })
            .collect();
        let (w, tall) = (self.width as f32, self.tall());
        let ground = self.ground();
        let depth = (tall - ground).max(1);
        let pools = (self.width / 22).max(1);
        self.puddles = (0..pools)
            .map(|_| {
                (
                    rng.range(2.0, w * 0.6),
                    (ground + 1 + rng.below(depth.max(2) as usize - 1) as i32) as f32,
                    rng.range(2.5, 6.0),
                )
            })
            .collect();
        let tufts = (self.width / 6).max(1);
        self.tufts = (0..tufts)
            .map(|i| {
                let x = (i as f32 * 6.0 + rng.range(0.0, 4.0)) as i32;
                let y = ground + 1 + rng.below(depth.max(2) as usize) as i32;
                let h = 1 + (unit(i as i32, 0, self.seed) * 2.5) as i32;
                (x, y, h)
            })
            .collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        self.since_strike += dt;
        let speed = self.motion.speed();
        let width = self.width as f32;
        for i in 0..self.drops.len() {
            let drop = &mut self.drops[i];
            drop.y += drop.speed * dt;
            drop.x = wrap(drop.x + WIND * speed * dt, width);
            let (x, y, depth) = (drop.x.round() as i32, drop.y, drop.depth);
            let floor = self.rain_floor(depth, x);
            if y >= floor as f32 {
                self.drops[i] = self.new_drop(rng, true);
                // a near drop landing in a puddle sends out a ripple
                if depth == 2 && self.ripples.len() < 24 {
                    if let Some(&(px, py, half)) = self
                        .puddles
                        .iter()
                        .find(|p| (p.0 - x as f32).abs() < p.2 * 0.7)
                    {
                        let rx = (px + rng.range(-half, half) * 0.5).round() as i32;
                        self.ripples.push((rx, py.round() as i32, 0.0));
                    }
                }
            }
        }
        let pace = self.motion.pace();
        for ripple in &mut self.ripples {
            ripple.2 += dt * pace;
        }
        self.ripples.retain(|r| r.2 < RIPPLE_LENGTH);

        if self.motion.calm {
            self.bolt = None;
            return;
        }
        if let Some(bolt) = &mut self.bolt {
            bolt.age += dt;
            if bolt.age >= FLASH_LENGTH {
                self.bolt = None;
                self.next_bolt = rng.range(MIN_LIGHTNING_GAP, LONGEST_WAIT);
            }
        } else {
            self.next_bolt -= dt;
            if self.next_bolt <= 0.0 && self.since_strike >= MIN_LIGHTNING_GAP {
                match self.make_bolt(rng) {
                    Some(bolt) => {
                        self.bolt = Some(bolt);
                        self.since_strike = 0.0;
                        self.strikes += 1;
                    }
                    None => self.next_bolt = MIN_LIGHTNING_GAP,
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = Colours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixels::IMAGERY_CEILING;
    use crate::scenes::paint;
    use crate::scenes::testing::{apart, check_pixel_scene, pixels};
    use crate::scenes::testing::{check_determinism, check_scene, render};
    use crate::theme;

    fn bolt_starts(storm: &mut Storm, rng: &mut Rng, seconds: usize) -> Vec<f32> {
        let mut starts = Vec::new();
        let mut was = false;
        for i in 0..(seconds * 20) {
            storm.step(0.05, rng);
            if storm.flashing() && !was {
                starts.push(i as f32 * 0.05);
            }
            was = storm.flashing();
        }
        starts
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("storm");
        check_determinism("storm");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("storm", 40.0);
    }

    #[test]
    fn the_storm_keeps_the_lightning_limits() {
        for motion in [Motion::FULL, Motion::LIVELY, Motion::GENTLE] {
            let mut rng = Rng::new(1);
            let mut storm = Storm::new(80, 24, motion, &mut rng);
            let starts = bolt_starts(&mut storm, &mut rng, 300);
            assert!(starts.len() >= 20, "{}", starts.len());
            // the first bolt waits the full gap, so the change to a storm never opens with one
            assert!(starts[0] >= MIN_LIGHTNING_GAP - 0.01, "{}", starts[0]);
            for pair in starts.windows(2) {
                assert!(pair[1] - pair[0] >= MIN_LIGHTNING_GAP - 0.01, "{pair:?}");
            }
        }
        let mut rng = Rng::new(1);
        let mut calm = Storm::new(80, 24, Motion::CALM, &mut rng);
        assert!(bolt_starts(&mut calm, &mut rng, 120).is_empty());
        const { assert!(FLASH_LENGTH < 1.0 / 3.0 && MIN_LIGHTNING_GAP >= 5.0) };
    }

    #[test]
    fn resizing_never_brings_lightning_early() {
        let mut rng = Rng::new(4);
        let mut storm = Storm::new(80, 24, Motion::FULL, &mut rng);
        let mut starts = Vec::new();
        let mut was = false;
        for i in 0..(120 * 20) {
            if i % 13 == 0 {
                let size = if i % 2 == 0 { (60, 20) } else { (80, 24) };
                storm.resize(size.0, size.1, &mut rng);
            }
            storm.step(0.05, &mut rng);
            if storm.flashing() && !was {
                starts.push(i as f32 * 0.05);
            }
            was = storm.flashing();
        }
        for pair in starts.windows(2) {
            assert!(pair[1] - pair[0] >= MIN_LIGHTNING_GAP - 0.01, "{pair:?}");
        }
    }

    #[test]
    fn the_bolt_is_a_thin_line_and_the_tree_bends_slowly() {
        let theme = theme::default_theme();
        let c = Colours::new(theme);
        let mut rng = Rng::new(2);
        let mut storm = Storm::new(80, 24, Motion::FULL, &mut rng);
        let mut seen = false;
        for _ in 0..(30 * 20) {
            storm.step(0.05, &mut rng);
            if storm.flashing() {
                let bolt = storm.bolt.as_ref().unwrap();
                // never more than a sliver of the sky
                assert!(bolt.path.len() < 80 * 48 / 20, "{}", bolt.path.len());
                let frame = render(&mut storm, 80, 24, 0);
                seen |= pixels(&frame).contains(&c.bolt);
            }
        }
        assert!(seen);
        assert!(storm.tree_base().is_some());
        let frame = render(&mut storm, 80, 24, 1);
        let all = pixels(&frame);
        assert!(all.contains(&c.bark) && all.contains(&c.leaves.1));
        const { assert!(GUST_PERIOD / 4.0 > 1.0 / 3.0) };
    }

    #[test]
    fn the_layers_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            apart(theme, c.sky.at(0.0), c.sky.at(1.0), 0.02, "the sky");
            for bank in &c.banks {
                apart(
                    theme,
                    bank.at(0.0),
                    bank.at(1.0),
                    0.03,
                    "a cloud's base and crest",
                );
            }
            apart(
                theme,
                c.far_hill,
                c.break_light,
                0.03,
                "the hills and the break",
            );
            apart(
                theme,
                c.mid_hill,
                c.far_hill,
                0.03,
                "the field and the far hills",
            );
            apart(
                theme,
                c.hedge,
                c.mid_hill,
                0.03,
                "the hedgerow and the field",
            );
            apart(
                theme,
                c.near_hill.at(0.0),
                c.mid_hill,
                0.02,
                "the hill and the field",
            );
            apart(theme, c.rain[2], c.sky.at(0.5), 0.04, "the rain");
            apart(
                theme,
                c.bark,
                c.near_hill.at(0.0),
                0.1,
                "the tree and the hill",
            );
            apart(
                theme,
                c.leaves.1,
                c.sky.at(1.0),
                0.1,
                "the crown and the sky",
            );
            apart(
                theme,
                c.bolt,
                c.banks[1].at(0.5),
                0.1,
                "the bolt and the cloud",
            );
            let mut imagery = vec![
                c.curtain,
                c.break_light,
                c.far_hill,
                c.mid_hill,
                c.hedge,
                c.grass,
                c.puddle,
                c.sheen,
                c.wall,
                c.roof,
                c.glow,
            ];
            imagery.extend(c.rain);
            imagery.extend(c.sky.steps());
            for bank in &c.banks {
                imagery.extend(bank.steps());
            }
            for colour in imagery {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "{} in {}",
                    colour.hex(),
                    theme.name
                );
            }
            for colour in [c.bark, c.leaves.0, c.leaves.1, c.leaves.2] {
                assert!(theme.contrast_of(colour) >= paint::SOLID - 0.01);
            }
        }
    }
}
