//! A lily pond in solid pixels, seen from above: still water holding soft drifting reflections
//! of the sky, round lily pads with a notch and fine veins, each casting a faint shadow on the
//! water. Water lilies open on a few of them. Frogs sit on the pads and now and then hop to
//! another, their shadow sliding across the water beneath them and a ring of ripples spreading
//! where they leave and where they land. Dragonflies dart between hovers over the water.
//!
//! The water, its reflections, the shadows and the ripples are tints of the background held below
//! the non-text floor; the pads, the flowers, the frogs and the dragonflies keep the theme's own
//! drawing colours, so they stand out as shapes and keep their outline when colour is off.

use std::f32::consts::{PI, TAU};

use super::cycle;
use super::paint::{noise, solid, strong, stronger, tint};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "A pond of lily pads from above, frogs hopping between them and dragonflies darting about";
pub const CAPTIONS: &[&str] = &[
    "A calm pond and a clean build.",
    "Every frog on its pad.",
    "Nothing to fix. Hop to the next thing.",
];

/// The shortest time between two hops at the standard pace.
pub const HOP_GAP: f32 = 6.0;
/// How long a hop takes.
const HOP_SECONDS: f32 = 0.9;
/// How long a ring of ripples takes to spread and fade.
const RIPPLE_SECONDS: f32 = 2.4;
/// How far the reflections drift in a second at the standard pace, in noise cells.
const LIGHT_DRIFT: f32 = 0.05;
/// The smallest pad the large frog sits on.
const FROG_PAD: f32 = 5.0;

/// A frog seen from above, facing up: eyes on top of its head, front legs reaching forward and
/// long back legs folded at its sides. `G` body, `g` its lighter back, `o` an eye, `f` and `b`
/// the legs.
const FROG: [&str; 9] = [
    "..GG.GG..",
    "..GoGoG..",
    "f.GGGGG.f",
    "ffGgggGff",
    "..GgggG..",
    "..GgggG..",
    "bbGGgGGbb",
    "b.GGGGG.b",
    "bb.....bb",
];
/// A small frog for a small pond.
const FROG_SMALL: [&str; 5] = [".o.o.", "fGGGf", ".GgG.", "bGGGb", "b...b"];
/// A water lily from above: pointed outer petals, paler inner ones and a gold heart.
const LILY: [&str; 7] = [
    "..p.p..", ".ppPpp.", "ppPcPpp", ".PcccP.", "ppPcPpp", ".ppPpp.", "..p.p..",
];
const LILY_SMALL: [&str; 3] = [".p.", "pcp", ".p."];

/// A colour's own hue at a pale, clear lightness.
fn lifted(colour: Rgb) -> Rgb {
    let lab = crate::pixels::Lab::from_rgb(colour);
    crate::pixels::Lab { l: 0.8, ..lab }.to_rgb()
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct Colours {
    pub deep: Rgb,
    pub shallow: Rgb,
    pub light: Rgb,
    pub glint: Rgb,
    pub shadow: Rgb,
    pub ripple: Rgb,
    pub pad: Rgb,
    pub pad_lit: Rgb,
    pub vein: Rgb,
    pub petal: Rgb,
    pub petal_inner: Rgb,
    pub heart: Rgb,
    pub frog: Rgb,
    pub frog_rim: Rgb,
    pub frog_back: Rgb,
    pub frog_legs: Rgb,
    pub eye: Rgb,
    pub dragonfly: Rgb,
    pub wing: Rgb,
    water: Ramp,
}

impl Colours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let dark = theme.dark;
        let shades = Shades::new(theme);
        let (deep, shallow, light, shadow) = if dark {
            (
                tint(theme, theme.blue(), 0.18, 1.25),
                tint(theme, theme.cyan(), 0.32, 1.6),
                tint(theme, theme.bright_cyan(), 0.5, 2.05),
                tint(theme, theme.blue(), 0.06, 1.07),
            )
        } else {
            (
                tint(theme, theme.blue(), 0.55, 2.2),
                tint(theme, theme.cyan(), 0.32, 1.6),
                tint(theme, theme.info, 0.16, 1.22),
                tint(theme, theme.blue(), 0.8, 2.75),
            )
        };
        let pad = theme.green();
        // on a light theme every shape is dark, so a frog there is the theme's amber rather
        // than a green that would sink into its pad, with a pale back and a pale rim
        let (frog, petal) = if dark {
            (
                strong(theme, mix(theme.green(), theme.yellow(), 0.85)),
                stronger(theme, theme.magenta(), 0.35),
            )
        } else {
            (theme.yellow(), theme.magenta())
        };
        Self {
            deep,
            shallow,
            light,
            glint: mix(light, shallow, 0.5),
            shadow,
            ripple: if dark {
                tint(theme, theme.foreground, 0.55, 2.6)
            } else {
                tint(theme, theme.info, 0.03, 1.05)
            },
            pad,
            pad_lit: stronger(theme, theme.bright_green(), 0.2),
            vein: solid(theme, pad, shades.darkest(), 0.35),
            petal,
            petal_inner: if dark {
                stronger(theme, petal, 0.4)
            } else {
                strong(theme, lifted(petal))
            },
            heart: theme.yellow(),
            frog,
            frog_back: if dark {
                stronger(theme, frog, 0.3)
            } else {
                strong(theme, lifted(frog))
            },
            frog_rim: if dark {
                // a deep shadow ring, below the outline floor, so the frog stands clear of its
                // pad with colour off as well as on
                mix(pad, shades.darkest(), 0.7)
            } else {
                light
            },
            frog_legs: solid(theme, frog, shades.darkest(), 0.3),
            eye: shades.darkest(),
            dragonfly: theme.bright_blue(),
            wing: if dark {
                tint(theme, theme.foreground, 0.75, 2.9)
            } else {
                tint(theme, theme.info, 0.02, 1.03)
            },
            water: Ramp::new(deep, shallow, 6),
        }
    }

    #[cfg(test)]
    pub(crate) fn palette(theme: &ThemeData) -> crate::scenes::testing::Palette {
        let c = Self::new(theme);
        crate::scenes::testing::Palette {
            imagery: vec![
                ("deep water", c.deep),
                ("shallow water", c.shallow),
                ("light", c.light),
                ("ripple", c.ripple),
            ],
            shapes: vec![
                ("pad", c.pad),
                ("petal", c.petal),
                ("flower heart", c.heart),
                ("frog", c.frog),
                ("dragonfly", c.dragonfly),
            ],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Pad {
    x: f32,
    y: f32,
    r: f32,
    /// Which way the notch points.
    notch: f32,
    flower: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct Frog {
    pad: usize,
    /// A hop under way: the pad it left and how far through it is.
    hop: Option<(usize, f32)>,
}

#[derive(Clone, Debug, PartialEq)]
struct Ripple {
    x: f32,
    y: f32,
    age: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct Dragonfly {
    x: f32,
    y: f32,
    /// Where it is darting to; it hovers until `rest` runs out.
    target: (f32, f32),
    rest: f32,
    heading: f32,
}

pub struct LilyPads {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    pads: Vec<Pad>,
    frogs: Vec<Frog>,
    ripples: Vec<Ripple>,
    flies: Vec<Dragonfly>,
    next_hop: f32,
    hops: u32,
}

impl LilyPads {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut pond = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            pads: Vec::new(),
            frogs: Vec::new(),
            ripples: Vec::new(),
            flies: Vec::new(),
            next_hop: HOP_GAP * 0.5,
            hops: 0,
        };
        pond.resize(width, height, rng);
        pond
    }

    fn tall(&self) -> f32 {
        self.height as f32 * 2.0
    }

    /// Where a frog is and how high it is off the water: on its pad or partway through a hop
    /// along a low arc.
    fn frog_at(&self, frog: &Frog) -> (f32, f32, f32) {
        let pad = &self.pads[frog.pad];
        match frog.hop {
            Some((from, done)) => {
                let start = &self.pads[from];
                let eased = done * done * (3.0 - 2.0 * done);
                let lift = (done * PI).sin() * 3.0;
                (
                    start.x + (pad.x - start.x) * eased,
                    start.y + (pad.y - start.y) * eased - lift,
                    lift,
                )
            }
            None => (pad.x, pad.y, 0.0),
        }
    }

    fn new_target(&self, rng: &mut Rng) -> (f32, f32) {
        (
            rng.range(2.0, (self.width as f32 - 2.0).max(2.5)),
            rng.range(2.0, (self.tall() - 2.0).max(2.5)),
        )
    }

    fn big_frogs(&self) -> bool {
        self.pads.iter().any(|p| p.r >= FROG_PAD)
    }

    fn paint_water(&self, canvas: &mut Canvas, c: &Colours) {
        let (wide, tall) = (canvas.width() as i32, canvas.height() as i32);
        let drift = self.t as f32 * self.motion.speed() * LIGHT_DRIFT;
        for y in 0..tall {
            for x in 0..wide {
                let (u, v) = (x as f32 / wide as f32 - 0.5, y as f32 / tall as f32 - 0.5);
                // the pond is deepest in the middle and shallows towards its edges
                let edge = ((u * u + v * v).sqrt() * 1.5).min(1.0);
                // long soft streaks of reflected sky, drifting slowly
                let n = noise(x as f32 / 22.0 + drift, y as f32 / 7.0, 101);
                let level = edge * 0.65 + n * 0.5 - 0.1;
                // the brightest reflections are fringed by a softer step, never a hard edge
                let colour = if n > 0.8 {
                    c.light
                } else if n > 0.74 {
                    c.glint
                } else {
                    c.water.at(level)
                };
                canvas.set(x, y, colour);
            }
        }
    }

    fn paint_pad(canvas: &mut Canvas, c: &Colours, pad: &Pad, shadow: bool) {
        let (ox, oy) = if shadow { (1.0, 1.5) } else { (0.0, 0.0) };
        let (cx, cy) = (pad.x + ox, pad.y + oy);
        let reach = pad.r.ceil() as i32 + 1;
        for y in cy as i32 - reach..=cy as i32 + reach {
            for x in cx as i32 - reach..=cx as i32 + reach {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let d = (dx * dx + dy * dy).sqrt();
                if d > pad.r {
                    continue;
                }
                let angle = dy.atan2(dx);
                let off = ((angle - pad.notch + PI).rem_euclid(TAU) - PI).abs();
                // the notch: a narrow wedge cut in to the middle
                if off < 0.3 && d > 0.8 {
                    continue;
                }
                if shadow {
                    canvas.set(x, y, c.shadow);
                    continue;
                }
                let rim = d > pad.r - 1.3 && dx + dy < -pad.r * 0.3;
                // fine veins run out from the middle; the widest pads get them
                let vein = pad.r >= 6.5
                    && d > 1.0
                    && d < pad.r - 1.0
                    && ((angle - pad.notch).rem_euclid(TAU / 7.0) - TAU / 14.0).abs() * d < 0.5;
                let colour = if rim {
                    c.pad_lit
                } else if vein {
                    c.vein
                } else {
                    c.pad
                };
                canvas.set(x, y, colour);
            }
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        self.paint_water(canvas, c);
        for ripple in &self.ripples {
            let r = 1.5 + ripple.age * 2.6;
            if ripple.age < RIPPLE_SECONDS * 0.8 {
                let steps = (r * 8.0) as i32 + 8;
                for i in 0..steps {
                    let a = i as f32 / steps as f32 * TAU;
                    let (x, y) = (ripple.x + a.cos() * r * 1.2, ripple.y + a.sin() * r);
                    canvas.set(x.round() as i32, y.round() as i32, c.ripple);
                }
            }
        }
        for pad in &self.pads {
            Self::paint_pad(canvas, c, pad, true);
        }
        for pad in &self.pads {
            Self::paint_pad(canvas, c, pad, false);
            if pad.flower {
                let rows: &[&str] = if pad.r >= 4.5 { &LILY } else { &LILY_SMALL };
                let half = rows.len() as i32 / 2;
                // the flower sits off the middle, away from the notch
                let fx = (pad.x - pad.notch.cos() * pad.r * 0.3).round() as i32;
                let fy = (pad.y - pad.notch.sin() * pad.r * 0.3).round() as i32;
                canvas.sprite(
                    fx - half,
                    fy - half,
                    rows,
                    &[('p', c.petal), ('P', c.petal_inner), ('c', c.heart)],
                );
            }
        }

        let big = self.big_frogs();
        let rows: &[&str] = if big { &FROG } else { &FROG_SMALL };
        let (w, h) = (rows[0].len() as i32, rows.len() as i32);
        let palette = [
            ('G', c.frog),
            ('g', c.frog_back),
            ('o', c.eye),
            ('f', c.frog_legs),
            ('b', c.frog_legs),
        ];
        for frog in &self.frogs {
            let (x, y, lift) = self.frog_at(frog);
            let left = x.round() as i32 - w / 2;
            let top = y.round() as i32 - h / 2;
            if lift > 0.3 {
                // its shadow on the water stays below it while it is in the air
                let shadow = (y + lift).round() as i32 - h / 2 + 1;
                for (dy, row) in rows.iter().enumerate() {
                    for (dx, ch) in row.chars().enumerate() {
                        if ch != '.' {
                            canvas.set(left + dx as i32 + 1, shadow + dy as i32, c.shadow);
                        }
                    }
                }
            }
            // a dark rim round the frog keeps its outline clear against the pad
            for (dy, row) in rows.iter().enumerate() {
                for (dx, ch) in row.chars().enumerate() {
                    if ch == '.' {
                        continue;
                    }
                    for (ox, oy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        canvas.set(left + dx as i32 + ox, top + dy as i32 + oy, c.frog_rim);
                    }
                }
            }
            canvas.sprite(left, top, rows, &palette);
        }

        for fly in &self.flies {
            // the heading in eighths of a turn, so the body is always a clean line
            let eighth = ((fly.heading / (TAU / 8.0)).round() as i32).rem_euclid(8);
            let (hx, hy) = [
                (1, 0),
                (1, 1),
                (0, 1),
                (-1, 1),
                (-1, 0),
                (-1, -1),
                (0, -1),
                (1, -1),
            ][eighth as usize];
            let (nx, ny) = (-hy, hx);
            let (bx, by) = (fly.x.round() as i32, fly.y.round() as i32);
            // two pairs of see-through wings swept back from just behind the head, then the
            // long thin body with a wider head
            for along in [1, -1] {
                for side in [-1, 1] {
                    for k in 1..=3 {
                        let back = along - k / 2;
                        canvas.set(
                            bx + hx * back + nx * side * k,
                            by + hy * back + ny * side * k,
                            c.wing,
                        );
                    }
                }
            }
            for i in -5..=2 {
                canvas.set(bx + hx * i, by + hy * i, c.dragonfly);
            }
            canvas.set(bx + hx * 2 + nx, by + hy * 2 + ny, c.dragonfly);
            canvas.set(bx + hx * 2 - nx, by + hy * 2 - ny, c.dragonfly);
        }
    }
}

impl Scene for LilyPads {
    fn name(&self) -> &'static str {
        "lily-pads"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let (w, tall) = (self.width as f32, self.tall());
        self.pads.clear();
        let base = (tall / 8.0).clamp(1.8, 11.0);
        let wanted = ((w * tall) / (base * base * 9.0)) as usize;
        let wanted = wanted.clamp(3, 40);
        for _ in 0..wanted * 8 {
            if self.pads.len() >= wanted {
                break;
            }
            let r = base * rng.range(0.7, 1.25);
            let (x, y) = (rng.range(0.0, w), rng.range(0.0, tall));
            // pads keep a little open water between them
            let clear = self.pads.iter().all(|p| {
                let (dx, dy) = (p.x - x, p.y - y);
                (dx * dx + dy * dy).sqrt() > p.r + r + 2.0
            });
            if clear {
                self.pads.push(Pad {
                    x,
                    y,
                    r,
                    notch: rng.range(0.0, TAU),
                    flower: false,
                });
            }
        }
        self.frogs.clear();
        let frogs = (self.pads.len() / 4).clamp(1, 5).min(self.pads.len());
        // frogs take the biggest pads and flowers open on some of the others
        let mut order: Vec<usize> = (0..self.pads.len()).collect();
        order.sort_by(|&a, &b| self.pads[b].r.total_cmp(&self.pads[a].r));
        for &pad in order.iter().take(frogs) {
            self.frogs.push(Frog { pad, hop: None });
        }
        for (k, &pad) in order.iter().skip(frogs).enumerate() {
            self.pads[pad].flower = k % 3 == 0;
        }
        self.ripples.clear();
        self.flies.clear();
        let flies = self.motion.scaled((self.width / 40).clamp(1, 4));
        for _ in 0..flies {
            let (x, y) = self.new_target(rng);
            let target = self.new_target(rng);
            self.flies.push(Dragonfly {
                x,
                y,
                target,
                rest: rng.range(0.0, 2.0),
                heading: 0.0,
            });
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let moved = dt * self.motion.speed();

        // a hop is an event in place, so lively never makes frogs hop more often than standard
        self.next_hop -= dt * self.motion.pace();
        if self.next_hop <= 0.0 {
            self.next_hop = HOP_GAP + rng.range(0.0, 5.0);
            let resting: Vec<usize> = (0..self.frogs.len())
                .filter(|&i| self.frogs[i].hop.is_none())
                .collect();
            if !resting.is_empty() && self.pads.len() > self.frogs.len() {
                let f = resting[rng.below(resting.len())];
                let taken: Vec<usize> = self.frogs.iter().map(|f| f.pad).collect();
                let here = &self.pads[self.frogs[f].pad];
                let least = if self.big_frogs() { FROG_PAD } else { 0.0 };
                // the nearest free pad without a flower that is big enough to sit on
                let next = (0..self.pads.len())
                    .filter(|i| {
                        !taken.contains(i) && !self.pads[*i].flower && self.pads[*i].r >= least
                    })
                    .min_by(|&a, &b| {
                        let d = |p: &Pad| (p.x - here.x).powi(2) + (p.y - here.y).powi(2);
                        d(&self.pads[a]).total_cmp(&d(&self.pads[b]))
                    });
                if let Some(next) = next {
                    let from = self.frogs[f].pad;
                    self.ripples.push(Ripple {
                        x: self.pads[from].x,
                        y: self.pads[from].y,
                        age: 0.0,
                    });
                    self.frogs[f] = Frog {
                        pad: next,
                        hop: Some((from, 0.0)),
                    };
                    self.hops += 1;
                }
            }
        }
        let mut landed = Vec::new();
        for frog in &mut self.frogs {
            if let Some((from, done)) = frog.hop {
                let done = done + moved / HOP_SECONDS;
                if done >= 1.0 {
                    frog.hop = None;
                    landed.push(frog.pad);
                } else {
                    frog.hop = Some((from, done));
                }
            }
        }
        for pad in landed {
            self.ripples.push(Ripple {
                x: self.pads[pad].x,
                y: self.pads[pad].y,
                age: 0.0,
            });
        }
        for ripple in &mut self.ripples {
            ripple.age += moved;
        }
        self.ripples.retain(|r| r.age < RIPPLE_SECONDS);

        for i in 0..self.flies.len() {
            let fly = &self.flies[i];
            let (dx, dy) = (fly.target.0 - fly.x, fly.target.1 - fly.y);
            let d = (dx * dx + dy * dy).sqrt();
            if fly.rest > 0.0 {
                // hovering: a slight drift on the spot
                let t = self.t;
                let fly = &mut self.flies[i];
                fly.rest -= moved;
                fly.y += (cycle(t, 1.5) + i as f32).sin() * 0.3 * moved;
            } else if d < 0.5 {
                let target = self.new_target(rng);
                let fly = &mut self.flies[i];
                fly.target = target;
                fly.rest = rng.range(1.5, 4.0);
            } else {
                // reduced motion darts more slowly still, so no cell it crosses changes more
                // often than at the standard pace
                let pace = if self.motion.calm { 8.0 } else { 14.0 };
                let step = (pace * moved).min(d);
                let fly = &mut self.flies[i];
                fly.x += dx / d * step;
                fly.y += dy / d * step;
                fly.heading = dy.atan2(dx);
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
    use crate::pixels::distance;
    use crate::scenes::testing::{check_determinism, check_scene};
    use crate::scenes::testing::{check_palette, check_pixel_scene};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("lily-pads");
        check_determinism("lily-pads");
    }

    #[test]
    fn passes_the_pixel_checks() {
        check_pixel_scene("lily-pads", 6.0);
        check_palette(Colours::palette);
    }

    #[test]
    fn the_pond_stays_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            for (a, b, what) in [
                (c.deep, c.shallow, "the deep and shallow water"),
                (c.light, c.shallow, "the light and the water"),
                (c.ripple, c.deep, "the ripples and the water"),
                (c.shadow, c.deep, "the shadows and the water"),
                (c.pad_lit, c.pad, "a pad and its rim"),
                (c.vein, c.pad, "a pad and its veins"),
            ] {
                assert!(distance(a, b) >= 0.03, "{what} in {}", theme.name);
            }
            for (a, what) in [(c.frog, "a frog"), (c.petal, "a lily")] {
                assert!(
                    distance(a, c.pad) >= 0.06,
                    "{what} on a pad in {}",
                    theme.name
                );
            }
        }
        for rows in [&FROG[..], &FROG_SMALL[..], &LILY[..], &LILY_SMALL[..]] {
            assert!(rows.iter().all(|r| r.len() == rows[0].len()));
        }
    }

    #[test]
    fn frogs_sit_on_pads_big_enough_for_them() {
        let mut rng = Rng::new(3);
        let pond = LilyPads::new(120, 34, Motion::FULL, &mut rng);
        assert!(pond.big_frogs());
        for frog in &pond.frogs {
            assert!(
                pond.pads[frog.pad].r >= FROG_PAD,
                "{}",
                pond.pads[frog.pad].r
            );
            assert!(!pond.pads[frog.pad].flower);
        }
    }

    #[test]
    fn frogs_hop_from_pad_to_pad_slowly_and_never_share() {
        for motion in Motion::ALL {
            let mut rng = Rng::new(7);
            let mut pond = LilyPads::new(80, 24, motion, &mut rng);
            let mut times = Vec::new();
            let mut last = pond.hops;
            for i in 0..(120 * 20) {
                pond.step(0.05, &mut rng);
                if pond.hops != last {
                    times.push(i as f32 * 0.05);
                    last = pond.hops;
                }
                let mut pads: Vec<usize> = pond.frogs.iter().map(|f| f.pad).collect();
                pads.sort_unstable();
                pads.dedup();
                assert_eq!(pads.len(), pond.frogs.len(), "two frogs on one pad");
            }
            assert!(times.len() >= 3, "{motion:?}: {}", times.len());
            let gap = HOP_GAP / motion.pace();
            for w in times.windows(2) {
                assert!(w[1] - w[0] >= gap - 0.06, "{motion:?}: {w:?}");
            }
        }
    }
}
