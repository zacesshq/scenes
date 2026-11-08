//! A kettle boiling, painted in solid pixels: a kitchen counter under a tiled splashback, a
//! window onto a garden and, on the counter, a jug kettle on its base beside an empty mug with a
//! tea bag in it. The kettle's lamp comes on, bubbles rise in its water gauge and the lid
//! rattles, steam rises and curls from the spout, the switch clicks off and the kettle lifts,
//! tips and pours into the mug, then goes back on its base while steam rises from the cup.
//!
//! The same counter is the `kettle-cold` fail scene: there the lamp comes on and goes off again
//! without the water ever boiling, no steam rises and a note by the mug says to try again.
//!
//! The wall, the tiles, the window's view and the cupboards are tints of the background held
//! below the non-text floor, so they read as background. The kettle, the mug, the counter and
//! the window frame keep the theme's own drawing colours, so they stand out as shapes and keep
//! their outline with colour off.

use std::f32::consts::TAU;

use super::cycle;
use super::paint::{self, Cloud, ease, shaded, solid, strong};
use crate::colour::Rgb;
use crate::frame::{Cell, Frame};
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "A kettle on a kitchen counter that boils, clicks off and pours into a mug, in the evening or by day in light themes";

pub const CAPTIONS: &[&str] = &[
    "Build passed. The kettle has boiled.",
    "Click. Pour. All green.",
    "A clean build deserves a cup of tea.",
    "Everything passed while the kettle boiled.",
    "Steam rising on a green build.",
];

/// Story seconds the water takes to come to the boil.
pub const BOIL: f32 = 12.0;
/// The switch clicks off once the water has boiled for a few seconds.
pub const CLICK: f32 = 16.0;
/// The pour: the lift to the mug, the tip, the pour itself, the tip back and the return.
const LIFT: (f32, f32) = (17.0, 20.0);
const TIP: (f32, f32) = (20.0, 22.5);
const POUR: (f32, f32) = (22.0, 27.0);
const UNTIP: (f32, f32) = (27.0, 29.0);
const LOWER: (f32, f32) = (29.0, 32.0);
/// How far the kettle tips to pour, in radians.
const TIP_ANGLE: f32 = 0.7;
/// The cold kettle clicks off after this long, well before it could boil.
pub const COLD_CLICK: f32 = 4.0;
/// The note by the mug appears this long after the cold kettle clicks off.
const NOTE_AFTER: f32 = 1.5;
/// Seconds a curl of steam takes to sway from one side to the other and back.
pub const SWAY_PERIOD: f32 = 2.6;
/// Seconds the lid takes to lift and settle once while the water boils.
pub const RATTLE_PERIOD: f32 = 0.9;
/// Seconds a curl of steam lasts.
const STEAM_LIFE: f32 = 4.5;
/// Pixels a second steam rises at, at the standard speed.
const RISE: f32 = 3.0;
const WALL_STEPS: usize = 7;

/// The kettle facing right: lid knob, lid, body with a water gauge, the handle on the left and
/// the spout on the right. `l` is the lit side, `s` the shaded side, `g` the gauge's glass and
/// `w` its water.
const KETTLE: [&str; 13] = [
    ".....kkkk.......",
    "....LLLLLLL.....",
    "...lbbbbbbbsSS..",
    "HHHlbbbbbbbbsSSS",
    "H..lbbbbbbbbs...",
    "H..lbggbbbbbbs..",
    "H..lbggbbbbbbs..",
    "H..lbwwbbbbbbs..",
    "H..lbwwbbbbbbs..",
    "H..lbwwbbbbbbs..",
    "HHHlbwwbbbbbbs..",
    "...lbbbbbbbbbs..",
    "...ddddddddddd..",
];
/// The kettle's pivot in sprite pixels: the middle of its foot, where it sits on the base.
const PIVOT: (f32, f32) = (8.5, 13.0);
/// The tip of the spout in sprite pixels.
const SPOUT: (f32, f32) = (15.5, 3.5);
/// The gauge's water rows in the sprite.
const GAUGE: (usize, usize, usize) = (7, 10, 5);

/// The mug: rim, body with a painted band and handle. The `i` row is the inside, dark while the
/// mug is empty and tea once it is full.
const MUG: [&str; 8] = [
    "lrrrrrrrs...",
    "liiiiiiis...",
    "lbbbbbbbshh.",
    "ldddddddsh.h",
    "lbbbbbbbs..h",
    "lbbbbbbbshh.",
    "lbbbbbbbs...",
    ".sssssss....",
];

#[derive(Clone, Debug)]
struct Puff {
    x: f32,
    y: f32,
    age: f32,
    phase: f32,
    drift: f32,
}

/// The kitchen's colours in one theme, every one derived from the theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct KitchenColours {
    pub wall_top: Rgb,
    pub wall_low: Rgb,
    pub tile: Rgb,
    pub grout: Rgb,
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub trees: Rgb,
    pub hedge: Rgb,
    pub cupboard: Rgb,
    pub cupboard_seam: Rgb,
    pub cloud: Rgb,
    pub cloud_shade: Rgb,
    pub steam: Rgb,
    pub frame: Rgb,
    pub frame_lit: Rgb,
    pub top: Rgb,
    pub top_lit: Rgb,
    pub top_dark: Rgb,
    pub knob: Rgb,
    pub face: Rgb,
    pub kettle: Rgb,
    pub kettle_lit: Rgb,
    pub kettle_shade: Rgb,
    pub kettle_dark: Rgb,
    pub glass: Rgb,
    pub water: Rgb,
    pub bubble: Rgb,
    pub base: Rgb,
    pub lamp_on: Rgb,
    pub lamp_off: Rgb,
    pub mug: Rgb,
    pub mug_lit: Rgb,
    pub mug_shade: Rgb,
    pub mug_band: Rgb,
    pub inside: Rgb,
    pub tea: Rgb,
    pub tag: Rgb,
    pub string: Rgb,
    pub tin: Rgb,
    pub jar_glass: Rgb,
    pub jar_glint: Rgb,
    pub jar_fill: [Rgb; 3],
    pub lids: [Rgb; 3],
    pub tin_lit: Rgb,
    pub note: Rgb,
    pub note_edge: Rgb,
    pub ink: Rgb,
    pub pin: Rgb,
    wall: Ramp,
    sky: Ramp,
}

impl KitchenColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        // a dark theme is the kitchen in the evening with the light on, a light theme the same
        // kitchen by day with a bright sky in the window
        let (wall_top, wall_low, tile, grout, sky_top, sky_low, trees, hedge, cupboard, seam) =
            if theme.dark {
                (
                    shades.tint(theme.warning, 0.12, 1.22),
                    shades.tint(theme.warning, 0.26, 1.5),
                    shades.tint(theme.info, 0.3, 1.45),
                    shades.tint(theme.info, 0.14, 1.18),
                    shades.tint(theme.blue(), 0.2, 1.3),
                    shades.tint(theme.magenta(), 0.42, 1.95),
                    shades.tint(theme.green(), 0.16, 1.22),
                    shades.tint(theme.green(), 0.32, 1.55),
                    shades.tint(theme.warning, 0.4, 1.85),
                    shades.tint(theme.warning, 0.6, 2.35),
                )
            } else {
                (
                    shades.tint(theme.warning, 0.3, 1.45),
                    shades.tint(theme.warning, 0.16, 1.22),
                    shades.tint(theme.info, 0.14, 1.18),
                    shades.tint(theme.info, 0.45, 1.75),
                    shades.tint(theme.info, 0.5, 1.9),
                    shades.tint(theme.info, 0.14, 1.15),
                    shades.tint(theme.green(), 0.75, 2.5),
                    shades.tint(theme.green(), 0.5, 2.0),
                    shades.tint(theme.warning, 0.5, 1.75),
                    shades.tint(theme.warning, 0.85, 2.6),
                )
            };
        let (cloud, cloud_shade) = paint::cloud_tones(theme, 2.2);
        let wood = strong(
            theme,
            shades.darker(mix(theme.ansi[3], theme.red(), 0.35), 0.25),
        );
        let (top_dark, top, top_lit) = shaded(theme, wood);
        let kettle = theme.red();
        let (kettle_shade, _, kettle_lit) = shaded(theme, kettle);
        let mug = theme.cyan();
        let (mug_shade, _, mug_lit) = shaded(theme, mug);
        // the window frame is painted wood, kept as imagery so the view stays calm
        let frame = if theme.dark {
            shades.tint(theme.foreground, 0.5, IMAGERY_CEILING - 0.35)
        } else {
            shades.tint(theme.muted, 0.6, IMAGERY_CEILING - 0.35)
        };
        let tea = strong(
            theme,
            shades.darker(mix(theme.yellow(), theme.red(), 0.5), 0.45),
        );
        let ink = theme.foreground;
        // the note is pale paper by day and a dim sheet under the evening light, its words
        // always at the full text contrast against it
        let paper = if theme.dark { 0.3 } else { 0.4 };
        let note = shades.under_text(theme.ansi[11], paper, &[ink], theme.floor());
        Self {
            wall_top,
            wall_low,
            tile,
            grout,
            sky_top,
            sky_low,
            trees,
            hedge,
            cupboard,
            cupboard_seam: seam,
            // steam is paler than the wall in every theme; a light theme's wall is tinted
            // enough for the white of the page to show against it
            cloud,
            cloud_shade,
            steam: if theme.dark {
                shades.tint(theme.bright(), 0.6, IMAGERY_CEILING - 0.2)
            } else {
                theme.background
            },
            frame,
            frame_lit: if theme.dark {
                shades.tint(theme.foreground, 0.7, IMAGERY_CEILING - 0.1)
            } else {
                shades.tint(theme.muted, 0.3, IMAGERY_CEILING - 1.0)
            },
            top,
            top_lit,
            top_dark,
            knob: theme.foreground,
            face: if theme.dark {
                shades.tint(theme.bright(), 0.35, 1.9)
            } else {
                theme.background
            },
            kettle,
            kettle_lit,
            kettle_shade,
            kettle_dark: solid(theme, kettle, shades.darkest(), 0.5),
            glass: strong(theme, mix(theme.grey(), theme.info, 0.3)),
            water: theme.bright_cyan(),
            // bubbles are pale in the water on either kind of theme
            bubble: if theme.dark {
                theme.bright()
            } else {
                theme.background
            },
            base: strong(theme, mix(theme.grey(), theme.foreground, 0.3)),
            lamp_on: theme.red(),
            lamp_off: strong(theme, theme.grey()),
            mug,
            mug_lit,
            mug_shade,
            mug_band: theme.magenta(),
            inside: solid(theme, mug_shade, shades.darkest(), 0.5),
            tea,
            tag: theme.yellow(),
            string: theme.foreground,
            tin: theme.green(),
            jar_glass: shades.tint(theme.info, 0.3, 1.45),
            jar_glint: if theme.dark {
                shades.tint(theme.bright(), 0.5, 2.0)
            } else {
                theme.background
            },
            jar_fill: [
                shades.tint(theme.yellow(), 0.6, 2.3),
                shades.tint(theme.red(), 0.5, 2.0),
                shades.tint(theme.green(), 0.55, 2.2),
            ],
            lids: [theme.blue(), theme.magenta(), theme.yellow()],
            tin_lit: shaded(theme, theme.green()).2,
            note,
            note_edge: theme.yellow(),
            ink,
            pin: theme.red(),
            wall: Ramp::new(wall_top, wall_low, WALL_STEPS),
            sky: Ramp::new(sky_top, sky_low, WALL_STEPS),
        }
    }
}

/// The window in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Window {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

pub struct Kettle {
    width: usize,
    height: usize,
    motion: Motion,
    /// Seconds into the story, scaled by the motion setting.
    story: f32,
    t: f64,
    cold: bool,
    puffs: Vec<Puff>,
    since_spout: f32,
    since_mug: f32,
    window: Option<Window>,
    /// A cloud drifting slowly across the view, in the glass's own pixels.
    cloud: Option<Cloud>,
}

impl Kettle {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        Self::build(width, height, motion, rng, false)
    }

    /// The kettle that never boils, for the `kettle-cold` fail scene.
    pub fn cold(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        Self::build(width, height, motion, rng, true)
    }

    fn build(width: usize, height: usize, motion: Motion, rng: &mut Rng, cold: bool) -> Self {
        let mut kettle = Self {
            width: 1,
            height: 1,
            motion,
            story: 0.0,
            t: 0.0,
            cold,
            puffs: Vec::new(),
            since_spout: 0.0,
            since_mug: 0.0,
            window: None,
            cloud: None,
        };
        kettle.resize(width, height, rng);
        kettle
    }

    fn wide(&self) -> i32 {
        self.width as i32
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// Pixels a sprite pixel takes: one on a classic terminal, more on a big screen.
    fn scale(&self) -> i32 {
        (self.tall() / 44).min(self.wide() / 60).clamp(1, 6)
    }

    /// The pixel row of the counter top.
    fn counter(&self) -> i32 {
        (self.tall() as f32 * 0.72).round() as i32
    }

    /// The base's left column and top row.
    fn base_at(&self) -> (i32, i32) {
        let s = self.scale();
        let left = ((self.wide() as f32 * 0.5) as i32 - 8 * s).min(self.group_left());
        (left, self.counter() - 2 * s)
    }

    /// The left edge of the base, the gap and the mug taken as one group centred on a narrow
    /// screen, so the mug never crowds the base or runs off the right edge.
    fn group_left(&self) -> i32 {
        let s = self.scale();
        ((self.wide() - GROUP * s) / 2).max(0)
    }

    /// The mug's left column and top row.
    fn mug_at(&self) -> (i32, i32) {
        let s = self.scale();
        let (base_left, _) = self.base_at();
        let left = ((self.wide() as f32 * 0.72) as i32 - 4 * s).max(base_left + (GROUP - 12) * s);
        (left, self.counter() - MUG.len() as i32 * s)
    }

    /// The middle of the mug's mouth.
    fn mouth(&self) -> (f32, f32) {
        let s = self.scale() as f32;
        let (left, top) = self.mug_at();
        (left as f32 + 4.5 * s, top as f32 + s)
    }

    /// Where a point of the kettle sprite lands for a pivot position and angle.
    fn place(&self, pivot: (f32, f32), angle: f32, point: (f32, f32)) -> (f32, f32) {
        let s = self.scale() as f32;
        let (dx, dy) = ((point.0 - PIVOT.0) * s, (point.1 - PIVOT.1) * s);
        let (sin, cos) = angle.sin_cos();
        (pivot.0 + dx * cos - dy * sin, pivot.1 + dx * sin + dy * cos)
    }

    /// The pivot when the kettle sits on its base.
    fn home(&self) -> (f32, f32) {
        let s = self.scale() as f32;
        let (left, top) = self.base_at();
        (left as f32 + PIVOT.0 * s, top as f32)
    }

    /// The pivot that puts the spout over the mug once the kettle has tipped.
    fn over_mug(&self) -> (f32, f32) {
        let s = self.scale() as f32;
        let (mx, my) = self.mouth();
        let tip = self.place((0.0, 0.0), TIP_ANGLE, SPOUT);
        (mx - 1.0 * s - tip.0, my - 7.0 * s - tip.1)
    }

    /// The kettle's pivot and angle at this point in the story.
    fn pose(&self) -> ((f32, f32), f32) {
        let home = self.home();
        if self.cold {
            return (home, 0.0);
        }
        let p = self.story;
        let over = self.over_mug();
        let along = |(a, b): (f32, f32)| ease((p - a) / (b - a));
        let lerp = |k: f32| {
            (
                home.0 + (over.0 - home.0) * k,
                home.1 + (over.1 - home.1) * k,
            )
        };
        if p < LIFT.0 {
            (home, 0.0)
        } else if p < TIP.0 {
            (lerp(along(LIFT)), 0.0)
        } else if p < UNTIP.0 {
            (over, TIP_ANGLE * along(TIP))
        } else if p < LOWER.0 {
            (over, TIP_ANGLE * (1.0 - along(UNTIP)))
        } else {
            (lerp(1.0 - along(LOWER)), 0.0)
        }
    }

    /// Whether the switch's lamp is lit.
    fn lamp_on(&self) -> bool {
        let off = if self.cold { COLD_CLICK } else { CLICK };
        self.story >= 0.6 && self.story < off
    }

    /// How hard the water is boiling, from 0 for still to 1 for a rolling boil.
    fn heat(&self) -> f32 {
        if self.cold {
            // the element barely warms the water before it gives up
            return (self.story / COLD_CLICK).clamp(0.0, 1.0)
                * 0.15
                * f32::from(u8::from(self.lamp_on()));
        }
        if self.story < CLICK {
            (self.story / BOIL).clamp(0.0, 1.0)
        } else {
            (1.0 - (self.story - CLICK) / 3.0).clamp(0.0, 1.0)
        }
    }

    fn pouring(&self) -> bool {
        !self.cold && (POUR.0..POUR.1).contains(&self.story)
    }

    /// Whether the mug holds tea yet.
    fn full(&self) -> bool {
        !self.cold && self.story >= POUR.0 + 1.5
    }

    fn note_shown(&self) -> bool {
        self.cold && self.story >= COLD_CLICK + NOTE_AFTER
    }

    fn sway_period(&self) -> f32 {
        SWAY_PERIOD / self.motion.pace()
    }

    fn rattle_period(&self) -> f32 {
        RATTLE_PERIOD / self.motion.pace()
    }

    /// Seconds between curls of steam from the spout, none while the water is cool.
    fn spout_gap(&self) -> Option<f32> {
        let heat = self.heat();
        if self.cold || heat < 0.4 || self.story > LIFT.0 + 1.0 {
            return None;
        }
        let gap = 1.1 - 0.8 * ((heat - 0.4) / 0.6);
        Some(gap * if self.motion.calm { 2.0 } else { 1.0 } / self.motion.busy())
    }

    fn mug_gap(&self) -> Option<f32> {
        self.full().then(|| {
            let gap = if self.motion.calm { 1.0 } else { 0.5 };
            gap / self.motion.busy()
        })
    }

    fn paint_wall(&self, canvas: &mut Canvas, c: &KitchenColours) {
        let s = self.scale();
        let counter = self.counter();
        for y in 0..counter {
            canvas.fill_row(y, c.wall.at(y as f32 / counter.max(1) as f32));
        }
        // a tiled splashback behind the counter, a few rows of tiles high
        let size = 6 * s;
        let rows = self.tile_rows();
        let top = counter - rows * size;
        canvas.fill_rect(0, top, self.wide(), counter - top, c.tile);
        let mut y = top;
        let mut row = 0;
        while y < counter {
            canvas.fill_rect(0, y, self.wide(), s, c.grout);
            let offset = if row % 2 == 0 { 0 } else { size / 2 };
            let mut x = offset;
            while x < self.wide() {
                canvas.fill_rect(x, y, s, size, c.grout);
                x += size;
            }
            y += size;
            row += 1;
        }
    }

    fn paint_window(&self, canvas: &mut Canvas, c: &KitchenColours) {
        let Some(win) = self.window else {
            return;
        };
        let s = self.scale();
        let (l, t, w, h) = (win.left, win.top, win.width, win.height);
        let bar = 2 * s;
        let (il, it, iw, ih) = (l + bar, t + bar, w - 2 * bar, h - 2 * bar);
        for y in 0..ih {
            canvas.fill_rect(il, it + y, iw, 1, c.sky.at(y as f32 / ih.max(1) as f32));
        }
        if let Some(cloud) = &self.cloud {
            // the cloud is painted on a canvas the size of the glass, so it slides behind the
            // frame at either side instead of spilling over it
            let rows = (ih.max(1) as usize).div_ceil(2);
            let mut glass = Canvas::new(iw.max(1) as usize, rows, c.sky_top);
            let marker = c.sky_top;
            cloud.paint(&mut glass, c.cloud, c.cloud_shade);
            for y in 0..ih {
                for x in 0..iw {
                    if let Some(p) = glass.get(x, y).filter(|&p| p != marker) {
                        canvas.set(il + x, it + y, p);
                    }
                }
            }
        }
        // a line of trees and a hedge across the bottom of the view
        let base = it + ih;
        for x in 0..iw {
            let crown = paint::noise((x / s) as f32 / 4.0, 0.5, 21);
            let tree = (ih as f32 * (0.22 + 0.28 * crown)) as i32;
            canvas.fill_rect(il + x, base - tree, 1, tree, c.trees);
            let hedge = (ih as f32 * 0.12) as i32
                + (paint::noise((x / s) as f32 / 2.0, 3.0, 4) * 2.0 * s as f32) as i32;
            canvas.fill_rect(il + x, base - hedge, 1, hedge, c.hedge);
        }
        // the frame, a glazing bar down the middle and a sill
        canvas.fill_rect(l, t, w, bar, c.frame);
        canvas.fill_rect(l, t + h - bar, w, bar, c.frame);
        canvas.fill_rect(l, t, bar, h, c.frame);
        canvas.fill_rect(l + w - bar, t, bar, h, c.frame);
        canvas.fill_rect(l + w / 2 - s / 2, t, s, h, c.frame);
        canvas.fill_rect(l, t, w, s, c.frame_lit);
        canvas.fill_rect(l - 2 * s, t + h, w + 4 * s, s, c.frame_lit);
        canvas.fill_rect(l - 2 * s, t + h + s, w + 4 * s, s, c.frame);
    }

    fn paint_counter(&self, canvas: &mut Canvas, c: &KitchenColours) {
        let s = self.scale();
        let counter = self.counter();
        let tall = self.tall();
        let wide = self.wide();
        canvas.fill_rect(0, counter, wide, s, c.top_lit);
        canvas.fill_rect(0, counter + s, wide, s, c.top);
        canvas.fill_rect(0, counter + 2 * s, wide, s, c.top_dark);
        let doors_top = counter + 3 * s;
        canvas.fill_rect(0, doors_top, wide, tall - doors_top, c.cupboard);
        // doors of a fitted kitchen: seams between them and a knob near the top of each
        let door = (wide / 4).max(12 * s);
        let mut x = door / 2;
        let mut i = 0;
        while x < wide + door {
            canvas.fill_rect(x, doors_top, s, tall - doors_top, c.cupboard_seam);
            let knob_x = if i % 2 == 0 { x - 3 * s } else { x + 2 * s };
            if tall - doors_top > 4 * s {
                canvas.fill_rect(knob_x, doors_top + 2 * s, s, s, c.knob);
            }
            x += door;
            i += 1;
        }
        canvas.fill_rect(0, doors_top, wide, s, c.cupboard_seam);
    }

    fn paint_base(&self, canvas: &mut Canvas, c: &KitchenColours) {
        let s = self.scale();
        let (left, top) = self.base_at();
        canvas.fill_rect(left + s, top, 15 * s, s, c.base);
        canvas.fill_rect(left, top + s, 17 * s, s, c.base);
        let lamp = if self.lamp_on() {
            c.lamp_on
        } else {
            c.lamp_off
        };
        canvas.fill_rect(left + 2 * s, top + s, 2 * s, s, lamp);
    }

    fn paint_kettle(&self, canvas: &mut Canvas, c: &KitchenColours) {
        let s = self.scale() as f32;
        let (pivot, angle) = self.pose();
        // the lid lifts a pixel now and then while the water is at a rolling boil
        let rattle = !self.cold
            && self.heat() > 0.8
            && self.story < CLICK
            && cycle(self.t, TAU / self.rattle_period()).sin() > 0.55;
        let level = self.water_rows();
        let bubbles = self.bubbles();
        // a tipped kettle drops its lit and shaded edges, which would only alias into stripes
        let tipped = angle.abs() > 0.05;
        let colour_of = |ch: char, row: usize, col: usize| -> Option<Rgb> {
            Some(match ch {
                'k' | 'L' => c.kettle_lit,
                'H' => c.kettle_shade,
                'l' if tipped => c.kettle,
                'l' => c.kettle_lit,
                'b' => c.kettle,
                's' if tipped => c.kettle,
                's' | 'S' => c.kettle_shade,
                'd' => c.kettle_dark,
                'g' => c.glass,
                'w' => {
                    if row < GAUGE.0 + (GAUGE.1 - GAUGE.0 + 1).saturating_sub(level) {
                        c.glass
                    } else if bubbles.contains(&(row, col)) {
                        c.bubble
                    } else {
                        c.water
                    }
                }
                _ => return None,
            })
        };
        let (sin, cos) = angle.sin_cos();
        let reach = 20.0 * s;
        let (x0, x1) = (
            (pivot.0 - reach).floor() as i32,
            (pivot.0 + reach).ceil() as i32,
        );
        let (y0, y1) = (
            (pivot.1 - reach).floor() as i32,
            (pivot.1 + reach).ceil() as i32,
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (dx, dy) = (x as f32 + 0.5 - pivot.0, y as f32 + 0.5 - pivot.1);
                let (lx, ly) = (dx * cos + dy * sin, -dx * sin + dy * cos);
                let u = (lx / s + PIVOT.0).floor();
                let mut v = (ly / s + PIVOT.1).floor();
                if u < 0.0 || v < -1.0 {
                    continue;
                }
                // a lifted lid leaves its knob and lid a row higher
                if rattle && v < 2.0 {
                    v += 1.0;
                    if v >= 2.0 {
                        continue;
                    }
                }
                if v < 0.0 {
                    continue;
                }
                let (col, row) = (u as usize, v as usize);
                let Some(line) = KETTLE.get(row) else {
                    continue;
                };
                let Some(ch) = line.as_bytes().get(col).map(|&b| b as char) else {
                    continue;
                };
                if let Some(colour) = colour_of(ch, row, col) {
                    canvas.set(x, y, colour);
                }
            }
        }
    }

    /// How many rows of the gauge show water: full until the pour, then a little lower.
    fn water_rows(&self) -> usize {
        let rows = GAUGE.1 - GAUGE.0 + 1;
        if !self.cold && self.story >= POUR.0 + 2.5 {
            rows - 2
        } else {
            rows
        }
    }

    /// The gauge cells holding a bubble right now, as sprite rows and columns.
    fn bubbles(&self) -> Vec<(usize, usize)> {
        let heat = self.heat();
        if heat < 0.05 {
            return Vec::new();
        }
        let rows = (GAUGE.1 - GAUGE.0 + 1) as f32;
        let count = if heat > 0.7 {
            3
        } else if heat > 0.3 {
            2
        } else {
            1
        };
        let pace = (0.35 + 0.65 * heat) * self.motion.pace();
        (0..count)
            .map(|i| {
                // each bubble climbs the gauge on its own clock, a row at a time
                let climb = (self.t as f32 * 1.6 * pace + i as f32 * 0.37).fract();
                let row = GAUGE.1 - ((climb * rows) as usize).min(GAUGE.1 - GAUGE.0);
                (row, GAUGE.2 + (i % 2))
            })
            .collect()
    }

    fn paint_stream(&self, canvas: &mut Canvas, c: &KitchenColours) {
        if !self.pouring() || self.story < TIP.1 - 0.3 {
            return;
        }
        let (pivot, angle) = self.pose();
        let tip = self.place(pivot, angle, SPOUT);
        let (mx, my) = self.mouth();
        let thick = self.scale();
        paint::line(canvas, (tip.0, tip.1 + 1.0), (mx, my), thick, c.water);
    }

    fn paint_mug(&self, canvas: &mut Canvas, c: &KitchenColours) {
        let s = self.scale();
        let (left, top) = self.mug_at();
        let inside = if self.full() { c.tea } else { c.inside };
        let palette = [
            ('l', c.mug_lit),
            ('b', c.mug),
            ('s', c.mug_shade),
            ('r', c.mug_lit),
            ('i', inside),
            ('d', c.mug_band),
            ('h', c.mug),
        ];
        for (dy, row) in MUG.iter().enumerate() {
            for (dx, ch) in row.chars().enumerate() {
                if let Some(&(_, colour)) = palette.iter().find(|(k, _)| *k == ch) {
                    canvas.fill_rect(left + dx as i32 * s, top + dy as i32 * s, s, s, colour);
                }
            }
        }
        // the tea bag's string down from the rim and its tag hanging on the near side
        let px = |canvas: &mut Canvas, dx: i32, dy: i32, colour: Rgb| {
            canvas.fill_rect(left + dx * s, top + dy * s, s, s, colour);
        };
        px(canvas, -1, 0, c.string);
        px(canvas, -1, 1, c.string);
        for (dx, dy) in [(-2, 2), (-1, 2), (-2, 3), (-1, 3)] {
            px(canvas, dx, dy, c.tag);
        }
    }

    /// Rows of tiles in the splashback, kept low so the wall above has room for the window.
    fn tile_rows(&self) -> i32 {
        if self.tall() < 44 { 1 } else { 2 }
    }

    /// A shelf of storage jars on the wall right of the window, on a screen wide enough.
    fn paint_shelf(&self, canvas: &mut Canvas, c: &KitchenColours) -> bool {
        let Some(win) = self.window else {
            return false;
        };
        let s = self.scale();
        let x0 = (win.left + win.width + 10 * s).max(self.wide() * 58 / 100);
        let x1 = self.wide() * 93 / 100;
        let jar = 7 * s;
        let gap = 4 * s;
        let count = ((x1 - x0 + gap) / (jar + gap)).min(4);
        if count < 2 {
            return false;
        }
        // low enough that the tallest jar clears the top of the screen
        let y = (win.top + win.height * 3 / 5).max(12 * s);
        // the shelf stays clear of the kettle as it lifts to pour
        let kettle_top = self.over_mug().1 - (PIVOT.1 + 2.0) * s as f32;
        if (y + 4 * s) as f32 >= kettle_top {
            return false;
        }
        let span = count * (jar + gap) - gap + 4 * s;
        let left = x1 - span;
        canvas.fill_rect(left, y, span, s, c.top_lit);
        canvas.fill_rect(left, y + s, span, s, c.top_dark);
        for bx in [left + 2 * s, left + span - 3 * s] {
            canvas.fill_rect(bx, y + 2 * s, s, 2 * s, c.top_dark);
        }
        let heights = [9, 7, 10, 8];
        for k in 0..count {
            let h = heights[k as usize % 4] * s;
            let jx = left + 2 * s + k * (jar + gap);
            let top = y - h;
            canvas.fill_rect(jx, top + s, jar, h - s, c.jar_glass);
            let filled = h * 3 / 5;
            canvas.fill_rect(
                jx + s,
                y - filled,
                jar - 2 * s,
                filled,
                c.jar_fill[k as usize % 3],
            );
            canvas.fill_rect(jx, top, jar, s, c.lids[k as usize % 3]);
            canvas.fill_rect(jx + s, top + 2 * s, s, h / 3, c.jar_glint);
        }
        true
    }

    /// A round wall clock high on the right, where a small screen has no room for the shelf.
    fn paint_clock(&self, canvas: &mut Canvas, c: &KitchenColours) {
        let Some(win) = self.window else {
            return;
        };
        let s = self.scale() as f32;
        let r = 6.0 * s;
        let cx = (self.wide() as f32 * 0.86).round();
        let cy = (win.top as f32 + r + s).round();
        if cx - r <= (win.left + win.width) as f32 + 2.0 * s {
            return;
        }
        paint::disc(canvas, cx, cy, r, c.top);
        paint::disc(canvas, cx, cy, r - s, c.face);
        let si = s as i32;
        let (x, y) = (cx as i32, cy as i32);
        // a tick at each quarter and the hands at three o'clock, square to the grid so they
        // stay crisp, drawn once and left still
        for (dx, dy) in [(0, -4), (4, 0), (0, 4), (-4, 0)] {
            canvas.fill_rect(x + dx * si, y + dy * si, si, si, c.knob);
        }
        canvas.fill_rect(x, y - 3 * si, si, 4 * si, c.knob);
        canvas.fill_rect(x, y, 3 * si, si, c.knob);
    }

    fn paint_tin(&self, canvas: &mut Canvas, c: &KitchenColours) {
        let s = self.scale();
        let (mug_left, _) = self.mug_at();
        let left = mug_left + 18 * s;
        if left + 8 * s >= self.wide() || self.tall() < 30 {
            return;
        }
        let top = self.counter() - 9 * s;
        canvas.fill_rect(left, top, 8 * s, s, c.tin_lit);
        canvas.fill_rect(left, top + s, 8 * s, 8 * s, c.tin);
        canvas.fill_rect(left + s, top + s, s, 8 * s, c.tin_lit);
        canvas.fill_rect(left, top + 4 * s, 8 * s, s, c.tag);
    }

    fn paint_steam(&self, canvas: &mut Canvas, c: &KitchenColours) {
        let period = self.sway_period();
        for puff in &self.puffs {
            let sway = cycle(self.t, TAU / period) + puff.phase * TAU;
            let x = (puff.x + sway.sin() * 1.3).round() as i32;
            let y = puff.y.round() as i32;
            let fade = (1.0 - puff.age / STEAM_LIFE).clamp(0.0, 1.0);
            let amount = (0.25 + 0.55 * fade).min(0.8);
            let s = self.scale();
            for (dx, dy, k) in [(0, 0, 1.0), (1, 0, 0.6), (0, -1, 0.5), (-1, 0, 0.4)] {
                if k < 1.0 && puff.age < STEAM_LIFE * 0.3 {
                    continue;
                }
                for sy in 0..s {
                    for sx in 0..s {
                        canvas.blend(x + dx * s + sx, y + dy * s + sy, c.steam, amount * k);
                    }
                }
            }
        }
    }

    /// The note by the mug: its cells and the row its words sit on, for the text drawn over it.
    fn note_at(&self) -> Option<(i32, i32, i32)> {
        if !self.note_shown() {
            return None;
        }
        let text = NOTE.chars().count() as i32;
        let w = text + 3;
        let (mug_left, mug_top) = self.mug_at();
        let row = (mug_top / 2) - 3;
        let left = (mug_left + 4 * self.scale() - w / 2).clamp(0, (self.wide() - w).max(0));
        (row >= 1 && self.wide() >= w + 2).then_some((left, row, w))
    }

    fn paint_note(&self, canvas: &mut Canvas, c: &KitchenColours) {
        let Some((left, row, w)) = self.note_at() else {
            return;
        };
        let (top, bottom) = (row * 2 - 1, row * 2 + 3);
        canvas.fill_rect(left, top, w, bottom - top, c.note);
        canvas.fill_rect(left, bottom - 1, w, 1, c.note_edge);
        canvas.fill_rect(left + w - 1, top, 1, bottom - top, c.note_edge);
        canvas.fill_rect(left + w / 2, top - 1, 1, 2, c.pin);
    }

    fn paint(&self, canvas: &mut Canvas, c: &KitchenColours) {
        self.paint_wall(canvas, c);
        self.paint_window(canvas, c);
        self.paint_counter(canvas, c);
        self.paint_tin(canvas, c);
        if !self.paint_shelf(canvas, c) {
            self.paint_clock(canvas, c);
        }
        self.paint_base(canvas, c);
        self.paint_mug(canvas, c);
        self.paint_stream(canvas, c);
        self.paint_kettle(canvas, c);
        self.paint_steam(canvas, c);
        self.paint_note(canvas, c);
    }
}

/// Sprite pixels across the base, the gap after it and the mug, as one group.
const GROUP: i32 = 32;

/// The words on the note by the cold kettle's mug.
const NOTE: &str = "try again";

impl Scene for Kettle {
    fn name(&self) -> &'static str {
        if self.cold {
            "kettle-cold"
        } else {
            "kettle-boil"
        }
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.puffs.clear();
        let s = self.scale();
        let w = self.wide();
        let counter = self.counter();
        let splash = self.tile_rows() * 6 * s;
        let room = counter - splash - 4 * s;
        self.window = if w >= 40 && room >= 14 {
            let width = (w * 3 / 10).clamp(16, 120);
            Some(Window {
                left: (w / 12).max(3),
                top: (room / 6).max(2),
                width,
                height: room - (room / 6).max(2) - 2 * s,
            })
        } else {
            None
        };
        let bar = 2 * s;
        let speed = self.motion.speed();
        self.cloud = self.window.map(|win| {
            let (iw, ih) = ((win.width - 2 * bar) as f32, (win.height - 2 * bar) as f32);
            Cloud::random(
                rng,
                iw,
                (ih * 0.32, ih * 0.42),
                (iw * 0.22, iw * 0.3),
                1.5 * speed,
            )
        });
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        self.story = (self.story + dt * self.motion.speed()).min(1.0e6);
        let speed = self.motion.speed();
        let s = self.scale() as f32;
        if let Some(gap) = self.spout_gap() {
            self.since_spout += dt;
            if self.since_spout >= gap {
                self.since_spout = 0.0;
                let (pivot, angle) = self.pose();
                let tip = self.place(pivot, angle, SPOUT);
                self.puffs.push(Puff {
                    x: tip.0 + s,
                    y: tip.1 - s,
                    age: 0.0,
                    phase: rng.range(0.0, 1.0),
                    drift: rng.range(0.3, 0.9),
                });
            }
        }
        if let Some(gap) = self.mug_gap() {
            self.since_mug += dt;
            if self.since_mug >= gap {
                self.since_mug = 0.0;
                let (mx, my) = self.mouth();
                self.puffs.push(Puff {
                    x: mx + rng.range(-2.5, 2.5) * s,
                    y: my - 2.0 * s,
                    age: 0.0,
                    phase: rng.range(0.0, 1.0),
                    drift: rng.range(-0.2, 0.2),
                });
            }
        }
        for puff in &mut self.puffs {
            puff.age += dt;
            puff.y -= RISE * speed * dt * s.sqrt();
            puff.x += puff.drift * speed * dt;
        }
        self.puffs.retain(|p| p.age < STEAM_LIFE && p.y >= 0.0);
        let glass = self.window.map(|win| (win.width - 4 * self.scale()) as f32);
        if let (Some(cloud), Some(glass)) = (&mut self.cloud, glass) {
            cloud.drift(dt, glass);
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = KitchenColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
        if let Some((left, row, _)) = self.note_at() {
            for (i, ch) in NOTE.chars().enumerate() {
                frame.put_cell(
                    left + 1 + i as i32,
                    row,
                    Cell {
                        ch,
                        colour: Some(colours.ink),
                        bg: Some(colours.note),
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
        apart, assert_imagery, check_determinism, check_pixel_scene, check_scene, pixels,
    };
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("kettle-boil");
        check_determinism("kettle-boil");
        check_pixel_scene("kettle-boil", 24.0);
    }

    #[test]
    fn sprites_are_rectangular() {
        assert!(KETTLE.iter().all(|r| r.len() == KETTLE[0].len()));
        assert!(MUG.iter().all(|r| r.len() == MUG[0].len()));
    }

    fn run(scene: &mut Kettle, seconds: f32) {
        let mut rng = Rng::new(4);
        for _ in 0..(seconds * 20.0) as usize {
            scene.step(0.05, &mut rng);
        }
    }

    #[test]
    fn the_kettle_boils_clicks_off_pours_and_goes_back_on_its_base() {
        let mut rng = Rng::new(3);
        let mut kettle = Kettle::new(80, 24, Motion::FULL, &mut rng);
        run(&mut kettle, 2.0);
        assert!(kettle.lamp_on() && kettle.puffs.is_empty());
        run(&mut kettle, 12.0);
        assert!(kettle.heat() >= 1.0 && !kettle.puffs.is_empty());
        run(&mut kettle, 10.0);
        assert!(!kettle.lamp_on());
        let (pivot, angle) = kettle.pose();
        assert!(angle > 0.5 && pivot != kettle.home());
        assert!(kettle.pouring());
        // the spout is over the mug's mouth while it pours
        let tip = kettle.place(pivot, angle, SPOUT);
        let (mx, _) = kettle.mouth();
        assert!((tip.0 - mx).abs() < 4.0, "{} {mx}", tip.0);
        run(&mut kettle, 12.0);
        assert_eq!(kettle.pose(), (kettle.home(), 0.0));
        assert!(kettle.full());
        let c = KitchenColours::new(theme::default_theme());
        let mut frame = Frame::new(80, 24);
        kettle.draw(&mut frame, theme::default_theme());
        let all = pixels(&frame);
        assert!(all.contains(&c.tea) && all.contains(&c.kettle) && all.contains(&c.lamp_off));
        assert!(
            kettle
                .puffs
                .iter()
                .any(|p| (p.x - kettle.mouth().0).abs() < 8.0)
        );
    }

    #[test]
    fn steam_sways_and_the_lid_rattles_slowly() {
        let mut rng = Rng::new(5);
        for motion in Motion::ALL {
            let kettle = Kettle::new(80, 24, motion, &mut rng);
            assert!(kettle.sway_period() >= SWAY_PERIOD);
            assert!(kettle.rattle_period() >= RATTLE_PERIOD);
            // the lid lifts once a period, so it changes twice a period at most
            assert!(2.0 / kettle.rattle_period() <= 3.0);
        }
    }

    #[test]
    fn the_colours_stay_distinct_and_in_bounds_in_every_theme() {
        for theme in theme::all() {
            let c = KitchenColours::new(theme);
            assert_imagery(
                theme,
                &[
                    ("wall top", c.wall_top),
                    ("wall low", c.wall_low),
                    ("tiles", c.tile),
                    ("grout", c.grout),
                    ("sky top", c.sky_top),
                    ("sky low", c.sky_low),
                    ("trees", c.trees),
                    ("hedge", c.hedge),
                    ("cupboards", c.cupboard),
                    ("cupboard seams", c.cupboard_seam),
                    ("steam", c.steam),
                    ("window frame", c.frame),
                    ("window frame's lit edge", c.frame_lit),
                    ("jar glass", c.jar_glass),
                ],
            );
            apart(theme, c.wall_top, c.wall_low, 0.03, "the wall's gradient");
            apart(theme, c.tile, c.grout, 0.03, "the tiles and the grout");
            apart(theme, c.sky_top, c.sky_low, 0.03, "the sky's gradient");
            apart(theme, c.trees, c.hedge, 0.03, "the trees and the hedge");
            apart(
                theme,
                c.cupboard,
                c.cupboard_seam,
                0.03,
                "the doors and their seams",
            );
            apart(theme, c.steam, c.wall_top, 0.04, "the steam and the wall");
            apart(theme, c.steam, c.tile, 0.04, "the steam and the tiles");
            apart(theme, c.kettle, c.mug, 0.1, "the kettle and the mug");
            apart(theme, c.kettle, c.tile, 0.1, "the kettle and the tiles");
            apart(theme, c.water, c.kettle, 0.1, "the water and the kettle");
            apart(theme, c.lamp_on, c.lamp_off, 0.1, "the lamp on and off");
            apart(theme, c.tea, c.mug, 0.1, "the tea and the mug");
            apart(theme, c.tea, c.inside, 0.05, "the tea and the empty mug");
            apart(theme, c.note, c.tile, 0.03, "the note and the tiles");
            assert!(c.ink.contrast(c.note) >= theme.floor(), "{}", theme.name);
            for colour in [c.kettle, c.mug, c.water, c.lamp_on, c.tag, c.tin] {
                assert!(theme.contrast_of(colour) >= theme.floor(), "{}", theme.name);
            }
            for colour in [c.top, c.top_lit, c.top_dark, c.base, c.tea, c.inside] {
                assert!(
                    theme.contrast_of(colour) >= paint::SOLID - 0.01,
                    "{}",
                    theme.name
                );
            }
        }
    }
}
