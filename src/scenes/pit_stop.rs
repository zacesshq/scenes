//! Pit stop: a pit lane seen side on, under the lights at night or in daylight on a light theme.
//! A pit building runs along the back with its garages, the open one in the middle lit and
//! stocked with stacks of tyres, a timing board hanging over its door. A race car of no real
//! team slides in along the lane and stops in its box, marked out on the ground. The crew jack
//! it up, a gunner kneels at each wheel, the old wheels come off to the carriers on the far side
//! of the car and the new ones go on from the near side, the jacks drop and the lollipop board,
//! held out over the car from the far side, lifts. The board shows the stop time
//! and the car pulls away. The carriers walk the old wheels back to the stacks and fetch new ones
//! before the next car comes in.
//!
//! The sky, the building, the garage, the lane and the pit wall are tints of the background held
//! below the non-text floor. The car, its wheels, the crew, the jacks and the lollipop are solid,
//! so with colour off the stop still reads. The drawing is shared with `pit_stop_stuck`, which
//! tells the same stop with one wheel that will not come off.

use super::paint::{self, Blender, ease, gradient};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::{error_colour, warning_colour};

pub const SUMMARY: &str = "A race car sliding into its pit box at night, the crew changing its wheels as the lollipop lifts, by day in light themes";

pub const CAPTIONS: &[&str] = &[
    "Four new wheels and away. Clean build.",
    "A tidy stop. Every check passed.",
    "Out of the box and back up to speed.",
];

/// Seconds for the car to slide in from the edge and stop in its box, at the standard pace.
pub(crate) const ARRIVE: f32 = 7.0;
/// Seconds for the jacks to lift the car or set it down.
pub(crate) const JACK: f32 = 0.8;
/// Seconds for an old wheel to come off or a new one to go on.
pub(crate) const SWAP: f32 = 1.2;
/// Seconds for the lollipop to swing up once the car is down.
const LIFT: f32 = 1.0;
/// Seconds for the car to pull away and leave the screen.
const LEAVE: f32 = 6.0;
/// Seconds the carriers take to walk to the stacks and back between cars.
const FETCH: f32 = 9.0;
/// Seconds of one whole stop, from one car arriving to the next.
pub const CYCLE: f32 = 30.0;
/// Seconds into the first stop the scene starts at, so the car is already braking into view.
pub(crate) const START: f64 = 3.0;
/// The stop times the board shows in turn, in hundredths of a second.
const TIMES: [u32; 5] = [241, 236, 252, 229, 247];

/// The cars, facing right, at three sizes: `w` the wings and floor, `e` the lit top of the rear
/// wing, `b` the body, `l` its lit top, `d` its shaded lower side, `s` the stripe, `h` the
/// driver's helmet and `v` the visor. The wheels are painted over them. The rear wing is a wide
/// flat slab lit along its top on a pylon over the gearbox, with its endplate running down to
/// the floor, so it reads as a wing rather than a flag on a pole.
///
/// The crew at the same three sizes, facing right: `h` the helmet, `v` the visor, `o` the top of
/// the two-tone overalls, `p` the trousers, `g` the gloves and `k` the boots.
const MID_CAR: [&str; 10] = [
    "eeeeee......................................",
    "wwwwww............ll........................",
    "wwwwww........lllbbbbhh.....................",
    ".w..ww.....lllbbbbbbhhhvl...................",
    ".w..ww..lbbbbbbbbbbbbbbbbbbll...............",
    ".....wlbbbbsssssssssssssbbbbbbbll...........",
    "......bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbll.....",
    "......dddddddddddddddddddddddddddddddddddw..",
    ".......ddddddddddddddddddddddd.....ddwwwwwww",
    "........dddddddddddddddddddd................",
];
const LARGE_CAR: [&str; 15] = [
    "eeeeeeeee.......................................................",
    "wwwwwwwww.......................................................",
    "wwwwwwwww.......................lll.............................",
    "wwwwwwwww...................lllbbbbbhh..........................",
    ".ww...ww.................lllbbbbbbbhhhhv........................",
    ".ww...ww............lllllbbbbbbbbbbhhhvvl.......................",
    "......ww.......lllbbbbbbbbbbbbbbbbbbbbbbbbbll...................",
    "......ww...lllbbbbbbssssssssssssssssssbbbbbbbbbll...............",
    "........lbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbblll..........",
    "........bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbll....",
    "........ddddddddddddddddddddddddddddddddddddddddddddddddddddddw.",
    ".........ddddddddddddddddddddddddddddddddddd.........ddwwwwwwwww",
    "..........dddddddddddddddddddddddddddddddd......................",
    "................................................................",
    "................................................................",
];
const SMALL_CAR: [&str; 6] = [
    "eeeee...................",
    "wwwww.....lbhv..........",
    ".w.w...llbbbbbbll.......",
    "...lbbbssssssbbbbbbll...",
    "...bbbbbbbbbbbbbbbbbbbw.",
    "....dddddddddddddddddwww",
];
const MID_STAND: [&str; 9] = [
    ".hhh.", ".hvv.", ".ooo.", "ooooo", "ooooo", "gpppg", ".p.p.", ".p.p.", ".k.k.",
];
const MID_STRIDE: [&str; 9] = [
    ".hhh.", ".hvv.", ".ooo.", "ooooo", "ooooo", "gpppg", "pp.pp", "p...p", "k...k",
];
const MID_KNEEL: [&str; 8] = [
    "..hhh.", "..hvv.", "..ooo.", ".ooooo", ".oooog", ".ppppp", ".p...p", "kk...k",
];
const LARGE_STAND: [&str; 13] = [
    "..hhh..", ".hhhhv.", ".hhhvv.", "..ooo..", ".ooooo.", "ooooooo", "o.ooo.o", "o.ooo.o",
    "g.ppp.g", "..ppp..", "..p.p..", "..p.p..", ".kk.kk.",
];
const LARGE_STRIDE: [&str; 13] = [
    "..hhh..", ".hhhhv.", ".hhhvv.", "..ooo..", ".ooooo.", "ooooooo", "o.ooo.o", "o.ooo.o",
    "g.ppp.g", "..ppp..", ".pp.pp.", ".p...p.", "kk...kk",
];
const LARGE_KNEEL: [&str; 10] = [
    "...hhh..", "..hhhhv.", "..hhhvv.", "...ooo..", "..oooooo", "..ooooog", "..ooo...", ".pppppp.",
    ".pp...p.", "kkk...kk",
];
const SMALL_STAND: [&str; 5] = [".h.", "ooo", "ooo", "p.p", "k.k"];
const SMALL_KNEEL: [&str; 4] = [".h.", "ooo", "pp.", "kk."];

/// One size of the stop: the car, its wheels, the crew and where each of them stands.
#[derive(Debug, PartialEq)]
pub(crate) struct Kit {
    car: &'static [&'static str],
    /// The rear and front wheel centres and radii on the car, in sprite pixels.
    wheels: [(f32, f32, f32); 2],
    stand: &'static [&'static str],
    stride: &'static [&'static str],
    kneel: &'static [&'static str],
    /// Each crew member's left column, in sprite pixels from the car's left edge in its box,
    /// in the order of `Crew`. The off carriers and the lollipop stand on the far side of the
    /// car, so the near side is no wider than the car and its two jacks.
    spots: [i32; 9],
    /// Whether the whole crew works; on a small screen only the gunners and the lollipop do.
    pub crew: bool,
    /// How far nearer the viewer the crew's boots stand than the tyres.
    depth: i32,
    /// How high a standing figure's hands are above its boots, then a kneeling figure's.
    hand: i32,
    kneel_hand: i32,
    /// How high above the boots a carried wheel's middle is.
    hold: i32,
    /// The lollipop's pole length, the radius of its board and the pole's angle held down, as a
    /// fraction of a half turn.
    pole: f32,
    disc: f32,
    down: f32,
}

const SMALL: Kit = Kit {
    car: &SMALL_CAR,
    wheels: [(5.0, 4.0, 2.3), (19.0, 4.2, 2.1)],
    stand: &SMALL_STAND,
    stride: &SMALL_STAND,
    kneel: &SMALL_KNEEL,
    spots: [27, 27, 2, 27, 27, 17, 27, 27, 27],
    crew: false,
    depth: 3,
    hand: 2,
    kneel_hand: 2,
    hold: 2,
    pole: 5.0,
    disc: 1.1,
    down: 0.87,
};

const MID: Kit = Kit {
    car: &MID_CAR,
    wheels: [(8.0, 6.2, 3.8), (34.0, 6.6, 3.4)],
    stand: &MID_STAND,
    stride: &MID_STRIDE,
    kneel: &MID_KNEEL,
    spots: [-7, 14, 3, 13, 24, 33, 27, 46, 36],
    crew: true,
    depth: 6,
    hand: 3,
    kneel_hand: 3,
    hold: 4,
    pole: 12.8,
    disc: 2.2,
    down: 0.71,
};

const LARGE: Kit = Kit {
    car: &LARGE_CAR,
    wheels: [(12.5, 8.8, 5.6), (49.0, 9.6, 4.8)],
    stand: &LARGE_STAND,
    stride: &LARGE_STRIDE,
    kneel: &LARGE_KNEEL,
    spots: [-8, 20, 5, 21, 32, 48, 38, 65, 57],
    crew: true,
    depth: 9,
    hand: 4,
    kneel_hand: 4,
    hold: 6,
    pole: 18.4,
    disc: 3.0,
    down: 0.75,
};

impl Kit {
    fn width(&self) -> i32 {
        self.car[0].len() as i32
    }

    pub(crate) fn figure_width(&self) -> i32 {
        self.stand[0].len() as i32
    }

    /// The columns the car and crew need at one scale.
    fn needs(&self, scale: i32) -> (i32, i32) {
        let left = -self.spots.iter().copied().min().unwrap_or(0);
        let right =
            self.spots.iter().copied().max().unwrap_or(0) + self.figure_width() - self.width();
        let wide = (2 * left.max(right).max(0) + self.width()) * scale;
        // the stop takes no more than about half the height, leaving room for the building
        let tall = ((self.car.len() as i32 + self.depth) * scale) * 19 / 10;
        (wide, tall)
    }
}

/// The pit lane's colours in one theme.
pub(crate) struct Colours {
    pub sky: Ramp,
    pub roof: Rgb,
    pub roof_lip: Rgb,
    pub lamp: Rgb,
    pub wall: Rgb,
    pub seam: Rgb,
    pub door: Rgb,
    pub slat: Rgb,
    pub garage: Ramp,
    pub cabinet: Rgb,
    pub glow: Rgb,
    pub lane: Ramp,
    pub box_line: Rgb,
    pub lane_line: Rgb,
    pub kerb: Rgb,
    pub pit_wall: Rgb,
    pub pit_cap: Rgb,
    /// The car's body from shaded side to lit top.
    pub body: (Rgb, Rgb, Rgb),
    pub stripe: Rgb,
    pub carbon: Rgb,
    pub wing: Rgb,
    pub helmet: Rgb,
    pub visor: Rgb,
    pub tyre: Rgb,
    pub rim: Rgb,
    /// The band round a fresh tyre and round a worn one.
    pub fresh: Rgb,
    pub worn: Rgb,
    pub suit: Rgb,
    pub trousers: Rgb,
    pub crew_helmet: Rgb,
    pub boot: Rgb,
    pub gun: Rgb,
    pub jack: Rgb,
    pub pole: Rgb,
    pub stop: Rgb,
    pub go: Rgb,
    pub frame: Rgb,
    pub nut: Rgb,
    pub text: Rgb,
}

/// A solid tone of `hue` for a lit shape, from 0 for its shaded side to 1 for its highlight,
/// never falling below the solid contrast, so the shape keeps its outline.
pub(crate) fn tone(theme: &ThemeData, hue: Rgb, lit: f32) -> Rgb {
    let lit = lit.clamp(0.0, 1.0);
    let target = f64::from(if theme.dark {
        3.5 + lit * 5.0
    } else {
        3.4 + (1.0 - lit) * 3.2
    });
    let background = theme.background;
    let shades = Shades::new(theme);
    let far = if theme.dark {
        shades.lightest()
    } else {
        shades.darkest()
    };
    // the path runs from the background through the hue to the theme's far end, so a tone
    // keeps as much of its hue as its contrast allows
    let (from, to) = if theme.contrast_of(hue) >= target {
        (background, hue)
    } else {
        (hue, far)
    };
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for _ in 0..22 {
        let mid = (low + high) / 2.0;
        if mix(from, to, mid).contrast(background) >= target {
            high = mid;
        } else {
            low = mid;
        }
    }
    mix(from, to, high)
}

impl Colours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let dark = theme.dark;
        let grey = mix(theme.muted, theme.info, 0.3);
        let warm = mix(theme.yellow(), theme.foreground, 0.5);
        // a night race under floodlights on a dark theme; a bright afternoon on a light one
        let (sky_top, sky_low) = if dark {
            (
                shades.tint(theme.blue(), 0.12, 1.2),
                shades.tint(mix(theme.blue(), theme.magenta(), 0.3), 0.3, 1.6),
            )
        } else {
            (
                shades.tint(theme.info, 0.36, 1.75),
                shades.tint(theme.info, 0.1, 1.15),
            )
        };
        let (roof, roof_lip, wall, seam, door, slat) = if dark {
            (
                shades.tint(grey, 0.1, 1.15),
                shades.tint(grey, 0.42, 2.0),
                shades.tint(grey, 0.3, 1.65),
                shades.tint(grey, 0.18, 1.35),
                shades.tint(grey, 0.2, 1.4),
                shades.tint(grey, 0.12, 1.22),
            )
        } else {
            (
                shades.tint(grey, 0.62, 2.5),
                shades.tint(grey, 0.3, 1.5),
                shades.tint(grey, 0.12, 1.15),
                shades.tint(grey, 0.32, 1.55),
                shades.tint(grey, 0.24, 1.38),
                shades.tint(grey, 0.4, 1.75),
            )
        };
        let (garage_back, garage_front, cabinet, glow) = if dark {
            (
                shades.tint(mix(grey, warm, 0.3), 0.36, 1.8),
                shades.tint(mix(grey, warm, 0.4), 0.52, 2.2),
                shades.tint(theme.red(), 0.3, 1.5),
                shades.tint(warm, 0.8, 2.75),
            )
        } else {
            (
                shades.tint(grey, 0.5, 2.15),
                shades.tint(grey, 0.36, 1.8),
                shades.tint(theme.red(), 0.45, 2.0),
                shades.tint(warm, 0.3, 1.3),
            )
        };
        let (lane_far, lane_near, box_line, lane_line, kerb, pit_wall, pit_cap) = if dark {
            (
                shades.tint(grey, 0.34, 1.75),
                shades.tint(grey, 0.2, 1.4),
                shades.tint(theme.yellow(), 0.55, 2.4),
                shades.tint(theme.foreground, 0.62, 2.7),
                shades.tint(theme.foreground, 0.3, 1.8),
                shades.tint(grey, 0.26, 1.55),
                shades.tint(grey, 0.48, 2.1),
            )
        } else {
            (
                shades.tint(grey, 0.3, 1.5),
                shades.tint(grey, 0.44, 1.85),
                shades.tint(theme.yellow(), 0.75, 2.5),
                shades.tint(theme.info, 0.02, 1.03),
                shades.tint(grey, 0.08, 1.1),
                shades.tint(grey, 0.2, 1.3),
                shades.tint(grey, 0.06, 1.07),
            )
        };
        let body_hue = theme.cyan();
        let suit_hue = mix(theme.blue(), theme.cyan(), 0.12);
        let metal = mix(theme.muted, theme.info, 0.3);
        Self {
            sky: Ramp::new(sky_top, sky_low, 6),
            roof,
            roof_lip,
            lamp: if dark {
                shades.tint(theme.bright_yellow(), 0.8, 2.8)
            } else {
                shades.tint(theme.info, 0.01, 1.02)
            },
            wall,
            seam,
            door,
            slat,
            garage: Ramp::new(garage_back, garage_front, 4),
            cabinet,
            glow,
            lane: Ramp::new(lane_far, lane_near, 5),
            box_line,
            lane_line,
            kerb,
            pit_wall,
            pit_cap,
            body: (
                tone(theme, body_hue, 0.15),
                tone(theme, body_hue, 0.55),
                tone(theme, body_hue, 0.95),
            ),
            stripe: tone(theme, theme.yellow(), 0.8),
            carbon: tone(theme, metal, 0.0),
            wing: tone(theme, metal, 0.85),
            helmet: tone(theme, theme.foreground, 1.0),
            visor: if dark {
                shades.tint(theme.muted, 0.05, 1.08)
            } else {
                shades.tint(theme.info, 0.2, 1.4)
            },
            tyre: if dark {
                shades.tint(theme.muted, 0.04, 1.06)
            } else {
                paint::solid(theme, theme.muted, shades.darkest(), 0.85)
            },
            rim: tone(theme, metal, 0.7),
            fresh: tone(theme, warning_colour(theme), 0.7),
            worn: tone(theme, metal, 0.25),
            suit: tone(theme, suit_hue, 0.6),
            trousers: tone(theme, suit_hue, 0.1),
            crew_helmet: tone(theme, theme.foreground, 0.9),
            boot: tone(theme, theme.muted, 0.0),
            gun: tone(theme, metal, 0.5),
            jack: tone(theme, warning_colour(theme), 0.3),
            pole: tone(theme, metal, 0.6),
            stop: error_colour(theme),
            go: tone(theme, theme.green(), 0.6),
            frame: tone(theme, metal, 0.2),
            nut: error_colour(theme),
            text: if dark {
                theme.foreground
            } else {
                shades.darkest()
            },
        }
    }
}

/// Where everything sits at one size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Layout {
    pub wide: i32,
    pub tall: i32,
    /// The size of the car and crew drawn.
    pub kit: &'static Kit,
    /// Each sprite pixel is drawn as a square this many pixels wide.
    pub scale: i32,
    pub sky_bottom: i32,
    pub roof_bottom: i32,
    pub door_top: i32,
    /// The first row of the lane.
    pub floor: i32,
    /// The row the tyres stand on.
    pub ground: i32,
    /// The row the crew's boots stand on, a little nearer than the car.
    pub feet: i32,
    pub pit_wall: i32,
    /// The middle of the box, where the car stops.
    pub box_x: i32,
    pub door: (i32, i32),
}

impl Layout {
    pub(crate) fn new(width: usize, height: usize) -> Self {
        let wide = width.max(1) as i32;
        let tall = height.max(1) as i32 * 2;
        // the biggest car and crew that fit, so the stop fills the middle of the screen
        let choices: [(&'static Kit, i32); 8] = [
            (&SMALL, 1),
            (&SMALL, 2),
            (&MID, 1),
            (&LARGE, 1),
            (&MID, 2),
            (&LARGE, 2),
            (&MID, 3),
            (&LARGE, 3),
        ];
        let (kit, scale) = choices
            .into_iter()
            .rev()
            .find(|(kit, scale)| {
                let (w, h) = kit.needs(*scale);
                w <= wide && h <= tall
            })
            .unwrap_or((&SMALL, 1));
        let s = scale;
        let sky_bottom = (tall as f32 * 0.1).round() as i32;
        let roof_bottom = sky_bottom + (tall / 22).max(1);
        // the stop works from the bottom up: a low pit wall, a narrow strip of lane below the
        // crew, the crew's boots, the car a little further back and the garage behind it
        let pit_wall = tall - (tall / 12).max(2);
        let feet = (pit_wall - 3 * s - 1).max(1);
        let ground = feet - kit.depth * s;
        let rows = kit.car.len() as i32 * s;
        let floor = (ground - (rows as f32 * 0.45).round() as i32).max(roof_bottom + 3);
        let door_top = roof_bottom + ((floor - roof_bottom) as f32 * 0.42).round() as i32;
        let box_x = wide / 2;
        let car_w = kit.width() * s;
        // wide enough that the tyre stacks inside stand clear of the car in its box
        let half = (car_w / 2 + 14 * s).min(wide * 21 / 50).max(car_w / 2 + 3);
        Self {
            wide,
            tall,
            kit,
            scale,
            sky_bottom,
            roof_bottom,
            door_top,
            floor,
            ground,
            feet,
            pit_wall,
            box_x,
            door: (box_x - half, box_x + half),
        }
    }

    /// The car's length in pixels.
    pub(crate) fn car_len(&self) -> i32 {
        self.kit.width() * self.scale
    }

    /// The car's left edge when it sits in its box.
    pub(crate) fn stop_x(&self) -> f32 {
        (self.box_x - self.car_len() / 2) as f32
    }

    /// The car's top row, on the ground and before any lift.
    fn car_top(&self) -> i32 {
        self.ground - self.kit.car.len() as i32 * self.scale + 1
    }

    /// The text row and the column span of the timing board, if there is room for it.
    pub(crate) fn board(&self) -> Option<(i32, i32, i32)> {
        let room = self.door_top - self.roof_bottom;
        if room < 7 || self.wide < 24 {
            return None;
        }
        let row = (self.roof_bottom + self.door_top + 2) / 4;
        Some((row, self.box_x - 5, self.box_x + 5))
    }

    /// Where a wheel of the car sits at a given car position and lift, in canvas pixels.
    pub(crate) fn wheel_at(&self, car_x: f32, lift: i32, which: usize) -> (f32, f32, f32) {
        let (cx, cy, r) = self.kit.wheels[which];
        let s = self.scale as f32;
        (
            car_x.round() + cx * s,
            (self.car_top() - lift) as f32 + cy * s,
            r * s,
        )
    }

    /// The radius of a tyre.
    pub(crate) fn tyre_radius(&self) -> f32 {
        self.kit.wheels[0].2 * self.scale as f32
    }

    /// A crew member's standing spot along the lane: the left column of their figure.
    pub(crate) fn spot(&self, who: Crew) -> i32 {
        self.stop_x() as i32 + self.kit.spots[who as usize] * self.scale
    }

    /// Whether a crew member stands on the far side of the car, behind it. On a small screen
    /// the few who work all stand on the near side.
    pub(crate) fn far(&self, who: Crew) -> bool {
        self.kit.crew && matches!(who, Crew::RearOff | Crew::FrontOff | Crew::Lollipop)
    }

    /// The row a crew member's boots stand on at their spot: a little behind the tyres for the
    /// far side, a little in front of them for the near side.
    pub(crate) fn feet_of(&self, who: Crew) -> i32 {
        if self.far(who) {
            self.ground - 2 * self.scale
        } else {
            self.feet
        }
    }

    /// The tyre stacks either side of the open garage: the middle column and the bottom row.
    pub(crate) fn stacks(&self) -> [(f32, f32); 2] {
        let r = self.tyre_radius();
        let (left, right) = self.door;
        let y = (self.floor + 1) as f32;
        // never behind the car in its box, where the dark tyres would swallow its rear wing;
        // on a narrow screen they stand out by the neighbouring doors instead
        let clear = (self.car_len() / 2) as f32 + r + 1.0;
        let middle = self.box_x as f32;
        [
            ((left as f32 + r + 2.0).min(middle - clear), y),
            ((right as f32 - r - 2.0).max(middle + clear), y),
        ]
    }

    /// Where a carrier stands to reach a stack: their left column and the row of their boots.
    pub(crate) fn at_stack(&self, which: usize) -> (f32, f32) {
        let (x, y) = self.stacks()[which];
        let r = self.tyre_radius();
        let s = self.scale as f32;
        let w = (self.kit.figure_width() + 1) as f32 * s;
        let x = if which == 0 { x + r + s } else { x - r - w };
        (x, y + 2.0 * s)
    }
}

/// Who stands where in the crew.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Crew {
    RearJack,
    RearOff,
    RearGun,
    RearOn,
    FrontOn,
    FrontGun,
    FrontOff,
    FrontJack,
    Lollipop,
}

impl Crew {
    pub(crate) fn on_carrier(which: usize) -> Crew {
        if which == 0 {
            Crew::RearOn
        } else {
            Crew::FrontOn
        }
    }

    pub(crate) fn off_carrier(which: usize) -> Crew {
        if which == 0 {
            Crew::RearOff
        } else {
            Crew::FrontOff
        }
    }

    /// Whether they face right, towards the front of the car, at their spot.
    fn faces_right(self) -> bool {
        matches!(
            self,
            Crew::RearJack | Crew::FrontOff | Crew::RearGun | Crew::FrontOn
        )
    }
}

/// One wheel's part of the stop.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub(crate) struct WheelPose {
    /// How far the old wheel has come off towards its carrier, from 0 on the car to 1 held.
    pub off: f32,
    /// How far the new wheel has gone on from its carrier, from 0 held to 1 on the car.
    pub on: f32,
    pub gun: bool,
    /// A wheel nut that will not shift, marked on the hub.
    pub stuck: bool,
}

/// A crew member away from their spot: how far along the way to `to` they are and what they
/// carry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Walk {
    pub who: Crew,
    /// Where they walk to: their left column and the row of their boots.
    pub to: (f32, f32),
    /// How far along, from 0 at their spot to 1 there.
    pub along: f32,
    /// Whether they carry a wheel and whether it is fresh.
    pub wheel: Option<bool>,
    /// Whether they kneel once they get there and which way they face.
    pub kneel: bool,
    pub right: bool,
}

/// Everything that moves in one moment of a stop.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Pose {
    /// The car's left edge while it is on screen.
    pub car_x: Option<f32>,
    pub lift: i32,
    pub wheels: [WheelPose; 2],
    /// Whether the car wears its new wheels.
    pub fresh: bool,
    /// The lollipop's swing, from 0 down in front of the car to 1 held up.
    pub lollipop: f32,
    /// Whether the on carriers hold new wheels and the off carriers old ones.
    pub new_in_hand: bool,
    pub old_in_hand: bool,
    pub walks: Vec<Walk>,
    /// The text on the timing board.
    pub board: String,
}

impl Pose {
    pub(crate) fn waiting(board: String) -> Self {
        Self {
            car_x: None,
            lift: 0,
            wheels: [WheelPose::default(); 2],
            fresh: false,
            lollipop: 0.0,
            new_in_hand: true,
            old_in_hand: false,
            walks: Vec::new(),
            board,
        }
    }
}

/// Paints a sprite with each of its pixels drawn as a `scale` square, facing right or left.
fn blit(
    canvas: &mut Canvas,
    (x, y): (i32, i32),
    rows: &[&str],
    palette: &[(char, Rgb)],
    scale: i32,
    right: bool,
) {
    for (dy, row) in rows.iter().enumerate() {
        let len = row.chars().count() as i32;
        for (dx, ch) in row.chars().enumerate() {
            let Some(&(_, colour)) = palette.iter().find(|(key, _)| *key == ch) else {
                continue;
            };
            let col = if right {
                dx as i32
            } else {
                len - 1 - dx as i32
            };
            canvas.fill_rect(x + col * scale, y + dy as i32 * scale, scale, scale, colour);
        }
    }
}

fn lerp2(a: (f32, f32), b: (f32, f32), t: f32) -> (f32, f32) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// The painter for the pit lane, shared by the passing and the stuck stop.
pub(crate) struct Painter<'a> {
    pub l: Layout,
    pub c: &'a Colours,
    pub background: Rgb,
}

impl Painter<'_> {
    fn paint_backdrop(&self, canvas: &mut Canvas) {
        let l = &self.l;
        let c = self.c;
        gradient(canvas, &c.sky, 0, l.sky_bottom);
        // the roof of the pit building with its row of lamps under the lip
        canvas.fill_rect(
            0,
            l.sky_bottom,
            l.wide,
            l.roof_bottom - l.sky_bottom,
            c.roof,
        );
        canvas.fill_rect(0, l.roof_bottom - 1, l.wide, 1, c.roof_lip);
        // the wall in tall panels
        canvas.fill_rect(0, l.roof_bottom, l.wide, l.floor - l.roof_bottom, c.wall);
        let panel = (l.wide / 14).max(6);
        let mut x = panel / 2;
        while x < l.wide {
            canvas.fill_rect(x, l.roof_bottom, 1, l.door_top - l.roof_bottom - 1, c.seam);
            x += panel;
        }
        let step = (l.wide / 9).max(8);
        let mut x = step / 2;
        while x < l.wide {
            canvas.fill_rect(x - 1, l.roof_bottom, 3, 1, c.lamp);
            x += step;
        }
        // the neighbouring garages, their roller doors pulled down
        let (left, right) = l.door;
        let gap = (l.wide / 40).max(2);
        let width = right - left;
        for k in [-2, -1, 1, 2] {
            let x0 = if k < 0 {
                left + k * (width + gap)
            } else {
                right + gap + (k - 1) * (width + gap)
            };
            canvas.fill_rect(x0 - 1, l.door_top - 1, width + 2, 1, c.roof_lip);
            canvas.fill_rect(x0, l.door_top, width, l.floor - l.door_top, c.door);
            let mut y = l.door_top + 1;
            while y < l.floor {
                canvas.fill_rect(x0, y, width, 1, c.slat);
                y += 2;
            }
        }
        // the open garage: lit from inside, the back wall darker than the floor in front
        let inner = l.floor - l.door_top;
        for y in l.door_top..l.floor {
            let v = (y - l.door_top) as f32 / inner.max(1) as f32;
            canvas.fill_rect(left, y, width, 1, c.garage.at(v));
        }
        // tool cabinets along the back wall under a strip light
        if inner >= 6 {
            let cab_h = (inner / 3).max(2);
            let cab_w = (width / 8).max(3);
            let top = l.floor - cab_h - (inner / 5).max(1);
            let mut x = left + width / 2 - cab_w * 2;
            for _ in 0..3 {
                canvas.fill_rect(x, top, cab_w, cab_h, c.cabinet);
                canvas.fill_rect(x, top + cab_h / 2, cab_w, 1, c.garage.at(0.0));
                x += cab_w + (cab_w / 3).max(1);
            }
        }
        canvas.fill_rect(left + 2, l.door_top + 1, width - 4, 1, c.glow);
        // the door frame
        canvas.fill_rect(left - 1, l.door_top - 1, width + 2, 1, c.roof_lip);
        canvas.fill_rect(left - 1, l.door_top, 1, inner, c.roof_lip);
        canvas.fill_rect(right, l.door_top, 1, inner, c.roof_lip);
    }

    fn paint_lane(&self, canvas: &mut Canvas) {
        let l = &self.l;
        let c = self.c;
        let span = (l.pit_wall - l.floor).max(1);
        for y in l.floor..l.pit_wall {
            let v = (y - l.floor) as f32 / span as f32;
            for x in 0..l.wide {
                // patches in the asphalt, fixed to the lane
                let n = paint::noise(x as f32 / 7.0, y as f32 / 2.0, 41);
                canvas.set(x, y, c.lane.at(v + (n - 0.5) * 0.35));
            }
        }
        // the light spilling out of the open garage onto the lane
        let (left, right) = l.door;
        let mut mixer = Blender::new(4);
        let reach = (span / 3).max(1);
        for y in l.floor..(l.floor + reach) {
            let fade = 1.0 - (y - l.floor) as f32 / reach as f32;
            for x in left..right {
                mixer.blend(canvas, x, y, c.glow, 0.4 * fade);
            }
        }
        canvas.fill_rect(0, l.floor, l.wide, 1, c.kerb);
        // the box, painted on the lane in perspective round where the car stops
        let car = l.car_len();
        let (bx0, bx1) = (
            l.box_x - car / 2 - 4 * l.scale,
            l.box_x + car / 2 + 4 * l.scale,
        );
        let back = (l.ground - 3 * l.scale).max(l.floor + 2);
        let front = (l.feet + l.scale).min(l.pit_wall - 1);
        canvas.fill_rect(bx0 + 1, back, bx1 - bx0 - 2, 1, c.box_line);
        canvas.fill_rect(bx0 - 1, front, bx1 - bx0 + 2, 1, c.box_line);
        canvas.line((bx0 + 1, back), (bx0 - 1, front), c.box_line);
        canvas.line((bx1 - 2, back), (bx1, front), c.box_line);
        // the dashed line between the working lane and the fast lane
        let dash_y = front + (l.pit_wall - front) / 2;
        if dash_y < l.pit_wall - 1 && dash_y > front + 1 {
            let dash = (l.wide / 16).max(4);
            let mut x = dash / 3;
            while x < l.wide {
                canvas.fill_rect(x, dash_y, dash, 1, c.lane_line);
                x += dash * 2;
            }
        }
        // the pit wall along the front with its lit cap
        canvas.fill_rect(0, l.pit_wall, l.wide, l.tall - l.pit_wall, c.pit_wall);
        canvas.fill_rect(0, l.pit_wall, l.wide, 1, c.pit_cap);
        let post = (l.wide / 12).max(6);
        let mut x = post / 2;
        while x < l.wide {
            canvas.fill_rect(x, l.pit_wall + 1, 1, l.tall - l.pit_wall - 1, c.pit_cap);
            x += post;
        }
    }

    fn paint_stacks(&self, canvas: &mut Canvas) {
        let l = &self.l;
        if !l.kit.crew {
            return;
        }
        let r = l.tyre_radius();
        let w = (r * 2.0).round() as i32;
        let h = ((r * 0.9).round() as i32).max(2);
        for (x, y) in l.stacks() {
            // three tyres lying flat on each other, a fresh band round each
            for k in 0..3 {
                let top = y as i32 - (k + 1) * h;
                canvas.fill_rect(x as i32 - w / 2 + 1, top, w - 2, h, self.c.tyre);
                canvas.fill_rect(x as i32 - w / 2, top + 1, w, h - 2, self.c.tyre);
                canvas.fill_rect(x as i32 - w / 2 + 1, top + h / 2, w - 2, 1, self.c.fresh);
            }
        }
    }

    /// A wheel side on: the tyre, its coloured band, the rim and the hub.
    fn wheel(&self, canvas: &mut Canvas, (cx, cy): (f32, f32), r: f32, fresh: bool, stuck: bool) {
        let c = self.c;
        let band = if fresh { c.fresh } else { c.worn };
        let reach = r.ceil() as i32 + 1;
        let (px, py) = (cx.floor() as i32, cy.floor() as i32);
        for y in py - reach..=py + reach {
            for x in px - reach..=px + reach {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let d = (dx * dx + dy * dy).sqrt();
                if d > r {
                    continue;
                }
                let colour = if d < r * 0.5 && stuck {
                    c.nut
                } else if d < r * 0.45 {
                    c.rim
                } else if d < r * 0.68 && r >= 3.5 {
                    band
                } else {
                    c.tyre
                };
                canvas.set(x, y, colour);
            }
        }
    }

    fn paint_car(&self, canvas: &mut Canvas, pose: &Pose) {
        let Some(car_x) = pose.car_x else {
            return;
        };
        let l = &self.l;
        let c = self.c;
        let (dark, body, lit) = c.body;
        let palette = [
            ('w', c.carbon),
            ('e', c.wing),
            ('b', body),
            ('l', lit),
            ('d', dark),
            ('s', c.stripe),
            ('h', c.helmet),
            ('v', c.visor),
        ];
        let at = (car_x.round() as i32, l.car_top() - pose.lift);
        blit(canvas, at, l.kit.car, &palette, l.scale, true);
        for (which, wheel) in pose.wheels.iter().enumerate() {
            let (cx, cy, r) = l.wheel_at(car_x, pose.lift, which);
            // the small layout has no carriers, so its wheels stay on and only the gunners work
            if wheel.off <= 0.0 || !l.kit.crew {
                self.wheel(canvas, (cx, cy), r, pose.fresh, wheel.stuck);
            } else if wheel.on > 0.0 {
                let from = self.held(
                    l.spot(Crew::on_carrier(which)) as f32,
                    l.feet as f32,
                    Crew::on_carrier(which).faces_right(),
                );
                let at = lerp2(from, (cx, cy), ease(wheel.on));
                self.wheel(canvas, at, r, true, false);
            } else {
                // the bare hub while the wheel is off
                paint::disc(canvas, cx, cy, (r * 0.3).max(0.6), c.rim);
            }
        }
    }

    /// Where a crew member holds a wheel: out in front of them at waist height.
    fn held(&self, x: f32, feet: f32, right: bool) -> (f32, f32) {
        let s = self.l.scale as f32;
        let r = self.l.tyre_radius();
        let w = (self.l.kit.figure_width() - 1) as f32 * s;
        let cx = if right {
            x + w + r * 0.6
        } else {
            x + s - r * 0.6
        };
        (cx, feet - self.l.kit.hold as f32 * s)
    }

    fn figure(
        &self,
        canvas: &mut Canvas,
        (x, feet): (i32, i32),
        kneel: bool,
        stride: bool,
        right: bool,
    ) {
        let l = &self.l;
        let c = self.c;
        let palette = [
            ('h', c.crew_helmet),
            ('v', c.visor),
            ('o', c.suit),
            ('p', c.trousers),
            ('g', c.boot),
            ('k', c.boot),
        ];
        let rows = match (kneel, stride) {
            (true, _) => l.kit.kneel,
            (false, true) => l.kit.stride,
            (false, false) => l.kit.stand,
        };
        let top = feet - rows.len() as i32 * l.scale + 1;
        blit(canvas, (x, top), rows, &palette, l.scale, right);
    }

    /// Paints the crew on one side of the car: the far side before the car is painted over
    /// them, the near side after it.
    fn paint_crew(&self, canvas: &mut Canvas, pose: &Pose, far: bool) {
        let l = &self.l;
        let c = self.c;
        let s = l.scale;
        let crew: &[Crew] = if l.kit.crew {
            &[
                Crew::RearJack,
                Crew::RearOff,
                Crew::RearOn,
                Crew::FrontOn,
                Crew::FrontOff,
                Crew::FrontJack,
                Crew::Lollipop,
                Crew::RearGun,
                Crew::FrontGun,
            ]
        } else {
            &[Crew::RearGun, Crew::FrontGun, Crew::Lollipop]
        };
        let stopped = pose.car_x.is_some_and(|x| (x - l.stop_x()).abs() < 0.5);
        for &who in crew.iter().filter(|&&who| l.far(who) == far) {
            let walk = pose.walks.iter().find(|w| w.who == who);
            let home = (l.spot(who) as f32, l.feet_of(who) as f32);
            let (x, feet) = walk.map_or(home, |w| lerp2(home, w.to, ease(w.along)));
            let (x, feet) = (x.round() as i32, feet.round() as i32);
            let walking = walk.is_some_and(|w| w.along > 0.0 && w.along < 1.0);
            let stride = walking && (x / (2 * s)) % 2 == 1;
            let gunner = matches!(who, Crew::RearGun | Crew::FrontGun);
            let kneel = match walk {
                Some(w) => w.kneel && w.along >= 1.0,
                None => gunner,
            };
            let right = match walk {
                Some(w) if w.along >= 1.0 => w.right,
                Some(w) if w.along > 0.0 => w.to.0 >= home.0,
                _ => who.faces_right(),
            };
            // the jacks: a long bar from the hands down to the foot of the jack at the car's end
            if matches!(who, Crew::RearJack | Crew::FrontJack) {
                let end = if who == Crew::RearJack {
                    l.stop_x() as i32 + s
                } else {
                    l.stop_x() as i32 + l.car_len() - 2 * s
                };
                let raised = if stopped { pose.lift } else { 0 };
                let reach = (l.kit.figure_width() - 1) * s;
                let hands = if right { x + reach } else { x };
                let hy = feet - l.kit.hand * s + raised * 2;
                let jy = l.ground - raised;
                paint::line(
                    canvas,
                    (hands as f32, hy as f32),
                    (end as f32, jy as f32),
                    s,
                    c.jack,
                );
                canvas.fill_rect(end - s, l.ground, 3 * s, s, c.jack);
            }
            self.figure(canvas, (x, feet), kneel, stride, right);
            // a gunner's gun at the hub while it works
            if gunner && walk.is_none_or(|w| w.kneel && w.along >= 1.0) {
                let which = if who == Crew::RearGun { 0 } else { 1 };
                let gun = walk.map_or(pose.wheels[which].gun, |_| true);
                if let (true, Some(car_x)) = (gun, pose.car_x) {
                    let target = if walk.is_some() { 0 } else { which };
                    let (cx, cy, _) = l.wheel_at(car_x, pose.lift, target);
                    let reach = (l.kit.kneel[0].len() as i32 - 1) * s;
                    let hands = if right { x + reach } else { x };
                    let hy = feet - l.kit.kneel_hand * s;
                    paint::line(canvas, (hands as f32, hy as f32), (cx, cy), s, c.gun);
                    canvas.fill_rect(hands, hy, s, 2 * s, c.gun);
                }
            }
            // a wheel in hand: a new one before the stop, the old one after it
            let carried = match walk {
                Some(w) => w.wheel,
                None => self.in_hand(who, pose),
            };
            if let Some(fresh) = carried {
                let at = self.held(x as f32, feet as f32, right);
                self.wheel(canvas, at, l.tyre_radius(), fresh, false);
            }
            // the lollipop is held out over the car, so it is painted after it
            if who == Crew::Lollipop && !l.far(who) {
                self.lollipop(canvas, (x, feet), pose.lollipop);
            }
        }
        // a wheel on its way off the car to its carrier on the far side, passing behind the car
        if let (true, true, Some(car_x)) = (far, l.kit.crew, pose.car_x) {
            for (which, wheel) in pose.wheels.iter().enumerate() {
                if wheel.off > 0.0 && wheel.off < 1.0 {
                    let (cx, cy, r) = l.wheel_at(car_x, pose.lift, which);
                    let who = Crew::off_carrier(which);
                    let to =
                        self.held(l.spot(who) as f32, l.feet_of(who) as f32, who.faces_right());
                    let at = lerp2((cx, cy), to, ease(wheel.off));
                    self.wheel(canvas, at, r, false, false);
                }
            }
        }
    }

    /// What a carrier standing at their spot holds.
    fn in_hand(&self, who: Crew, pose: &Pose) -> Option<bool> {
        if !self.l.kit.crew {
            return None;
        }
        (0..2).find_map(|which| {
            let wheel = pose.wheels[which];
            if who == Crew::on_carrier(which) && pose.new_in_hand && wheel.on <= 0.0 {
                Some(true)
            } else if who == Crew::off_carrier(which) && (wheel.off >= 1.0 || pose.old_in_hand) {
                Some(false)
            } else {
                None
            }
        })
    }

    /// The lollipop: a pole from the hands with a round board, down in front of the driver and
    /// swung up to let the car go, red side towards the car while it waits.
    fn lollipop(&self, canvas: &mut Canvas, (x, feet): (i32, i32), up: f32) {
        let l = &self.l;
        let c = self.c;
        let s = l.scale as f32;
        let hands = (x as f32, (feet - l.kit.hand * l.scale) as f32);
        // held down, the board sits over the nose clear above the heads of the crew in front
        // from the far side the pole leans out low over the car, so the board hangs just
        // ahead of the driver's helmet rather than high over the roof
        let far = l.far(Crew::Lollipop);
        let down = if far { 0.86 } else { l.kit.down };
        let angle = std::f32::consts::PI * (down + (0.45 - down) * ease(up));
        let length = l.kit.pole * s * if far { 0.7 } else { 1.0 };
        let tip = (
            hands.0 + angle.cos() * length,
            hands.1 - angle.sin() * length,
        );
        paint::line(canvas, hands, tip, l.scale, c.pole);
        let r = l.kit.disc * s;
        let face = if up > 0.5 { c.go } else { c.stop };
        paint::disc(canvas, tip.0, tip.1 - r * 0.5, r, face);
    }

    fn paint_board(&self, canvas: &mut Canvas) {
        let Some((row, x0, x1)) = self.l.board() else {
            return;
        };
        let c = self.c;
        let (y0, y1) = (row * 2 - 1, row * 2 + 3);
        // the board hangs from the roof on two wires, its face left plain for the time
        let top = self.l.roof_bottom;
        canvas.fill_rect(x0 + 1, top, 1, y0 - top, c.frame);
        canvas.fill_rect(x1 - 2, top, 1, y0 - top, c.frame);
        canvas.fill_rect(x0 - 1, y0 - 1, x1 - x0 + 2, y1 - y0 + 2, c.frame);
        canvas.fill_rect(x0, y0, x1 - x0, y1 - y0, self.background);
    }

    /// Paints the whole stop into a canvas.
    pub(crate) fn paint(&self, canvas: &mut Canvas, pose: &Pose) {
        self.paint_backdrop(canvas);
        self.paint_lane(canvas);
        self.paint_stacks(canvas);
        self.paint_board(canvas);
        self.paint_crew(canvas, pose, true);
        self.paint_car(canvas, pose);
        self.paint_crew(canvas, pose, false);
        if self.l.far(Crew::Lollipop) {
            let who = Crew::Lollipop;
            let at = (self.l.spot(who), self.l.feet_of(who));
            self.lollipop(canvas, at, pose.lollipop);
        }
    }

    /// Writes the board's text over the painted frame.
    pub(crate) fn label(&self, frame: &mut Frame, pose: &Pose) {
        if let Some((row, x0, x1)) = self.l.board() {
            let len = pose.board.chars().count() as i32;
            let x = x0 + (x1 - x0 - len + 1) / 2;
            paint::label(frame, x, row, &pose.board, self.c.text);
        }
    }
}

/// Draws a pose of the pit lane into a frame.
pub(crate) fn draw_pose(frame: &mut Frame, theme: &ThemeData, layout: Layout, pose: &Pose) {
    let colours = Colours::new(theme);
    let mut canvas = Canvas::for_frame(frame, theme.background);
    let painter = Painter {
        l: layout,
        c: &colours,
        background: theme.background,
    };
    painter.paint(&mut canvas, pose);
    canvas.blit(frame);
    painter.label(frame, pose);
}

/// How far along a phase that starts at `from` and lasts `length` a moment `t` is, from 0 to 1.
pub(crate) fn phase(t: f32, from: f32, length: f32) -> f32 {
    ((t - from) / length.max(0.001)).clamp(0.0, 1.0)
}

/// The car's left edge while it slides in: quick at first, braking into the box.
pub(crate) fn arriving(layout: &Layout, t: f32) -> f32 {
    let start = -(layout.car_len() as f32) - 2.0;
    let k = phase(t, 0.0, ARRIVE);
    let eased = 1.0 - (1.0 - k).powi(3);
    start + (layout.stop_x() - start) * eased
}

/// The jacks' lift in pixels while the car is up.
pub(crate) fn jacked(layout: &Layout, up: f32) -> i32 {
    (up * layout.scale as f32).round() as i32
}

/// Formats a stop time in hundredths as the board shows it.
pub(crate) fn seconds(hundredths: u32) -> String {
    format!("{}.{:02}s", hundredths / 100, hundredths % 100)
}

pub struct PitStop {
    width: usize,
    height: usize,
    motion: Motion,
    /// Seconds into the scene at the standard pace.
    t: f64,
}

impl PitStop {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut scene = Self {
            width: 1,
            height: 1,
            motion,
            t: START,
        };
        scene.resize(width, height, rng);
        scene
    }

    fn layout(&self) -> Layout {
        Layout::new(self.width, self.height)
    }

    /// When the car leaves the screen, in seconds into a stop.
    fn gone() -> f32 {
        ARRIVE + 2.0 * JACK + 0.3 + 2.0 * SWAP + 0.3 + LIFT + LEAVE
    }

    /// The pose at this moment of the loop.
    fn pose(&self) -> Pose {
        let l = self.layout();
        let lap = (self.t / f64::from(CYCLE)).floor() as usize;
        let t = (self.t % f64::from(CYCLE)) as f32;
        let last = seconds(TIMES[(lap + TIMES.len() - 1) % TIMES.len()]);
        let this = seconds(TIMES[lap % TIMES.len()]);
        let jack_up = ARRIVE;
        let guns = jack_up + JACK;
        let off = guns + 0.3;
        let on = off + SWAP;
        let guns_off = on + SWAP;
        let jack_down = guns_off;
        let lift = jack_down + JACK;
        let go = lift + LIFT;
        let gone = Self::gone();
        let mut pose = Pose::waiting(if t < lift { last } else { this });
        if t < ARRIVE {
            pose.car_x = Some(arriving(&l, t));
            return pose;
        }
        if t < gone {
            let k = phase(t, go, LEAVE);
            let start = l.stop_x();
            let end = l.wide as f32 + 2.0;
            // pulls away slowly, then gathers speed down the lane
            pose.car_x = Some(start + (end - start) * k * k);
            pose.lift = jacked(&l, phase(t, jack_up, JACK) - phase(t, jack_down, JACK));
            pose.fresh = t >= guns_off;
            if !pose.fresh {
                for wheel in &mut pose.wheels {
                    wheel.gun = t >= guns;
                    wheel.off = phase(t, off, SWAP);
                    wheel.on = phase(t, on, SWAP);
                }
            }
            pose.new_in_hand = t < on;
            pose.old_in_hand = t >= on;
            // the lollipop swings up, then back down once the car is clear
            pose.lollipop = phase(t, lift, LIFT) - phase(t, go + LEAVE * 0.5, LIFT);
            return pose;
        }
        // the box is empty: the carriers walk the old wheels to the stacks and come back with
        // new ones
        let k = phase(t, gone, FETCH);
        let along = if k < 0.5 { k * 2.0 } else { 2.0 - k * 2.0 };
        pose.new_in_hand = k >= 1.0;
        for which in 0..2 {
            let to = l.at_stack(which);
            let s = l.scale as f32;
            pose.walks.push(Walk {
                who: Crew::off_carrier(which),
                to,
                along,
                wheel: (k < 0.5).then_some(false),
                kneel: false,
                right: which == 1,
            });
            let beside = if which == 0 { 6.0 * s } else { -6.0 * s };
            pose.walks.push(Walk {
                who: Crew::on_carrier(which),
                to: (to.0 + beside, to.1),
                along,
                wheel: (k >= 0.5).then_some(true),
                kneel: false,
                right: which == 1,
            });
        }
        pose
    }
}

impl Scene for PitStop {
    fn name(&self) -> &'static str {
        "pit-stop"
    }

    fn resize(&mut self, width: usize, height: usize, _rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt * self.motion.speed());
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        draw_pose(frame, theme, self.layout(), &self.pose());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixels::IMAGERY_CEILING;
    use crate::scenes::testing::{apart, check_determinism, check_pixel_scene, check_scene};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("pit-stop");
        check_determinism("pit-stop");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("pit-stop", 6.0);
    }

    #[test]
    fn the_layers_stay_apart_and_shapes_keep_their_outline_in_every_theme() {
        for theme in crate::theme::all() {
            let c = Colours::new(theme);
            for colour in [c.wall, c.garage.at(0.5), c.lane.at(0.0), c.lane.at(1.0)]
                .into_iter()
                .chain([c.box_line, c.pit_wall, c.sky.at(0.5), c.cabinet])
            {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "{}",
                    theme.name
                );
            }
            for colour in [
                c.body.0, c.body.1, c.stripe, c.suit, c.rim, c.fresh, c.stop, c.go,
            ] {
                assert!(
                    theme.contrast_of(colour) >= paint::SOLID - 0.01,
                    "{} in {}",
                    colour.hex(),
                    theme.name
                );
            }
            apart(
                theme,
                c.body.1,
                c.lane.at(0.0),
                0.08,
                "the car and the lane",
            );
            apart(theme, c.suit, c.lane.at(1.0), 0.08, "the crew and the lane");
            apart(theme, c.suit, c.body.1, 0.05, "the crew and the car");
            apart(theme, c.fresh, c.worn, 0.08, "fresh and worn tyres");
            apart(theme, c.stop, c.go, 0.08, "the two sides of the lollipop");
            apart(theme, c.nut, c.rim, 0.08, "the stuck nut and the rim");
        }
    }

    #[test]
    fn the_stop_fills_the_frame_with_crew_on_both_sides_of_the_car() {
        for (w, h) in [(80, 24), (120, 34), (200, 50)] {
            let l = Layout::new(w, h);
            let car = l.kit.car.len() as i32 * l.scale;
            assert!(
                car * 4 >= l.tall,
                "the car is {car} of {} rows at {w}x{h}",
                l.tall
            );
            assert!(l.far(Crew::RearOff) && l.far(Crew::Lollipop), "{w}x{h}");
            assert!(!l.far(Crew::RearGun) && !l.far(Crew::FrontJack), "{w}x{h}");
            // the far side stands behind the tyres and the near side in front of them
            assert!(l.feet_of(Crew::FrontOff) < l.ground && l.feet_of(Crew::RearOn) > l.ground);
            // the tyre stacks never stand behind the car in its box
            for (x, _) in l.stacks() {
                let off = (x - l.box_x as f32).abs();
                assert!(off > (l.car_len() / 2) as f32, "{w}x{h}");
            }
        }
    }

    #[test]
    fn the_car_stops_in_its_box_then_leaves_with_new_wheels_and_a_time() {
        let mut rng = Rng::new(1);
        let mut scene = PitStop::new(100, 30, Motion::FULL, &mut rng);
        let l = scene.layout();
        let mut jacked_in_box = false;
        let mut boards = vec![scene.pose().board];
        for _ in 0..(CYCLE * 20.0) as usize {
            scene.step(0.05, &mut rng);
            let pose = scene.pose();
            if pose.car_x == Some(l.stop_x()) && pose.lift > 0 {
                jacked_in_box = true;
            }
            if boards.last() != Some(&pose.board) {
                boards.push(pose.board.clone());
            }
        }
        assert!(jacked_in_box, "the car never stopped jacked up in its box");
        // the board changes once a stop, to the new time
        assert_eq!(boards.len(), 2, "{boards:?}");
        assert!(
            scene.pose().car_x.is_some_and(|x| x < l.stop_x()),
            "a new car comes in"
        );
        scene.t = f64::from(PitStop::gone() + 0.5);
        let pose = scene.pose();
        assert_eq!(pose.car_x, None, "the car has left the screen");
        assert!(!pose.walks.is_empty(), "the carriers fetch new wheels");
    }
}
