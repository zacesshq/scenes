//! Harbour: a small harbour town at night, painted in solid pixels. A row of houses with lit
//! windows stands behind a stone quay, lamps along the quay throw long reflections that shimmer
//! slowly on the water, moored boats sway at their masts as the water laps their hulls, a striped
//! lighthouse keeps a steady lamp at the end of the quay and now and then a ferry crosses the far
//! water.
//!
//! A light theme shows the same harbour on a bright afternoon instead: a blue sky with slow
//! clouds, sunlit panes in the windows, the lamps unlit, a white and red lighthouse and painted
//! boats with white wheelhouses.
//!
//! Every colour comes from the active theme. The sky, the houses, the quay and the water are
//! tints of the background held below the non-text floor, so they read as background; the boats,
//! the lamps, the lit windows and the lighthouse keep the theme's own drawing colours, so they
//! stand out as shapes and keep their outline when colour is off.

use std::f32::consts::TAU;

use super::{count_for, cycle};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "A harbour town at night, with shaded boats, quay lamps, a lighthouse and their reflections, on a sunny afternoon in light themes";

pub const CAPTIONS: &[&str] = &[
    "Everything is moored and secure.",
    "Safe in harbour. The build passed.",
    "Calm water, clean build.",
    "The lamps are lit and nothing is adrift.",
    "All hands report green.",
];

/// Seconds between two changes of a reflection's shimmer. Each reflection pixel changes at most
/// once a step, so well under twice a second.
pub const SHIMMER_PERIOD: f32 = 0.8;
/// Seconds for one sway of a mast, which passes through four positions on the way.
pub const SWAY_PERIOD: f32 = 4.0;

/// Steps in the sky and water ramps.
const SKY_STEPS: usize = 10;
const WATER_STEPS: usize = 10;
/// How wide the ferry is, in pixels.
const FERRY_WIDTH: i32 = 18;

/// A small deterministic hash, for the shimmer and the fixed details.
fn hash(a: i64, b: i64) -> u64 {
    let mut z =
        (a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (b as u64).wrapping_add(0x632B_E59B);
    z = (z ^ (z >> 29)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^ (z >> 32)
}

/// The harbour's colours in one theme, every one derived from the theme's own. Some are kept
/// only so the tests can check the bands against each other.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct HarbourColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub glow: Rgb,
    pub walls: [Rgb; 4],
    pub roof: Rgb,
    pub dark_window: Rgb,
    pub stone: Rgb,
    pub stone_light: Rgb,
    pub mortar: Rgb,
    pub water_top: Rgb,
    pub water_deep: Rgb,
    pub swell: Rgb,
    pub star: Rgb,
    pub faint_star: Rgb,
    pub moon: Rgb,
    pub lamp: Rgb,
    pub window: Rgb,
    pub post: Rgb,
    pub stripe: Rgb,
    pub tower: Rgb,
    pub hulls: [Rgb; 3],
    pub sail: Rgb,
    pub ferry: Rgb,
    pub funnel: Rgb,
    pub cloud: Rgb,
    pub cloud_shade: Rgb,
    sky: Ramp,
    water: Ramp,
    stars: bool,
    /// Whether this is a light theme's afternoon, with the lamps unlit and clouds overhead.
    day: bool,
}

/// Pure white, the lighthouse and the painted woodwork by day: a touch above an off-white
/// background, so it stays imagery.
const WHITE: Rgb = Rgb::new(255, 255, 255);

impl HarbourColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        // a night sky on a dark theme lightens towards the town's glow; on a light theme it is
        // an evening sky, deeper at the top and paling to a warm horizon
        let (sky_top, sky_low, glow) = if theme.dark {
            (
                shades.tint(theme.info, 0.14, 1.3),
                shades.tint(theme.info, 0.36, 1.8),
                shades.tint(theme.warning, 0.45, 2.1),
            )
        } else {
            (
                shades.tint(theme.info, 0.6, 2.2),
                shades.tint(theme.info, 0.18, 1.3),
                shades.tint(theme.info, 0.12, 1.2),
            )
        };
        let wall = |hue: Rgb| {
            if theme.dark {
                shades.tint(hue, 0.42, 2.3)
            } else {
                shades.tint(hue, 0.3, 1.7)
            }
        };
        let walls = [
            wall(theme.red()),
            wall(theme.yellow()),
            wall(theme.cyan()),
            wall(theme.grey()),
        ];
        // slate roofs, darker than the walls in every theme
        let roof = if theme.dark {
            shades.tint(theme.red(), 0.18, 1.35)
        } else {
            shades.tint(mix(theme.red(), theme.foreground, 0.6), 0.9, 2.9)
        };
        let dark_window = if theme.dark {
            shades.tint(theme.blue(), 0.1, 1.15)
        } else {
            shades.tint(theme.blue(), 0.9, 2.95)
        };
        let (stone, stone_light, mortar) = if theme.dark {
            (
                shades.tint(theme.grey(), 0.6, 2.45),
                shades.tint(theme.grey(), 0.8, 2.9),
                shades.tint(theme.grey(), 0.2, 1.3),
            )
        } else {
            (
                shades.tint(theme.grey(), 0.45, 2.2),
                shades.tint(theme.grey(), 0.2, 1.45),
                shades.tint(theme.grey(), 0.8, 2.9),
            )
        };
        // the water darkens with depth in every theme
        let (water_top, water_deep) = if theme.dark {
            (
                shades.tint(theme.blue(), 0.38, 1.9),
                shades.tint(theme.blue(), 0.12, 1.15),
            )
        } else {
            (
                shades.tint(theme.blue(), 0.35, 1.9),
                shades.tint(theme.blue(), 0.75, 2.85),
            )
        };
        let swell = if theme.dark {
            mix(water_top, theme.bright_blue(), 0.35)
        } else {
            mix(water_top, theme.background, 0.6)
        };
        let day = !theme.dark;
        let (cloud, cloud_shade) = super::paint::cloud_tones(theme, 1.8);
        // by day the panes catch the sky and the lamps stand unlit
        let (lamp, window) = if day {
            (
                shades.tint(theme.yellow(), 0.25, 1.25),
                mix(WHITE, theme.info, 0.08),
            )
        } else {
            (theme.bright_yellow(), theme.yellow())
        };
        Self {
            cloud,
            cloud_shade,
            day,
            sky_top,
            sky_low,
            glow,
            walls,
            roof,
            dark_window,
            stone,
            stone_light,
            mortar,
            water_top,
            water_deep,
            swell,
            star: theme.foreground,
            faint_star: theme.muted,
            moon: theme.bright_yellow(),
            lamp,
            window,
            post: theme.muted,
            stripe: theme.red(),
            tower: if day { WHITE } else { theme.foreground },
            hulls: if day {
                [theme.blue(), theme.red(), theme.green()]
            } else {
                [theme.foreground, theme.bright_cyan(), theme.red()]
            },
            sail: theme.muted,
            ferry: if day { WHITE } else { theme.foreground },
            funnel: theme.red(),
            sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
            water: Ramp::new(water_top, water_deep, WATER_STEPS),
            stars: theme.dark,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hull {
    Sail,
    Cabin,
    Rowing,
}

impl Hull {
    /// The hull's width and height in pixels.
    fn size(self) -> (i32, i32) {
        match self {
            Hull::Sail => (13, 3),
            Hull::Cabin => (12, 3),
            Hull::Rowing => (7, 2),
        }
    }

    /// How far above the waterline the boat reaches, mast or cabin included.
    fn tall(self) -> i32 {
        match self {
            Hull::Sail => 13,
            Hull::Cabin => 6,
            Hull::Rowing => 2,
        }
    }
}

#[derive(Clone, Debug)]
struct Boat {
    x: i32,
    /// The pixel row of the hull's bottom.
    y: i32,
    hull: Hull,
    colour: usize,
    phase: f32,
}

#[derive(Clone, Debug)]
struct Building {
    x: i32,
    width: i32,
    /// The wall's height in pixels, below the roof.
    height: i32,
    wall: usize,
    pitched: bool,
    /// Windows as column and row inside the wall with whether each is lit.
    windows: Vec<(i32, i32, bool)>,
}

pub struct Harbour {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    stars: Vec<(i32, i32, bool)>,
    buildings: Vec<Building>,
    lamps: Vec<i32>,
    boats: Vec<Boat>,
    ferry_x: f32,
    ferry_wait: f32,
}

impl Harbour {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut harbour = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            stars: Vec::new(),
            buildings: Vec::new(),
            lamps: Vec::new(),
            boats: Vec::new(),
            ferry_x: -(FERRY_WIDTH as f32),
            ferry_wait: rng.range(2.0, 10.0),
        };
        harbour.resize(width, height, rng);
        harbour
    }

    /// The quay's row: the sky and the town are above it, the water below.
    fn quay(&self) -> i32 {
        ((self.height as f32 * 0.42) as i32).max(2)
    }

    /// The quay's top pixel row. The quay wall fills that cell; the water starts below it.
    fn quay_px(&self) -> i32 {
        self.quay() * 2
    }

    fn water_top(&self) -> i32 {
        self.quay_px() + 2
    }

    fn lighthouse_fits(&self) -> bool {
        self.width >= 50 && self.quay() >= 6
    }

    fn ferry_fits(&self) -> bool {
        self.width >= 40 && self.height as i32 - self.quay() >= 8
    }

    fn lamps_fit(&self) -> bool {
        self.quay_px() >= 8
    }

    fn shimmer_period(&self) -> f32 {
        SHIMMER_PERIOD / self.motion.pace()
    }

    fn sway_period(&self) -> f32 {
        SWAY_PERIOD / self.motion.pace()
    }

    /// The step of the shimmer, so every reflection changes on the same slow beat.
    fn shimmer_step(&self) -> u64 {
        (self.t / f64::from(self.shimmer_period())) as u64
    }

    /// A mast's lean for a sway from -1 to 1: one of three positions, upright under reduced
    /// motion.
    fn lean(&self, phase: f32) -> i32 {
        if self.motion.calm {
            return 0;
        }
        let lean = (cycle(self.t, TAU / self.sway_period()) + phase).sin();
        if lean > 0.6 {
            1
        } else if lean < -0.6 {
            -1
        } else {
            0
        }
    }

    /// The lighthouse's column and the top of its lamp room.
    fn lighthouse_at(&self) -> (i32, i32) {
        let x = self.width as i32 - 6;
        let tall = (self.quay_px() - 3).min(18);
        (x, self.quay_px() - tall)
    }

    /// Paints a soft glow: `colour` mixed over the canvas, fading out to `radius`.
    fn glow(canvas: &mut Canvas, cx: i32, cy: i32, radius: f32, colour: Rgb, strength: f32) {
        let r = radius.ceil() as i32;
        for dy in -r..=r {
            for dx in -r..=r {
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                if d > 0.0 && d <= radius {
                    canvas.blend(cx + dx, cy + dy, colour, strength * (1.0 - d / radius));
                }
            }
        }
    }

    fn paint_sky(&self, canvas: &mut Canvas, c: &HarbourColours) {
        let quay = self.quay_px();
        for y in 0..quay.min(canvas.height() as i32) {
            let t = y as f32 / quay.max(1) as f32;
            let base = c.sky.at(t);
            // the town's light warms the lowest part of the sky
            let colour = if t > 0.7 {
                mix(base, c.glow, ((t - 0.7) / 0.3).min(1.0) * 0.6)
            } else {
                base
            };
            canvas.fill_row(y, colour);
        }
        if c.stars {
            for &(x, y, bright) in &self.stars {
                canvas.set(x, y, if bright { c.star } else { c.faint_star });
            }
        }
        if c.day {
            // a few clouds drifting slowly over the town
            let quay = self.quay_px() as f32;
            let span = canvas.width() as f32 + 40.0;
            let drift = (self.t as f32 * 0.6 * self.motion.speed()) % span;
            for (i, (start, base, width)) in
                [(0.1, 0.28, 16.0), (0.55, 0.18, 22.0), (0.85, 0.4, 12.0)]
                    .into_iter()
                    .enumerate()
            {
                let x = (start * span + drift) % span - 20.0;
                let cloud =
                    super::paint::Cloud::new(x, quay * base + 4.0, width, 0.0, 70 + i as u32);
                if quay >= 12.0 {
                    cloud.paint(canvas, c.cloud, c.cloud_shade);
                }
            }
        }
        // a waxing moon with its dark side faintly showing; it never changes
        if !c.day && self.width >= 40 && self.quay() >= 7 {
            let (cx, cy, r) = (8.0_f32, 5.0_f32, 3.6_f32);
            let dark = mix(c.moon, c.sky_top, 0.75);
            for y in 0..11 {
                for x in 3..14 {
                    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                    let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
                    if d > r {
                        continue;
                    }
                    let shadow = (px - cx + 1.8).powi(2) + (py - cy).powi(2) <= r * r;
                    canvas.set(x, y, if shadow { dark } else { c.moon });
                }
            }
        }
    }

    fn paint_town(&self, canvas: &mut Canvas, c: &HarbourColours) {
        let base = self.quay_px() - 2;
        for b in &self.buildings {
            let wall = c.walls[b.wall % c.walls.len()];
            let top = base - b.height + 1;
            canvas.fill_rect(b.x, top, b.width, b.height, wall);
            // a shaded right edge gives each house a little depth
            canvas.fill_rect(b.x + b.width - 1, top, 1, b.height, mix(wall, c.roof, 0.35));
            if b.pitched {
                let rise = (b.width / 2).min(4);
                for i in 0..rise {
                    canvas.fill_rect(b.x + i, top - rise + i, b.width - 2 * i, 1, c.roof);
                }
                // the eaves overhang a little
                canvas.fill_rect(b.x - 1, top - 1, b.width + 2, 1, c.roof);
            } else {
                canvas.fill_rect(b.x, top - 1, b.width, 1, c.roof);
                if b.width >= 6 {
                    canvas.fill_rect(b.x + b.width - 3, top - 3, 1, 2, c.roof);
                }
            }
            for &(wx, wy, lit) in &b.windows {
                let colour = if lit { c.window } else { c.dark_window };
                canvas.fill_rect(b.x + wx, top + wy, 1, 2, colour);
            }
        }
    }

    fn paint_quay(&self, canvas: &mut Canvas, c: &HarbourColours) {
        let wide = canvas.width() as i32;
        let quay = self.quay_px();
        // the pavement in front of the houses, then the quay wall's coping and its blocks
        canvas.fill_rect(0, quay - 1, wide, 1, c.stone);
        canvas.fill_rect(0, quay, wide, 1, c.stone_light);
        for x in 0..wide {
            let colour = if (x + 2) % 5 == 0 { c.mortar } else { c.stone };
            canvas.set(x, quay + 1, colour);
        }
        if self.lamps_fit() {
            for &x in &self.lamps {
                canvas.fill_rect(x, quay - 4, 1, 3, c.post);
                canvas.set(x, quay - 5, c.post);
                if c.day {
                    // an unlit lantern: a small glass head under a dark cap
                    canvas.set(x, quay - 6, c.lamp);
                    canvas.set(x, quay - 7, c.post);
                } else {
                    Self::glow(canvas, x, quay - 6, 3.0, c.lamp, 0.45);
                    canvas.set(x, quay - 6, c.lamp);
                }
            }
        }
    }

    fn paint_lighthouse(&self, canvas: &mut Canvas, c: &HarbourColours) {
        if !self.lighthouse_fits() {
            return;
        }
        let (x, top) = self.lighthouse_at();
        let base = self.quay_px() - 1;
        // the tower narrows towards the top and is striped in bands
        let tower_top = top + 4;
        for y in tower_top..=base {
            let t = (y - tower_top) as f32 / (base - tower_top).max(1) as f32;
            let half = if t > 0.5 { 2 } else { 1 };
            let band = ((base - y) / 3) % 2 == 0;
            let colour = if band { c.tower } else { c.stripe };
            canvas.fill_rect(x - half, y, 2 * half + 1, 1, colour);
            // the side away from the moon is in shade
            canvas.set(x + half, y, mix(colour, c.sky_top, 0.35));
        }
        // the gallery, the lamp room and the cap
        canvas.fill_rect(x - 2, tower_top - 1, 5, 1, c.post);
        if !c.day {
            Self::glow(canvas, x, top + 1, 4.0, c.lamp, 0.4);
        }
        canvas.fill_rect(x - 1, top + 1, 3, 2, c.lamp);
        canvas.fill_rect(x - 1, top, 3, 1, c.stripe);
        canvas.set(x, top - 1, c.stripe);
    }

    fn paint_water(&self, canvas: &mut Canvas, c: &HarbourColours) {
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let top = self.water_top();
        let span = (tall - top).max(1) as f32;
        for y in top..tall {
            canvas.fill_row(y, c.water.at((y - top) as f32 / span));
        }
        // a few long, low swell lines drifting slowly
        let drift = self.t * f64::from(0.4 * self.motion.speed());
        for y in (top + 1..tall).step_by(3) {
            let offset = (drift * if y % 2 == 0 { 1.0 } else { -0.7 }).floor() as i64;
            let depth = (y - top) as f32 / span;
            let colour = mix(c.water.at(depth), c.swell, 0.8 - 0.5 * depth);
            for x in 0..wide {
                let wx = i64::from(x) + offset;
                let run = wx.div_euclid(14);
                let at = wx.rem_euclid(14);
                if hash(run, i64::from(y)) % 3 == 0 && (2..7).contains(&at) {
                    canvas.set(x, y, colour);
                }
            }
        }
    }

    /// A lamp's reflection: a column of light down the water, broken here and there on a slow
    /// beat, with short glints either side near the top, fading as it goes down.
    fn paint_reflection(&self, canvas: &mut Canvas, c: &HarbourColours, x: i32, seed: i64) {
        let top = self.water_top();
        let tall = canvas.height() as i32;
        let length = ((tall - top) * 2 / 3).max(1);
        let step = self.shimmer_step() as i64;
        let span = (tall - top).max(1) as f32;
        for dy in 0..length {
            let y = top + dy;
            let h = hash(seed * 31 + i64::from(dy), step);
            if dy > 1 && h % 5 == 0 {
                continue;
            }
            let fade = 1.0 - dy as f32 / length as f32;
            let water = c.water.at(dy as f32 / span);
            let shift = if dy > 2 && h % 4 == 0 {
                ((h >> 8) % 2) as i32 * 2 - 1
            } else {
                0
            };
            canvas.set(x + shift, y, mix(water, c.lamp, 0.25 + 0.7 * fade));
            if dy < length / 3 && (h >> 4) % 3 == 0 {
                canvas.set(x + shift - 1, y, mix(water, c.window, 0.5 * fade));
                canvas.set(x + shift + 1, y, mix(water, c.window, 0.5 * fade));
            }
        }
    }

    fn paint_ferry(&self, canvas: &mut Canvas, c: &HarbourColours) {
        if !self.ferry_fits() || self.ferry_wait > 0.0 {
            return;
        }
        let x = self.ferry_x.round() as i32;
        let y = self.water_top() + 1;
        // the funnel, the cabins with a row of lit windows and the hull
        canvas.fill_rect(x + 11, y, 2, 2, c.funnel);
        canvas.fill_rect(x + 4, y + 2, 11, 2, c.ferry);
        for i in 0..5 {
            canvas.set(x + 5 + 2 * i, y + 3, c.lamp);
        }
        canvas.fill_rect(x + 1, y + 4, FERRY_WIDTH - 2, 1, c.ferry);
        canvas.fill_rect(
            x + 2,
            y + 5,
            FERRY_WIDTH - 3,
            1,
            mix(c.ferry, c.water_deep, 0.35),
        );
        canvas.set(x, y + 4, mix(c.ferry, c.water_top, 0.5));
    }

    fn paint_boat(&self, canvas: &mut Canvas, c: &HarbourColours, boat: &Boat) {
        let colour = c.hulls[boat.colour % c.hulls.len()];
        let (w, h) = boat.hull.size();
        let deck = boat.y - h + 1;
        let tall = canvas.height() as i32;
        let span = (tall - self.water_top()).max(1) as f32;
        let water = |y: i32| c.water.at((y - self.water_top()) as f32 / span);
        // a dim reflection of the hull just below it, broken up by the water
        let step = self.shimmer_step() as i64;
        for dy in 1..=2 {
            let y = boat.y + dy;
            for dx in dy..w - dy {
                if hash(i64::from(boat.x + dx) * 7 + i64::from(dy), step) % 4 != 0 {
                    canvas.set(boat.x + dx, y, mix(water(y), colour, 0.3 / dy as f32));
                }
            }
        }
        // the hull: a raised, pointed bow on the right, a rounder stern and a keel that darkens
        // towards the water
        for r in 0..h {
            let shade = mix(colour, c.water_deep, 0.22 * r as f32);
            let bow = if r == 0 { 0 } else { r + 1 };
            canvas.fill_rect(boat.x + r, deck + r, w - r - bow, 1, shade);
        }
        canvas.set(boat.x + w - 1, deck - 1, colour);
        match boat.hull {
            Hull::Sail => {
                let mast = boat.x + w / 2 - 1;
                let height = boat.hull.tall() - h;
                let lean = self.lean(boat.phase);
                for i in 1..=height {
                    let shift = if i > height * 2 / 3 { lean } else { 0 };
                    canvas.set(mast + shift, deck - i, c.post);
                }
                // the sail is furled along the boom while the boat is moored
                canvas.fill_rect(mast + 1, deck - 2, w / 2 - 1, 1, c.sail);
                canvas.set(mast + lean, deck - height - 1, c.lamp);
            }
            Hull::Cabin => {
                // a wheelhouse towards the stern with a sloping front and lit portholes
                let cx = boat.x + 2;
                canvas.fill_rect(cx, deck - 3, 5, 3, c.ferry);
                canvas.set(cx + 5, deck - 2, c.ferry);
                canvas.set(cx + 5, deck - 1, c.ferry);
                canvas.fill_rect(cx, deck - 3, 5, 1, mix(c.ferry, c.roof, 0.4));
                canvas.set(cx + 1, deck - 2, c.lamp);
                canvas.set(cx + 3, deck - 2, c.lamp);
            }
            Hull::Rowing => {
                canvas.set(boat.x + w / 2, deck, mix(colour, c.water_deep, 0.4));
            }
        }
        // the water laps either side of the hull, rising and falling with the sway
        if boat.y < tall - 1 {
            let lap = if self.motion.calm {
                0
            } else {
                i32::from((cycle(self.t, TAU / self.sway_period()) + boat.phase).sin() > 0.0)
            };
            let y = boat.y;
            canvas.set(boat.x - 1 + lap, y, mix(water(y), c.swell, 0.9));
            canvas.set(boat.x + w - lap, y, mix(water(y), c.swell, 0.9));
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &HarbourColours) {
        self.paint_sky(canvas, c);
        self.paint_town(canvas, c);
        self.paint_water(canvas, c);
        if self.lamps_fit() && !c.day {
            for &x in &self.lamps {
                self.paint_reflection(canvas, c, x, i64::from(x));
            }
        }
        if self.lighthouse_fits() && !c.day {
            let (x, _) = self.lighthouse_at();
            self.paint_reflection(canvas, c, x, 999);
        }
        self.paint_quay(canvas, c);
        self.paint_lighthouse(canvas, c);
        self.paint_ferry(canvas, c);
        for boat in &self.boats {
            self.paint_boat(canvas, c, boat);
        }
    }
}

impl Scene for Harbour {
    fn name(&self) -> &'static str {
        "harbour"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let quay = self.quay_px();
        let w = self.width as i32;

        // the town: a row of houses of one to four storeys behind the quay
        self.buildings.clear();
        let tallest = (quay - 6).clamp(2, 14);
        let right_edge = if self.lighthouse_fits() { w - 10 } else { w };
        let mut x = rng.below(3) as i32;
        while x < right_edge - 4 {
            let width = 5 + rng.below(6) as i32;
            let height = 2 + rng.below(tallest as usize - 1) as i32;
            let mut windows = Vec::new();
            for wy in (1..height - 2).step_by(3) {
                for wx in (1..width - 2).step_by(2) {
                    windows.push((wx, wy, rng.chance(0.45)));
                }
            }
            self.buildings.push(Building {
                x,
                width,
                height,
                wall: rng.below(4),
                pitched: rng.chance(0.6),
                windows,
            });
            x += width + rng.below(2) as i32;
        }

        let sky = (quay - tallest - 6).max(0);
        let stars = count_for(self.width, (sky / 2).max(1) as usize, 30, 2, 120);
        self.stars = if sky > 0 {
            (0..stars)
                .map(|_| {
                    (
                        rng.below(self.width) as i32,
                        rng.below(sky as usize) as i32,
                        rng.chance(0.25),
                    )
                })
                .collect()
        } else {
            Vec::new()
        };

        let spacing = if self.width >= 60 { 14 } else { 9 };
        self.lamps = (0..)
            .map(|i| 3 + i * spacing)
            .take_while(|&x| x < right_edge - 2)
            .collect();

        // boats sit in the water, the larger ones further out
        self.boats.clear();
        let water_top = self.water_top();
        let bottom = self.height as i32 * 2 - 1;
        let wanted = self
            .motion
            .scaled(count_for(self.width, self.height, 260, 1, 8));
        for _ in 0..wanted * 4 {
            if self.boats.len() >= wanted {
                break;
            }
            let roomy = bottom - water_top >= 12;
            let hull = match rng.below(3) {
                0 if roomy => Hull::Sail,
                1 if roomy => Hull::Cabin,
                _ => Hull::Rowing,
            };
            let (hw, hh) = hull.size();
            let highest = water_top + hull.tall() - hh + 4;
            let lowest = bottom - 2;
            if lowest < highest || w < hw + 2 {
                continue;
            }
            let y = highest + rng.below((lowest - highest + 1) as usize) as i32;
            let x = rng.below((w - hw) as usize) as i32;
            let clash = self.boats.iter().any(|b| {
                let (bw, _) = b.hull.size();
                x < b.x + bw + 2 && b.x < x + hw + 2 && (y - b.y).abs() < 6
            });
            if !clash {
                self.boats.push(Boat {
                    x,
                    y,
                    hull,
                    colour: rng.below(3),
                    phase: rng.range(0.0, TAU),
                });
            }
        }
        // nearer boats are drawn last so they sit in front
        self.boats.sort_by_key(|b| b.y);
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        if !self.ferry_fits() {
            return;
        }
        if self.ferry_wait > 0.0 {
            self.ferry_wait -= dt;
            return;
        }
        self.ferry_x += 0.8 * self.motion.speed() * dt;
        if self.ferry_x > self.width as f32 + 1.0 {
            self.ferry_x = -(FERRY_WIDTH as f32);
            self.ferry_wait = rng.range(8.0, 25.0);
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = HarbourColours::new(theme);
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

    fn drawn(harbour: &Harbour, width: usize, height: usize, theme: &ThemeData) -> Frame {
        let mut frame = Frame::new(width, height);
        harbour.draw(&mut frame, theme);
        frame
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("harbour");
        check_determinism("harbour");
        check_pixel_scene("harbour", 6.0);
    }

    #[test]
    fn the_shimmer_and_the_sway_stay_slow() {
        const { assert!(SHIMMER_PERIOD >= 1.0 / 3.0) };
        // four mast positions a sway, so under one change a second
        const { assert!(SWAY_PERIOD / 4.0 >= 1.0 / 3.0) };
        for motion in Motion::ALL {
            let mut rng = Rng::new(2);
            let harbour = Harbour::new(80, 24, motion, &mut rng);
            assert!(harbour.shimmer_period() >= SHIMMER_PERIOD);
            assert!(harbour.sway_period() >= SWAY_PERIOD);
            if motion.calm {
                assert_eq!(harbour.lean(1.0), 0);
            }
        }
    }

    #[test]
    fn the_quay_lamps_and_their_reflections_are_drawn() {
        let theme = theme::default_theme();
        let c = HarbourColours::new(theme);
        let mut rng = Rng::new(6);
        let mut harbour = Harbour::new(80, 24, Motion::FULL, &mut rng);
        let frame = render(&mut harbour, 80, 24, 40);
        let quay = harbour.quay();
        // the quay's cell is the coping over the wall all the way across
        for x in 0..80 {
            let (top, _) = frame.get(x, quay).unwrap().pixel_pair().unwrap();
            assert_eq!(top, c.stone_light);
        }
        let all = pixels(&frame);
        assert!(all.contains(&c.lamp));
        assert!(all.contains(&c.stripe), "no lighthouse");
        // the reflections reach into the water
        let reflected = (quay + 1..24).any(|y| {
            (0..80).any(|x| {
                frame
                    .get(x, y)
                    .and_then(|cell| cell.pixel_pair())
                    .is_some_and(|(a, b)| {
                        [a, b]
                            .iter()
                            .any(|&p| distance(p, c.lamp) < distance(c.water_top, c.lamp) * 0.6)
                    })
            })
        });
        assert!(reflected);
        let mut small_rng = Rng::new(1);
        let mut small = Harbour::new(20, 8, Motion::CALM, &mut small_rng);
        assert!(!small.lighthouse_fits());
        let frame = render(&mut small, 20, 8, 20);
        assert!(frame.drawn() > 0);
    }

    #[test]
    fn the_colours_stay_apart_and_in_their_bands_in_every_theme() {
        for theme in theme::all() {
            let c = HarbourColours::new(theme);
            let name = theme.name;
            apart(
                theme,
                c.sky_top,
                c.sky_low,
                0.03,
                "the sky's top and bottom",
            );
            apart(theme, c.sky_low, c.stone, 0.03, "the sky and the quay");
            apart(theme, c.stone, c.water_top, 0.03, "the quay and the water");
            apart(
                theme,
                c.water_top,
                c.water_deep,
                0.03,
                "the shallow and deep water",
            );
            apart(theme, c.water_top, c.swell, 0.03, "the water and its swell");
            apart(theme, c.roof, c.sky_low, 0.03, "the roofs and the sky");
            for wall in c.walls {
                apart(theme, wall, c.sky_low, 0.03, "a house and the sky");
                apart(theme, wall, c.roof, 0.03, "a house and its roof");
            }
            assert!(c.water_deep.luminance() < c.water_top.luminance(), "{name}");
            for (what, colour) in [
                ("sky top", c.sky_top),
                ("sky low", c.sky_low),
                ("glow", c.glow),
                ("roof", c.roof),
                ("stone", c.stone),
                ("water", c.water_top),
                ("deep water", c.water_deep),
            ]
            .into_iter()
            .chain(c.walls.iter().map(|&w| ("wall", w)))
            {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            // at night the lamps, the windows and the white paint shine as shapes; by day they
            // are pale imagery and the painted hulls and the red stripes carry the outline
            let solid: Vec<Rgb> = if c.day {
                c.hulls.iter().copied().chain([c.stripe]).collect()
            } else {
                c.hulls
                    .iter()
                    .copied()
                    .chain([c.lamp, c.window, c.stripe, c.tower, c.ferry])
                    .collect()
            };
            if c.day {
                for (what, colour) in [
                    ("lamp", c.lamp),
                    ("window", c.window),
                    ("tower", c.tower),
                    ("ferry", c.ferry),
                    ("cloud", c.cloud),
                    ("cloud shade", c.cloud_shade),
                ] {
                    assert!(
                        theme.contrast_of(colour) < IMAGERY_CEILING,
                        "the {what} in {name}"
                    );
                }
                apart(theme, c.cloud, c.sky_top, 0.05, "a cloud and the sky");
                apart(theme, c.window, c.dark_window, 0.1, "the panes");
            }
            for colour in solid {
                assert!(theme.contrast_of(colour) >= theme.floor(), "{name}");
                apart(theme, colour, c.water_top, 0.1, "a shape and the water");
                apart(
                    theme,
                    colour,
                    c.water_deep,
                    0.1,
                    "a shape and the deep water",
                );
            }
            apart(
                theme,
                c.window,
                c.walls[1],
                0.1,
                "a lit window and its wall",
            );
            apart(theme, c.stripe, c.tower, 0.1, "the lighthouse's stripes");
        }
    }

    #[test]
    fn the_256_colour_fallback_keeps_the_bands_apart() {
        for theme in theme::all() {
            let c = HarbourColours::new(theme);
            let near = |colour: Rgb| xterm_colour(pixel_256(colour, theme.background));
            let mut rng = Rng::new(8);
            let mut harbour = Harbour::new(80, 24, Motion::FULL, &mut rng);
            for _ in 0..60 {
                harbour.step(0.05, &mut rng);
            }
            let frame = drawn(&harbour, 80, 24, theme);
            let worst = pixels(&frame)
                .into_iter()
                .map(|colour| distance(colour, near(colour)))
                .fold(0.0_f32, f32::max);
            assert!(worst <= 0.15, "{} is {worst:.3} off", theme.name);
            assert_ne!(near(c.sky_low), near(c.stone), "{}", theme.name);
            assert_ne!(near(c.stone), near(c.water_top), "{}", theme.name);
            assert_ne!(near(c.water_top), near(c.water_deep), "{}", theme.name);
            for hull in c.hulls {
                assert_ne!(near(hull), near(c.water_top), "{}", theme.name);
            }
        }
    }
}
