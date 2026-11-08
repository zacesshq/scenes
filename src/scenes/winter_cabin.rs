//! Winter cabin: a log cabin in a snowy clearing at night, painted in solid pixels. Its window
//! is lit and its light spills onto the snow in front, smoke curls from a stone chimney, snow lies
//! thick on the roof and on the branches of the pines near and far, snow falls gently and a moon
//! hangs over the trees.
//!
//! A light theme shows the same clearing on a winter afternoon: a pale blue sky with a faint
//! daytime moon, white flakes with a soft cool edge drifting down and the window warm against the
//! logs.
//!
//! Every colour comes from the active theme. The sky, the far pines and the snow on the ground
//! are tints of the background held below the non-text floor, so they read as background; the
//! cabin, the near pines, the moon and the falling snow keep the theme's own drawing colours, so
//! they stand out as shapes and keep their outline when colour is off.

use std::f32::consts::TAU;

use super::{count_for, cycle, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, distance, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "A log cabin in the snow at night, with a lit window, chimney smoke, snowy pines and falling snow, in the afternoon in light themes";

pub const CAPTIONS: &[&str] = &[
    "Warm inside. The build passed.",
    "Snowed in with a clean build.",
    "The fire is lit and the tests are green.",
    "Put the kettle on. Nothing to fix.",
    "Quiet night, tidy code.",
];

/// Seconds a puff of smoke spends at each of its three sizes.
pub const PUFF_STAGE: f32 = 1.2;

/// Steps in the sky ramp.
const SKY_STEPS: usize = 10;
/// The puffs' radius at each stage and how strongly each covers what is behind it.
const PUFFS: [(f32, f32); 3] = [(0.9, 0.75), (1.6, 0.55), (2.3, 0.35)];

/// A small deterministic hash, for the fixed texture of the snow.
fn hash(a: i64, b: i64) -> u64 {
    let mut z = (a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (b as u64).wrapping_mul(0xD1B5_4A32_D192_ED03);
    z = (z ^ (z >> 29)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^ (z >> 32)
}

/// The cabin's size in pixels: its width, the height of its log walls and of its roof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CabinSize {
    wide: i32,
    walls: i32,
    roof: i32,
}

const LARGE_CABIN: CabinSize = CabinSize {
    wide: 26,
    walls: 9,
    roof: 7,
};
const SMALL_CABIN: CabinSize = CabinSize {
    wide: 14,
    walls: 5,
    roof: 4,
};

/// The cabin night's colours in one theme, every one derived from the theme's own. Some are kept
/// only so the tests can check the bands against each other.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct CabinColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub snow: Rgb,
    pub snow_shade: Rgb,
    pub far_pine: Rgb,
    pub far_snow: Rgb,
    pub spill: Rgb,
    pub star: Rgb,
    pub moon: Rgb,
    pub logs: Rgb,
    pub log_line: Rgb,
    pub roof: Rgb,
    pub roof_snow: Rgb,
    pub chimney: Rgb,
    pub window: Rgb,
    pub door: Rgb,
    pub pine: Rgb,
    pub pine_shade: Rgb,
    pub trunk: Rgb,
    pub flake: Rgb,
    pub far_flake: Rgb,
    pub smoke: Rgb,
    pub flake_edge: Option<Rgb>,
    sky: Ramp,
    stars: bool,
}

/// Pure white, the falling snow by day: a touch above an off-white background, so it stays
/// imagery while showing against the blue.
const WHITE: Rgb = Rgb::new(255, 255, 255);

impl CabinColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        // a night sky on a dark theme and a pale winter dusk on a light one; the snow on the
        // ground is the lightest imagery either way, with blue shadows in its hollows
        let (sky_top, sky_low, snow, snow_shade, far_pine, far_snow) = if theme.dark {
            (
                shades.tint(theme.info, 0.12, 1.25),
                shades.tint(theme.blue(), 0.35, 1.8),
                shades.tint(mix(theme.bright(), theme.info, 0.25), 0.5, 2.8),
                shades.tint(theme.info, 0.45, 2.2),
                shades.tint(theme.green(), 0.15, 1.25),
                shades.tint(theme.bright(), 0.3, 2.0),
            )
        } else {
            (
                shades.tint(theme.info, 0.45, 2.2),
                shades.tint(theme.info, 0.18, 1.4),
                shades.tint(theme.info, 0.04, 1.06),
                shades.tint(theme.info, 0.2, 1.35),
                shades.tint(theme.green(), 0.85, 2.95),
                shades.tint(theme.info, 0.08, 1.12),
            )
        };
        let spill = if theme.dark {
            let spill = shades.tint(theme.bright_yellow(), 0.6, 2.9);
            // a pale bright yellow, as on Warm Dark, would match the snow, so the light takes
            // the deeper yellow there
            if distance(spill, snow) < 0.05 {
                shades.tint(theme.yellow(), 0.6, 2.9)
            } else {
                spill
            }
        } else {
            shades.tint(theme.yellow(), 0.25, 1.35)
        };
        // snow is white on a dark theme; on a light one white would vanish into the page, so
        // lying snow is the pale imagery and falling snow a soft blue grey
        // a lit window glows brighter than the logs around it, which on a light theme means a
        // pale pane in a dark wall
        let window = if theme.dark {
            theme.bright_yellow()
        } else {
            shades.tint(theme.yellow(), 0.35, 1.5)
        };
        // falling snow is white in every theme; by day each near flake has a soft cool edge, so
        // it still shows where it passes over the lying snow
        let (roof_snow, flake, flake_edge) = if theme.dark {
            (theme.bright(), theme.bright(), None)
        } else {
            (snow, WHITE, Some(shades.tint(theme.info, 0.35, 1.45)))
        };
        // the logs are a warm brown, between the theme's yellow and red
        let logs = if theme.dark {
            let brown = mix(theme.yellow(), theme.red(), 0.35);
            // a theme whose red is already warm, such as Warm Dark, needs the logs nearer the
            // yellow to tell them from the roof
            if distance(brown, theme.red()) < 0.1 {
                mix(theme.yellow(), theme.red(), 0.1)
            } else {
                brown
            }
        } else {
            theme.yellow()
        };
        let pine = theme.green();
        Self {
            sky_top,
            sky_low,
            snow,
            snow_shade,
            far_pine,
            far_snow,
            spill,
            star: theme.muted,
            // a full moon at night; by day a faint white moon in the blue
            moon: if theme.dark {
                theme.bright_yellow()
            } else {
                mix(WHITE, sky_low, 0.15)
            },
            flake_edge,
            logs,
            log_line: mix(logs, theme.background, 0.45),
            roof: theme.red(),
            roof_snow,
            chimney: theme.grey(),
            window,
            door: mix(logs, theme.background, 0.3),
            pine,
            pine_shade: mix(pine, theme.background, 0.35),
            trunk: mix(logs, theme.background, 0.25),
            flake,
            far_flake: if theme.dark { theme.muted } else { snow_shade },
            smoke: theme.grey(),
            sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
            stars: theme.dark,
        }
    }
}

#[derive(Clone, Debug)]
struct Pine {
    x: i32,
    /// The pixel row its trunk stands on.
    base: i32,
    /// Its height in pixels.
    size: i32,
    near: bool,
    snow: u64,
}

#[derive(Clone, Debug)]
struct Flake {
    x: f32,
    y: f32,
    speed: f32,
    phase: f32,
    near: bool,
}

#[derive(Clone, Debug)]
struct Puff {
    x: f32,
    y: f32,
    age: f32,
    phase: f32,
}

pub struct WinterCabin {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    pines: Vec<Pine>,
    flakes: Vec<Flake>,
    puffs: Vec<Puff>,
    next_puff: f32,
    stars: Vec<(i32, i32)>,
    /// How far the snow's surface rises above the ground line in each column, in pixels.
    drifts: Vec<i32>,
}

impl WinterCabin {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut scene = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            pines: Vec::new(),
            flakes: Vec::new(),
            puffs: Vec::new(),
            next_puff: 0.0,
            stars: Vec::new(),
            drifts: Vec::new(),
        };
        scene.resize(width, height, rng);
        scene
    }

    /// The row of the snow's surface.
    fn ground(&self) -> i32 {
        ((self.height as f32 * 0.74) as i32).clamp(1, (self.height as i32 - 1).max(1))
    }

    /// The pixel row of the snow's surface.
    fn ground_px(&self) -> i32 {
        self.ground() * 2
    }

    fn cabin(&self) -> CabinSize {
        if self.height >= 16 && self.width >= 40 {
            LARGE_CABIN
        } else {
            SMALL_CABIN
        }
    }

    /// The cabin's left column and the top pixel row of its walls.
    fn cabin_at(&self) -> (i32, i32) {
        let size = self.cabin();
        let x = (self.width as i32 * 2 / 5 - size.wide / 2).max(0);
        (x, self.ground_px() - size.walls)
    }

    /// The top of the chimney, where the smoke leaves it, in pixels.
    fn chimney(&self) -> (f32, f32) {
        let (x, top) = self.cabin_at();
        let size = self.cabin();
        let cx = x + size.wide * 2 / 3;
        let roof_at =
            top - size.roof + (cx - x - size.wide / 2).abs() * size.roof / (size.wide / 2);
        (cx as f32 + 0.5, (roof_at - 3) as f32)
    }

    fn puff_stage(&self) -> f32 {
        PUFF_STAGE / self.motion.pace()
    }

    fn new_flake(&self, rng: &mut Rng, from_top: bool) -> Flake {
        let near = rng.chance(0.35);
        Flake {
            x: rng.range(0.0, self.width as f32),
            y: if from_top {
                -rng.range(0.0, 4.0)
            } else {
                rng.range(0.0, self.ground_px() as f32)
            },
            speed: if near {
                rng.range(2.4, 3.6)
            } else {
                rng.range(1.2, 2.0)
            } * self.motion.speed(),
            phase: rng.range(0.0, TAU),
            near,
        }
    }

    /// Half the width of a pine's row: the branches come in tiers, each tier a little wider than
    /// the last, which reads as a fir rather than a plain triangle.
    fn half_width(row: i32, size: i32) -> i32 {
        let tier = (size / 4).max(2);
        (row % tier) * 2 / 3 + row / tier + i32::from(row > 0)
    }

    fn paint_pine(canvas: &mut Canvas, c: &CabinColours, pine: &Pine) {
        let (body, shade, snow) = if pine.near {
            (c.pine, c.pine_shade, c.roof_snow)
        } else {
            (c.far_pine, c.far_pine, c.far_snow)
        };
        let top = pine.base - pine.size;
        let tier = (pine.size / 4).max(2);
        let trunk = (pine.size / 6).max(1);
        canvas.fill_rect(pine.x, pine.base - trunk + 1, 1, trunk, c.trunk);
        for i in 0..pine.size - trunk + 1 {
            let y = top + i;
            let hw = Self::half_width(i, pine.size);
            for dx in -hw..=hw {
                // snow rests on the top of each tier, unevenly
                let resting = i % tier == 0 || (i % tier == 1 && dx.abs() == hw);
                let patchy = (pine.snow >> ((i * 5 + dx + 40).rem_euclid(60))) & 3 != 0;
                let colour = if resting && (patchy || i == 0) {
                    snow
                } else if dx > 0 {
                    shade
                } else {
                    body
                };
                canvas.set(pine.x + dx, y, colour);
            }
        }
    }

    fn paint_cabin(&self, canvas: &mut Canvas, c: &CabinColours) {
        let size = self.cabin();
        let (x, top) = self.cabin_at();
        let ground = self.ground_px();

        // the chimney stands behind the roof's ridge line
        let (cx, cy) = self.chimney();
        let (cx, cy) = (cx as i32, cy as i32);
        canvas.fill_rect(cx - 1, cy + 1, 3, top - cy, c.chimney);
        canvas.fill_rect(cx - 1, cy + 1, 3, 1, c.roof_snow);

        // the log walls with dark lines between the logs and round log ends at the corners
        canvas.fill_rect(x + 1, top, size.wide - 2, size.walls, c.logs);
        for y in (top + 1..ground).step_by(2) {
            canvas.fill_rect(x + 1, y, size.wide - 2, 1, c.log_line);
        }
        for y in (top..ground).step_by(2) {
            canvas.set(x, y, c.logs);
            canvas.set(x + size.wide - 1, y, c.logs);
        }

        // the lit window and the door
        let large = size == LARGE_CABIN;
        let (wx, wy, ww, wh) = if large {
            (x + size.wide - 10, top + 2, 6, 4)
        } else {
            (x + size.wide - 6, top + 1, 3, 2)
        };
        canvas.fill_rect(wx, wy, ww, wh, c.window);
        if large {
            // the glazing bars split the window into four panes
            canvas.fill_rect(wx + ww / 2, wy, 1, wh, c.log_line);
            canvas.fill_rect(wx, wy + wh / 2, ww, 1, c.log_line);
        }
        let (dx, dw) = if large { (x + 4, 4) } else { (x + 2, 3) };
        let door_top = ground - if large { 7 } else { 4 };
        canvas.fill_rect(dx, door_top, dw, ground - door_top, c.door);
        canvas.set(dx + dw - 1, door_top + (ground - door_top) / 2, c.window);

        // the roof, overhanging the walls, with a thick layer of snow on it
        for i in 0..size.roof {
            let y = top - size.roof + i;
            let half = (i + 1) * (size.wide / 2 + 1) / size.roof;
            let left = x + size.wide / 2 - half;
            let right = x + size.wide / 2 + half - 1;
            canvas.fill_rect(left, y, right - left + 1, 1, c.roof);
            // the snow lies on the upper face of the roof, thinning at the eaves
            let depth = if large { 2 } else { 1 };
            if i < size.roof - 1 {
                canvas.fill_rect(left, y, depth.min(right - left + 1), 1, c.roof_snow);
                canvas.fill_rect(right - depth + 1, y, depth, 1, c.roof_snow);
            }
        }
        let eaves = top - 1;
        canvas.fill_rect(x - 1, eaves, size.wide + 2, 1, c.roof);
        canvas.fill_rect(x - 1, eaves - 1, 2, 1, c.roof_snow);
        canvas.fill_rect(x + size.wide - 1, eaves - 1, 2, 1, c.roof_snow);
        canvas.set(x + size.wide / 2 - 1, top - size.roof - 1, c.roof_snow);
        canvas.set(x + size.wide / 2, top - size.roof - 1, c.roof_snow);
    }

    fn paint(&self, canvas: &mut Canvas, c: &CabinColours) {
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let ground = self.ground_px().min(tall);

        for y in 0..ground {
            canvas.fill_row(y, c.sky.at(y as f32 / ground.max(1) as f32));
        }
        if c.stars {
            for &(x, y) in &self.stars {
                canvas.set(x, y, c.star);
            }
        }
        // a full moon with a soft halo; it never changes
        if self.width >= 40 && self.height >= 14 {
            let (mx, my, r) = (wide as f32 - 9.0, 5.5_f32, 3.0_f32);
            for y in 0..12 {
                for x in (mx as i32 - 6)..(mx as i32 + 7) {
                    let d = ((x as f32 + 0.5 - mx).powi(2) + (y as f32 + 0.5 - my).powi(2)).sqrt();
                    if d <= r {
                        let crater = (x + y) % 4 == 0 && d < r - 1.0;
                        canvas.set(
                            x,
                            y,
                            if crater {
                                mix(c.moon, c.sky_top, 0.3)
                            } else {
                                c.moon
                            },
                        );
                    } else if d <= r + 2.0 {
                        canvas.blend(x, y, c.moon, 0.25 * (1.0 - (d - r) / 2.0));
                    }
                }
            }
        }

        for pine in self.pines.iter().filter(|p| !p.near) {
            Self::paint_pine(canvas, c, pine);
        }

        // the snow on the ground, rising and falling in drifts, with blue shadow in each hollow
        for x in 0..wide {
            let rise = self.drifts.get(x as usize).copied().unwrap_or(0);
            for y in (ground - rise).max(0)..tall {
                let depth = y - (ground - rise);
                let hollow = rise == 0 && depth < 2;
                let glint = depth > 1 && hash(i64::from(x), i64::from(y)) % 19 == 0;
                let colour = if hollow || glint {
                    c.snow_shade
                } else {
                    c.snow
                };
                canvas.set(x, y, colour);
            }
        }

        // smoke rises behind the falling snow but in front of the sky
        let stage = self.puff_stage();
        for puff in &self.puffs {
            let i = ((puff.age / stage) as usize).min(PUFFS.len() - 1);
            let (r, strength) = PUFFS[i];
            let reach = r.ceil() as i32;
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    let (px, py) = (puff.x.round() as i32 + dx, puff.y.round() as i32 + dy);
                    let d = ((px as f32 + 0.5 - puff.x).powi(2)
                        + (py as f32 + 0.5 - puff.y).powi(2))
                    .sqrt();
                    if d <= r {
                        canvas.blend(px, py, c.smoke, strength);
                    }
                }
            }
        }

        self.paint_cabin(canvas, c);

        // the window's light spills onto the snow in front of the cabin
        if ground + 1 < tall {
            let (x, _) = self.cabin_at();
            let size = self.cabin();
            let (cx, cy) = (x + size.wide - 7, ground + 1);
            let (rx, ry) = (
                size.wide as f32 / 3.0,
                ((tall - ground) as f32 * 0.5).max(1.0),
            );
            for py in cy..tall {
                for px in cx - rx as i32 - 1..=cx + rx as i32 + 1 {
                    let ox = (px - cx) as f32 / rx;
                    let oy = (py - cy) as f32 / ry;
                    let d = ox * ox + oy * oy;
                    if d < 1.0 {
                        canvas.blend(px, py, c.spill, 0.55 * (1.0 - d));
                    }
                }
            }
        }

        for pine in self.pines.iter().filter(|p| p.near) {
            Self::paint_pine(canvas, c, pine);
        }

        for flake in &self.flakes {
            let colour = if flake.near { c.flake } else { c.far_flake };
            let (x, y) = (flake.x.round() as i32, flake.y.floor() as i32);
            if let Some(edge) = c.flake_edge.filter(|_| flake.near) {
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    canvas.set(x + dx, y + dy, edge);
                }
            }
            canvas.set(x, y, colour);
        }
    }
}

impl Scene for WinterCabin {
    fn name(&self) -> &'static str {
        "winter-cabin"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let ground = self.ground_px();
        let (cabin_x, _) = self.cabin_at();
        let cabin_w = self.cabin().wide;

        // far pines stand along the edge of the clearing; near ones at the sides in the snow
        self.pines.clear();
        let near_size = ((self.height as f32 * 0.9) as i32).clamp(6, 22);
        let has_near = self.width >= 30 && self.height >= 10;
        let near_reach = if has_near {
            Self::half_width(near_size, near_size) * 2 + 2
        } else {
            0
        };
        let far_size = (ground / 3).clamp(4, 12);
        let mut x = rng.below(4) as i32;
        while x < self.width as i32 {
            let reach = Self::half_width(far_size, far_size) + 1;
            let clear_of_cabin = x + reach < cabin_x || x - reach > cabin_x + cabin_w;
            let clear_of_near =
                x - reach > near_reach && x + reach < self.width as i32 - 1 - near_reach;
            if clear_of_cabin && clear_of_near && rng.chance(0.75) {
                self.pines.push(Pine {
                    x,
                    base: ground,
                    size: far_size - rng.below(3) as i32,
                    near: false,
                    snow: rng.next_u64(),
                });
            }
            x += Self::half_width(far_size, far_size) + 1 + rng.below(4) as i32;
        }
        if has_near {
            let base = (self.height as i32 * 2 - 1).min(ground + 4);
            let reach = Self::half_width(near_size, near_size);
            for x in [reach, self.width as i32 - 1 - reach] {
                self.pines.push(Pine {
                    x,
                    base,
                    size: near_size,
                    near: true,
                    snow: rng.next_u64(),
                });
            }
        }

        let flakes = self.motion.scaled(count_for(
            self.width,
            self.ground().max(1) as usize,
            30,
            3,
            220,
        ));
        self.flakes = (0..flakes).map(|_| self.new_flake(rng, false)).collect();
        let sky = (ground - far_size - 2).max(1);
        let stars = count_for(self.width, (sky / 2).max(1) as usize, 70, 1, 60);
        self.stars = (0..stars)
            .map(|_| (rng.below(self.width) as i32, rng.below(sky as usize) as i32))
            .collect();
        let p = rng.range(0.0, TAU);
        self.drifts = (0..self.width)
            .map(|x| {
                let wave = (x as f32 * 0.11 + p).sin() + 0.5 * (x as f32 * 0.27 + p * 2.0).sin();
                (wave * 0.9).round().max(0.0) as i32
            })
            .collect();
        self.puffs.clear();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let ground = self.ground_px() as f32;
        let width = self.width as f32;
        for i in 0..self.flakes.len() {
            let flake = &mut self.flakes[i];
            flake.y += flake.speed * dt;
            let sway = (cycle(self.t, 0.6 * self.motion.speed()) + flake.phase).sin() * 0.6;
            flake.x = wrap(flake.x + sway * self.motion.speed() * dt, width);
            if flake.y >= ground {
                self.flakes[i] = self.new_flake(rng, true);
            }
        }

        let stage = self.puff_stage();
        let speed = self.motion.speed();
        for puff in &mut self.puffs {
            puff.age += dt;
            puff.y -= 1.0 * speed * dt;
            // a light breeze leans the smoke to the right as it climbs
            puff.x += (0.35 + 0.4 * (puff.age * 1.3 + puff.phase).sin()) * speed * dt;
        }
        self.puffs.retain(|p| p.age < stage * 3.0 && p.y >= -2.0);
        self.next_puff -= dt;
        if self.next_puff <= 0.0 {
            self.next_puff = if self.motion.calm { 2.0 } else { 1.1 } / self.motion.busy();
            let (x, y) = self.chimney();
            if y >= 2.0 {
                self.puffs.push(Puff {
                    x,
                    y,
                    age: 0.0,
                    phase: rng.range(0.0, TAU),
                });
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = CabinColours::new(theme);
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

    fn drawn(scene: &WinterCabin, width: usize, height: usize, theme: &ThemeData) -> Frame {
        let mut frame = Frame::new(width, height);
        scene.draw(&mut frame, theme);
        frame
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("winter-cabin");
        check_determinism("winter-cabin");
        check_pixel_scene("winter-cabin", 6.0);
    }

    #[test]
    fn the_cabin_window_is_lit_and_smoke_rises() {
        const { assert!(PUFF_STAGE >= 1.0 / 3.0) };
        let theme = theme::default_theme();
        let c = CabinColours::new(theme);
        let mut rng = Rng::new(4);
        let mut scene = WinterCabin::new(80, 24, Motion::FULL, &mut rng);
        let frame = render(&mut scene, 80, 24, 100);
        let all = pixels(&frame);
        for colour in [c.window, c.logs, c.roof, c.roof_snow, c.chimney, c.pine] {
            assert!(all.contains(&colour), "{}", colour.hex());
        }
        // smoke has risen above the chimney's top
        let (cx, cy) = scene.chimney();
        let smoky = scene
            .puffs
            .iter()
            .any(|p| p.y < cy - 2.0 && (p.x - cx).abs() < 20.0);
        assert!(smoky);
    }

    #[test]
    fn snow_never_falls_below_the_ground_and_small_screens_get_a_small_cabin() {
        let mut rng = Rng::new(7);
        let mut scene = WinterCabin::new(20, 8, Motion::LIVELY, &mut rng);
        assert_eq!(scene.cabin(), SMALL_CABIN);
        for _ in 0..600 {
            scene.step(0.05, &mut rng);
            assert!(scene.flakes.iter().all(|f| f.y < scene.ground_px() as f32));
        }
        let theme = theme::default_theme();
        let frame = render(&mut scene, 20, 8, 1);
        assert!(pixels(&frame).contains(&CabinColours::new(theme).window));
        assert_eq!(
            WinterCabin::new(80, 24, Motion::FULL, &mut rng).cabin(),
            LARGE_CABIN
        );
    }

    #[test]
    fn the_colours_stay_apart_and_in_their_bands_in_every_theme() {
        for theme in theme::all() {
            let c = CabinColours::new(theme);
            let name = theme.name;
            apart(
                theme,
                c.sky_top,
                c.sky_low,
                0.03,
                "the sky's top and bottom",
            );
            apart(theme, c.snow, c.sky_low, 0.03, "the snow and the sky");
            apart(
                theme,
                c.snow,
                c.snow_shade,
                0.03,
                "the snow and its shadows",
            );
            apart(
                theme,
                c.far_pine,
                c.sky_low,
                0.05,
                "the far pines and the sky",
            );
            apart(
                theme,
                c.far_pine,
                c.far_snow,
                0.05,
                "the far pines and their snow",
            );
            for (what, colour) in [
                ("sky top", c.sky_top),
                ("sky low", c.sky_low),
                ("snow", c.snow),
                ("snow shade", c.snow_shade),
                ("far pine", c.far_pine),
                ("far snow", c.far_snow),
                ("spill", c.spill),
            ] {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            if theme.dark {
                assert!(theme.contrast_of(c.flake) >= theme.floor(), "{name}");
                assert!(theme.contrast_of(c.window) >= theme.floor(), "{name}");
                apart(theme, c.roof_snow, c.roof, 0.1, "the roof and its snow");
                apart(theme, c.pine, c.roof_snow, 0.1, "a pine and its snow");
            } else {
                // by day the flakes and the moon are white imagery against the blue, each near
                // flake ringed by a cool edge that shows against the lying snow
                let edge = c.flake_edge.expect("a cool edge by day");
                for colour in [c.flake, edge, c.moon] {
                    assert!(theme.contrast_of(colour) < IMAGERY_CEILING, "{name}");
                }
                apart(
                    theme,
                    c.flake,
                    c.sky_top,
                    0.1,
                    "the falling snow and the sky",
                );
                apart(theme, c.moon, c.sky_top, 0.1, "the moon and the sky");
                apart(
                    theme,
                    edge,
                    c.snow,
                    0.05,
                    "a flake's edge and the lying snow",
                );
            }
            let mut solid = vec![c.logs, c.roof, c.pine, c.chimney];
            if theme.dark {
                solid.push(c.moon);
            }
            for colour in solid {
                assert!(theme.contrast_of(colour) >= theme.floor(), "{name}");
                apart(theme, colour, c.sky_top, 0.1, "a shape and the sky");
                apart(theme, colour, c.snow, 0.1, "a shape and the snow");
            }
            apart(theme, c.window, c.logs, 0.06, "the window and the logs");
            apart(theme, c.roof, c.logs, 0.08, "the roof and the logs");
            apart(theme, c.roof_snow, c.roof, 0.1, "the roof and its snow");
            apart(theme, c.pine, c.roof_snow, 0.1, "a pine and its snow");
            apart(
                theme,
                c.spill,
                c.snow,
                0.03,
                "the window's light and the snow",
            );
        }
    }

    #[test]
    fn the_256_colour_fallback_keeps_the_bands_apart() {
        for theme in theme::all() {
            let c = CabinColours::new(theme);
            let near = |colour: Rgb| xterm_colour(pixel_256(colour, theme.background));
            let mut rng = Rng::new(8);
            let mut scene = WinterCabin::new(80, 24, Motion::FULL, &mut rng);
            for _ in 0..60 {
                scene.step(0.05, &mut rng);
            }
            let frame = drawn(&scene, 80, 24, theme);
            let worst = pixels(&frame)
                .into_iter()
                .map(|colour| distance(colour, near(colour)))
                .fold(0.0_f32, f32::max);
            assert!(worst <= 0.15, "{} is {worst:.3} off", theme.name);
            assert_ne!(near(c.snow), near(c.sky_low), "{}", theme.name);
            assert_ne!(near(c.far_pine), near(c.sky_low), "{}", theme.name);
            assert_ne!(near(c.window), near(c.logs), "{}", theme.name);
        }
    }
}
