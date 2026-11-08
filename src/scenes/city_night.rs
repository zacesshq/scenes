//! City night, painted in solid pixels: a sky that darkens upwards from a city glow on the
//! horizon, a hazy far skyline, near buildings with lit edges and rows of windows where one
//! window at a time lights up or goes dark, a crescent moon, a river that catches the lights
//! and, on a screen with room, an arched bridge that a lit train crosses now and then.
//!
//! A light theme shows the city at dusk instead: a fading blue sky with a pale moon, the panes
//! dark glass catching the evening and the lights coming on in them a soft warm glow.
//!
//! Every colour comes from the active theme. The sky, the buildings, the quay and the water are
//! tints of the background held below the non-text floor, so they read as background; the lit
//! windows, the moon and the train keep the theme's own drawing colours, so they stand out and
//! keep their outline when colour is off.

use super::{count_for, cycle, paint};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "A painted night skyline where windows light and darken one at a time, a moon, a river and a passing train, at dusk in light themes";

pub const CAPTIONS: &[&str] = &[
    "The city sleeps. So do the errors.",
    "Every light in the right window.",
    "A clean build after dark.",
    "Quiet streets and a green build.",
    "The last train home is on time.",
];

/// Seconds between two windows changing, at the standard pace. Only one window changes at a
/// time, so no cell changes in place more than once in this long.
pub const WINDOW_PERIOD: f32 = 1.2;
/// Pixels a second the train crosses at.
const TRAIN_SPEED: f32 = 9.0;
/// Pixel width of one carriage and of the locomotive.
const CAR_WIDTH: usize = 14;
const ENGINE_WIDTH: usize = 12;
/// Steps in the sky ramp; few steps keep the escape output small.
const SKY_STEPS: usize = 10;

/// A carriage: roof, a row of lit windows, the body and the wheels.
const CAR: [&str; 4] = [
    ".rrrrrrrrrrrr.",
    "bwwbwwbwwbwwbb",
    "bbbbbbbbbbbbbb",
    ".kk.......kk..",
];
/// The locomotive facing right; mirrored when the train runs left.
const ENGINE: [&str; 4] = [
    ".rrrrrrr....",
    "bbwwbbbbbbb.",
    "bbbbbbbbbbbl",
    ".kk....kk...",
];

/// What a building has on its roof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Roof {
    Flat,
    Antenna,
    Stepped,
    Tank,
}

#[derive(Clone, Debug)]
struct Building {
    x: i32,
    width: i32,
    /// Height in pixels above the ground row.
    height: i32,
    roof: Roof,
}

#[derive(Clone, Debug)]
struct Train {
    x: f32,
    speed: f32,
    cars: usize,
}

impl Train {
    fn length(&self) -> f32 {
        (self.cars * (CAR_WIDTH + 1) + ENGINE_WIDTH) as f32
    }
}

/// The city's colours in one theme, every one derived from the theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct CityColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub star: Rgb,
    pub far: Rgb,
    pub far_window: Rgb,
    pub wall: Rgb,
    pub wall_lit: Rgb,
    pub wall_shade: Rgb,
    pub dark_window: Rgb,
    pub quay: Rgb,
    pub water_top: Rgb,
    pub water_deep: Rgb,
    pub ripple: Rgb,
    pub reflection: Rgb,
    pub moon: Rgb,
    pub moon_shade: Rgb,
    pub window: Rgb,
    pub window_top: Rgb,
    pub beacon: Rgb,
    pub deck: Rgb,
    pub girder: Rgb,
    pub train: Rgb,
    pub train_roof: Rgb,
    pub train_window: Rgb,
    pub wheel: Rgb,
    pub lamp: Rgb,
    /// Whether the moon's dark side and halo show, which only reads on a dark sky.
    pub earthshine: bool,
    sky: Ramp,
    water: Ramp,
}

impl CityColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let glow_hue = mix(theme.magenta(), theme.warning, 0.5);
        let haze_hue = mix(theme.info, theme.muted, 0.5);
        // a dark theme glows towards the horizon and keeps the near buildings darker than the
        // far ones, as haze lifts what is distant; a light theme reads as dusk, with the near
        // buildings the strongest shapes
        let (sky_top, sky_low, far, wall) = if theme.dark {
            (
                shades.tint(theme.info, 0.08, 1.1),
                shades.tint(glow_hue, 0.4, 1.9),
                shades.tint(haze_hue, 0.4, 1.55),
                shades.tint(theme.muted, 0.14, 1.25),
            )
        } else {
            (
                shades.tint(theme.info, 0.5, 1.9),
                shades.tint(glow_hue, 0.25, 1.3),
                shades.tint(haze_hue, 0.45, 1.65),
                shades.tint(theme.muted, 0.6, 2.5),
            )
        };
        let wall_lit = shades.tint(
            mix(wall, shades.lightest(), 0.35),
            1.0,
            IMAGERY_CEILING - 0.3,
        );
        let wall_shade = shades.darker(wall, 0.4);
        // unlit glass is cooler than the wall, so it never reads as a dim lit window
        let dark_window = if theme.dark {
            shades.darker(mix(wall, theme.blue(), 0.3), 0.25)
        } else {
            // by day an unlit pane is dark glass, just solid, so the towers keep their outline
            paint::strong(theme, shades.tint(mix(wall, theme.blue(), 0.5), 1.0, 2.9))
        };
        // at night a lit window shines in the theme's yellow; at dusk it is a soft warm glow,
        // lighter than the wall around it
        let (window, window_top, lamp) = if theme.dark {
            (theme.yellow(), theme.bright_yellow(), theme.bright_yellow())
        } else {
            let glow = shades.tint(mix(theme.yellow(), theme.warning, 0.3), 0.6, 1.45);
            (glow, mix(glow, WHITE, 0.5), glow)
        };
        let (moon, star) = if theme.dark {
            (theme.bright_yellow(), None)
        } else {
            (mix(WHITE, sky_top, 0.12), Some(sky_top))
        };
        let (water_top, water_deep) = if theme.dark {
            (
                shades.tint(theme.blue(), 0.35, 1.7),
                shades.tint(theme.blue(), 0.1, 1.12),
            )
        } else {
            (
                shades.tint(theme.blue(), 0.3, 1.6),
                shades.tint(theme.blue(), 0.6, 2.5),
            )
        };
        let quay = shades.tint(theme.muted, if theme.dark { 0.3 } else { 0.4 }, 1.9);
        let ripple = shades.tint(shades.lighter(water_top, 0.35), 1.0, IMAGERY_CEILING - 0.1);
        let reflection = shades.tint(theme.yellow(), 0.6, IMAGERY_CEILING - 0.05);
        let far_window = shades.tint(mix(far, theme.yellow(), 0.45), 1.0, 2.2);
        let star = star.unwrap_or_else(|| shades.lighter(sky_top, 0.6));
        let train = theme.red();
        Self {
            sky_top,
            sky_low,
            star,
            far,
            far_window,
            wall,
            wall_lit,
            wall_shade,
            dark_window,
            quay,
            water_top,
            water_deep,
            ripple,
            reflection,
            moon,
            moon_shade: mix(sky_top, moon, 0.18),
            window,
            window_top,
            beacon: theme.red(),
            deck: theme.grey(),
            girder: mix(quay, water_top, 0.35),
            train,
            train_roof: shades.lighter(train, 0.3),
            train_window: window_top,
            wheel: shades.darker(train, 0.6),
            lamp,
            earthshine: theme.dark,
            sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
            water: Ramp::new(water_top, water_deep, SKY_STEPS / 2),
        }
    }
}

/// Pure white, the daytime moon: a touch above an off-white background, so it stays imagery.
const WHITE: Rgb = Rgb::new(255, 255, 255);

pub struct CityNight {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    far: Vec<Building>,
    near: Vec<Building>,
    /// The windows of the near buildings: each one's top left pixel and whether it is lit.
    windows: Vec<(i32, i32, bool)>,
    stars: Vec<(i32, i32, bool)>,
    since_window: f32,
    train: Option<Train>,
    next_train: f32,
    trains_seen: u32,
    window_changes: u32,
}

impl CityNight {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut city = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            far: Vec::new(),
            near: Vec::new(),
            windows: Vec::new(),
            stars: Vec::new(),
            since_window: 0.0,
            train: None,
            next_train: rng.range(4.0, 12.0),
            trains_seen: 0,
            window_changes: 0,
        };
        city.resize(width, height, rng);
        city
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    fn has_river(&self) -> bool {
        self.height >= 10
    }

    fn has_bridge(&self) -> bool {
        self.height >= 14 && self.width >= 30
    }

    fn has_moon(&self) -> bool {
        self.width >= 30 && self.height >= 12
    }

    /// The pixel row the buildings stand on: the first row below them.
    fn ground(&self) -> i32 {
        let tall = self.tall();
        if self.has_bridge() {
            tall - 8
        } else if self.has_river() {
            tall - 5
        } else {
            (tall - 1).max(1)
        }
    }

    /// The first pixel row of the river.
    fn river(&self) -> i32 {
        self.tall() - 4
    }

    /// The top pixel row of the bridge deck, which the train's wheels run on.
    fn deck(&self) -> i32 {
        self.tall() - 6
    }

    fn window_period(&self) -> f32 {
        WINDOW_PERIOD / self.motion.pace()
    }

    fn skyline(
        rng: &mut Rng,
        width: i32,
        low: i32,
        high: i32,
        narrow: i32,
        roofs: bool,
    ) -> Vec<Building> {
        let mut out = Vec::new();
        let mut x = -(rng.below(4) as i32);
        while x < width {
            let w = narrow + rng.below(8) as i32;
            let h = low + rng.below((high - low).max(1) as usize) as i32;
            let roof = if !roofs {
                Roof::Flat
            } else {
                match rng.below(8) {
                    0 | 1 => Roof::Antenna,
                    2 => Roof::Stepped,
                    3 => Roof::Tank,
                    _ => Roof::Flat,
                }
            };
            out.push(Building {
                x,
                width: w,
                height: h,
                roof,
            });
            x += w + rng.below(3) as i32;
        }
        out
    }

    #[cfg(test)]
    pub(crate) fn trains_seen(&self) -> u32 {
        self.trains_seen
    }

    fn paint_sky(&self, canvas: &mut Canvas, c: &CityColours) {
        let ground = self.ground();
        let wide = canvas.width() as i32;
        for y in 0..ground.min(canvas.height() as i32) {
            let t = (y as f32 / ground.max(1) as f32).powf(1.6);
            canvas.fill_row(y, c.sky.at(t));
        }
        for &(x, y, bright) in &self.stars {
            let colour = if bright {
                c.star
            } else {
                mix(c.sky_top, c.star, 0.5)
            };
            canvas.set(x, y, colour);
        }
        // a still crescent moon with a faint halo; it never changes, so it can never flash
        if self.has_moon() {
            let r = (self.tall() / 14).clamp(2, 6);
            let (cx, cy) = (wide / 6, r + 3);
            let rf = r as f32;
            for dy in -r - 2..=r + 2 {
                for dx in -r - 2..=r + 2 {
                    let d = ((dx * dx + dy * dy) as f32).sqrt();
                    let (sx, sy) = (dx as f32 - rf * 0.55, dy as f32 + rf * 0.3);
                    let shadowed = (sx * sx + sy * sy).sqrt() <= rf;
                    if d <= rf + 0.3 {
                        if !shadowed {
                            canvas.set(cx + dx, cy + dy, c.moon);
                        } else if c.earthshine {
                            canvas.set(cx + dx, cy + dy, c.moon_shade);
                        }
                    } else if c.earthshine && d <= rf + 2.0 {
                        canvas.blend(cx + dx, cy + dy, c.moon, 0.1);
                    }
                }
            }
        }
    }

    fn paint_buildings(&self, canvas: &mut Canvas, c: &CityColours) {
        let ground = self.ground();
        for b in &self.far {
            let top = ground - b.height;
            canvas.fill_rect(b.x, top, b.width, b.height, c.far);
            // a few still windows far off, so the haze has a city in it
            let mut y = top + 2;
            while y < ground - 1 {
                let mut x = b.x + 1;
                while x < b.x + b.width - 1 {
                    let h = (x as u32).wrapping_mul(0x9e37_79b1) ^ (y as u32).wrapping_mul(0x85eb);
                    if (h >> 9) % 5 == 0 {
                        canvas.set(x, y, c.far_window);
                    }
                    x += 2;
                }
                y += 3;
            }
        }
        for b in &self.near {
            let top = ground - b.height;
            let right = b.x + b.width - 1;
            canvas.fill_rect(b.x, top, b.width, b.height, c.wall);
            // moonlight from the upper left lights the left edge and the roof line and leaves
            // the right edge in shade, which gives each block its depth
            canvas.fill_rect(b.x, top, 1, b.height, c.wall_lit);
            canvas.fill_rect(right, top, 1, b.height, c.wall_shade);
            canvas.fill_rect(b.x, top, b.width, 1, c.wall_lit);
            match b.roof {
                Roof::Flat => {}
                Roof::Antenna => {
                    let x = b.x + b.width / 2;
                    let mast = (b.height / 3).clamp(2, 6);
                    canvas.fill_rect(x, top - mast, 1, mast, c.wall_lit);
                    canvas.set(x, top - mast - 1, c.beacon);
                }
                Roof::Stepped => {
                    let inset = (b.width / 4).max(1);
                    let w = b.width - 2 * inset;
                    if w >= 2 {
                        canvas.fill_rect(b.x + inset, top - 3, w, 3, c.wall);
                        canvas.fill_rect(b.x + inset, top - 3, w, 1, c.wall_lit);
                        canvas.fill_rect(b.x + inset, top - 3, 1, 3, c.wall_lit);
                    }
                }
                Roof::Tank => {
                    let x = b.x + b.width - 5;
                    if x > b.x {
                        canvas.sprite(
                            x,
                            top - 4,
                            &[".ll.", "llss", "llss", "l..s"],
                            &[('l', c.wall_lit), ('s', c.wall_shade)],
                        );
                    }
                }
            }
        }
        for &(x, y, lit) in &self.windows {
            if lit {
                canvas.fill_rect(x, y, 2, 1, c.window_top);
                canvas.fill_rect(x, y + 1, 2, 1, c.window);
            } else {
                canvas.fill_rect(x, y, 2, 2, c.dark_window);
            }
        }
    }

    fn paint_water(&self, canvas: &mut Canvas, c: &CityColours) {
        let ground = self.ground();
        let tall = canvas.height() as i32;
        let wide = canvas.width() as i32;
        if !self.has_river() {
            canvas.fill_row(ground, c.quay);
            return;
        }
        let river = self.river();
        canvas.fill_rect(0, ground, wide, river - ground, c.quay);
        let phase = cycle(self.t, 0.7 * self.motion.pace());
        let threshold = if self.motion.calm { 1.3 } else { 1.1 };
        for y in river..tall {
            let depth = (y - river) as f32 / (tall - river - 1).max(1) as f32;
            for x in 0..wide {
                let (xf, yf) = (x as f32, y as f32);
                let v = (xf * 0.29 + yf * 1.7 - phase).sin()
                    + 0.6 * (xf * 0.13 - yf + phase * 0.5 + 2.0).sin();
                let colour = if v > threshold {
                    c.ripple
                } else {
                    c.water.at(depth)
                };
                canvas.set(x, y, colour);
            }
        }
        // lit windows leave broken streaks of light on the water below them
        for &(x, _, lit) in &self.windows {
            if !lit || x % 3 != 0 {
                continue;
            }
            for (i, y) in (river..tall).enumerate() {
                if (x / 3 + i as i32) % 2 == 0 {
                    canvas.set(x, y, c.reflection);
                }
            }
        }
        if self.has_moon() {
            let mx = wide / 6;
            for (i, y) in (river..tall).enumerate() {
                let half = 1 + (i as i32 % 2);
                canvas.fill_rect(mx - half, y, 2 * half, 1, c.reflection);
            }
        }
    }

    fn paint_bridge(&self, canvas: &mut Canvas, c: &CityColours) {
        if !self.has_bridge() {
            return;
        }
        let deck = self.deck();
        let tall = canvas.height() as i32;
        let wide = canvas.width() as i32;
        let span = 18;
        // shallow arches spring from piers in the river up to the deck, drawn as one joined
        // line so they never break into loose pixels
        let mut last: Option<i32> = None;
        for x in 0..wide {
            let along = (x + 4).rem_euclid(span);
            if along < 2 {
                canvas.fill_rect(x, deck + 2, 1, tall - deck - 2, c.girder);
                last = None;
                continue;
            }
            let u = (along as f32 - 1.5) / (span as f32 - 3.0) * 2.0 - 1.0;
            let y = deck + 2 + ((u * u) * (tall - deck - 3) as f32).round() as i32;
            let from = last.unwrap_or(y);
            for row in from.min(y)..=from.max(y) {
                canvas.set(x, row, c.girder);
            }
            last = Some(y);
        }
        canvas.fill_row(deck, c.deck);
        canvas.fill_row(deck + 1, c.girder);
        // lamps along the deck
        let mut x = 5;
        while x < wide {
            canvas.set(x, deck - 1, c.girder);
            canvas.set(x, deck - 2, c.lamp);
            x += span;
        }
        if let Some(train) = &self.train {
            let palette = [
                ('r', c.train_roof),
                ('b', c.train),
                ('w', c.train_window),
                ('k', c.wheel),
                ('l', c.lamp),
            ];
            let right = train.speed > 0.0;
            let top = deck - CAR.len() as i32;
            let mut x = train.x.round() as i32;
            let engine: Vec<String> = if right {
                ENGINE.iter().map(|r| r.to_string()).collect()
            } else {
                ENGINE.iter().map(|r| r.chars().rev().collect()).collect()
            };
            let engine: Vec<&str> = engine.iter().map(String::as_str).collect();
            if !right {
                canvas.sprite(x, top, &engine, &palette);
                x += ENGINE_WIDTH as i32 + 1;
            }
            for _ in 0..train.cars {
                canvas.sprite(x, top, &CAR, &palette);
                // a coupling between cars
                canvas.set(x + CAR_WIDTH as i32, top + 2, c.wheel);
                x += CAR_WIDTH as i32 + 1;
            }
            if right {
                canvas.sprite(x, top, &engine, &palette);
            }
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &CityColours) {
        self.paint_sky(canvas, c);
        self.paint_buildings(canvas, c);
        self.paint_water(canvas, c);
        self.paint_bridge(canvas, c);
    }
}

impl Scene for CityNight {
    fn name(&self) -> &'static str {
        "city-night"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let w = self.width as i32;
        let ground = self.ground();
        let room = (ground - 2).max(2);
        // the far skyline stands taller than the near one, so it shows above and between it
        self.far = Self::skyline(rng, w, (room / 2).max(2), (room * 9 / 10).max(3), 5, false);
        self.near = Self::skyline(rng, w, (room / 3).max(3), (room * 7 / 10).max(4), 7, true);
        self.windows.clear();
        for b in &self.near {
            let top = ground - b.height;
            // windows two pixels square on even rows, so each fills one cell row, with wall
            // showing between them
            let mut y = top + 2;
            if y % 2 != 0 {
                y += 1;
            }
            while y + 2 < ground {
                let mut x = b.x + 2;
                while x + 2 < b.x + b.width {
                    self.windows.push((x, y, rng.chance(0.4)));
                    x += 3;
                }
                y += 4;
            }
        }
        let sky = ground.max(1) as usize;
        let wanted = count_for(self.width, sky / 2, 40, 2, 120);
        self.stars = (0..self.motion.scaled(wanted))
            .map(|_| {
                (
                    rng.below(self.width) as i32,
                    rng.below(sky) as i32,
                    rng.chance(0.3),
                )
            })
            .collect();
        self.train = None;
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        self.since_window += dt;
        let period = self.window_period();
        if self.since_window >= period {
            self.since_window = (self.since_window - period).min(period);
            if !self.windows.is_empty() {
                let i = rng.below(self.windows.len());
                self.windows[i].2 = !self.windows[i].2;
                self.window_changes += 1;
            }
        }

        if !self.has_bridge() {
            return;
        }
        let width = self.width as f32;
        if let Some(train) = &mut self.train {
            train.x += train.speed * dt;
            let length = train.length();
            if train.x > width + length || train.x < -2.0 * length {
                self.train = None;
                self.next_train = if self.motion.calm {
                    rng.range(30.0, 60.0)
                } else {
                    rng.range(15.0, 35.0)
                };
            }
        } else {
            self.next_train -= dt;
            if self.next_train <= 0.0 {
                let right = rng.chance(0.5);
                let cars = 2 + rng.below(3);
                let speed = TRAIN_SPEED * self.motion.speed();
                let mut train = Train {
                    x: 0.0,
                    speed: if right { speed } else { -speed },
                    cars,
                };
                train.x = if right { -train.length() } else { width };
                self.train = Some(train);
                self.trains_seen += 1;
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = CityColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colour::xterm_colour;

    use crate::pixels::distance;
    use crate::scenes::testing::{
        apart, check_determinism, check_pixel_scene, check_scene, pixels,
    };
    use crate::terminal::pixel_256;
    use crate::theme;

    fn drawn(city: &CityNight, width: usize, height: usize, theme: &ThemeData) -> Frame {
        let mut frame = Frame::new(width, height);
        city.draw(&mut frame, theme);
        frame
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("city-night");
        check_determinism("city-night");
        check_pixel_scene("city-night", 6.0);
    }

    #[test]
    fn sprites_are_rectangular() {
        assert!(CAR.iter().all(|r| r.len() == CAR_WIDTH));
        assert!(ENGINE.iter().all(|r| r.len() == ENGINE_WIDTH));
    }

    #[test]
    fn one_window_changes_at_a_time_and_never_quickly() {
        let mut rng = Rng::new(4);
        for motion in Motion::ALL {
            let mut city = CityNight::new(80, 24, motion, &mut rng);
            assert!(city.window_period() >= WINDOW_PERIOD);
            let before: Vec<bool> = city.windows.iter().map(|w| w.2).collect();
            city.step(0.05, &mut rng);
            let after: Vec<bool> = city.windows.iter().map(|w| w.2).collect();
            let changed = before.iter().zip(&after).filter(|(a, b)| a != b).count();
            assert!(changed <= 1);
            for _ in 0..(60 * 20) {
                city.step(0.05, &mut rng);
            }
            assert!(city.window_changes as f32 <= 60.0 / WINDOW_PERIOD + 1.0);
            assert!(city.window_changes > 10);
        }
    }

    #[test]
    fn windows_fill_whole_cells_so_a_change_touches_two_cells() {
        let mut rng = Rng::new(5);
        let city = CityNight::new(80, 24, Motion::FULL, &mut rng);
        assert!(city.windows.len() > 20, "{}", city.windows.len());
        for &(_, y, _) in &city.windows {
            assert_eq!(y % 2, 0);
            assert!(y + 2 <= city.ground());
        }
    }

    #[test]
    fn the_colours_stay_distinct_and_in_bounds_in_every_theme() {
        for theme in theme::all() {
            let c = CityColours::new(theme);
            let name = theme.name;
            apart(
                theme,
                c.sky_top,
                c.sky_low,
                0.03,
                "the sky's top and its glow",
            );
            apart(
                theme,
                c.far,
                c.sky_low,
                0.02,
                "the far skyline and the glow",
            );
            apart(theme, c.far, c.wall, 0.03, "the far and near buildings");
            apart(theme, c.wall, c.wall_lit, 0.03, "a wall and its lit edge");
            apart(
                theme,
                c.wall,
                c.dark_window,
                0.015,
                "a wall and a dark window",
            );
            apart(theme, c.wall, c.sky_top, 0.02, "the buildings and the sky");
            apart(theme, c.water_top, c.quay, 0.03, "the quay and the river");
            apart(
                theme,
                c.water_top,
                c.water_deep,
                0.03,
                "the shallow and deep river",
            );
            apart(theme, c.ripple, c.water_top, 0.03, "a ripple and the river");
            for (what, colour) in [
                ("sky top", c.sky_top),
                ("sky low", c.sky_low),
                ("far skyline", c.far),
                ("far windows", c.far_window),
                ("wall", c.wall),
                ("lit edge", c.wall_lit),
                ("quay", c.quay),
                ("water", c.water_top),
                ("deep water", c.water_deep),
                ("ripple", c.ripple),
                ("reflection", c.reflection),
            ] {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            // the things that matter keep the theme's floor and stand out from the walls; at
            // dusk the lights and the moon are pale imagery and the dark glass keeps the outline
            if theme.dark {
                for colour in [c.window, c.window_top, c.moon, c.train, c.train_window] {
                    assert!(theme.contrast_of(colour) >= theme.floor(), "{name}");
                    assert!(distance(colour, c.wall) >= 0.1, "{name}");
                    assert!(distance(colour, c.sky_low) >= 0.1, "{name}");
                }
            } else {
                assert!(theme.contrast_of(c.train) >= theme.floor(), "{name}");
                assert!(
                    theme.contrast_of(c.dark_window) >= IMAGERY_CEILING,
                    "{name}"
                );
                for colour in [c.window, c.window_top, c.moon, c.train_window] {
                    assert!(theme.contrast_of(colour) < IMAGERY_CEILING, "{name}");
                    assert!(distance(colour, c.wall) >= 0.1, "{name}");
                }
                assert!(distance(c.moon, c.sky_top) >= 0.1, "{name}");
            }
            assert!(
                distance(c.window, c.dark_window) >= 0.1,
                "{name}: {:.3}",
                distance(c.window, c.dark_window)
            );
        }
    }

    #[test]
    fn every_theme_paints_a_skyline_with_lit_windows_and_a_river() {
        for theme in theme::all() {
            let c = CityColours::new(theme);
            let mut rng = Rng::new(4);
            let mut city = CityNight::new(80, 24, Motion::FULL, &mut rng);
            for _ in 0..60 {
                city.step(0.05, &mut rng);
            }
            let frame = drawn(&city, 80, 24, theme);
            let all = pixels(&frame);
            for (what, colour) in [
                ("sky", c.sky_top),
                ("far skyline", c.far),
                ("wall", c.wall),
                ("lit window", c.window),
                ("dark window", c.dark_window),
                ("moon", c.moon),
                ("deck", c.deck),
            ] {
                assert!(all.contains(&colour), "no {what} in {}", theme.name);
            }
            let mut used = all.clone();
            used.sort_unstable();
            used.dedup();
            assert!(
                (15..=120).contains(&used.len()),
                "{} uses {} colours",
                theme.name,
                used.len()
            );
            // the bottom row is river, lit here and there by ripples and reflections
            let (_, bottom) = frame.get(40, 23).unwrap().pixel_pair().unwrap();
            assert!(
                c.water.steps().contains(&bottom)
                    || bottom == c.ripple
                    || bottom == c.reflection
                    || bottom == c.girder,
                "{}",
                theme.name
            );
        }
    }

    #[test]
    fn the_256_colour_fallback_keeps_the_bands_apart() {
        for theme in theme::all() {
            let c = CityColours::new(theme);
            let near = |colour: Rgb| xterm_colour(pixel_256(colour, theme.background));
            for (a, b, what) in [
                (c.wall, c.window, "wall and window"),
                (c.water_top, c.reflection, "river and reflection"),
                (c.sky_low, c.moon, "sky and moon"),
            ] {
                assert_ne!(near(a), near(b), "{what} collapse in {}", theme.name);
            }
            for colour in [c.sky_top, c.sky_low, c.wall, c.far, c.water_top] {
                assert!(
                    distance(colour, near(colour)) <= 0.15,
                    "{} in {}",
                    colour.hex(),
                    theme.name
                );
            }
        }
    }

    #[test]
    fn a_train_crosses_the_bridge_and_small_screens_have_none() {
        let theme = theme::default_theme();
        let c = CityColours::new(theme);
        let mut rng = Rng::new(9);
        let mut city = CityNight::new(80, 24, Motion::FULL, &mut rng);
        let mut seen = false;
        for _ in 0..(60 * 20) {
            city.step(0.05, &mut rng);
            if city.train.is_some() {
                let frame = drawn(&city, 80, 24, theme);
                // the train's windows ride just above the deck
                let row: Vec<Rgb> = (0..80)
                    .filter_map(|x| frame.get(x, 19).and_then(|cell| cell.pixel_pair()))
                    .flat_map(|(a, b)| [a, b])
                    .collect();
                seen |= row.contains(&c.train_window);
            }
        }
        assert!(seen && city.trains_seen() >= 1);
        let mut small = CityNight::new(20, 8, Motion::FULL, &mut rng);
        for _ in 0..(60 * 20) {
            small.step(0.05, &mut rng);
        }
        assert_eq!(small.trains_seen(), 0);
    }
}
