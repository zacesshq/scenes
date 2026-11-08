//! Countryside: a steam train crossing open country, seen from beside the line and painted in
//! solid pixels. The view keeps pace with the train, so the land slides past in layers: far hills
//! slowest, then a patchwork of fields between hedgerows, then the trees by the line, the track
//! and a fence right in front. Every layer is worked out from where it is in the world, so the
//! country never repeats and needs no memory. Smoke rises from the chimney in soft puffs that
//! swell and fade as they drift back.
//!
//! Every colour comes from the active theme. The sky, hills, fields and ballast are tints of the
//! background held below the non-text floor, so they read as background; the train, the trees,
//! the rails and the fence keep the theme's own drawing colours, so they stand out as shapes and
//! keep their outline when colour is off.

use std::f32::consts::TAU;

use super::paint;
use super::{cycle, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "A steam train crossing open country at evening, with hills, fields, trees and a fence sliding past, by day in light themes";

pub const CAPTIONS: &[&str] = &[
    "Right on time. Everything passed.",
    "All aboard a clean build.",
    "Full steam ahead.",
    "The line is clear all the way.",
    "No delays on this service.",
];

/// Columns a second the ground beside the track slides past at the standard pace.
pub const TRACK_SPEED: f32 = 2.5;
/// Seconds a puff of smoke spends at each of its three sizes, so it changes slowly as it drifts.
pub const PUFF_STAGE: f32 = 1.0;

/// Steps in the sky ramp.
const SKY_STEPS: usize = 10;

/// The engine, facing right: `b` the body, `d` its shaded underside, `c` the chimney and dome,
/// `w` the lit cab window, `r` the frame, `k` the wheels, `h` their hubs and `l` the lamp.
const LOCO: [&str; 10] = [
    "............cc...",
    "bbbbb.......cc...",
    ".bwb........cc.c.",
    ".bwb..bbbbbbbbbbb",
    ".bbb..bbbbbbbbbbl",
    ".bbbddddddddddddd",
    "rrrrrrrrrrrrrrrrr",
    ".kkk.kkk.kkk..kk.",
    ".khk.khk.khk..kk.",
    ".kkk.kkk.kkk.....",
];
/// A carriage: `t` the roof, `b` the body, `w` the lit windows, `d` the underframe and `k` the
/// wheels.
const CARRIAGE: [&str; 8] = [
    ".tttttttttttttttt.",
    "bbbbbbbbbbbbbbbbbb",
    "bwwbwwbwwbwwbwwbbb",
    "bwwbwwbwwbwwbwwbbb",
    "bbbbbbbbbbbbbbbbbb",
    "dddddddddddddddddd",
    ".kk.kk......kk.kk.",
    ".kk.kk......kk.kk.",
];
const LOCO_WIDTH: i32 = 17;
const CARRIAGE_WIDTH: i32 = 18;
/// The column of the chimney's mouth in the engine sprite.
const CHIMNEY: i32 = 12;

/// The puffs' radius at each stage and how strongly each covers the sky behind it.
const PUFFS: [(f32, f32); 3] = [(1.1, 0.9), (1.8, 0.7), (2.6, 0.45)];

/// A small deterministic hash, so the country is worked out from where it is in the world.
fn hash(a: i64, b: i64) -> u64 {
    let mut z = (a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (b as u64).wrapping_mul(0xD1B5_4A32_D192_ED03);
    z = (z ^ (z >> 29)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^ (z >> 32)
}

/// The country's colours in one theme, every one derived from the theme's own. Some are kept
/// only so the tests can check the bands against each other.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct CountryColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub cloud: Rgb,
    pub cloud_shade: Rgb,
    pub hills: Rgb,
    pub hill_trees: Rgb,
    /// The fields' crops: grass, a darker pasture and ripe wheat.
    pub fields: [Rgb; 3],
    pub hedge: Rgb,
    pub verge: Rgb,
    pub ballast: Rgb,
    pub sleeper: Rgb,
    pub rail: Rgb,
    pub sun: Rgb,
    /// The glow around the sun: the sun itself on a dark theme, a warm tint around a pale sun
    /// on a light one.
    pub halo: Rgb,
    pub dark: bool,
    pub pine: Rgb,
    pub oak: Rgb,
    pub trunk: Rgb,
    pub fence: Rgb,
    pub smoke: Rgb,
    pub engine: Rgb,
    pub frame: Rgb,
    pub window: Rgb,
    pub wheel: Rgb,
    pub carriages: [Rgb; 2],
    sky: Ramp,
    background: Rgb,
}

impl CountryColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        // a golden evening on a dark theme, a clear day on a light one
        let (sky_top, sky_low, cloud, cloud_shade) = if theme.dark {
            (
                shades.tint(theme.info, 0.2, 1.4),
                shades.tint(theme.warning, 0.45, 2.2),
                shades.tint(theme.foreground, 0.35, 2.3),
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
        let (hills, hill_trees) = if theme.dark {
            (
                shades.tint(theme.blue(), 0.3, 1.45),
                shades.tint(theme.green(), 0.15, 1.25),
            )
        } else {
            (
                shades.tint(theme.blue(), 0.35, 1.75),
                shades.tint(theme.green(), 0.6, 2.3),
            )
        };
        let fields = if theme.dark {
            [
                shades.tint(theme.green(), 0.38, 2.0),
                shades.tint(theme.green(), 0.25, 1.55),
                shades.tint(theme.yellow(), 0.42, 2.3),
            ]
        } else {
            [
                shades.tint(theme.green(), 0.3, 1.7),
                shades.tint(theme.green(), 0.55, 2.3),
                // a tint keeps no floor, so the wheat takes the palette's own yellow, which a
                // light theme with a 7 to 1 floor would otherwise swap for a browner colour
                shades.tint(theme.ansi[3], 0.35, 1.7),
            ]
        };
        let hedge = if theme.dark {
            shades.tint(theme.green(), 0.18, 1.4)
        } else {
            shades.tint(theme.green(), 0.75, 2.6)
        };
        let verge = if theme.dark {
            shades.tint(theme.green(), 0.32, 1.75)
        } else {
            shades.tint(theme.green(), 0.4, 1.9)
        };
        // the trees by the line are background too, so the train stands out in front of them
        let (pine, oak) = if theme.dark {
            (
                shades.tint(theme.green(), 0.5, 2.7),
                shades.tint(theme.bright_green(), 0.7, 2.95),
            )
        } else {
            (
                shades.tint(theme.green(), 0.9, 2.95),
                shades.tint(theme.green(), 0.7, 2.6),
            )
        };
        let ballast = if theme.dark {
            shades.tint(theme.grey(), 0.5, 2.2)
        } else {
            shades.tint(theme.grey(), 0.3, 1.55)
        };
        let sleeper = if theme.dark {
            shades.tint(theme.yellow(), 0.15, 1.3)
        } else {
            shades.tint(theme.yellow(), 0.85, 2.9)
        };
        Self {
            sky_top,
            sky_low,
            cloud,
            cloud_shade,
            hills,
            hill_trees,
            fields,
            hedge,
            verge,
            ballast,
            sleeper,
            rail: theme.grey(),
            // a daylit sky on a light theme has a pale sun in a warm glow, never a dark disc
            sun: if theme.dark {
                theme.bright_yellow()
            } else {
                theme.background
            },
            halo: if theme.dark {
                theme.bright_yellow()
            } else {
                mix(theme.background, shades.tint(theme.warning, 0.55, 2.5), 0.6)
            },
            dark: theme.dark,
            pine,
            oak,
            trunk: shades.tint(theme.yellow(), 0.6, 2.4),
            fence: theme.yellow(),
            smoke: theme.foreground,
            engine: theme.foreground,
            frame: theme.red(),
            window: theme.bright_yellow(),
            wheel: theme.grey(),
            carriages: [theme.red(), theme.bright_blue()],
            sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
            background: theme.background,
        }
    }
}

#[derive(Clone, Debug)]
struct Puff {
    /// Position in the air in pixels, which moves back past the train at the train's own speed.
    x: f32,
    y: f32,
    age: f32,
}

#[derive(Clone, Debug)]
struct Cloud {
    x: f32,
    y: f32,
    puffs: Vec<(f32, f32)>,
}

pub struct Countryside {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// How far the train has travelled, in columns of the nearest layer.
    travelled: f64,
    puffs: Vec<Puff>,
    next_puff: f32,
    clouds: Vec<Cloud>,
}

impl Countryside {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut country = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            travelled: 0.0,
            puffs: Vec::new(),
            next_puff: 0.0,
            clouds: Vec::new(),
        };
        country.resize(width, height, rng);
        country
    }

    /// The row the rails run along.
    fn rails(&self) -> i32 {
        self.height as i32 - 2
    }

    /// The pixel row of the rails' top.
    fn rails_px(&self) -> i32 {
        self.rails() * 2
    }

    /// The pixel row of the far hills' crest at a world column.
    fn hill_top(&self, wx: f32) -> i32 {
        let h = self.height as f32 * 2.0;
        let wave = (wx * 0.07).sin() + 0.6 * (wx * 0.023 + 1.3).sin();
        let amp = (h * 0.06).max(2.0);
        ((h * 0.4 + amp * wave).round() as i32).clamp(2, (self.rails_px() - 8).max(2))
    }

    fn carriages(&self) -> usize {
        let room = (self.width as f32 * 0.7) as i32;
        ((room - LOCO_WIDTH).max(0) / (CARRIAGE_WIDTH + 1)).min(4) as usize
    }

    fn train_width(&self) -> i32 {
        LOCO_WIDTH + self.carriages() as i32 * (CARRIAGE_WIDTH + 1)
    }

    /// The train's left edge: near the middle, easing forward and back a little so it looks
    /// like it is working rather than painted on.
    fn train_x(&self) -> i32 {
        let centre = (self.width as i32 - self.train_width()) / 2;
        let ease = cycle(self.t, TAU / 60.0).sin() * self.width as f32 * 0.06;
        centre + ease.round() as i32
    }

    /// The top of the train's sprites, so its wheels sit on the rails.
    fn train_top(&self) -> i32 {
        self.rails_px() - LOCO.len() as i32
    }

    fn chimney(&self) -> (f32, f32) {
        let loco = self.train_x() + self.train_width() - LOCO_WIDTH;
        ((loco + CHIMNEY) as f32, (self.train_top() - 1) as f32)
    }

    fn puff_stage(&self) -> f32 {
        PUFF_STAGE / self.motion.pace()
    }

    fn offset(&self, factor: f64) -> f64 {
        self.travelled * factor
    }

    /// The column something placed at a world column lands in, on a layer moving at `factor`.
    fn layer_x(&self, wx: i64, factor: f64) -> i32 {
        (wx as f64 - self.offset(factor)).round() as i32
    }

    fn paint_sky(&self, canvas: &mut Canvas, c: &CountryColours) {
        let wide = canvas.width() as i32;
        let horizon = (self.height as f32 * 2.0 * 0.5).max(1.0);
        for y in 0..canvas.height() as i32 {
            canvas.fill_row(y, c.sky.at(y as f32 / horizon));
        }
        // a still sun, which can never flash
        if self.height >= 14 && self.width >= 40 {
            let (cx, cy, r) = (wide as f32 - 9.0, 5.0_f32, 2.6_f32);
            for y in 0..10 {
                for x in (cx as i32 - 5)..(cx as i32 + 6) {
                    let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                    if d <= r {
                        canvas.set(x, y, c.sun);
                    } else if d <= r + 1.5 {
                        canvas.blend(x, y, c.halo, if c.dark { 0.3 } else { 0.6 });
                    }
                }
            }
        }
        for cloud in &self.clouds {
            for &(dx, r) in &cloud.puffs {
                let (cx, cy) = (cloud.x + dx, cloud.y - r * 0.6);
                for py in (cy - r).floor() as i32..=cloud.y as i32 {
                    for px in (cx - r).floor() as i32..=(cx + r).ceil() as i32 {
                        let (ox, oy) = (px as f32 + 0.5 - cx, py as f32 + 0.5 - cy);
                        if ox * ox + oy * oy <= r * r {
                            let low = py as f32 + 0.5 > cloud.y - 1.2;
                            canvas.set(px, py, if low { c.cloud_shade } else { c.cloud });
                        }
                    }
                }
            }
        }
    }

    fn paint_land(&self, canvas: &mut Canvas, c: &CountryColours) {
        let w = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let rails = self.rails_px();

        // the far hills, slowest of all, with a few small trees along the crest
        let hill_offset = self.offset(0.15) as f32;
        for x in 0..w {
            let wx = x as f32 + hill_offset;
            let top = self.hill_top(wx);
            for y in top..tall {
                canvas.set(x, y, c.hills);
            }
            if hash(wx.floor() as i64, 1) % 11 == 0 {
                canvas.fill_rect(x, top - 2, 1, 2, c.hill_trees);
                canvas.set(x - 1, top - 1, c.hill_trees);
            }
        }

        // fields between the hills and the line in bands, each band sliding a little faster than
        // the one above it and a little taller, so the ground seems to stretch away
        let near_top = rails - 3;
        let crest = (self.height as f32 * 2.0 * 0.48) as i32 + 2;
        let mut y = crest;
        let mut band = 0;
        while y < near_top {
            let depth = (y - crest) as f64 / (near_top - crest).max(1) as f64;
            let factor = 0.25 + 0.4 * depth;
            let tall_band = 3 + (depth * 5.0) as i32;
            let offset = self.offset(factor);
            let field_wide = 18 + (depth * 24.0) as i64;
            for x in 0..w {
                let wx = (x as f64 + offset).floor() as i64;
                let field = wx.div_euclid(field_wide);
                // mostly grass and pasture, with a field of wheat here and there
                let crop = c.fields[[0, 1, 0, 1, 2][(hash(field, band) % 5) as usize]];
                for dy in 0..tall_band.min(near_top - y) {
                    canvas.set(x, y + dy, crop);
                }
                // a hedgerow along the top of the band, with a gap at each field's gate
                let gate = wx.rem_euclid(field_wide) < 2 && hash(field, band + 100) % 2 == 0;
                if !gate {
                    canvas.set(x, y, c.hedge);
                }
                // a hedge tree stands up out of the hedgerow now and then
                if hash(wx, band + 7) % 37 == 0 && y >= 2 {
                    canvas.fill_rect(x, y - 2, 2, 2, c.hedge);
                }
            }
            y += tall_band;
            band += 1;
        }
        // the grass verge along the line
        for y in near_top.max(0)..rails {
            canvas.fill_row(y, c.verge);
        }
    }

    fn paint_trees(&self, canvas: &mut Canvas, c: &CountryColours) {
        let w = canvas.width() as i32;
        let ground = self.rails_px() - 1;
        let factor = 0.7;
        let start = self.offset(factor).floor() as i64 - 10;
        let size = (self.height as i32 * 2 / 5).clamp(6, 18);
        for wx in start..start + i64::from(w) + 20 {
            let h = hash(wx, 7);
            if h % 9 != 0 {
                continue;
            }
            let x = self.layer_x(wx, factor);
            let tall = size - (h >> 8) as i32 % 3;
            let trunk_tall = (tall / 4).max(1);
            let crown_bottom = ground - trunk_tall;
            canvas.fill_rect(x, crown_bottom + 1, 1, trunk_tall, c.trunk);
            if h % 2 == 0 {
                // a pine: tiers that widen downwards, shaded on the right
                let crown = tall - trunk_tall;
                for i in 0..crown {
                    let y = crown_bottom - crown + 1 + i;
                    let half = (1 + (i % 4) + i / 4) * 2 / 3;
                    for dx in -half..=half {
                        let colour = if dx > 0 && dx == half {
                            mix(c.pine, c.hedge, 0.5)
                        } else {
                            c.pine
                        };
                        canvas.set(x + dx, y, colour);
                    }
                }
            } else {
                // an oak: a round crown, lit from the top left and shaded at the bottom right
                let r = (tall - trunk_tall) as f32 / 2.0 + 0.5;
                let (cx, cy) = (x as f32 + 0.5, crown_bottom as f32 - r + 1.5);
                let reach = r.ceil() as i32 + 1;
                for dy in -reach..=reach {
                    for dx in -reach..=reach {
                        let (ox, oy) = (dx as f32, dy as f32);
                        let lumpy = r * (1.0 + 0.12 * (ox * 1.7 + oy).sin());
                        if ox * ox + oy * oy * 1.1 > lumpy * lumpy {
                            continue;
                        }
                        let colour = if ox + oy > r * 0.5 {
                            mix(c.oak, c.hedge, 0.45)
                        } else if ox + oy < -r * 0.9 {
                            mix(c.oak, c.cloud, 0.2)
                        } else {
                            c.oak
                        };
                        canvas.set((cx + ox) as i32, (cy + oy) as i32, colour);
                    }
                }
            }
        }
    }

    fn paint_train(&self, canvas: &mut Canvas, c: &CountryColours) {
        if self.rails() < 3 {
            return;
        }
        let top = self.train_top();
        let mut x = self.train_x();
        for i in 0..self.carriages() {
            let body = c.carriages[i % 2];
            let palette = [
                ('t', mix(c.wheel, c.engine, 0.3)),
                ('b', body),
                ('w', c.window),
                ('d', mix(body, c.wheel, 0.6)),
                ('k', c.wheel),
            ];
            canvas.sprite(x, top + 2, &CARRIAGE, &palette);
            // the coupling to the next vehicle
            canvas.set(x + CARRIAGE_WIDTH, top + 7, c.wheel);
            x += CARRIAGE_WIDTH + 1;
        }
        let palette = [
            ('b', c.engine),
            ('d', mix(c.engine, c.wheel, 0.5)),
            ('c', c.wheel),
            ('w', c.window),
            ('r', c.frame),
            ('k', c.wheel),
            ('h', c.engine),
            ('l', c.window),
        ];
        canvas.sprite(x, top, &LOCO, &palette);
    }

    fn paint_track(&self, canvas: &mut Canvas, c: &CountryColours) {
        let w = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let rails = self.rails_px();
        let offset = self.offset(1.0);
        for x in 0..w {
            let wx = (x as f64 + offset).floor() as i64;
            canvas.set(x, rails, c.rail);
            let colour = if wx.rem_euclid(3) == 0 {
                c.sleeper
            } else {
                c.ballast
            };
            canvas.set(x, rails + 1, colour);
        }
        // the fence nearest of all: a rail along the top and posts down to the ground
        let fence = rails + 2;
        if fence < tall {
            let offset = self.offset(1.4);
            for x in 0..w {
                let wx = (x as f64 + offset).floor() as i64;
                canvas.set(x, fence, c.verge);
                canvas.set(x, fence + 1, c.verge);
                if wx.rem_euclid(7) == 0 {
                    canvas.fill_rect(x, fence, 1, 2, c.fence);
                } else {
                    canvas.set(x, fence, mix(c.fence, c.verge, 0.25));
                }
            }
        }
    }

    fn paint_smoke(&self, canvas: &mut Canvas, c: &CountryColours) {
        let stage = self.puff_stage();
        for puff in &self.puffs {
            let i = ((puff.age / stage) as usize).min(PUFFS.len() - 1);
            let (r, strength) = PUFFS[i];
            let reach = r.ceil() as i32;
            let (cx, cy) = (puff.x, puff.y);
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    let (px, py) = (cx.round() as i32 + dx, cy.round() as i32 + dy);
                    let d =
                        ((px as f32 + 0.5 - cx).powi(2) + (py as f32 + 0.5 - cy).powi(2)).sqrt();
                    if d <= r {
                        // a puff is denser in the middle and lit from above
                        let edge = if d > r - 0.8 { 0.7 } else { 1.0 };
                        let lift = if py as f32 + 0.5 < cy { 1.0 } else { 0.85 };
                        let amount = strength * edge * lift;
                        if amount >= 0.5 {
                            canvas.blend(px, py, c.smoke, amount);
                        } else if let Some(under) = canvas.get(px, py) {
                            // a thin wisp is imagery, held below the ceiling, so only the dense
                            // middle of a puff stands out as a shape
                            let wisp = paint::held_imagery(c.background, under, c.smoke, amount);
                            canvas.set(px, py, wisp);
                        }
                    }
                }
            }
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &CountryColours) {
        self.paint_sky(canvas, c);
        self.paint_land(canvas, c);
        self.paint_trees(canvas, c);
        self.paint_smoke(canvas, c);
        self.paint_train(canvas, c);
        self.paint_track(canvas, c);
    }
}

impl Scene for Countryside {
    fn name(&self) -> &'static str {
        "countryside"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.puffs.clear();
        let clouds = (self.width / 30).clamp(1, 6);
        let sky = (self.height as f32 * 2.0 * 0.3).max(3.0);
        let scale = if self.height >= 20 { 1.0 } else { 0.6 };
        self.clouds = (0..clouds)
            .map(|_| {
                let puffs = 3 + rng.below(3);
                Cloud {
                    x: rng.range(0.0, self.width as f32),
                    y: rng.range(3.0, sky),
                    puffs: (0..puffs)
                        .map(|i| {
                            let middle = 1.0 - (i as f32 / (puffs - 1) as f32 - 0.5).abs();
                            (
                                i as f32 * 3.0 * scale,
                                (1.4 + 2.0 * middle + rng.range(0.0, 0.7)) * scale,
                            )
                        })
                        .collect(),
                }
            })
            .collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = TRACK_SPEED * self.motion.speed();
        self.travelled += f64::from(speed * dt);
        let width = self.width as f32;
        for cloud in &mut self.clouds {
            cloud.x = wrap(
                cloud.x - 0.15 * self.motion.speed() * dt + 20.0,
                width + 40.0,
            ) - 20.0;
        }

        let stage = self.puff_stage();
        for puff in &mut self.puffs {
            puff.age += dt;
            puff.x -= speed * 0.6 * dt;
            puff.y -= 1.2 * self.motion.speed() * dt;
        }
        self.puffs.retain(|p| p.age < stage * 3.0 && p.y > -3.0);
        self.next_puff -= dt;
        if self.next_puff <= 0.0 && self.rails() >= 5 {
            self.next_puff = if self.motion.calm { 1.6 } else { 0.8 } / self.motion.busy();
            let (x, y) = self.chimney();
            self.puffs.push(Puff {
                x: x + 0.5 + rng.range(-0.3, 0.3),
                y,
                age: 0.0,
            });
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = CountryColours::new(theme);
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

    fn drawn(country: &Countryside, width: usize, height: usize, theme: &ThemeData) -> Frame {
        let mut frame = Frame::new(width, height);
        country.draw(&mut frame, theme);
        frame
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("countryside");
        check_determinism("countryside");
        check_pixel_scene("countryside", 6.0);
    }

    #[test]
    fn sprites_are_rectangular() {
        for rows in [&LOCO[..], &CARRIAGE[..]] {
            assert!(rows.iter().all(|r| r.len() == rows[0].len()), "{rows:?}");
        }
        assert_eq!(LOCO[0].len() as i32, LOCO_WIDTH);
        assert_eq!(CARRIAGE[0].len() as i32, CARRIAGE_WIDTH);
        assert_eq!(LOCO[0].as_bytes()[CHIMNEY as usize], b'c');
    }

    #[test]
    fn the_land_slides_past_but_the_train_stays_in_view() {
        const { assert!(PUFF_STAGE >= 1.0 / 3.0) };
        let theme = theme::default_theme();
        let c = CountryColours::new(theme);
        let mut rng = Rng::new(1);
        let mut country = Countryside::new(100, 30, Motion::LIVELY, &mut rng);
        for _ in 0..(120 * 20) {
            country.step(0.05, &mut rng);
            let x = country.train_x();
            assert!(x >= 0 && x + country.train_width() <= 100, "{x}");
        }
        assert!(country.travelled > 100.0);
        let frame = render(&mut country, 100, 30, 10);
        let all = pixels(&frame);
        for colour in [c.engine, c.frame, c.window, c.carriages[0], c.carriages[1]] {
            assert!(all.contains(&colour), "{}", colour.hex());
        }
        // the rails along row 28 and the fence's posts in the bottom row
        let rails: Vec<(Rgb, Rgb)> = (0..100)
            .filter_map(|x| frame.get(x, 28).and_then(|cell| cell.pixel_pair()))
            .collect();
        assert!(rails.iter().all(|&(top, _)| top == c.rail));
        assert!((0..100).any(
            |x| frame.get(x, 29).and_then(|cell| cell.pixel_pair()) == Some((c.fence, c.fence))
        ));
    }

    #[test]
    fn a_small_screen_gets_just_the_engine() {
        let theme = theme::default_theme();
        let c = CountryColours::new(theme);
        let mut rng = Rng::new(2);
        let mut country = Countryside::new(20, 8, Motion::CALM, &mut rng);
        assert_eq!(country.carriages(), 0);
        let frame = render(&mut country, 20, 8, 10);
        assert!(pixels(&frame).contains(&c.frame));
    }

    #[test]
    fn the_colours_stay_apart_and_in_their_bands_in_every_theme() {
        for theme in theme::all() {
            let c = CountryColours::new(theme);
            let name = theme.name;
            apart(
                theme,
                c.sky_top,
                c.sky_low,
                0.03,
                "the sky's top and bottom",
            );
            apart(theme, c.hills, c.sky_low, 0.03, "the hills and the sky");
            apart(
                theme,
                c.fields[0],
                c.fields[1],
                0.03,
                "the grass and the pasture",
            );
            apart(
                theme,
                c.fields[0],
                c.fields[2],
                0.03,
                "the grass and the wheat",
            );
            apart(
                theme,
                c.hedge,
                c.fields[0],
                0.03,
                "the hedges and the fields",
            );
            apart(
                theme,
                c.hills,
                c.fields[0],
                0.03,
                "the hills and the fields",
            );
            apart(theme, c.ballast, c.verge, 0.03, "the ballast and the verge");
            apart(theme, c.pine, c.verge, 0.03, "the pines and the verge");
            apart(theme, c.oak, c.verge, 0.03, "the oaks and the verge");
            for (what, colour) in [
                ("sky top", c.sky_top),
                ("sky low", c.sky_low),
                ("hills", c.hills),
                ("hedge", c.hedge),
                ("verge", c.verge),
                ("ballast", c.ballast),
                ("sleeper", c.sleeper),
                ("pine", c.pine),
                ("oak", c.oak),
                ("trunk", c.trunk),
            ]
            .into_iter()
            .chain(c.fields.iter().map(|&f| ("field", f)))
            {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            for colour in [c.engine, c.frame, c.fence, c.rail]
                .into_iter()
                .chain(c.carriages)
            {
                assert!(theme.contrast_of(colour) >= theme.floor(), "{name}");
                apart(theme, colour, c.verge, 0.1, "a shape and the verge");
                apart(theme, colour, c.fields[0], 0.1, "a shape and the fields");
                apart(theme, colour, c.sky_low, 0.1, "a shape and the sky");
            }
            apart(
                theme,
                c.carriages[0],
                c.carriages[1],
                0.08,
                "the two carriages",
            );
            apart(
                theme,
                c.window,
                c.carriages[0],
                0.08,
                "a window and its carriage",
            );
            apart(
                theme,
                c.window,
                c.carriages[1],
                0.08,
                "a window and its carriage",
            );
        }
    }

    #[test]
    fn the_256_colour_fallback_keeps_the_bands_apart() {
        for theme in theme::all() {
            let c = CountryColours::new(theme);
            let near = |colour: Rgb| xterm_colour(pixel_256(colour, theme.background));
            let mut rng = Rng::new(8);
            let mut country = Countryside::new(80, 24, Motion::FULL, &mut rng);
            for _ in 0..60 {
                country.step(0.05, &mut rng);
            }
            let frame = drawn(&country, 80, 24, theme);
            let worst = pixels(&frame)
                .into_iter()
                .map(|colour| distance(colour, near(colour)))
                .fold(0.0_f32, f32::max);
            assert!(worst <= 0.15, "{} is {worst:.3} off", theme.name);
            assert_ne!(near(c.hills), near(c.sky_low), "{}", theme.name);
            assert_ne!(near(c.fields[0]), near(c.fields[2]), "{}", theme.name);
            assert_ne!(near(c.ballast), near(c.verge), "{}", theme.name);
            for colour in [c.engine, c.carriages[0], c.carriages[1]] {
                assert_ne!(near(colour), near(c.verge), "{}", theme.name);
            }
        }
    }
}
