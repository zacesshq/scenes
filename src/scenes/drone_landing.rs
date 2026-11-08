//! A drone landing, painted in solid pixels: a quadcopter with a body, four arms and their
//! rotors comes in over rolling country, a faint beam from the sensor under it mapping the
//! ground below as a line of dots. The dots fall on the ground's own contour, the drone gets
//! its position fix, settles on a marked landing pad and its rotors spin down.
//!
//! The same picture is the `landing-abort` fail scene: there the dots scatter above and below
//! the ground, the drone hovers over the pad without a match and climbs calmly away.
//!
//! The sky, clouds, hills, ground and beam are tints of the background held below the non-text
//! floor, so they read as background. The drone, the pad and the mapped dots keep the theme's
//! own drawing colours, so they stand out as shapes and keep their outline with colour off.

use std::collections::HashMap;

use super::cycle;
use super::paint::{self, Cloud, Wave, ease, held_imagery, shaded, strong, unit};
use crate::colour::Rgb;
use crate::frame::{Cell, Frame};
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "A quadcopter mapping rolling ground at dusk with a faint beam, then settling on a marked landing pad, by day in light themes";

pub const CAPTIONS: &[&str] = &[
    "Position fix good. Build down safely.",
    "The ground matches the map. All green.",
    "Touchdown. Every check passed.",
    "Rotors spinning down on a clean build.",
    "Mapped, matched and landed.",
];

/// Story seconds the approach takes, from off the left edge to a hover over the pad.
pub const APPROACH: f32 = 16.0;
/// When the beam starts mapping the ground and when it stops.
const SCAN_FROM: f32 = 2.5;
const SCAN_TO: f32 = APPROACH + 1.5;
/// The landing: a short hover, then the descent onto the pad.
const HOVER_TO: f32 = 19.0;
const LANDED: f32 = 25.0;
/// The rotors spin down over this long once the drone is down.
const SPIN_DOWN: f32 = 6.0;
/// The aborted approach: a hover over the pad, then a calm climb away.
const ABORT_HOVER_TO: f32 = 21.0;
const CLIMBED: f32 = 33.0;
/// Seconds the beam's sweep takes to cross from one side to the other and back.
pub const SWEEP_PERIOD: f32 = 3.2;
/// Seconds a hovering drone takes to bob up and back down by about a pixel.
pub const BOB_PERIOD: f32 = 4.0;
const SKY_STEPS: usize = 8;

/// The drone's colours, the land's and the sky's in one theme, every one derived from the
/// theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct DroneColours {
    pub background: Rgb,
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub cloud: Rgb,
    pub cloud_shade: Rgb,
    pub hills: Rgb,
    pub ground: Rgb,
    pub ground_low: Rgb,
    pub crest: Rgb,
    /// The hue the beam lifts whatever it crosses towards, held under the ceiling.
    pub beam: Rgb,
    pub blur: Rgb,
    pub body: Rgb,
    pub body_lit: Rgb,
    pub body_dark: Rgb,
    pub arm: Rgb,
    pub blade: Rgb,
    pub lens: Rgb,
    pub dot: Rgb,
    pub stray: Rgb,
    pub deck: Rgb,
    pub deck_lit: Rgb,
    pub stripe: Rgb,
    pub lamp: Rgb,
    pub status: Rgb,
    pub warn: Rgb,
    sky: Ramp,
}

impl DroneColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let green = theme.green();
        let hazy = mix(green, theme.info, 0.5);
        // a dark theme is the same field at dusk, deepest overhead; a light theme is a clear
        // day, a stronger blue overhead paling to the horizon
        let (sky_top, sky_low, hills, ground, ground_low, crest) = if theme.dark {
            (
                shades.tint(theme.blue(), 0.16, 1.25),
                shades.tint(theme.info, 0.42, 1.95),
                shades.tint(hazy, 0.32, 1.5),
                shades.tint(green, 0.5, 2.05),
                shades.tint(green, 0.3, 1.45),
                shades.tint(green, 0.68, 2.55),
            )
        } else {
            (
                shades.tint(theme.info, 0.5, 1.95),
                shades.tint(theme.info, 0.14, 1.18),
                shades.tint(hazy, 0.32, 1.5),
                shades.tint(green, 0.55, 2.2),
                shades.tint(green, 0.8, 2.75),
                shades.tint(green, 0.38, 1.7),
            )
        };
        let (cloud, cloud_shade) = paint::cloud_tones(theme, 2.2);
        let body = strong(theme, mix(theme.grey(), theme.foreground, 0.35));
        let (body_dark, _, body_lit) = shaded(theme, body);
        // the deck is a grey held near the solid floor, darker than the pale drone on a dark
        // theme and paler than the dark drone on a light one, so the yellow markings and the
        // drone both stand off it
        let deck = if theme.dark {
            paint::solid(theme, theme.grey(), shades.darkest(), 0.5)
        } else {
            paint::solid(theme, theme.grey(), shades.lightest(), 0.6)
        };
        Self {
            background: theme.background,
            sky_top,
            sky_low,
            cloud,
            cloud_shade,
            hills,
            ground,
            ground_low,
            crest,
            beam: theme.bright_cyan(),
            blur: shades.tint(theme.foreground, 0.5, IMAGERY_CEILING - 0.35),
            body,
            body_lit,
            body_dark,
            arm: body_dark,
            blade: theme.foreground,
            lens: theme.cyan(),
            dot: theme.cyan(),
            stray: theme.yellow(),
            deck,
            deck_lit: shaded(theme, deck).2,
            stripe: theme.yellow(),
            lamp: theme.green(),
            status: theme.green(),
            warn: theme.yellow(),
            sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
        }
    }
}

/// The drone's proportions in units: the body's half width, where each pair of arms ends,
/// each pair's rotor half width and the skids and legs.
#[derive(Clone, Copy, Debug)]
struct Geometry {
    body: i32,
    far_tip: i32,
    far_half: i32,
    near_tip: i32,
    near_half: i32,
    skid: (i32, i32),
    leg: i32,
}

/// The shape of the drone in units, a unit being `scale` pixels: the compact drone suits a
/// small screen.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Shape {
    compact: bool,
}

impl Shape {
    /// How far above the skids the sensor sits, in units.
    fn sensor(self) -> i32 {
        1
    }

    /// The half width of the whole drone, rotors included, in units.
    fn half_width(self) -> i32 {
        if self.compact { 8 } else { 13 }
    }

    /// The height of the whole drone in units.
    fn tall(self) -> i32 {
        if self.compact { 6 } else { 9 }
    }
}

pub struct DroneLanding {
    width: usize,
    height: usize,
    motion: Motion,
    /// Seconds into the story, scaled by the motion setting.
    story: f32,
    /// Seconds of wall time, for things that keep their own pace such as the sweep.
    t: f64,
    abort: bool,
    clouds: Vec<Cloud>,
    hills: Wave,
    land: Wave,
    seed: u32,
}

impl DroneLanding {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        Self::build(width, height, motion, rng, false)
    }

    /// The aborted approach, for the `landing-abort` fail scene.
    pub fn aborted(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        Self::build(width, height, motion, rng, true)
    }

    fn build(width: usize, height: usize, motion: Motion, rng: &mut Rng, abort: bool) -> Self {
        let mut scene = Self {
            width: 1,
            height: 1,
            motion,
            story: 0.0,
            t: 0.0,
            abort,
            clouds: Vec::new(),
            hills: Wave::new(rng, 14.0),
            land: Wave::new(rng, 11.0),
            seed: rng.next_u64() as u32,
        };
        scene.resize(width, height, rng);
        scene
    }

    fn wide(&self) -> i32 {
        self.width as i32
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    fn shape(&self) -> Shape {
        Shape {
            compact: self.wide() < 56 || self.tall() < 30,
        }
    }

    /// Pixels a drone unit takes: one on a classic terminal, more on a big screen.
    fn scale(&self) -> i32 {
        (self.tall() / 44).min(self.wide() / 56).clamp(1, 6)
    }

    /// The pad's centre column and half width.
    fn pad(&self) -> (i32, i32) {
        let s = self.scale();
        let half = if self.shape().compact { 6 } else { 9 } * s;
        ((self.wide() as f32 * 0.62) as i32, half)
    }

    /// The pixel row of the pad's deck, which the skids rest on.
    fn deck(&self) -> i32 {
        (self.tall() as f32 * 0.8) as i32
    }

    /// The row of the far hills' crest at a column.
    fn hill_at(&self, x: i32) -> i32 {
        let h = self.tall() as f32;
        (h * 0.6 + self.hills.at(x as f32 / self.scale() as f32) * h * 0.06).round() as i32
    }

    /// The row of the ground's surface at a column, flattened round the pad.
    fn ground_at(&self, x: i32) -> i32 {
        let h = self.tall() as f32;
        let rolling = h * 0.79 + self.land.at(x as f32 / self.scale() as f32) * h * 0.07;
        let (pad_x, half) = self.pad();
        let level = (self.deck() + 2 * self.scale()) as f32;
        let d = ((x - pad_x).abs() - half) as f32 / (half as f32 * 1.4).max(1.0);
        let flat = 1.0 - ease(d.max(0.0));
        (rolling + (level - rolling) * flat).round() as i32
    }

    /// Where the drone is: the centre column and the row its skids rest on, as fractions so the
    /// motion between pixels stays smooth.
    fn drone_at(&self) -> (f32, f32) {
        let s = self.scale() as f32;
        let deck = self.deck() as f32 - 1.0;
        let high = (self.shape().tall() as f32 + 5.0) * s;
        let hover = deck - 9.0 * s;
        let p = self.story;
        let bob = cycle(self.t, std::f32::consts::TAU / self.bob_period()).sin() * 0.6;
        let x = self.approach_x(p);
        // the descent eases in late, so the drone crosses the country high and drops at the end
        let y = high + (hover - high) * ease(p / APPROACH * 1.15 - 0.15);
        if p < APPROACH {
            return (x, y);
        }
        if !self.abort {
            if p < HOVER_TO {
                return (x, hover + bob);
            }
            let down = ease((p - HOVER_TO) / (LANDED - HOVER_TO));
            let fade = 1.0 - down;
            return (x, hover + (deck - hover) * down + bob * fade);
        }
        if p < ABORT_HOVER_TO {
            return (x, hover + bob);
        }
        let up = ease((p - ABORT_HOVER_TO) / (CLIMBED - ABORT_HOVER_TO));
        let away_x = (self.wide() as f32 * 0.84)
            .min(self.wide() as f32 - (self.shape().half_width() as f32 + 1.0) * s);
        // the drone holds below the band the clouds drift in, so it never sits in front of one
        let away_y = (high + 4.0 * s).max(self.tall() as f32 * 0.42).min(hover);
        (x + (away_x - x) * up, hover + (away_y - hover) * up + bob)
    }

    fn bob_period(&self) -> f32 {
        BOB_PERIOD / self.motion.pace()
    }

    fn sweep_period(&self) -> f32 {
        SWEEP_PERIOD / self.motion.pace()
    }

    /// How fast the rotors turn, from 1 in flight to 0 once they have stopped.
    fn spin(&self) -> f32 {
        if self.abort || self.story < LANDED {
            1.0
        } else {
            1.0 - ((self.story - LANDED) / SPIN_DOWN).clamp(0.0, 1.0)
        }
    }

    fn scanning(&self) -> bool {
        (SCAN_FROM..SCAN_TO).contains(&self.story)
    }

    /// The half width the beam covers on the ground.
    fn reach(&self) -> f32 {
        (if self.shape().compact { 6.0 } else { 10.0 }) * self.scale() as f32
    }

    /// The drone's column during the approach: quick across the country from off the left
    /// edge, slowing as it nears the pad.
    fn approach_x(&self, p: f32) -> f32 {
        let s = self.scale() as f32;
        let (pad_x, _) = self.pad();
        let start_x = -(self.shape().half_width() as f32) * s - 2.0;
        let u = (p / APPROACH).clamp(0.0, 1.0);
        let along = 1.0 - (1.0 - u) * (1.0 - u);
        start_x + (pad_x as f32 - start_x) * along
    }

    /// The columns of ground the beam has mapped so far.
    fn mapped(&self) -> Option<(f32, f32)> {
        if self.story < SCAN_FROM {
            return None;
        }
        let at = |p: f32| self.approach_x(p);
        let reach = self.reach();
        Some((at(SCAN_FROM) - reach, at(self.story.min(SCAN_TO)) + reach))
    }

    /// The words on the drone's readout and whether they report a problem.
    fn readout(&self) -> Option<(&'static str, Readout)> {
        if self.story < SCAN_FROM {
            return None;
        }
        Some(if self.story < APPROACH {
            ("MAPPING TERRAIN", Readout::Busy)
        } else if self.abort {
            ("NO TERRAIN MATCH", Readout::Bad)
        } else {
            ("POSITION FIX GOOD", Readout::Good)
        })
    }

    fn paint_sky(&self, canvas: &mut Canvas, c: &DroneColours) {
        let horizon = self.tall() * 7 / 10;
        for y in 0..self.tall() {
            canvas.fill_row(y, c.sky.at(y as f32 / horizon.max(1) as f32));
        }
        for cloud in &self.clouds {
            cloud.paint(canvas, c.cloud, c.cloud_shade);
        }
    }

    fn paint_land(&self, canvas: &mut Canvas, c: &DroneColours) {
        let tall = self.tall();
        for x in 0..self.wide() {
            let hill = self.hill_at(x);
            canvas.fill_rect(x, hill, 1, tall - hill, c.hills);
            let top = self.ground_at(x);
            canvas.fill_rect(x, top, 1, tall - top, c.ground);
            canvas.set(x, top, c.crest);
            // furrowed fields follow the roll of the land, closer together towards the front
            let s = self.scale();
            let mut y = top + 3 * s;
            let mut gap = 3 * s;
            while y < tall {
                canvas.fill_rect(x, y, 1, s, c.ground_low);
                y += gap + s;
                gap += s;
            }
        }
    }

    fn paint_pad(&self, canvas: &mut Canvas, c: &DroneColours) {
        let s = self.scale();
        let (pad_x, half) = self.pad();
        let deck = self.deck();
        let left = pad_x - half;
        canvas.fill_rect(left, deck, 2 * half + 1, 2 * s, c.deck);
        canvas.fill_rect(left, deck, 2 * half + 1, s, c.deck_lit);
        // chevrons along the deck's face mark it as a pad from the side
        let band = 3 * s;
        let mut x = left + s;
        let mut on = true;
        while x < pad_x + half - s + 1 {
            if on {
                let w = band.min(pad_x + half - s + 1 - x);
                canvas.fill_rect(x, deck + s, w, s, c.stripe);
            }
            x += band;
            on = !on;
        }
        // a lamp at each corner, steady
        for lx in [left, pad_x + half] {
            canvas.fill_rect(lx, deck - s, s, s, c.lamp);
        }
    }

    /// The row of the mapped dot for a column: on the ground's own surface when the scan
    /// matches, well above or below it when nothing lines up.
    fn dot_at(&self, x: i32) -> i32 {
        let s = self.scale();
        let surface = self.ground_at(x) - s;
        if !self.abort {
            return surface;
        }
        let r = unit(x / (3 * s), 0, self.seed);
        let off = ((r * 2.0 - 1.0) * 6.0 * s as f32).round() as i32;
        let off = if off.abs() < 2 * s {
            off.signum().max(1) * 3 * s
        } else {
            off
        };
        surface - off
    }

    fn paint_dots(&self, canvas: &mut Canvas, c: &DroneColours) {
        let Some((from, to)) = self.mapped() else {
            return;
        };
        let s = self.scale();
        let step = 3 * s;
        let mut x = 0;
        while x < self.wide() {
            if (x as f32) >= from && (x as f32) <= to {
                let colour = if self.abort { c.stray } else { c.dot };
                canvas.fill_rect(x, self.dot_at(x), s, s, colour);
            }
            x += step;
        }
    }

    fn paint_beam(&self, canvas: &mut Canvas, c: &DroneColours, cx: f32, fy: f32) {
        if !self.scanning() {
            return;
        }
        let s = self.scale() as f32;
        let from_y = fy - self.shape().sensor() as f32 * s + s;
        let reach = self.reach();
        let swing = cycle(self.t, std::f32::consts::TAU / self.sweep_period()).sin();
        let x0 = (cx - reach).floor() as i32;
        let x1 = (cx + reach).ceil() as i32;
        let mut faint: HashMap<Rgb, Rgb> = HashMap::new();
        let mut bright: HashMap<Rgb, Rgb> = HashMap::new();
        let bg = c.background;
        for x in x0..=x1 {
            let ground = self.ground_at(x);
            let span = (ground as f32 - from_y).max(1.0);
            for y in from_y.ceil() as i32..ground {
                let k = (y as f32 - from_y) / span;
                if (x as f32 - cx).abs() <= reach * k {
                    if let Some(under) = canvas.get(x, y) {
                        let lit = *faint
                            .entry(under)
                            .or_insert_with(|| held_imagery(bg, under, c.beam, 0.09));
                        canvas.set(x, y, lit);
                    }
                }
            }
        }
        // the sweep: one brighter line from the sensor to the ground, moving slowly
        let gx = cx + swing * reach * 0.9;
        let ground = self.ground_at(gx.round() as i32) as f32;
        let steps = (ground - from_y).max(1.0).ceil() as i32;
        let thick = self.scale().max(1);
        for i in 0..=steps {
            let k = i as f32 / steps as f32;
            let x = (cx + (gx - cx) * k).round() as i32;
            let y = (from_y + (ground - 1.0 - from_y) * k).round() as i32;
            for dx in 0..thick {
                if let Some(under) = canvas.get(x + dx, y) {
                    let lit = *bright
                        .entry(under)
                        .or_insert_with(|| held_imagery(bg, under, c.beam, 0.4));
                    canvas.set(x + dx, y, lit);
                }
            }
        }
    }

    fn paint_drone(&self, canvas: &mut Canvas, c: &DroneColours, cx: f32, fy: f32) {
        let s = self.scale();
        let cx = cx.round() as i32;
        let fy = fy.round() as i32;
        let block = |canvas: &mut Canvas, dx0: i32, dx1: i32, dy: i32, colour: Rgb| {
            let (a, b) = (dx0.min(dx1), dx0.max(dx1));
            canvas.fill_rect(cx + a * s, fy + dy * s, (b - a + 1) * s, s, colour);
        };
        let spin = self.spin();
        // a spinning rotor is a faint blur round a solid hub; once it stops it is a solid blade
        let rotor = |canvas: &mut Canvas, tip: i32, dy: i32, half: i32| {
            if spin > 0.12 {
                let reach = if spin > 0.5 { half } else { half - 1 };
                block(canvas, tip - reach, tip + reach, dy, c.blur);
                block(canvas, tip, tip, dy, c.blade);
            } else {
                block(canvas, tip - half, tip + half, dy, c.blade);
            }
        };
        let g = if self.shape().compact {
            Geometry {
                body: 3,
                far_tip: 2,
                far_half: 1,
                near_tip: 6,
                near_half: 2,
                skid: (2, 3),
                leg: 2,
            }
        } else {
            Geometry {
                body: 4,
                far_tip: 7,
                far_half: 2,
                near_tip: 10,
                near_half: 3,
                skid: (2, 5),
                leg: 3,
            }
        };
        let compact = self.shape().compact;
        let (top, bottom) = if compact { (-3, -2) } else { (-4, -2) };
        // the far pair first, higher and closer in, so the body and the near pair cover them
        for side in [-1, 1] {
            let tip = g.far_tip * side;
            block(canvas, g.body * side, tip, top, c.arm);
            let mast = if compact { 1 } else { 2 };
            for k in 1..=mast {
                block(canvas, tip, tip, top - k, c.arm);
            }
            rotor(canvas, tip, top - mast - 1, g.far_half);
        }
        // skids, legs and the sensor pod under the body
        for side in [-1, 1] {
            block(canvas, g.skid.0 * side, g.skid.1 * side, 0, c.arm);
            block(canvas, g.leg * side, g.leg * side, -1, c.arm);
        }
        if !compact {
            block(canvas, -1, 1, -1, c.body_dark);
        }
        block(canvas, 0, 0, -1, c.lens);
        // the body, rounded at the top and lit from above
        for dy in top..=bottom {
            let inset = i32::from(dy == top);
            let colour = if dy == top {
                c.body_lit
            } else if dy == bottom {
                c.body_dark
            } else {
                c.body
            };
            block(canvas, -g.body + inset, g.body - inset, dy, colour);
        }
        // the near pair straight out from the bottom of the body, each to a motor and rotor
        for side in [-1, 1] {
            let tip = g.near_tip * side;
            let arm_row = if compact { bottom } else { top + 1 };
            block(canvas, (g.body + 1) * side, tip, arm_row, c.arm);
            let mast = if compact { 1 } else { 2 };
            for k in 1..=mast {
                block(canvas, tip, tip, arm_row - k, c.body);
            }
            rotor(canvas, tip, arm_row - mast - 1, g.near_half);
        }
        // the status lamp on the body's flank
        let lamp = if self.abort && self.story >= APPROACH {
            c.warn
        } else {
            c.status
        };
        let lamp_row = if compact { bottom } else { top + 1 };
        block(canvas, g.body - 2, g.body - 2, lamp_row, lamp);
    }

    fn paint_readout(
        &self,
        canvas: &mut Canvas,
        theme: &ThemeData,
    ) -> Option<(i32, Rgb, Rgb, &'static str)> {
        let (text, kind) = self.readout()?;
        let len = text.chars().count() as i32 + 2;
        let rows = self.height as i32;
        if self.wide() < len + 4 || rows < 14 {
            return None;
        }
        let ink = match kind {
            Readout::Busy => theme.foreground,
            Readout::Good => theme.green(),
            Readout::Bad => theme.yellow(),
        };
        let shades = Shades::new(theme);
        let panel = shades.under_text(theme.info, 0.3, &[ink], theme.floor());
        let row = 1;
        canvas.fill_rect(2, row * 2, len, 2, panel);
        Some((row, ink, panel, text))
    }

    fn paint(&self, canvas: &mut Canvas, c: &DroneColours) {
        self.paint_sky(canvas, c);
        self.paint_land(canvas, c);
        let (cx, fy) = self.drone_at();
        self.paint_beam(canvas, c, cx, fy);
        self.paint_dots(canvas, c);
        self.paint_pad(canvas, c);
        self.paint_drone(canvas, c, cx, fy);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Readout {
    Busy,
    Good,
    Bad,
}

impl Scene for DroneLanding {
    fn name(&self) -> &'static str {
        if self.abort {
            "landing-abort"
        } else {
            "drone-landing"
        }
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let w = self.wide() as f32;
        let tall = self.tall() as f32;
        let count = self
            .motion
            .scaled(((w / 40.0).round() as usize).clamp(1, 6));
        let speed = self.motion.speed();
        self.clouds = (0..count)
            .map(|_| {
                Cloud::random(
                    rng,
                    w,
                    (tall * 0.1, tall * 0.26),
                    (w * 0.1, w * 0.2),
                    1.6 * speed,
                )
            })
            .collect();
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
        self.story = (self.story + dt * self.motion.speed()).min(1.0e6);
        let w = self.wide() as f32;
        for cloud in &mut self.clouds {
            cloud.drift(dt, w);
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = DroneColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        let readout = self.paint_readout(&mut canvas, theme);
        canvas.blit(frame);
        if let Some((row, ink, panel, text)) = readout {
            for (i, ch) in text.chars().enumerate() {
                frame.put_cell(
                    3 + i as i32,
                    row,
                    Cell {
                        ch,
                        colour: Some(ink),
                        bg: Some(panel),
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{
        apart, assert_imagery, check_determinism, check_pixel_scene, check_scene, pixels, render,
    };
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("drone-landing");
        check_determinism("drone-landing");
        check_pixel_scene("drone-landing", 28.0);
    }

    fn run(scene: &mut DroneLanding, seconds: f32) {
        let mut rng = Rng::new(4);
        for _ in 0..(seconds * 20.0) as usize {
            scene.step(0.05, &mut rng);
        }
    }

    #[test]
    fn the_drone_maps_the_ground_then_lands_on_the_pad_and_spins_down() {
        let mut rng = Rng::new(3);
        let mut scene = DroneLanding::new(80, 24, Motion::FULL, &mut rng);
        let (start, _) = scene.drone_at();
        assert!(start < 0.0, "the drone flies in from off the left edge");
        run(&mut scene, 10.0);
        assert!(scene.scanning());
        assert_eq!(scene.readout().map(|r| r.0), Some("MAPPING TERRAIN"));
        run(&mut scene, 25.0);
        let (x, y) = scene.drone_at();
        let (pad_x, half) = scene.pad();
        assert!((x - pad_x as f32).abs() < 1.0);
        assert!(((scene.deck() - 1) as f32 - y).abs() < 0.01, "{y}");
        assert!((x - pad_x as f32).abs() < half as f32);
        assert_eq!(scene.spin(), 0.0);
        assert!(!scene.scanning());
        assert_eq!(scene.readout().map(|r| r.0), Some("POSITION FIX GOOD"));
        // the mapped dots sit on the ground's own contour
        let c = DroneColours::new(theme::default_theme());
        let mut frame = Frame::new(80, 24);
        scene.draw(&mut frame, theme::default_theme());
        let all = pixels(&frame);
        assert!(all.contains(&c.dot));
        assert!((0..80).all(|x| scene.dot_at(x) == scene.ground_at(x) - 1));
        assert!(all.contains(&c.stripe) && all.contains(&c.blade));
        assert!(frame.row_text(1).contains("POSITION FIX GOOD"));
        // the aborted scan's dots never sit on the ground
        let abort = DroneLanding::aborted(80, 24, Motion::FULL, &mut rng);
        assert!((0..80).all(|x| (abort.dot_at(x) - (abort.ground_at(x) - 1)).abs() >= 2));
    }

    #[test]
    fn the_rotors_spin_down_slowly_and_the_sweep_is_slow() {
        let mut rng = Rng::new(1);
        for motion in Motion::ALL {
            let scene = DroneLanding::new(80, 24, motion, &mut rng);
            assert!(scene.sweep_period() >= SWEEP_PERIOD);
            assert!(scene.bob_period() >= BOB_PERIOD);
        }
        let mut scene = DroneLanding::new(80, 24, Motion::FULL, &mut rng);
        run(&mut scene, LANDED + SPIN_DOWN * 0.5);
        assert!(scene.spin() > 0.3 && scene.spin() < 0.7);
    }

    #[test]
    fn a_small_screen_gets_the_compact_drone_and_a_big_one_a_larger_drone() {
        let mut rng = Rng::new(2);
        assert!(
            DroneLanding::new(40, 12, Motion::FULL, &mut rng)
                .shape()
                .compact
        );
        assert!(
            !DroneLanding::new(80, 24, Motion::FULL, &mut rng)
                .shape()
                .compact
        );
        assert!(DroneLanding::new(200, 60, Motion::FULL, &mut rng).scale() >= 2);
        let frame = render(
            &mut DroneLanding::new(20, 8, Motion::FULL, &mut rng),
            20,
            8,
            40,
        );
        assert!(frame.drawn() > 0);
    }

    #[test]
    fn the_colours_stay_distinct_and_in_bounds_in_every_theme() {
        for theme in theme::all() {
            let c = DroneColours::new(theme);
            assert_imagery(
                theme,
                &[
                    ("sky top", c.sky_top),
                    ("sky low", c.sky_low),
                    ("hills", c.hills),
                    ("ground", c.ground),
                    ("low ground", c.ground_low),
                    ("crest", c.crest),
                    ("rotor blur", c.blur),
                ],
            );
            apart(theme, c.sky_top, c.sky_low, 0.03, "the sky's gradient");
            apart(theme, c.hills, c.sky_low, 0.03, "the hills and the sky");
            apart(theme, c.ground, c.hills, 0.03, "the ground and the hills");
            let lit = held_imagery(theme.background, c.sky_low, c.beam, 0.12);
            assert_imagery(theme, &[("beam", lit)]);
            apart(theme, lit, c.sky_low, 0.02, "the beam and the sky");
            apart(theme, c.dot, c.ground, 0.1, "the dots and the ground");
            apart(
                theme,
                c.stray,
                c.ground,
                0.1,
                "the stray dots and the ground",
            );
            apart(
                theme,
                c.dot,
                c.stray,
                0.08,
                "the matched and the stray dots",
            );
            apart(
                theme,
                c.stripe,
                c.deck,
                0.1,
                "the pad's markings and its deck",
            );
            apart(theme, c.body, c.sky_low, 0.1, "the drone and the sky");
            for colour in [c.body, c.body_lit, c.body_dark, c.deck, c.blade] {
                assert!(
                    theme.contrast_of(colour) >= paint::SOLID - 0.01,
                    "{}",
                    theme.name
                );
            }
            for colour in [c.dot, c.stray, c.stripe, c.lamp, c.blade] {
                assert!(theme.contrast_of(colour) >= theme.floor(), "{}", theme.name);
            }
        }
    }
}
