//! Koi pond: a garden pond seen from above, painted in solid pixels. The water deepens gently
//! towards the middle, koi seen from above swim slow curving paths with their bodies bending and
//! their tails sweeping, lily pads rest on the surface and now and then a soft ring spreads where
//! something touched it. A grassy bank runs along the top and a rim of rounded stones along the
//! bottom.
//!
//! Each koi is drawn from its spine rather than from a sprite, so it can face any way: a curved
//! line from the snout to the tail with a width along it, a fanned tail, pectoral fins behind the
//! head and small pelvic fins further back. Its markings are laid out along the body, so they
//! bend with it.
//!
//! Every colour comes from the active theme. The water, the shadows, the bank and the stones are
//! tints of the background held below the non-text floor, so they read as background; each koi
//! has a solid rim and the pads keep the theme's own green, so they stand out as shapes and keep
//! their outline when colour is off.

use std::f32::consts::{PI, TAU};

use super::paint::{held_imagery, noise, solid, strong, tint, unit};
use super::{count_for, cycle};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "A garden pond from above, with shaded koi gliding past lily pads and slow ripples";

pub const CAPTIONS: &[&str] = &[
    "Still water, clean build.",
    "The koi have nothing to report.",
    "Not a ripple of trouble.",
    "Everything compiles. Feed the fish.",
    "Calm on the surface and underneath.",
];

/// Seconds a koi takes for one full sweep of its tail, there and back, at the standard pace.
pub const TAIL_BEAT: f32 = 2.6;
/// Seconds a ripple takes to spread out and fade at the standard pace.
pub const RIPPLE_LIFE: f32 = 3.2;
/// The radius a ripple reaches as it fades, in pixels.
const RIPPLE_REACH: f32 = 7.0;
/// The fastest a koi turns, in radians a second at the standard pace, slow enough that every
/// turn is a wide curve.
const TURN_LIMIT: f32 = 0.65;

/// Steps in the water's depth ramp and in a ripple's lift; few steps keep the escape output small
/// while still reading as a smooth gradient.
const DEPTH_STEPS: usize = 12;
const RIPPLE_STEPS: usize = 3;
/// Segments along a koi's spine, enough that the largest koi bends smoothly.
const SEGMENTS: usize = 14;
/// How far along the body the koi is widest, as a share of its length.
const WIDEST: f32 = 0.32;
/// The tail fan's length as a share of the body's.
const TAIL_LENGTH: f32 = 0.31;

/// The colours a koi is marked in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ink {
    White,
    Red,
    Black,
    Gold,
    Pale,
}

const INKS: [Ink; 5] = [Ink::White, Ink::Red, Ink::Black, Ink::Gold, Ink::Pale];

/// The classic varieties: kohaku is white with red, sanke adds small black spots, showa is black
/// with red and white, yamabuki is solid gold and ogon a pale metallic gold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Coat {
    Kohaku,
    Sanke,
    Showa,
    Yamabuki,
    Ogon,
}

impl Coat {
    const ALL: [Coat; 5] = [
        Coat::Kohaku,
        Coat::Sanke,
        Coat::Showa,
        Coat::Yamabuki,
        Coat::Ogon,
    ];

    fn ground(self) -> Ink {
        match self {
            Coat::Kohaku | Coat::Sanke => Ink::White,
            Coat::Showa => Ink::Black,
            Coat::Yamabuki => Ink::Gold,
            Coat::Ogon => Ink::Pale,
        }
    }

    /// The metallic varieties catch the light along the back.
    fn metallic(self) -> bool {
        matches!(self, Coat::Yamabuki | Coat::Ogon)
    }

    /// The fins: white on the white and black varieties, as on real fish, gold on the gold ones.
    fn fin(self) -> Ink {
        match self {
            Coat::Kohaku | Coat::Sanke | Coat::Showa => Ink::White,
            Coat::Yamabuki => Ink::Gold,
            Coat::Ogon => Ink::Pale,
        }
    }
}

/// The colours of one ink on a koi: its edge, its body, the light along a metallic back and its
/// fins, which let some of the water through.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Tone {
    pub rim: Rgb,
    pub body: Rgb,
    pub sheen: Rgb,
    pub fin: Rgb,
    pub fin_edge: Rgb,
}

/// The pond's colours in one theme, every one derived from the theme's own. Some are kept only
/// so the tests can check the bands against each other.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct PondColours {
    pub shallow: Rgb,
    pub deep: Rgb,
    pub grass: Rgb,
    pub grass_dark: Rgb,
    pub grass_light: Rgb,
    pub soil: Rgb,
    pub stone: Rgb,
    pub stone_light: Rgb,
    pub stone_dark: Rgb,
    pub pad: Rgb,
    pub pad_dark: Rgb,
    pub pad_light: Rgb,
    pub petal: Rgb,
    pub petal_dark: Rgb,
    pub centre: Rgb,
    pub eye: Rgb,
    /// One tone for each ink, in the order of `INKS`.
    pub inks: [Tone; 5],
    /// For each depth step, the water and its shadow.
    water: Vec<Rgb>,
    shadow: Vec<Rgb>,
    /// For each depth step, the lift a ripple gives it.
    ripple: Vec<Ramp>,
}

impl PondColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let background = theme.background;
        // deeper water is darker in every theme: on a dark theme it sinks towards the
        // background, on a light one it moves away from it
        let (shallow, deep) = if theme.dark {
            (
                tint(theme, theme.cyan(), 0.42, 2.0),
                tint(theme, theme.blue(), 0.22, 1.35),
            )
        } else {
            (
                tint(theme, theme.cyan(), 0.2, 1.45),
                tint(theme, theme.blue(), 0.55, 2.2),
            )
        };
        let depth = Ramp::new(shallow, deep, DEPTH_STEPS);
        let water: Vec<Rgb> = depth.steps().to_vec();
        let shadow = water
            .iter()
            .map(|&base| {
                if theme.dark {
                    mix(base, background, 0.32)
                } else {
                    held_imagery(background, base, theme.foreground, 0.22)
                }
            })
            .collect();
        let lift = if theme.dark {
            theme.bright_cyan()
        } else {
            background
        };
        let ripple = water
            .iter()
            .map(|&base| {
                let top = held_imagery(background, base, lift, 0.4);
                Ramp::new(base, top, RIPPLE_STEPS)
            })
            .collect();

        let grass = tint(theme, theme.green(), 0.5, 2.3);
        let (grass_dark, grass_light) = if theme.dark {
            (
                tint(theme, theme.green(), 0.3, 1.6),
                tint(theme, theme.bright_green(), 0.62, 2.8),
            )
        } else {
            (
                tint(theme, theme.green(), 0.75, 2.8),
                tint(theme, theme.green(), 0.3, 1.6),
            )
        };
        let soil = tint(theme, theme.yellow(), 0.25, 1.4);
        // stones are lit from the top left: towards the theme's lightest colour on that side
        let (stone, stone_light, stone_dark) = if theme.dark {
            (
                tint(theme, theme.grey(), 0.5, 2.1),
                tint(theme, theme.foreground, 0.6, 2.8),
                tint(theme, theme.grey(), 0.28, 1.45),
            )
        } else {
            (
                tint(theme, theme.grey(), 0.4, 1.9),
                tint(theme, theme.grey(), 0.15, 1.3),
                tint(theme, theme.grey(), 0.75, 2.8),
            )
        };

        let pad = if theme.dark {
            strong(theme, theme.green())
        } else {
            solid(theme, theme.green(), background, 0.3)
        };
        let rim = |colour: Rgb| -> Rgb {
            // on a dark theme the body already stands well clear of the water, so its rim is a
            // gentle shade; on a light theme the rim is what outlines a pale koi
            let t = if theme.dark { 0.3 } else { 0.45 };
            if theme.contrast_of(colour) >= IMAGERY_CEILING + 0.2 {
                solid(theme, colour, shades.darkest(), t)
            } else {
                strong(theme, mix(colour, shades.darkest(), 0.45))
            }
        };
        let fin_water = water[DEPTH_STEPS / 2];
        let tone = |body: Rgb| {
            let fin = mix(body, fin_water, 0.3);
            Tone {
                rim: rim(body),
                body,
                sheen: mix(body, shades.lightest(), 0.4),
                fin,
                fin_edge: mix(fin, rim(body), 0.4),
            }
        };
        // a white koi on a light theme is the paper colour itself, outlined by its rim, rather
        // than a grey that would read as a dark fish
        let white = if theme.dark {
            mix(theme.foreground, background, 0.04)
        } else {
            mix(background, theme.yellow(), 0.05)
        };
        let red = strong(theme, mix(theme.red(), theme.yellow(), 0.18));
        let black = if theme.dark {
            mix(background, theme.grey(), 0.22)
        } else {
            mix(theme.foreground, background, 0.12)
        };
        // gold on a light theme is lifted towards the paper, so it reads as gold rather than
        // brown; its rim keeps the outline
        let gold = mix(theme.yellow(), theme.bright_yellow(), 0.5);
        let gold = if theme.dark {
            strong(theme, gold)
        } else {
            mix(gold, background, 0.35)
        };
        let pale = mix(gold, white, 0.5);
        Self {
            shallow,
            deep,
            grass,
            grass_dark,
            grass_light,
            soil,
            stone,
            stone_light,
            stone_dark,
            pad,
            pad_dark: solid(theme, pad, shades.darkest(), 0.35),
            pad_light: mix(pad, shades.lightest(), 0.3),
            // a pale flower on a light theme, where the theme's magenta would be too dark to
            // show against the pad
            petal: if theme.dark {
                strong(theme, theme.bright_magenta())
            } else {
                mix(theme.bright_magenta(), background, 0.6)
            },
            petal_dark: if theme.dark {
                strong(theme, theme.magenta())
            } else {
                mix(theme.bright_magenta(), background, 0.3)
            },
            centre: strong(theme, theme.bright_yellow()),
            eye: shades.darkest(),
            inks: [tone(white), tone(red), tone(black), tone(gold), tone(pale)],
            water,
            shadow,
            ripple,
        }
    }

    fn step(depth: f32) -> usize {
        ((depth.clamp(0.0, 1.0) * (DEPTH_STEPS - 1) as f32).round() as usize).min(DEPTH_STEPS - 1)
    }

    fn water(&self, depth: f32) -> Rgb {
        self.water[Self::step(depth)]
    }

    fn shadow(&self, depth: f32) -> Rgb {
        self.shadow[Self::step(depth)]
    }

    fn ripple(&self, depth: f32, lift: f32) -> Rgb {
        self.ripple[Self::step(depth)].at(lift)
    }

    fn tone(&self, ink: Ink) -> Tone {
        let i = INKS.iter().position(|&k| k == ink).unwrap_or(0);
        self.inks[i]
    }
}

/// A patch of colour on a koi, laid out along the body: how far back its centre is as a share of
/// the length, where it sits across from -1 on one side to 1 on the other and its two radii in
/// the same terms.
#[derive(Clone, Copy, Debug)]
struct Patch {
    along: f32,
    across: f32,
    long: f32,
    wide: f32,
    ink: Ink,
}

#[derive(Clone, Debug)]
struct Koi {
    /// The snout, in pixels.
    x: f32,
    y: f32,
    /// The way it is heading in radians and how fast that is turning.
    heading: f32,
    turn: f32,
    /// Pixels a second at the standard pace.
    speed: f32,
    /// The koi's size against the pond's standard koi.
    scale: f32,
    coat: Coat,
    patches: Vec<Patch>,
    /// Seeds the wandering and the ragged edges of the patches, so each koi is its own.
    seed: u32,
    phase: f32,
}

#[derive(Clone, Debug)]
struct Pad {
    x: f32,
    y: f32,
    radius: f32,
    /// The angle the notch is cut at.
    notch: f32,
    flower: bool,
    phase: f32,
}

#[derive(Clone, Debug)]
struct Ripple {
    x: f32,
    y: f32,
    age: f32,
}

/// One pixel of a koi.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Part {
    Body { ink: Ink, rim: bool, sheen: bool },
    Fin { ink: Ink, edge: bool },
    Eye,
}

/// What a pixel of a koi's grid holds while the koi is worked out.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Cover {
    Empty,
    Body,
    Fin,
}

/// A fin as an ellipse: its centre, the unit direction of its length and its two half sizes.
#[derive(Clone, Copy, Debug)]
struct Oval {
    centre: (f32, f32),
    axis: (f32, f32),
    long: f32,
    wide: f32,
}

impl Oval {
    fn contains(&self, x: f32, y: f32) -> bool {
        let (dx, dy) = (x - self.centre.0, y - self.centre.1);
        let a = (dx * self.axis.0 + dy * self.axis.1) / self.long;
        let b = (dy * self.axis.0 - dx * self.axis.1) / self.wide;
        a * a + b * b <= 1.0
    }
}

/// A koi's outline at one moment: its spine from the snout to the root of the tail, its length
/// and widest half width, the tail's root and direction and its fins.
struct Outline {
    spine: Vec<(f32, f32)>,
    length: f32,
    half: f32,
    tail: ((f32, f32), (f32, f32)),
    fins: Vec<Oval>,
}

impl Outline {
    /// The body's half width a share `s` of the way from the snout to the tail: a blunt rounded
    /// head, widest a little under a third of the way back, then a long taper to a narrow stalk.
    fn width(&self, s: f32) -> f32 {
        let half = self.half;
        let w = if s < WIDEST {
            let u = (WIDEST - s) / WIDEST;
            half * (0.42 + 0.58 * (1.0 - u * u).max(0.0).sqrt())
        } else {
            let u = ((s - WIDEST) / (1.0 - WIDEST)).min(1.0);
            let stalk = 0.33;
            half * (stalk + (1.0 - stalk) * (1.0 - u.powf(1.7)))
        };
        w.max(0.5)
    }

    /// The nearest point of the spine to a pixel centre, as how far along the body it is, from 0
    /// at the snout to 1 at the tail, how far to the side the pixel is, signed and how far away.
    fn locate(&self, x: f32, y: f32) -> (f32, f32, f32) {
        let mut best = (f32::MAX, 0.0, 0.0);
        let n = self.spine.len() - 1;
        for i in 0..n {
            let (a, b) = (self.spine[i], self.spine[i + 1]);
            let (ex, ey) = (b.0 - a.0, b.1 - a.1);
            let len2 = (ex * ex + ey * ey).max(1e-6);
            let t = (((x - a.0) * ex + (y - a.1) * ey) / len2).clamp(0.0, 1.0);
            let (qx, qy) = (a.0 + ex * t, a.1 + ey * t);
            let d2 = (x - qx).powi(2) + (y - qy).powi(2);
            if d2 < best.0 {
                let len = len2.sqrt();
                let side = ((x - qx) * ey - (y - qy) * ex) / len;
                best = (d2, (i as f32 + t) / n as f32, side);
            }
        }
        (best.1, best.2, best.0.sqrt())
    }

    /// Whether a pixel centre falls inside the tail fan: a fan that widens from the stalk and is
    /// cut back in the middle of its trailing edge.
    fn in_tail(&self, x: f32, y: f32) -> bool {
        let ((rx, ry), (dx, dy)) = self.tail;
        let reach = self.length * TAIL_LENGTH;
        let (ox, oy) = (x - rx, y - ry);
        let u = (ox * dx + oy * dy) / reach;
        if !(-0.05..=1.0).contains(&u) {
            return false;
        }
        let v = (oy * dx - ox * dy).abs();
        let stalk = self.width(1.0);
        let spread = stalk + (self.half * 0.95 - stalk) * u.max(0.0).powf(0.75);
        if v > spread {
            return false;
        }
        u <= 1.0 - 0.3 * (1.0 - v / spread.max(0.1))
    }
}

/// A straight line between two points.
type Segment = ((f32, f32), (f32, f32));

/// The shortest distance between two segments and the direction from the closest point on `b`
/// to the closest point on `a`, which is the way to move `a` to part them. Crossing segments
/// are parted along the line between their middles.
fn gap(a: Segment, b: Segment) -> (f32, (f32, f32)) {
    let closest_on = |p: (f32, f32), s: Segment| -> (f32, f32) {
        let (ex, ey) = (s.1.0 - s.0.0, s.1.1 - s.0.1);
        let len2 = (ex * ex + ey * ey).max(1e-6);
        let t = (((p.0 - s.0.0) * ex + (p.1 - s.0.1) * ey) / len2).clamp(0.0, 1.0);
        (s.0.0 + ex * t, s.0.1 + ey * t)
    };
    let cross = |o: (f32, f32), p: (f32, f32), q: (f32, f32)| {
        (p.0 - o.0) * (q.1 - o.1) - (p.1 - o.1) * (q.0 - o.0)
    };
    let crossing = cross(a.0, a.1, b.0) * cross(a.0, a.1, b.1) < 0.0
        && cross(b.0, b.1, a.0) * cross(b.0, b.1, a.1) < 0.0;
    let unit = |v: (f32, f32)| {
        let l = (v.0 * v.0 + v.1 * v.1).sqrt();
        if l > 1e-4 {
            (v.0 / l, v.1 / l)
        } else {
            (1.0, 0.0)
        }
    };
    if crossing {
        let mid = |s: Segment| ((s.0.0 + s.1.0) / 2.0, (s.0.1 + s.1.1) / 2.0);
        let (ma, mb) = (mid(a), mid(b));
        return (0.0, unit((ma.0 - mb.0, ma.1 - mb.1)));
    }
    let mut best = (f32::MAX, (1.0, 0.0));
    for (p, s, flip) in [
        (a.0, b, false),
        (a.1, b, false),
        (b.0, a, true),
        (b.1, a, true),
    ] {
        let q = closest_on(p, s);
        let v = if flip {
            (q.0 - p.0, q.1 - p.1)
        } else {
            (p.0 - q.0, p.1 - q.1)
        };
        let d = (v.0 * v.0 + v.1 * v.1).sqrt();
        if d < best.0 {
            best = (d, unit(v));
        }
    }
    best
}

pub struct KoiPond {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    koi: Vec<Koi>,
    pads: Vec<Pad>,
    ripples: Vec<Ripple>,
    next_ripple: f32,
    /// Stones along the rim as centre column, width and height in pixels.
    stones: Vec<(f32, f32, f32)>,
}

impl KoiPond {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut pond = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            koi: Vec::new(),
            pads: Vec::new(),
            ripples: Vec::new(),
            next_ripple: rng.range(1.0, 3.0),
            stones: Vec::new(),
        };
        pond.resize(width, height, rng);
        pond
    }

    fn pixels_tall(&self) -> f32 {
        self.height as f32 * 2.0
    }

    /// Pixel rows of the grassy bank at the top and the stone rim at the bottom.
    fn banks(&self) -> (f32, f32) {
        if self.height >= 20 {
            (4.0, 5.0)
        } else if self.height >= 8 {
            (2.0, 3.0)
        } else {
            (0.0, 0.0)
        }
    }

    /// The first and last pixel rows of open water.
    fn water(&self) -> (f32, f32) {
        let (bank, rim) = self.banks();
        (bank, (self.pixels_tall() - rim - 1.0).max(bank))
    }

    /// The length of the pond's standard koi in pixels, tail not included.
    fn koi_length(&self) -> f32 {
        let (top, bottom) = self.water();
        // a little larger than proportional on small screens, so a koi keeps enough pixels to
        // show its head, fins and tail
        (7.0 + self.width as f32 * 0.09)
            .min((bottom - top) * 0.42)
            .clamp(6.0, 26.0)
    }

    /// The bounds a koi's snout keeps inside: far enough from every edge that its body stays in
    /// the water.
    fn bounds(&self) -> (f32, f32, f32, f32) {
        let (top, bottom) = self.water();
        let length = self.koi_length();
        // more room at the sides than at the banks, since a koi near the side of the screen
        // would have its body cut off by the edge
        let (side, bank) = (length * 0.6, length * 0.3 + 4.0);
        let mid = |a: f32, b: f32| {
            if a <= b {
                (a, b)
            } else {
                ((a + b) / 2.0, (a + b) / 2.0)
            }
        };
        let (x0, x1) = mid(side, self.width as f32 - side);
        let (y0, y1) = mid(top + bank, bottom - bank);
        (x0, x1, y0, y1)
    }

    fn ripple_life(&self) -> f32 {
        RIPPLE_LIFE / self.motion.pace()
    }

    fn tail_beat(&self) -> f32 {
        TAIL_BEAT / self.motion.pace()
    }

    fn spawn_koi(&self, rng: &mut Rng, coat: Coat) -> Koi {
        let (x0, x1, y0, y1) = self.bounds();
        let mut patches = Vec::new();
        let red = |patches: &mut Vec<Patch>, rng: &mut Rng| {
            // a patch on the head, leaving the snout clear, then one or two along the back
            patches.push(Patch {
                along: rng.range(0.15, 0.2),
                across: rng.range(-0.15, 0.15),
                long: rng.range(0.09, 0.12),
                wide: rng.range(0.8, 1.1),
                ink: Ink::Red,
            });
            patches.push(Patch {
                along: rng.range(0.4, 0.52),
                across: rng.range(-0.35, 0.35),
                long: rng.range(0.1, 0.15),
                wide: rng.range(0.8, 1.3),
                ink: Ink::Red,
            });
            if rng.chance(0.6) {
                patches.push(Patch {
                    along: rng.range(0.66, 0.76),
                    across: rng.range(-0.3, 0.3),
                    long: rng.range(0.06, 0.09),
                    wide: rng.range(0.6, 1.0),
                    ink: Ink::Red,
                });
            }
        };
        match coat {
            Coat::Kohaku => red(&mut patches, rng),
            Coat::Sanke => {
                red(&mut patches, rng);
                for _ in 0..3 + rng.below(2) {
                    patches.push(Patch {
                        along: rng.range(0.32, 0.8),
                        across: rng.range(-0.6, 0.6),
                        long: rng.range(0.035, 0.055),
                        wide: rng.range(0.3, 0.45),
                        ink: Ink::Black,
                    });
                }
            }
            Coat::Showa => {
                for _ in 0..2 {
                    patches.push(Patch {
                        along: rng.range(0.3, 0.75),
                        across: if rng.chance(0.5) { -0.6 } else { 0.6 },
                        long: rng.range(0.08, 0.12),
                        wide: rng.range(0.5, 0.8),
                        ink: Ink::White,
                    });
                }
                red(&mut patches, rng);
            }
            Coat::Yamabuki | Coat::Ogon => {}
        }
        Koi {
            x: rng.range(x0, x1.max(x0 + 0.01)),
            y: rng.range(y0, y1.max(y0 + 0.01)),
            heading: rng.range(0.0, TAU),
            turn: 0.0,
            speed: rng.range(2.2, 3.2),
            scale: rng.range(0.85, 1.1),
            coat,
            patches,
            seed: rng.next_u64() as u32,
            phase: rng.range(0.0, TAU),
        }
    }

    /// A koi as a line from a little ahead of its snout back to its tail, `ahead` body lengths
    /// in front, which the separation keeps apart from other koi.
    fn span(&self, koi: &Koi, ahead: f32) -> Segment {
        let length = self.koi_length() * koi.scale;
        let (dx, dy) = (koi.heading.cos(), koi.heading.sin());
        (
            (koi.x + dx * length * ahead, koi.y + dy * length * ahead),
            (koi.x - dx * length * 1.25, koi.y - dy * length * 1.25),
        )
    }

    /// Works out a koi's outline from where it is, how it is turning and how far through its
    /// tail beat it is.
    fn outline(&self, koi: &Koi) -> Outline {
        let length = self.koi_length() * koi.scale;
        let half = (length * 0.18).max(1.0);
        // the body follows the curve the snout has just swum along, so a turning koi bends
        // into its turn
        let speed = (koi.speed * self.motion.speed()).max(0.3);
        let bend = (koi.turn * self.motion.speed() / speed).clamp(-1.4 / length, 1.4 / length);
        let calm = if self.motion.calm { 0.5 } else { 1.0 };
        let beat = cycle(self.t, TAU / self.tail_beat()) + koi.phase;
        let sway = 0.16 * calm;
        let ds = length / SEGMENTS as f32;
        let mut point = (koi.x, koi.y);
        let mut spine = vec![point];
        let mut angle = koi.heading + PI;
        for i in 0..SEGMENTS {
            let s = i as f32 / SEGMENTS as f32;
            // the swimming wave grows towards the tail and travels down the body
            let wave = sway * s * (beat - 2.4 * s).sin();
            angle = koi.heading + PI - bend * ds * i as f32 + wave;
            point = (point.0 + ds * angle.cos(), point.1 + ds * angle.sin());
            spine.push(point);
        }
        let flex = angle + 0.25 * calm * (beat - 3.0).sin();
        let mut outline = Outline {
            spine,
            length,
            half,
            tail: (point, (flex.cos(), flex.sin())),
            fins: Vec::new(),
        };
        // pectoral fins just behind the head sweep back and paddle slowly; small pelvic fins
        // sit further back on a koi large enough to show them
        let paddle = 0.15 * calm * (beat * 0.5).sin();
        let mut fins = Vec::new();
        let sets: &[(f32, f32, f32, f32)] = if length >= 13.0 {
            &[(0.26, 0.85, 0.13, 0.07), (0.6, 0.6, 0.075, 0.045)]
        } else {
            &[(0.26, 0.8, 0.12, 0.08)]
        };
        for &(at, sweep, long, wide) in sets {
            let i = ((at * SEGMENTS as f32) as usize).min(SEGMENTS - 1);
            let (a, b) = (outline.spine[i], outline.spine[i + 1]);
            let (mut dx, mut dy) = (b.0 - a.0, b.1 - a.1);
            let len = (dx * dx + dy * dy).sqrt().max(1e-6);
            dx /= len;
            dy /= len;
            let w = outline.width(at);
            for side in [-1.0_f32, 1.0] {
                let (nx, ny) = (-dy * side, dx * side);
                let swing = sweep + paddle * side;
                let axis = (
                    dx * swing.cos() + nx * swing.sin(),
                    dy * swing.cos() + ny * swing.sin(),
                );
                let (lf, wf) = ((length * long).max(1.4), (length * wide).max(0.8));
                let root = (a.0 + nx * (w - 0.6), a.1 + ny * (w - 0.6));
                fins.push(Oval {
                    centre: (root.0 + axis.0 * lf, root.1 + axis.1 * lf),
                    axis,
                    long: lf,
                    wide: wf,
                });
            }
        }
        outline.fins = fins;
        outline
    }

    /// The ink of a koi's body at a point laid out along it.
    fn ink(koi: &Koi, along: f32, across: f32) -> Ink {
        let mut ink = koi.coat.ground();
        for p in &koi.patches {
            // a little noise keeps the edges of the patches ragged, as real markings are
            let rough = 0.22 * (noise(along * 14.0, across * 2.5, koi.seed) - 0.5);
            let a = (along - p.along) / p.long;
            let c = (across - p.across) / p.wide;
            if a * a + c * c <= 1.0 + rough {
                ink = p.ink;
            }
        }
        ink
    }

    /// Every pixel a koi covers, with what part of it each one is. The shape is worked out on a
    /// small grid first so stray pixels can be dropped and pinholes filled before anything is
    /// painted, which keeps the silhouette clean.
    fn koi_pixels(&self, koi: &Koi) -> Vec<(i32, i32, Part)> {
        let outline = self.outline(koi);
        let pad = outline.half + outline.length * 0.35 + 2.0;
        let ((tx, ty), (tdx, tdy)) = outline.tail;
        let tip = (
            tx + tdx * outline.length * TAIL_LENGTH,
            ty + tdy * outline.length * TAIL_LENGTH,
        );
        let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for &(sx, sy) in outline.spine.iter().chain([&tip]) {
            x0 = x0.min(sx - pad);
            x1 = x1.max(sx + pad);
            y0 = y0.min(sy - pad);
            y1 = y1.max(sy + pad);
        }
        let (gx, gy) = (x0.floor() as i32, y0.floor() as i32);
        let w = (x1.ceil() as i32 - gx + 1).max(1) as usize;
        let h = (y1.ceil() as i32 - gy + 1).max(1) as usize;
        let mut cover = vec![Cover::Empty; w * h];
        let mut place = vec![(0.0_f32, 0.0_f32); w * h];
        for j in 0..h {
            for i in 0..w {
                let (cx, cy) = ((gx + i as i32) as f32 + 0.5, (gy + j as i32) as f32 + 0.5);
                let (s, side, d) = outline.locate(cx, cy);
                let r = outline.width(s);
                place[j * w + i] = (s, side / r);
                cover[j * w + i] = if d <= r + 0.1 && s < 1.0 {
                    Cover::Body
                } else if outline.in_tail(cx, cy) || outline.fins.iter().any(|f| f.contains(cx, cy))
                {
                    Cover::Fin
                } else {
                    Cover::Empty
                };
            }
        }
        let at = |cover: &[Cover], i: i32, j: i32| -> Cover {
            if i < 0 || j < 0 || i >= w as i32 || j >= h as i32 {
                Cover::Empty
            } else {
                cover[j as usize * w + i as usize]
            }
        };
        let near = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for _ in 0..2 {
            let before = cover.clone();
            for j in 0..h as i32 {
                for i in 0..w as i32 {
                    let here = at(&before, i, j);
                    let body = near
                        .iter()
                        .filter(|(dx, dy)| at(&before, i + dx, j + dy) == Cover::Body)
                        .count();
                    let any = near
                        .iter()
                        .filter(|(dx, dy)| at(&before, i + dx, j + dy) != Cover::Empty)
                        .count();
                    let k = j as usize * w + i as usize;
                    if here == Cover::Empty && any >= 3 {
                        // a pinhole: filled with whatever surrounds it
                        cover[k] = if body >= 2 { Cover::Body } else { Cover::Fin };
                    } else if here != Cover::Body && body >= 3 {
                        cover[k] = Cover::Body;
                    } else if here != Cover::Empty && any <= 1 {
                        // a pixel hanging on by a corner or less reads as noise
                        cover[k] = Cover::Empty;
                    }
                }
            }
        }
        // one eye either side of the head, each the single pixel under a point set in from the
        // edge, so a koi of any size and angle has exactly two
        let eyes: Vec<(i32, i32)> = if outline.length >= 11.0 {
            let at_s = 0.13 * SEGMENTS as f32;
            let k = at_s.floor() as usize;
            let (a, b) = (outline.spine[k], outline.spine[k + 1]);
            let f = at_s - k as f32;
            let (px, py) = (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f);
            let len = (b.0 - a.0).hypot(b.1 - a.1).max(1e-6);
            let (nx, ny) = (-(b.1 - a.1) / len, (b.0 - a.0) / len);
            let r = outline.width(0.13) * 0.55;
            [1.0_f32, -1.0]
                .iter()
                .map(|side| {
                    (
                        (px + nx * r * side).floor() as i32,
                        (py + ny * r * side).floor() as i32,
                    )
                })
                .collect()
        } else {
            Vec::new()
        };
        let mut out = Vec::new();
        for j in 0..h as i32 {
            for i in 0..w as i32 {
                let k = j as usize * w + i as usize;
                let (x, y) = (gx + i, gy + j);
                match cover[k] {
                    Cover::Empty => {}
                    Cover::Fin => {
                        let edge = near
                            .iter()
                            .any(|(dx, dy)| at(&cover, i + dx, j + dy) == Cover::Empty);
                        out.push((
                            x,
                            y,
                            Part::Fin {
                                ink: koi.coat.fin(),
                                edge,
                            },
                        ));
                    }
                    Cover::Body => {
                        let (s, across) = place[k];
                        // only the edge against the water is rimmed: a rim where a fin joins
                        // would cut dark marks into the body
                        let rim = near
                            .iter()
                            .any(|(dx, dy)| at(&cover, i + dx, j + dy) == Cover::Empty);
                        if eyes.contains(&(x, y)) {
                            out.push((x, y, Part::Eye));
                            continue;
                        }
                        let sheen =
                            koi.coat.metallic() && across.abs() < 0.3 && (0.15..0.75).contains(&s);
                        out.push((
                            x,
                            y,
                            Part::Body {
                                ink: Self::ink(koi, s, across),
                                rim,
                                sheen,
                            },
                        ));
                    }
                }
            }
        }
        out
    }

    /// How deep the water is at a pixel, from 0 at the banks to 1 in the middle, with a soft
    /// fixed unevenness so the bed does not look flat and the steps of the ramp meet along
    /// gentle curves rather than straight lines.
    fn depth(&self, x: i32, y: i32) -> f32 {
        let (top, bottom) = self.water();
        let half = ((bottom - top) / 2.0).max(1.0);
        let edge = ((y as f32 - top).min(bottom - y as f32).max(0.0) / half).min(1.0);
        let bed = noise(x as f32 * 0.035, y as f32 * 0.06, 41);
        (edge.powf(0.7) * 0.75 + 0.4 * (bed - 0.3)).clamp(0.0, 1.0)
    }

    fn paint(&self, canvas: &mut Canvas, colours: &PondColours) {
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let (top, bottom) = self.water();
        let (bank, rim) = self.banks();
        let in_water = |y: i32| (y as f32) >= top && (y as f32) <= bottom;

        for y in 0..tall {
            for x in 0..wide {
                canvas.set(x, y, colours.water(self.depth(x, y)));
            }
        }

        // ripples are faint rings of lighter water that widen and fade
        let life = self.ripple_life();
        for ripple in &self.ripples {
            let k = (ripple.age / life).clamp(0.0, 1.0);
            let radius = 2.5 + (RIPPLE_REACH - 2.5) * (1.0 - (1.0 - k).powi(2));
            let strength = 1.0 - k;
            let reach = radius.ceil() as i32 + 1;
            let (cx, cy) = (ripple.x, ripple.y);
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    let (x, y) = (cx.round() as i32 + dx, cy.round() as i32 + dy);
                    let (ox, oy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                    let d = (ox * ox + oy * oy).sqrt();
                    let inner = radius * 0.55;
                    let lift = if (d - radius).abs() < 0.5 {
                        strength
                    } else if radius > 3.0 && (d - inner).abs() < 0.5 {
                        strength * 0.5
                    } else {
                        continue;
                    };
                    if in_water(y) && lift > 0.15 {
                        canvas.set(x, y, colours.ripple(self.depth(x, y), lift));
                    }
                }
            }
        }

        let bodies: Vec<Vec<(i32, i32, Part)>> =
            self.koi.iter().map(|koi| self.koi_pixels(koi)).collect();
        // each koi's shadow lies on the bed right under its body, a pixel down and to the right,
        // so it shows only as a soft dark edge; the thin fins cast none, which would leave
        // stray dark pixels beside them
        for pixels in &bodies {
            for &(x, y, part) in pixels {
                if matches!(part, Part::Fin { .. }) {
                    continue;
                }
                let (sx, sy) = (x + 1, y + 1);
                if in_water(sy) {
                    canvas.set(sx, sy, colours.shadow(self.depth(sx, sy)));
                }
            }
        }
        for pixels in &bodies {
            for &(x, y, part) in pixels {
                let colour = match part {
                    Part::Fin { ink, edge } => {
                        let tone = colours.tone(ink);
                        if edge { tone.fin_edge } else { tone.fin }
                    }
                    Part::Eye => colours.eye,
                    Part::Body { ink, rim, sheen } => {
                        let tone = colours.tone(ink);
                        if rim {
                            tone.rim
                        } else if sheen {
                            tone.sheen
                        } else {
                            tone.body
                        }
                    }
                };
                canvas.set(x, y, colour);
            }
        }

        // the pads rest on the surface, so a koi slips underneath them
        let drift = cycle(self.t, 0.05 * self.motion.pace());
        for pad in &self.pads {
            // the centre sits on a pixel centre, so every pad is a symmetrical disc
            let cx = (pad.x + 0.6 * (drift + pad.phase).sin()).floor() + 0.5;
            let cy = (pad.y + 0.4 * (drift * 0.7 + pad.phase).cos()).floor() + 0.5;
            let reach = pad.radius.ceil() as i32 + 1;
            let edge = pad.radius;
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    let (px, py) = (cx.floor() as i32 + dx, cy.floor() as i32 + dy);
                    let (ox, oy) = (px as f32 + 0.5 - cx, py as f32 + 0.5 - cy);
                    let d = (ox * ox + oy * oy).sqrt();
                    if d > edge {
                        continue;
                    }
                    // the notch is a slice cut from the edge in to the middle
                    let angle = oy.atan2(ox);
                    let off = (angle - pad.notch + PI).rem_euclid(TAU) - PI;
                    if off.abs() < 0.3 && d > 0.6 {
                        continue;
                    }
                    // the rim is shaded on the side away from the light and a soft highlight
                    // sits inside the lit side, so the pad reads as a flat leaf, not a blob
                    let shade = ox + oy;
                    let colour = if d > edge - 1.0 && shade > 0.5 {
                        colours.pad_dark
                    } else if d > edge - 2.0 && d <= edge - 1.0 && shade < -pad.radius * 0.9 {
                        colours.pad_light
                    } else {
                        colours.pad
                    };
                    canvas.set(px, py, colour);
                }
            }
            if pad.flower {
                // a small water lily off the middle of the pad, away from its notch
                let (fx, fy) = (
                    (cx - pad.notch.cos() * pad.radius * 0.35).floor() as i32,
                    (cy - pad.notch.sin() * pad.radius * 0.35).floor() as i32,
                );
                for (dx, dy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    canvas.set(fx + dx, fy + dy, colours.petal_dark);
                }
                for (dx, dy) in [(0, -1), (-1, 0), (1, 0), (0, 1)] {
                    canvas.set(fx + dx, fy + dy, colours.petal);
                }
                canvas.set(fx, fy, colours.centre);
            }
        }

        // the grassy bank along the top, its edge gently uneven with a shaded fringe over the
        // water
        if bank > 0.0 {
            for x in 0..wide {
                let xf = x as f32;
                let wave = 0.7 * (xf * 0.21 + 1.3).sin() + 0.5 * (xf * 0.083).sin();
                let edge = (bank + wave.round()).max(1.0) as i32;
                for y in 0..edge {
                    let colour = if y == edge - 1 {
                        colours.grass_dark
                    } else if unit(x, y, 7) < 0.12 {
                        colours.grass_light
                    } else {
                        colours.grass
                    };
                    canvas.set(x, y, colour);
                }
            }
        }

        // the rim of rounded stones along the bottom, lit from the top left
        if rim > 0.0 {
            let floor = tall as f32;
            for y in (floor - rim) as i32..tall {
                for x in 0..wide {
                    canvas.set(x, y, colours.soil);
                }
            }
            for &(cx, w, h) in &self.stones {
                let cy = floor - h / 2.0 + 0.5;
                let (rx, ry) = (w / 2.0, h / 2.0);
                for py in (cy - ry).floor() as i32..=(cy + ry).ceil() as i32 {
                    for px in (cx - rx).floor() as i32..=(cx + rx).ceil() as i32 {
                        let ox = (px as f32 + 0.5 - cx) / rx;
                        let oy = (py as f32 + 0.5 - cy) / ry;
                        let d = ox * ox + oy * oy;
                        if d > 1.0 {
                            continue;
                        }
                        let colour = if oy < -0.3 && ox < 0.35 && d < 0.7 {
                            colours.stone_light
                        } else if d > 0.72 && ox + oy > -0.2 {
                            colours.stone_dark
                        } else {
                            colours.stone
                        };
                        canvas.set(px, py, colour);
                    }
                }
            }
        }
    }

    /// Turns and moves one koi: it wanders in slow curves and turns away from the banks ahead
    /// of it and from any koi close by. It never turns faster than `TURN_LIMIT`.
    fn swim(&mut self, i: usize, dt: f32) {
        let speed_factor = self.motion.speed();
        let length = self.koi_length();
        let (x0, x1, y0, y1) = self.bounds();
        let bodies: Vec<Segment> = self.koi.iter().map(|k| self.span(k, 0.0)).collect();
        let t = self.t as f32;
        let koi = &self.koi[i];
        let (dx, dy) = (koi.heading.cos(), koi.heading.sin());
        let seed = (koi.seed % 1000) as f32;
        let wander = 0.22 * (t * 0.11 + seed).sin() + 0.14 * (t * 0.047 + seed * 1.7).sin();

        // a point ahead of the snout: if it is past a bank, the koi turns back towards the water
        let look = length * 1.4;
        let (ax, ay) = (koi.x + dx * look, koi.y + dy * look);
        let mut push = (0.0_f32, 0.0_f32);
        if ax < x0 {
            push.0 += x0 - ax;
        }
        if ax > x1 {
            push.0 -= ax - x1;
        }
        if ay < y0 {
            push.1 += y0 - ay;
        }
        if ay > y1 {
            push.1 -= ay - y1;
        }
        push.0 *= 2.0 / look;
        push.1 *= 2.0 / look;
        // how close this koi or the water just ahead of it comes to another koi: it turns away
        // harder the closer it is
        let ahead = self.span(koi, 0.8);
        let room = length * 0.85;
        let mut nudge = (0.0_f32, 0.0_f32);
        for (j, &other) in bodies.iter().enumerate() {
            if j == i {
                continue;
            }
            let (d, (ux, uy)) = gap(ahead, other);
            if d < room {
                let k = (room - d) / room * 3.0;
                push.0 += ux * k;
                push.1 += uy * k;
            }
            // two koi already too close also drift apart, so even a pair caught turning the
            // same way never settle into one mass
            let (d, (ux, uy)) = gap(bodies[i], other);
            let close = room - d;
            if close > 0.0 {
                nudge.0 += ux * close * 1.2;
                nudge.1 += uy * close * 1.2;
            }
        }
        // koi steer round the pads rather than under them, so they stay in view
        for pad in &self.pads {
            for k in [0.0, 0.5, 1.0] {
                let (px, py) = (
                    ahead.0.0 + (ahead.1.0 - ahead.0.0) * k,
                    ahead.0.1 + (ahead.1.1 - ahead.0.1) * k,
                );
                let (ox, oy) = (px - pad.x, py - pad.y);
                let d = (ox * ox + oy * oy).sqrt().max(0.1);
                let room = pad.radius + 4.0;
                if d < room {
                    let k = (room - d) / room * 1.4;
                    push.0 += ox / d * k;
                    push.1 += oy / d * k;
                }
            }
        }
        // a koi whose snout has passed the edge of its water is drawn firmly back towards the
        // middle, so it can never get stuck against a bank or in a corner
        if koi.x <= x0 || koi.x >= x1 || koi.y <= y0 || koi.y >= y1 {
            let (mx, my) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
            let (cx, cy) = (mx - koi.x, my - koi.y);
            let d = (cx * cx + cy * cy).sqrt().max(0.1);
            push.0 += cx / d * 2.5;
            push.1 += cy / d * 2.5;
        }
        let mut target = wander;
        if push.0 != 0.0 || push.1 != 0.0 {
            let want = (dy + push.1 * 2.0).atan2(dx + push.0 * 2.0);
            let off = (want - koi.heading + PI).rem_euclid(TAU) - PI;
            target += off * 0.9;
        }
        let target = target.clamp(-TURN_LIMIT, TURN_LIMIT);
        let outside = koi.x <= x0 || koi.x >= x1 || koi.y <= y0 || koi.y >= y1;
        let koi = &mut self.koi[i];
        let ease = if outside { 2.0 } else { 1.3 };
        koi.turn += (target - koi.turn) * (dt * ease).min(1.0);
        koi.heading = (koi.heading + koi.turn * speed_factor * dt).rem_euclid(TAU);
        let pace = 0.85 + 0.15 * (t * 0.09 + seed * 0.3).sin();
        let v = koi.speed * pace * speed_factor;
        let drift = (nudge.0 * speed_factor * dt, nudge.1 * speed_factor * dt);
        // the steering keeps the snout inside its bounds; the hard limit at the sides is a little
        // wider so a koi being parted from another there still has room to move, while at the
        // banks it is tighter so no head slips under the grass or the stones
        let slack = length * 0.25;
        koi.x = (koi.x + koi.heading.cos() * v * dt + drift.0).clamp(x0 - slack, x1 + slack);
        koi.y = (koi.y + koi.heading.sin() * v * dt + drift.1)
            .clamp(y0 - slack * 0.5, y1 + slack * 0.5);
    }
}

impl Scene for KoiPond {
    fn name(&self) -> &'static str {
        "koi-pond"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let (top, bottom) = self.water();
        let room = bottom - top;

        self.koi.clear();
        if self.width >= 10 && room >= 6.0 {
            // six koi at 120 by 34, growing more slowly than the area so a small pond still
            // has a few and a large one is not crowded
            let area = (self.width * self.height) as f32 / 4080.0;
            let wanted = self
                .motion
                .scaled(((6.0 * area.powf(0.6)).round() as usize).clamp(1, 16));
            // the varieties take turns from a random start, so even a small pond is mixed
            let first = rng.below(Coat::ALL.len());
            for n in 0..wanted {
                let coat = Coat::ALL[(first + n * 2) % Coat::ALL.len()];
                // of a few tries, the koi starts where it is furthest from all the others
                let mut best: Option<(f32, Koi)> = None;
                for _ in 0..24 {
                    let koi = self.spawn_koi(rng, coat);
                    let mine = self.span(&koi, 0.0);
                    let gap = self
                        .koi
                        .iter()
                        .map(|k| gap(mine, self.span(k, 0.0)).0)
                        .fold(f32::MAX, f32::min);
                    if best.as_ref().is_none_or(|(g, _)| gap > *g) {
                        best = Some((gap, koi));
                    }
                }
                if let Some((_, koi)) = best {
                    self.koi.push(koi);
                }
            }
        }

        self.pads.clear();
        let pad_count = count_for(self.width, self.height, 800, 1, 8);
        for _ in 0..pad_count * 6 {
            if self.pads.len() >= pad_count {
                break;
            }
            let large = self.width >= 30 && room >= 14.0 && rng.chance(0.6);
            let radius = if large {
                // radii that make clean round discs on the pixel grid, with no single pixel
                // sticking out at the top, bottom or sides
                if self.width >= 100 {
                    [4.3, 5.2][rng.below(2)]
                } else {
                    [3.3, 4.3][rng.below(2)]
                }
            } else {
                3.3
            };
            if (self.width as f32) < 2.0 * radius + 4.0 || room < 2.0 * radius + 2.0 {
                break;
            }
            let x = rng.range(radius + 1.0, self.width as f32 - radius - 1.0);
            // pads gather along the banks, some tucked partly under the edge, leaving the open
            // water to the koi
            // the grass edge is nearly straight, so a pad can tuck under it; the stones are
            // ragged, so a pad by them stays clear of them
            let y = if rng.chance(0.5) {
                top + rng.range(0.6, 1.2) * radius
            } else {
                bottom - rng.range(1.0, 1.3) * radius - 1.0
            };
            let clash = self.pads.iter().any(|p| {
                let (dx, dy) = (p.x - x, p.y - y);
                (dx * dx + dy * dy).sqrt() < p.radius + radius + 4.0
            });
            if !clash {
                self.pads.push(Pad {
                    x,
                    y,
                    radius,
                    notch: rng.range(0.0, TAU),
                    flower: large && rng.chance(0.5),
                    phase: rng.range(0.0, TAU),
                });
            }
        }
        self.ripples.clear();

        let (_, rim) = self.banks();
        self.stones.clear();
        if rim > 0.0 {
            let mut x = rng.range(-3.0, 0.0);
            while x < self.width as f32 + 3.0 {
                let w = rng.range(5.0, 9.0);
                let h = rim + rng.range(0.5, 2.5);
                self.stones.push((x + w / 2.0, w, h));
                x += w + rng.range(0.5, 1.5);
            }
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        for i in 0..self.koi.len() {
            self.swim(i, dt);
        }

        let life = self.ripple_life();
        for ripple in &mut self.ripples {
            ripple.age += dt;
        }
        self.ripples.retain(|r| r.age < life);
        self.next_ripple -= dt;
        if self.next_ripple <= 0.0 {
            self.next_ripple = if self.motion.calm {
                rng.range(8.0, 14.0)
            } else {
                rng.range(4.0, 8.0) / self.motion.busy()
            };
            let (top, bottom) = self.water();
            let width = self.width as f32;
            if self.ripples.len() < 3 && self.width >= 16 && bottom - top >= 12.0 {
                self.ripples.push(Ripple {
                    x: rng.range(4.0, width - 4.0),
                    y: rng.range(top + 4.0, bottom - 4.0),
                    age: 0.0,
                });
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = PondColours::new(theme);
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
        apart, check_determinism, check_pixel_scene, check_scene, pixels, render,
    };
    use crate::terminal::pixel_256;
    use crate::theme;

    fn drawn(pond: &KoiPond, width: usize, height: usize, theme: &ThemeData) -> Frame {
        let mut frame = Frame::new(width, height);
        pond.draw(&mut frame, theme);
        frame
    }

    /// A pond at 80 by 24 with one koi of a coat, its snout at a point and facing a way.
    fn one_koi(coat: Coat, heading: f32) -> (KoiPond, Koi) {
        let mut rng = Rng::new(2);
        let mut pond = KoiPond::new(80, 24, Motion::FULL, &mut rng);
        let mut koi = pond.spawn_koi(&mut rng, coat);
        koi.x = 40.0;
        koi.y = 24.0;
        koi.heading = heading;
        koi.turn = 0.0;
        koi.scale = 1.0;
        pond.koi = vec![koi.clone()];
        (pond, koi)
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("koi-pond");
        check_determinism("koi-pond");
        check_pixel_scene("koi-pond", 6.0);
    }

    #[test]
    fn tails_and_ripples_move_slowly_and_more_slowly_when_calm() {
        const { assert!(TAIL_BEAT >= 2.0) };
        const { assert!(RIPPLE_LIFE >= 2.0) };
        for motion in Motion::ALL {
            let mut rng = Rng::new(1);
            let pond = KoiPond::new(60, 20, motion, &mut rng);
            assert!(pond.tail_beat() >= TAIL_BEAT);
            assert!(pond.ripple_life() >= RIPPLE_LIFE);
            if motion.calm {
                assert!(pond.tail_beat() >= 2.0 * TAIL_BEAT);
            }
        }
    }

    #[test]
    fn koi_stay_in_the_water_and_the_pond_is_drawn() {
        let mut rng = Rng::new(4);
        let mut pond = KoiPond::new(60, 20, Motion::LIVELY, &mut rng);
        let (top, bottom) = pond.water();
        for _ in 0..(120 * 20) {
            pond.step(0.05, &mut rng);
            for koi in &pond.koi {
                assert!(koi.y >= top && koi.y <= bottom, "{}", koi.y);
                assert!(koi.x >= 0.0 && koi.x <= 60.0, "{}", koi.x);
            }
        }
        let theme = theme::default_theme();
        let c = PondColours::new(theme);
        let frame = render(&mut pond, 60, 20, 1);
        let all = pixels(&frame);
        assert!(all.contains(&c.pad), "no pad");
        assert!(c.inks.iter().any(|t| all.contains(&t.rim)), "no koi");
        assert!(all.contains(&c.stone), "no rim");
        assert!(all.contains(&c.grass), "no bank");
        let small = render(&mut KoiPond::new(20, 8, Motion::CALM, &mut rng), 20, 8, 20);
        assert!(small.drawn() > 10);
    }

    #[test]
    fn there_are_five_to_seven_koi_at_120_by_34() {
        let mut rng = Rng::new(3);
        let pond = KoiPond::new(120, 34, Motion::FULL, &mut rng);
        assert!((5..=7).contains(&pond.koi.len()), "{}", pond.koi.len());
        let calm = KoiPond::new(120, 34, Motion::GENTLE, &mut rng);
        assert!(calm.koi.len() <= pond.koi.len());
    }

    #[test]
    fn koi_keep_apart() {
        for (w, h, motion) in [(120, 34, Motion::FULL), (80, 24, Motion::FULL)] {
            let mut rng = Rng::new(1);
            let mut pond = KoiPond::new(w, h, motion, &mut rng);
            let mut closest = f32::MAX;
            for _ in 0..(120 * 20) {
                pond.step(0.05, &mut rng);
                let spans: Vec<Segment> = pond.koi.iter().map(|k| pond.span(k, 0.0)).collect();
                for i in 0..spans.len() {
                    for j in i + 1..spans.len() {
                        closest = closest.min(gap(spans[i], spans[j]).0);
                    }
                }
            }
            // the spines of two koi never cross and rarely come within a body's width
            assert!(closest > 2.0, "{w}x{h}: {closest:.2}");
        }
    }

    #[test]
    fn a_koi_has_a_clean_body_fins_a_tail_and_eyes() {
        let (pond, koi) = one_koi(Coat::Kohaku, 0.0);
        let parts = pond.koi_pixels(&koi);
        let body = parts
            .iter()
            .filter(|p| matches!(p.2, Part::Body { .. }))
            .count();
        let fins = parts
            .iter()
            .filter(|p| matches!(p.2, Part::Fin { .. }))
            .count();
        let eyes = parts.iter().filter(|p| p.2 == Part::Eye).count();
        assert!(body >= 60, "{body}");
        assert!(fins >= 12, "{fins}");
        assert_eq!(eyes, 2, "{eyes} eyes");
        // the fins and tail sit either side of and behind the body: some fin pixels are behind
        // the last body pixel and some are beside the body
        let tail_end = parts
            .iter()
            .filter(|p| matches!(p.2, Part::Body { .. }))
            .map(|p| p.0)
            .min()
            .expect("body");
        assert!(
            parts
                .iter()
                .any(|p| matches!(p.2, Part::Fin { .. }) && p.0 < tail_end)
        );
        // a solid silhouette: no pinholes and no pixel hanging on by a corner
        let covered: std::collections::HashSet<(i32, i32)> =
            parts.iter().map(|p| (p.0, p.1)).collect();
        let near = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        let (x0, x1) = (
            parts.iter().map(|p| p.0).min(),
            parts.iter().map(|p| p.0).max(),
        );
        let (y0, y1) = (
            parts.iter().map(|p| p.1).min(),
            parts.iter().map(|p| p.1).max(),
        );
        for y in y0.unwrap()..=y1.unwrap() {
            for x in x0.unwrap()..=x1.unwrap() {
                let around = near
                    .iter()
                    .filter(|(dx, dy)| covered.contains(&(x + dx, y + dy)))
                    .count();
                if covered.contains(&(x, y)) {
                    assert!(around >= 2, "a stray pixel at {x},{y}");
                } else {
                    assert!(around <= 2, "a pinhole at {x},{y}");
                }
            }
        }
    }

    #[test]
    fn a_koi_looks_the_same_size_facing_any_way() {
        let counts: Vec<usize> = (0..16)
            .map(|i| {
                let (pond, koi) = one_koi(Coat::Ogon, i as f32 * TAU / 16.0);
                pond.koi_pixels(&koi).len()
            })
            .collect();
        let mean = counts.iter().sum::<usize>() as f32 / counts.len() as f32;
        for count in counts {
            assert!(
                (count as f32 - mean).abs() < mean * 0.15,
                "{count} against {mean}"
            );
        }
    }

    #[test]
    fn each_variety_carries_its_markings() {
        let inks = |coat: Coat| -> Vec<Ink> {
            let (pond, koi) = one_koi(coat, 0.4);
            pond.koi_pixels(&koi)
                .into_iter()
                .filter_map(|p| match p.2 {
                    Part::Body { ink, .. } => Some(ink),
                    _ => None,
                })
                .collect()
        };
        let kohaku = inks(Coat::Kohaku);
        assert!(kohaku.contains(&Ink::White) && kohaku.contains(&Ink::Red));
        let sanke = inks(Coat::Sanke);
        assert!(sanke.contains(&Ink::Black) && sanke.contains(&Ink::Red));
        let showa = inks(Coat::Showa);
        assert!(showa.contains(&Ink::Black) && showa.contains(&Ink::White));
        assert!(inks(Coat::Yamabuki).iter().all(|&i| i == Ink::Gold));
        assert!(inks(Coat::Ogon).iter().all(|&i| i == Ink::Pale));
    }

    #[test]
    fn the_colours_stay_apart_and_in_their_bands_in_every_theme() {
        for theme in theme::all() {
            let c = PondColours::new(theme);
            let name = theme.name;
            apart(theme, c.shallow, c.deep, 0.03, "the shallow and deep water");
            apart(theme, c.grass, c.shallow, 0.03, "the bank and the water");
            apart(theme, c.stone, c.deep, 0.03, "the stones and the water");
            apart(
                theme,
                c.stone_light,
                c.stone_dark,
                0.03,
                "the lit and shaded stone",
            );
            assert!(c.deep.luminance() < c.shallow.luminance(), "{name}");
            let mut imagery = vec![
                ("shallow", c.shallow),
                ("deep", c.deep),
                ("grass", c.grass),
                ("stone", c.stone),
                ("soil", c.soil),
            ];
            imagery.extend(c.shadow.iter().map(|&s| ("shadow", s)));
            for (what, colour) in imagery {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            // every koi keeps its outline with colour off; the pads keep theirs too
            for colour in c.inks.iter().map(|t| t.rim).chain([c.pad, c.pad_dark]) {
                assert!(
                    theme.contrast_of(colour) >= IMAGERY_CEILING,
                    "{name}: {:.2}:1",
                    theme.contrast_of(colour)
                );
                apart(theme, colour, c.shallow, 0.08, "a shape and the water");
                apart(theme, colour, c.deep, 0.08, "a shape and the deep water");
            }
            let tone = |ink| c.tone(ink).body;
            apart(
                theme,
                tone(Ink::White),
                tone(Ink::Red),
                0.08,
                "white and red",
            );
            apart(
                theme,
                tone(Ink::Black),
                tone(Ink::Red),
                0.08,
                "black and red",
            );
            apart(
                theme,
                tone(Ink::White),
                tone(Ink::Black),
                0.08,
                "white and black",
            );
            // a black koi on a dark theme is as dark as deep water, as it is in a real pond, and
            // reads by its lighter rim, which is checked above
            for ink in INKS {
                if !(theme.dark && ink == Ink::Black) {
                    apart(theme, tone(ink), c.deep, 0.05, "a koi and the water");
                }
            }
        }
    }

    #[test]
    fn the_256_colour_fallback_keeps_the_bands_apart() {
        for theme in theme::all() {
            let c = PondColours::new(theme);
            let near = |colour: Rgb| xterm_colour(pixel_256(colour, theme.background));
            let mut rng = Rng::new(8);
            let mut pond = KoiPond::new(80, 24, Motion::FULL, &mut rng);
            for _ in 0..60 {
                pond.step(0.05, &mut rng);
            }
            let frame = drawn(&pond, 80, 24, theme);
            let worst = pixels(&frame)
                .into_iter()
                .map(|colour| distance(colour, near(colour)))
                .fold(0.0_f32, f32::max);
            assert!(worst <= 0.15, "{} is {worst:.3} off", theme.name);
            assert_ne!(near(c.shallow), near(c.deep), "{}", theme.name);
            assert_ne!(near(c.grass), near(c.deep), "{}", theme.name);
            assert_ne!(near(c.pad), near(c.shallow), "{}", theme.name);
        }
    }
}
