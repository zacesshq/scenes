//! Rain: a wet street in the evening, painted in solid pixels. Low cloud drifts over a far
//! skyline softened by the rain, a row of buildings with lit windows stands behind the pavement
//! and street lamps pour warm light onto a wet road that mirrors them. Rain falls in sheets at
//! three depths, faint and fine far off and long and bright close by, splashing on the road and
//! ringing the puddles, while someone under an umbrella walks slowly along the pavement. Unless
//! motion is reduced a distant bolt of lightning shows behind the skyline every few seconds at
//! most. The storm the watch mode shows while a build fails is a scene of its own, in `storm.rs`.
//!
//! On a light theme the same street is an overcast afternoon: the lamps are off, the windows
//! hold the pale sky and the rain is a cool grey.
//!
//! The sky, the buildings, the street, the glow and the rain are tints of the background held
//! below the non-text floor, so they read as background. The lamps, their posts, the walker and
//! the bolt keep the theme's own drawing colours, so they stand out as shapes and keep their
//! outline when colour is off. A bolt never lights the sky: only its own jagged line changes.

use super::paint::{Blender, Overcast, clouds, solid, strong, unit};
use super::wrap;
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// The shortest gap between two flashes, whatever the random numbers say.
pub const MIN_LIGHTNING_GAP: f32 = 5.0;
/// How long one flash stays on screen.
const FLASH_LENGTH: f32 = 0.12;
const SPLASH_LENGTH: f32 = 0.25;
const RIPPLE_LENGTH: f32 = 1.4;
/// Pixels across for each pixel down: the rain leans a little with the wind.
const SLANT: f32 = 0.18;

#[derive(Clone, Debug)]
struct Drop {
    x: f32,
    /// The pixel row of the drop's head.
    y: f32,
    speed: f32,
    /// 0 for far, 1 for the middle distance and 2 for close by.
    layer: u8,
    /// The pixel row it lands on.
    lands: i32,
}

#[derive(Clone, Debug)]
struct Splash {
    x: i32,
    y: i32,
    age: f32,
    /// A ring on a puddle rather than a splash on the road.
    ring: bool,
}

#[derive(Clone, Debug)]
struct Bolt {
    /// The bolt's pixels, from the cloud downwards.
    pixels: Vec<(i32, i32)>,
    age: f32,
}

#[derive(Clone, Debug)]
struct Building {
    x: i32,
    width: i32,
    /// Its height above the pavement in pixels.
    height: i32,
    seed: u32,
}

/// The colours of the street in one theme, every one derived from the theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct RainColours {
    pub dark: bool,
    pub sky: Ramp,
    pub cloud: Ramp,
    pub far_city: Rgb,
    pub far_window: Rgb,
    pub building: Rgb,
    pub building_edge: Rgb,
    pub window_off: Rgb,
    pub window_on: Rgb,
    pub pavement: Rgb,
    pub kerb: Rgb,
    pub road: Rgb,
    pub road_sheen: Rgb,
    pub dash: Rgb,
    pub puddle: Rgb,
    pub ripple: Rgb,
    pub glow_far: Rgb,
    pub glow: Rgb,
    pub reflection: Rgb,
    pub rain_far: Rgb,
    pub rain_mid: Rgb,
    pub rain_near: Rgb,
    pub rain_lit: Rgb,
    pub lamp: Rgb,
    pub post: Rgb,
    pub umbrella: Rgb,
    pub umbrella_shade: Rgb,
    pub coat: Rgb,
    pub face: Rgb,
    pub bolt: Rgb,
}

impl RainColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let s = Shades::new(theme);
        let o = Overcast::new(theme);
        let warm = theme.yellow();
        let umbrella = strong(theme, theme.red());
        let common = |c: Self| c;
        let post = strong(theme, theme.muted);
        if theme.dark {
            common(Self {
                dark: true,
                sky: o.sky.clone(),
                cloud: o.cloud.clone(),
                far_city: s.tint(theme.blue(), 0.2, 1.32),
                far_window: s.tint(warm, 0.3, 1.45),
                building: s.tint(theme.muted, 0.06, 1.08),
                building_edge: s.tint(theme.muted, 0.22, 1.4),
                window_off: s.tint(theme.blue(), 0.16, 1.25),
                window_on: s.tint(warm, 0.6, 2.5),
                pavement: s.tint(theme.muted, 0.26, 1.5),
                kerb: s.tint(theme.foreground, 0.4, 2.0),
                road: s.tint(theme.blue(), 0.08, 1.12),
                road_sheen: s.tint(theme.blue(), 0.2, 1.35),
                dash: s.tint(theme.foreground, 0.3, 1.65),
                puddle: s.tint(theme.info, 0.3, 1.7),
                ripple: s.tint(theme.foreground, 0.55, 2.5),
                glow_far: s.tint(warm, 0.18, 1.35),
                glow: s.tint(warm, 0.32, 1.7),
                reflection: s.tint(warm, 0.5, 2.2),
                rain_far: s.tint(o.rain_far, 0.6, 1.5),
                rain_mid: o.rain_far,
                rain_near: o.rain_near,
                rain_lit: s.tint(mix(warm, theme.foreground, 0.5), 0.8, 2.85),
                lamp: theme.bright_yellow(),
                post,
                umbrella,
                umbrella_shade: solid(theme, umbrella, s.darkest(), 0.35),
                coat: strong(theme, theme.blue()),
                face: strong(theme, mix(theme.yellow(), theme.red(), 0.4)),
                bolt: theme.bright_yellow(),
            })
        } else {
            Self {
                dark: false,
                sky: o.sky.clone(),
                cloud: o.cloud.clone(),
                far_city: s.tint(mix(theme.info, theme.muted, 0.5), 0.42, 1.55),
                far_window: s.tint(theme.info, 0.12, 1.15),
                building: s.tint(theme.muted, 0.5, 2.0),
                building_edge: s.tint(theme.muted, 0.3, 1.55),
                window_off: s.tint(theme.blue(), 0.6, 2.4),
                window_on: s.tint(theme.info, 0.14, 1.2),
                pavement: s.tint(theme.muted, 0.32, 1.6),
                kerb: s.tint(theme.muted, 0.12, 1.15),
                road: s.tint(theme.muted, 0.6, 2.3),
                road_sheen: s.tint(theme.info, 0.4, 1.8),
                dash: s.tint(theme.info, 0.1, 1.12),
                puddle: s.tint(theme.info, 0.16, 1.22),
                ripple: s.tint(theme.blue(), 0.5, 2.0),
                glow_far: s.tint(theme.info, 0.1, 1.12),
                glow: s.tint(theme.info, 0.06, 1.06),
                reflection: s.tint(theme.info, 0.2, 1.3),
                rain_far: s.tint(o.rain_far, 0.6, 1.4),
                rain_mid: o.rain_far,
                rain_near: o.rain_near,
                rain_lit: o.rain_near,
                // by day the lamps are off: a grey glass head on its post
                lamp: post,
                post,
                umbrella,
                umbrella_shade: solid(theme, umbrella, s.darkest(), 0.35),
                coat: strong(theme, theme.blue()),
                face: strong(theme, mix(theme.yellow(), theme.red(), 0.4)),
                bolt: theme.bright_yellow(),
            }
        }
    }
}

pub struct Rain {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    drops: Vec<Drop>,
    splashes: Vec<Splash>,
    far: Vec<Building>,
    near: Vec<Building>,
    /// Lamp posts along the pavement, by column.
    lamps: Vec<i32>,
    /// Puddles on the road, as (middle column, middle row, half width).
    puddles: Vec<(i32, i32, i32)>,
    /// The walker's column.
    walker: f32,
    cloud_drift: f32,
    bolt: Option<Bolt>,
    next_bolt: f32,
    /// Seconds since the last bolt appeared. Every strike waits for this to reach the minimum
    /// gap, whatever else resets, so a resize can never bring a bolt early.
    since_strike: f32,
    flashes: u32,
}

impl Rain {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut rain = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            drops: Vec::new(),
            splashes: Vec::new(),
            far: Vec::new(),
            near: Vec::new(),
            lamps: Vec::new(),
            puddles: Vec::new(),
            walker: 0.0,
            cloud_drift: 0.0,
            bolt: None,
            next_bolt: rng.range(MIN_LIGHTNING_GAP, 14.0),
            since_strike: 0.0,
            flashes: 0,
        };
        rain.resize(width, height, rng);
        rain
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The pixel row of the pavement's kerb, where the buildings stand.
    fn kerb(&self) -> i32 {
        (self.tall() as f32 * 0.68).round() as i32
    }

    /// The first pixel row of the road.
    fn road(&self) -> i32 {
        self.kerb() + ((self.tall() as f32 * 0.06).round() as i32).max(2)
    }

    /// The pixel row the clouds hang down to.
    fn cloud_base(&self) -> i32 {
        (self.tall() as f32 * 0.24).round() as i32
    }

    /// The pixel row a lamp's head hangs at.
    fn lamp_top(&self) -> i32 {
        self.kerb() - (self.tall() as f32 * 0.3).round() as i32
    }

    fn new_drop(&self, rng: &mut Rng, from_top: bool) -> Drop {
        let tall = self.tall() as f32;
        let roll = rng.unit();
        let layer = if roll < 0.5 {
            0
        } else if roll < 0.86 {
            1
        } else {
            2
        };
        let speed = [24.0, 36.0, 52.0][layer as usize] * rng.range(0.85, 1.15);
        let (kerb, road, tall_i) = (self.kerb(), self.road(), self.tall());
        let lands = match layer {
            0 => kerb - rng.below(((tall * 0.1) as usize).max(1)) as i32,
            1 => road + rng.below(((tall_i - road) as usize / 2).max(1)) as i32,
            _ => {
                road + ((tall_i - road) / 2)
                    + rng.below(((tall_i - road) as usize / 2).max(1)) as i32
            }
        };
        Drop {
            x: rng.range(-4.0, self.width as f32 + 4.0),
            y: if from_top {
                -rng.range(0.0, 6.0)
            } else {
                rng.range(-4.0, lands as f32)
            },
            speed: speed * self.motion.speed(),
            layer,
            lands: lands.min(tall_i - 1),
        }
    }

    fn make_bolt(&self, rng: &mut Rng) -> Option<Bolt> {
        if self.width < 7 || self.height < 6 {
            return None;
        }
        let length = ((self.height / 3) as i32).clamp(2, 7) * 2;
        let mut x = rng.range(3.0, self.width as f32 - 3.0) as i32;
        let top = (self.cloud_base() - 2).max(0);
        let mut pixels = Vec::new();
        for y in top..top + length {
            // the bolt zigzags, mostly in one direction at a time, like a real one
            if rng.chance(0.55) {
                x += if rng.chance(0.5) { 1 } else { -1 };
            }
            pixels.push((x, y));
        }
        let start = length as usize / 3;
        if let Some(&(fx, fy)) = pixels.get(start) {
            let side = if rng.chance(0.5) { 1 } else { -1 };
            for k in 1..=(length / 4).max(1) {
                pixels.push((fx + side * k, fy + k));
            }
        }
        Some(Bolt { pixels, age: 0.0 })
    }

    #[cfg(test)]
    pub(crate) fn flashes(&self) -> u32 {
        self.flashes
    }

    #[cfg(test)]
    pub(crate) fn flashing(&self) -> bool {
        self.bolt.is_some()
    }

    /// How strongly a lamp lights a pixel, from 0 to 1, on a dark theme.
    fn lamp_light(&self, x: i32, y: i32) -> f32 {
        let top = self.lamp_top();
        let mut best = 0.0_f32;
        for &lx in &self.lamps {
            let (dx, dy) = ((x - lx - 2) as f32, (y - top - 1) as f32);
            let r = (self.tall() as f32 * 0.16).max(3.0);
            // a soft round glow round the head and a cone of light spreading down from it
            let round = 1.0 - (dx * dx + dy * dy).sqrt() / r;
            let cone = if dy > 0.0 && y < self.road() {
                let spread = 1.5 + dy * 0.45;
                (1.0 - dx.abs() / spread).max(0.0) * 0.55
            } else {
                0.0
            };
            best = best.max(round).max(cone);
        }
        best
    }

    fn paint_buildings(&self, canvas: &mut Canvas, c: &RainColours, rows: &[Building], far: bool) {
        let kerb = self.kerb();
        for b in rows {
            let top = kerb - b.height;
            if far {
                canvas.fill_rect(b.x, top, b.width, b.height, c.far_city);
                // a few far windows still lit through the rain
                let mut y = top + 2;
                while y < kerb - 2 {
                    let mut x = b.x + 1;
                    while x < b.x + b.width - 1 {
                        if unit(x, y, b.seed) < 0.18 {
                            canvas.set(x, y, c.far_window);
                        }
                        x += 2;
                    }
                    y += 3;
                }
                continue;
            }
            canvas.fill_rect(b.x, top, b.width, b.height, c.building);
            // a lit roof line and a lit side towards the lamps give the block its edges
            canvas.fill_rect(b.x, top, b.width, 1, c.building_edge);
            canvas.fill_rect(b.x + b.width - 1, top, 1, b.height, c.building_edge);
            let mut y = top + 3;
            while y + 2 < kerb - 1 {
                let mut x = b.x + 2;
                while x + 2 < b.x + b.width - 1 {
                    let on = unit(x, y, b.seed) < if c.dark { 0.4 } else { 0.25 };
                    let colour = if on { c.window_on } else { c.window_off };
                    canvas.fill_rect(x, y, 2, 2, colour);
                    x += 4;
                }
                y += 4;
            }
        }
    }

    fn paint_drops(&self, canvas: &mut Canvas, c: &RainColours, layer: u8) {
        let length = [2, 4, 6][layer as usize];
        let colour = [c.rain_far, c.rain_mid, c.rain_near][layer as usize];
        for drop in self.drops.iter().filter(|d| d.layer == layer) {
            for k in 0..length {
                let y = drop.y.floor() as i32 - k;
                if y < 0 || y > drop.lands {
                    continue;
                }
                let x = (drop.x - SLANT * k as f32).round() as i32;
                let lit = c.dark && layer > 0 && self.lamp_light(x, y) > 0.2;
                canvas.set(x, y, if lit { c.rain_lit } else { colour });
            }
        }
    }

    fn paint_walker(&self, canvas: &mut Canvas, c: &RainColours) {
        // on a small screen the walker would tower over the street, so the street stays empty
        if self.height < 14 {
            return;
        }
        let tall = self.tall();
        let feet = self.road() - 1;
        let height = ((tall as f32 * 0.15).round() as i32).clamp(7, 20);
        let x = self.walker.round() as i32;
        // a slow stride: the legs swap about once a second
        let stride = (self.t as f32 * self.motion.speed() * 1.6).sin();
        let legs = (height / 3).max(2);
        let body_top = feet - legs - (height - legs) + 3;
        let swing = (stride * 1.5).round() as i32;
        canvas.line((x, feet - legs), (x - swing, feet), c.coat);
        canvas.line((x + 1, feet - legs), (x + 1 + swing, feet), c.coat);
        canvas.fill_rect(x - 1, body_top, 4, feet - legs - body_top + 1, c.coat);
        // the umbrella: a dome with a shaded underside, on a handle held up from the body
        let r = (height as f32 * 0.42).max(3.0);
        let cy = (body_top - 2) as f32;
        let cx = x as f32 + 0.5;
        for y in (cy - r).floor() as i32..=cy as i32 {
            for px in (cx - r - 1.0).floor() as i32..=(cx + r + 1.0).ceil() as i32 {
                let (dx, dy) = (
                    (px as f32 + 0.5 - cx) / (r + 1.0),
                    (y as f32 + 0.5 - cy) / r,
                );
                if dx * dx + dy * dy <= 1.0 {
                    let under = y as f32 >= cy - 0.5;
                    canvas.set(px, y, if under { c.umbrella_shade } else { c.umbrella });
                }
            }
        }
        // a tip on top and a scalloped hem, so it reads as an umbrella rather than a cap
        canvas.set(x, (cy - r).floor() as i32 - 1, c.post);
        let hem = cy as i32 + 1;
        let mut px = (cx - r - 1.0).floor() as i32 + 1;
        while px < (cx + r + 1.0).ceil() as i32 {
            canvas.set(px, hem, c.umbrella_shade);
            px += 3;
        }
        canvas.fill_rect(x, body_top - 2, 2, 2, c.face);
        canvas.line((x + 1, cy as i32 + 1), (x + 1, body_top), c.post);
    }

    fn paint(&self, canvas: &mut Canvas, c: &RainColours) {
        let tall = canvas.height() as i32;
        let wide = canvas.width() as i32;
        let (kerb, road) = (self.kerb(), self.road());
        let mut blender = Blender::new(6);

        for y in 0..tall {
            canvas.fill_row(y, c.sky.at(y as f32 / kerb.max(1) as f32));
        }
        clouds(
            canvas,
            &mut blender,
            &c.cloud,
            0,
            self.cloud_base(),
            self.cloud_drift,
            17,
            0.75,
        );
        if let Some(bolt) = &self.bolt {
            for &(x, y) in &bolt.pixels {
                canvas.set(x, y, c.bolt);
            }
        }
        self.paint_buildings(canvas, c, &self.far, true);
        self.paint_drops(canvas, c, 0);
        self.paint_buildings(canvas, c, &self.near, false);

        // the pavement with its kerb edge, then the wet road with a dashed line down it
        if road < tall && kerb > 0 {
            canvas.fill_rect(0, kerb, wide, road - kerb, c.pavement);
            canvas.fill_rect(0, road - 1, wide, 1, c.kerb);
            for y in road..tall {
                // the road's wet sheen comes and goes in soft bands across it
                let band = ((y - road) as f32 * 0.9 + (y as f32 * 0.37).sin() * 2.0).sin();
                canvas.fill_row(y, if band > 0.6 { c.road_sheen } else { c.road });
            }
            let lane = road + (tall - road) / 2;
            let mut x = 0;
            while x < wide {
                canvas.fill_rect(x, lane, 4, 1, c.dash);
                x += 9;
            }
            for &(px, py, half) in &self.puddles {
                for dy in -1_i32..=1 {
                    let reach = half - dy.abs() * (half / 3).max(1);
                    canvas.fill_rect(px - reach, py + dy, reach * 2 + 1, 1, c.puddle);
                }
            }
        }

        if c.dark {
            // the lamps' light: a glow in the air and on the pavement, then long wavering
            // reflections in the wet road beneath each lamp
            let top = self.lamp_top().max(0);
            for y in top - 8..road {
                for x in 0..wide {
                    let light = self.lamp_light(x, y);
                    if light > 0.45 {
                        blender.blend(canvas, x, y, c.glow, 0.8);
                    } else if light > 0.15 {
                        blender.blend(canvas, x, y, c.glow_far, 0.8);
                    }
                }
            }
            for &lx in &self.lamps {
                for y in road..tall {
                    let fade = (y - road) as f32 / (tall - road).max(1) as f32;
                    // the reflection sways slowly as a whole rather than zigzagging row by row
                    let waver =
                        ((y as f32 * 0.22 + self.t as f32 * self.motion.speed() * 0.6).sin() * 1.1)
                            .round() as i32;
                    let half = 1 + (fade * 2.0) as i32;
                    for dx in -half..=half {
                        let colour = if dx.abs() < half || half == 1 {
                            c.reflection
                        } else {
                            c.glow_far
                        };
                        canvas.set(lx + 2 + dx + waver, y, colour);
                    }
                }
            }
        }

        for splash in &self.splashes {
            let grow = (splash.age
                / if splash.ring {
                    RIPPLE_LENGTH
                } else {
                    SPLASH_LENGTH
                }
                * 3.0) as i32;
            if splash.ring {
                let r = grow + 1;
                canvas.set(splash.x - r, splash.y, c.ripple);
                canvas.set(splash.x + r, splash.y, c.ripple);
                if r > 1 {
                    canvas.set(splash.x - r + 1, splash.y - 1, c.ripple);
                    canvas.set(splash.x + r - 1, splash.y - 1, c.ripple);
                }
            } else if grow == 0 {
                canvas.set(splash.x, splash.y - 1, c.rain_near);
            } else {
                canvas.set(splash.x - 1, splash.y - 1, c.rain_near);
                canvas.set(splash.x + 1, splash.y - 1, c.rain_near);
            }
        }

        self.paint_drops(canvas, c, 1);
        // the lamp posts with a curved arm and the lamp hanging from it
        let top = self.lamp_top();
        if top > 2 {
            for &lx in &self.lamps {
                canvas.fill_rect(lx, top - 1, 1, road - top + 1, c.post);
                canvas.fill_rect(lx - 1, road - 2, 3, 2, c.post);
                canvas.fill_rect(lx, top - 2, 3, 1, c.post);
                canvas.set(lx + 3, top - 1, c.post);
                canvas.fill_rect(lx + 1, top, 3, 2, c.lamp);
                canvas.fill_rect(lx + 1, top - 1, 3, 1, c.post);
            }
        }
        self.paint_walker(canvas, c);
        self.paint_drops(canvas, c, 2);
    }
}

impl Scene for Rain {
    fn name(&self) -> &'static str {
        "rain"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let (w, tall, kerb) = (self.width as i32, self.tall(), self.kerb());
        self.far.clear();
        let mut x = -(rng.below(4) as i32);
        while x < w {
            let width = 4 + rng.below(9) as i32;
            let height = ((tall as f32 * rng.range(0.22, 0.4)) as i32).max(2);
            self.far.push(Building {
                x,
                width,
                height,
                seed: rng.next_u64() as u32,
            });
            x += width + rng.below(3) as i32;
        }
        self.near.clear();
        let mut x = -(rng.below(6) as i32);
        while x < w {
            let width = ((tall as f32 * rng.range(0.25, 0.5)) as i32).max(6);
            let height = ((tall as f32 * rng.range(0.16, 0.32)) as i32).max(2);
            self.near.push(Building {
                x,
                width,
                height,
                seed: rng.next_u64() as u32,
            });
            x += width
                + if rng.chance(0.4) {
                    3 + rng.below(8) as i32
                } else {
                    0
                };
        }
        let spacing = (w / 3).clamp(18, 70);
        self.lamps = (0..)
            .map(|i| spacing / 2 + i * spacing)
            .take_while(|&x| x < w - 3)
            .collect();
        let road = self.road();
        self.puddles.clear();
        if tall - road > 4 {
            let mut x = rng.below(10) as i32;
            while x < w {
                let half = 3 + rng.below(5) as i32;
                let y = road + 2 + rng.below((tall - road - 3).max(1) as usize) as i32;
                self.puddles.push((x + half, y, half));
                x += 2 * half + 8 + rng.below(16) as i32;
            }
        }
        let wanted = self
            .motion
            .scaled(((self.width * self.height) / 16).clamp(8, 500));
        self.drops = (0..wanted).map(|_| self.new_drop(rng, false)).collect();
        self.splashes.clear();
        // the walker starts well inside the street, where the eye falls first
        self.walker = rng.range(0.25, 0.5) * self.width as f32;
        let _ = kerb;
        if self.bolt.take().is_some() {
            self.next_bolt = self.next_bolt.max(MIN_LIGHTNING_GAP);
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        self.since_strike += dt;
        let speed = self.motion.speed();
        let width = self.width as f32;
        self.cloud_drift += 1.2 * speed * dt;
        self.walker = wrap(self.walker + 2.2 * speed * dt + 8.0, width + 16.0) - 8.0;

        for i in 0..self.drops.len() {
            let drop = &mut self.drops[i];
            drop.y += drop.speed * dt;
            drop.x += drop.speed * SLANT * dt;
            if drop.y >= drop.lands as f32 {
                let (x, y, layer) = (drop.x.round() as i32, drop.lands, drop.layer);
                self.drops[i] = self.new_drop(rng, true);
                if layer > 0 && self.splashes.len() < (self.width / 2).clamp(4, 120) {
                    let ring = self
                        .puddles
                        .iter()
                        .any(|&(px, py, half)| (x - px).abs() < half && (y - py).abs() <= 1);
                    self.splashes.push(Splash {
                        x,
                        y,
                        age: 0.0,
                        ring,
                    });
                }
            }
        }
        for splash in &mut self.splashes {
            splash.age += dt * speed;
        }
        self.splashes
            .retain(|s| s.age < if s.ring { RIPPLE_LENGTH } else { SPLASH_LENGTH });

        if self.motion.calm {
            self.bolt = None;
            return;
        }
        if let Some(bolt) = &mut self.bolt {
            bolt.age += dt;
            if bolt.age >= FLASH_LENGTH {
                self.bolt = None;
                self.next_bolt = rng.range(MIN_LIGHTNING_GAP, 15.0);
            }
        } else {
            self.next_bolt -= dt;
            if self.next_bolt <= 0.0 && self.since_strike >= MIN_LIGHTNING_GAP {
                match self.make_bolt(rng) {
                    Some(bolt) => {
                        self.bolt = Some(bolt);
                        self.since_strike = 0.0;
                        self.flashes += 1;
                    }
                    None => self.next_bolt = MIN_LIGHTNING_GAP,
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = RainColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixels::{IMAGERY_CEILING, distance};
    use crate::scenes::testing::{check_determinism, check_pixel_scene, check_scene, pixels};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("rain");
        check_determinism("rain");
        check_pixel_scene("rain", 6.0);
    }

    fn drawn(rain: &Rain, width: usize, height: usize, theme: &ThemeData) -> Frame {
        let mut frame = Frame::new(width, height);
        rain.draw(&mut frame, theme);
        frame
    }

    #[test]
    fn the_street_and_the_rain_stay_distinct_in_every_theme() {
        for theme in theme::all() {
            let c = RainColours::new(theme);
            let name = theme.name;
            for (a, b, what) in [
                (c.far_city, c.sky.at(1.0), "the far city and the sky"),
                (c.building, c.far_city, "the buildings and the far city"),
                (c.window_on, c.building, "the lit windows and the walls"),
                (c.pavement, c.road, "the pavement and the road"),
                (c.puddle, c.road, "the puddles and the road"),
                (c.rain_near, c.road, "the near rain and the road"),
                (c.rain_mid, c.building, "the rain and the walls"),
            ] {
                assert!(distance(a, b) >= 0.03, "{what} collapse in {name}");
            }
            for (what, colour) in [
                ("far city", c.far_city),
                ("building", c.building),
                ("window", c.window_on),
                ("pavement", c.pavement),
                ("road", c.road),
                ("puddle", c.puddle),
                ("ripple", c.ripple),
                ("glow", c.glow),
                ("reflection", c.reflection),
                ("near rain", c.rain_near),
                ("lit rain", c.rain_lit),
            ] {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            for colour in [c.post, c.umbrella, c.coat, c.bolt] {
                assert!(theme.contrast_of(colour) >= IMAGERY_CEILING, "{name}");
            }
            if theme.dark {
                assert!(theme.contrast_of(c.lamp) >= theme.floor(), "{name}");
            }
        }
    }

    #[test]
    fn rain_falls_at_three_depths_onto_a_lamplit_street() {
        let theme = theme::default_theme();
        let c = RainColours::new(theme);
        let mut rng = Rng::new(8);
        let mut rain = Rain::new(80, 24, Motion::FULL, &mut rng);
        for _ in 0..40 {
            rain.step(0.05, &mut rng);
        }
        let all = pixels(&drawn(&rain, 80, 24, theme));
        for (what, colour) in [
            ("far rain", c.rain_far),
            ("rain", c.rain_mid),
            ("near rain", c.rain_near),
            ("lamp", c.lamp),
            ("reflection", c.reflection),
            ("umbrella", c.umbrella),
            ("puddle", c.puddle),
        ] {
            assert!(all.contains(&colour), "no {what}");
        }
        assert!(rain.splashes.iter().any(|s| !s.ring), "no splash");
    }

    #[test]
    fn lightning_never_strikes_under_reduced_motion() {
        let mut rng = Rng::new(1);
        let mut rain = Rain::new(80, 24, Motion::CALM, &mut rng);
        for _ in 0..(300 * 20) {
            rain.step(0.05, &mut rng);
            assert!(!rain.flashing());
        }
        assert_eq!(rain.flashes(), 0);
    }

    #[test]
    fn lightning_keeps_at_least_five_seconds_between_flashes() {
        let mut rng = Rng::new(21);
        let mut rain = Rain::new(80, 24, Motion::FULL, &mut rng);
        let mut t = 0.0_f32;
        let mut starts = Vec::new();
        let mut was_flashing = false;
        for _ in 0..(300 * 20) {
            rain.step(0.05, &mut rng);
            t += 0.05;
            if rain.flashing() && !was_flashing {
                starts.push(t);
            }
            was_flashing = rain.flashing();
        }
        assert!(
            starts.len() >= 10,
            "only {} flashes in five minutes",
            starts.len()
        );
        for pair in starts.windows(2) {
            assert!(pair[1] - pair[0] >= MIN_LIGHTNING_GAP - 0.01, "{pair:?}");
        }
        assert_eq!(starts.len() as u32, rain.flashes());
    }

    #[test]
    fn a_bolt_is_one_jagged_line_in_the_upper_part_and_never_lights_the_sky() {
        let theme = theme::default_theme();
        let c = RainColours::new(theme);
        let mut rng = Rng::new(21);
        let mut rain = Rain::new(80, 24, Motion::FULL, &mut rng);
        let mut seen = 0;
        for _ in 0..(60 * 20) {
            rain.step(0.05, &mut rng);
            let Some(bolt) = &rain.bolt else {
                continue;
            };
            seen += 1;
            assert!(bolt.pixels.iter().all(|&(_, y)| y < 24));
            assert!(bolt.pixels.len() < 30);
            let frame = drawn(&rain, 80, 24, theme);
            let lit = pixels(&frame).iter().filter(|&&p| p == c.bolt).count();
            // the lamps share the bolt's colour on some themes, so count only above them
            let lamps = rain.lamps.len() * 6;
            assert!(lit <= bolt.pixels.len() + lamps);
        }
        assert!(seen > 0);
    }

    #[test]
    fn resizing_never_brings_lightning_early() {
        let mut rng = Rng::new(21);
        let mut rain = Rain::new(80, 24, Motion::FULL, &mut rng);
        let mut t = 0.0_f32;
        let mut starts = Vec::new();
        let mut was_flashing = false;
        // a window edge dragged for five minutes: a new size on every frame
        for frame in 0..(300 * 20) {
            let width = 60 + (frame % 40);
            let height = 20 + (frame % 7);
            rain.resize(width, height, &mut rng);
            rain.step(0.05, &mut rng);
            t += 0.05;
            if rain.flashing() && !was_flashing {
                starts.push(t);
            }
            was_flashing = rain.flashing();
        }
        assert!(!starts.is_empty());
        for pair in starts.windows(2) {
            assert!(pair[1] - pair[0] >= MIN_LIGHTNING_GAP - 0.01, "{pair:?}");
        }
    }

    #[test]
    fn a_tiny_screen_gets_no_bolt() {
        let mut rng = Rng::new(4);
        let mut rain = Rain::new(6, 4, Motion::FULL, &mut rng);
        for _ in 0..(60 * 20) {
            rain.step(0.05, &mut rng);
        }
        assert_eq!(rain.flashes(), 0);
    }
}
