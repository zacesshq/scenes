//! A short circuit in solid pixels: a circuit board seen from above, with a square processor
//! with pins on all four sides, a smaller chip, a crystal, resistors, two capacitors, gold edge
//! fingers, mounting holes and copper traces routed with 45 degree bends. One trace has burnt
//! through: its ends glow, the copper blackens back along it and a scorch mark spreads across the
//! board round the break, with a soft glow, a few slow sparks and a thin curl of smoke rising
//! from it. The damage spreads for about a minute and then holds.
//!
//! The board, the copper, the chips and the scorch are tints of the background held below the
//! non-text floor. The pins, the printed outlines, the glowing break and the sparks keep the
//! theme's own drawing colours, so the board keeps its outline when colour is off. Sparks are a
//! pixel or two each, drift rather than flash and come no more often than `SPARK_PERIOD`.

use super::paint::{self, Blender, disc, noise, tint};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::error_colour;

pub const SUMMARY: &str = "A circuit board where one trace has burnt through, its copper blackening, a scorch spreading and slow sparks rising";
pub const CAPTIONS: &[&str] = &[
    "Short circuit. Trace it back to the first error.",
    "Something burnt out in the build.",
    "Sparks on the board. Check the failing step.",
];

/// The shortest time between two bursts of sparks at the standard pace.
pub const SPARK_PERIOD: f32 = 1.6;
/// How long a spark lives.
const SPARK_LIFE: f32 = 0.9;
/// How long the damage takes to spread back along the trace and the scorch to reach its size.
pub const FAULT_SECONDS: f32 = 50.0;
/// How far the damage has spread when the scene starts, so the fault shows from the start.
const FAULT_START: f32 = 12.0;
/// Seconds for one slow swell of the glow round the break. It is long and gentle, so the glow
/// breathes rather than flickers.
const GLOW_PERIOD: f32 = 5.0;

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct Colours {
    pub board: Ramp,
    pub trace: Rgb,
    pub copper: Rgb,
    pub hole: Rgb,
    pub chip: Rgb,
    pub bevel: Rgb,
    pub pins: Rgb,
    pub silk: Rgb,
    pub resistor: Rgb,
    pub band: Rgb,
    pub can: (Rgb, Rgb),
    pub stripe: Rgb,
    pub scorch: Ramp,
    pub char: Rgb,
    pub glow: Rgb,
    pub smoke: Rgb,
    pub fault: Rgb,
    pub spark: Rgb,
    pub ember: Rgb,
}

impl Colours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let dark = theme.dark;
        let mask = mix(theme.green(), theme.cyan(), 0.15);
        let gold = mix(theme.yellow(), theme.warning, 0.25);
        let soot = mix(theme.red(), theme.yellow(), 0.35);
        let (board_top, board_low, trace, copper, hole) = if dark {
            (
                tint(theme, mask, 0.3, 1.6),
                tint(theme, mask, 0.22, 1.42),
                tint(theme, mix(mask, theme.yellow(), 0.3), 0.5, 2.15),
                tint(theme, gold, 0.72, 2.75),
                tint(theme, theme.muted, 0.02, 1.03),
            )
        } else {
            (
                tint(theme, mask, 0.5, 2.15),
                tint(theme, mask, 0.6, 2.4),
                tint(theme, mix(mask, theme.yellow(), 0.3), 0.32, 1.65),
                tint(theme, gold, 0.6, 2.3),
                tint(theme, theme.info, 0.02, 1.03),
            )
        };
        // a chip is darker than the board in every theme: nearly black on a dark theme with a
        // lighter edge, the darkest solid on a light one
        let (chip, bevel, pins, silk) = if dark {
            (
                tint(theme, theme.muted, 0.05, 1.07),
                tint(theme, theme.muted, 0.28, 1.6),
                theme.muted,
                theme.foreground,
            )
        } else {
            (
                theme.foreground,
                paint::solid(theme, theme.foreground, theme.muted, 0.35),
                tint(theme, theme.muted, 0.45, 2.1),
                tint(theme, theme.info, 0.03, 1.05),
            )
        };
        let (scorch_edge, scorch_mid, char) = if dark {
            (
                tint(theme, soot, 0.36, 1.75),
                tint(theme, soot, 0.18, 1.32),
                tint(theme, theme.muted, 0.01, 1.01),
            )
        } else {
            (
                tint(theme, soot, 0.55, 2.25),
                tint(theme, soot, 0.78, 2.7),
                tint(theme, mix(soot, theme.foreground, 0.5), 0.95, 2.95),
            )
        };
        Self {
            board: Ramp::new(board_top, board_low, 4),
            trace,
            copper,
            hole,
            chip,
            bevel,
            pins,
            silk,
            resistor: if dark {
                tint(theme, mix(theme.info, theme.blue(), 0.5), 0.55, 2.3)
            } else {
                tint(theme, mix(theme.info, theme.blue(), 0.5), 0.3, 1.6)
            },
            band: tint(theme, theme.red(), 0.8, 2.6),
            can: if dark {
                (
                    tint(theme, theme.blue(), 0.42, 1.95),
                    tint(theme, theme.blue(), 0.25, 1.5),
                )
            } else {
                (
                    tint(theme, theme.blue(), 0.6, 2.4),
                    tint(theme, theme.blue(), 0.8, 2.85),
                )
            },
            stripe: tint(theme, theme.foreground, 0.4, 2.0),
            scorch: Ramp::new(scorch_edge, scorch_mid, 4),
            char,
            glow: tint(theme, theme.warning, if dark { 0.6 } else { 0.4 }, 2.6),
            smoke: if dark {
                tint(theme, theme.foreground, 0.35, 1.9)
            } else {
                tint(theme, theme.muted, 0.35, 1.7)
            },
            fault: error_colour(theme),
            spark: if dark {
                theme.bright_yellow()
            } else {
                paint::strong(theme, theme.warning)
            },
            ember: paint::strong(theme, mix(theme.warning, theme.red(), 0.4)),
        }
    }

    #[cfg(test)]
    pub(crate) fn palette(theme: &ThemeData) -> crate::scenes::testing::Palette {
        let c = Self::new(theme);
        let mut imagery = vec![
            ("board", c.board.at(0.0)),
            ("board low", c.board.at(1.0)),
            ("trace", c.trace),
            ("copper", c.copper),
            ("scorch", c.scorch.at(1.0)),
            ("char", c.char),
            ("smoke", c.smoke),
            ("glow", c.glow),
        ];
        let mut shapes = vec![("fault", c.fault)];
        if theme.dark {
            shapes.push(("pins", c.pins));
            shapes.push(("print", c.silk));
            shapes.push(("spark", c.spark));
            imagery.push(("chip", c.chip));
        } else {
            shapes.push(("chip", c.chip));
            imagery.push(("pins", c.pins));
            imagery.push(("print", c.silk));
        }
        crate::scenes::testing::Palette { imagery, shapes }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

#[derive(Clone, Debug, PartialEq)]
struct Spark {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    age: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct Wisp {
    x: f32,
    y: f32,
    age: f32,
}

/// The pixels of a straight or 45 degree line between two points, end included.
fn trace_line(out: &mut Vec<(i32, i32)>, from: (i32, i32), to: (i32, i32)) {
    let (mut x, mut y) = from;
    let (sx, sy) = ((to.0 - x).signum(), (to.1 - y).signum());
    for _ in 0..4096 {
        out.push((x, y));
        if (x, y) == to {
            break;
        }
        let (dx, dy) = ((to.0 - x).abs(), (to.1 - y).abs());
        if dx > dy {
            x += sx;
        } else if dy > dx {
            y += sy;
        } else {
            x += sx;
            y += sy;
        }
    }
}

/// A trace that runs across first and then turns 45 degrees into its end.
fn across_then_diagonal(from: (i32, i32), to: (i32, i32)) -> Vec<(i32, i32)> {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let run = (dx.abs() - dy.abs()).max(0) * dx.signum();
    let turn = (from.0 + run, from.1);
    let mut out = Vec::new();
    trace_line(&mut out, from, turn);
    trace_line(&mut out, turn, to);
    out.dedup();
    out
}

/// A trace that runs across, cuts its corner at 45 degrees and runs straight up or down into
/// its end, as a bus does into a row of pins.
fn across_then_up(from: (i32, i32), to: (i32, i32)) -> Vec<(i32, i32)> {
    let (sx, sy) = ((to.0 - from.0).signum(), (to.1 - from.1).signum());
    let cut = 2.min((to.0 - from.0).abs()).min((to.1 - from.1).abs());
    let a = (to.0 - sx * cut, from.1);
    let b = (to.0, from.1 + sy * cut);
    let mut out = Vec::new();
    trace_line(&mut out, from, a);
    trace_line(&mut out, a, b);
    trace_line(&mut out, b, to);
    out.dedup();
    out
}

/// A trace that runs down or up first and then turns 45 degrees into its end.
fn along_then_diagonal(from: (i32, i32), to: (i32, i32)) -> Vec<(i32, i32)> {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let run = (dy.abs() - dx.abs()).max(0) * dy.signum();
    let turn = (from.0, from.1 + run);
    let mut out = Vec::new();
    trace_line(&mut out, from, turn);
    trace_line(&mut out, turn, to);
    out.dedup();
    out
}

/// Where every part of the board sits at one size.
#[derive(Clone, Debug, Default)]
struct Board {
    big: Option<Rect>,
    small: Option<Rect>,
    crystal: Option<Rect>,
    resistors: Vec<Rect>,
    smd: Vec<Rect>,
    caps: Vec<(i32, i32, i32)>,
    fingers: Vec<Rect>,
    holes: Vec<(i32, i32)>,
    vias: Vec<(i32, i32)>,
    traces: Vec<Vec<(i32, i32)>>,
    /// The faulting trace; `gap` is where along it the break is.
    fault: Vec<(i32, i32)>,
    gap: usize,
}

pub struct ShortCircuit {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// The fault's own clock, run at the motion speed.
    fault_t: f32,
    board: Board,
    sparks: Vec<Spark>,
    wisps: Vec<Wisp>,
    next_spark: f32,
    next_wisp: f32,
    /// When the last burst of sparks went off, on the scene's clock.
    last_burst: f64,
    bursts: u32,
}

impl ShortCircuit {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut scene = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            fault_t: FAULT_START,
            board: Board::default(),
            sparks: Vec::new(),
            wisps: Vec::new(),
            next_spark: 1.0,
            next_wisp: 0.0,
            last_burst: -10.0,
            bursts: 0,
        };
        scene.resize(width, height, rng);
        scene
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// How far the damage has spread, from 0 to 1.
    fn damage(&self) -> f32 {
        (self.fault_t / FAULT_SECONDS).clamp(0.0, 1.0)
    }

    /// The break in the faulting trace, where the sparks jump.
    fn break_point(&self) -> Option<(i32, i32)> {
        self.board.fault.get(self.board.gap).copied()
    }

    /// The scorch mark's largest radius, in pixels.
    fn scorch_size(&self) -> f32 {
        (self.tall() as f32 * 0.2)
            .min(self.width as f32 * 0.14)
            .max(2.0)
    }

    fn lay_out(&mut self) {
        let (w, h) = (self.width as i32, self.tall());
        let mut b = Board::default();
        if w < 30 || h < 16 {
            // too small for parts: a single burnt trace across the board
            let y = h / 2;
            b.fault = across_then_diagonal((0, y), (w - 1, y));
            b.gap = b.fault.len() / 2;
            self.board = b;
            return;
        }
        let size = ((h as f32 * 0.4).min(w as f32 * 0.22) as i32).max(8) / 2 * 2;
        let big = Rect {
            x: w / 10 + 3,
            y: h * 22 / 100,
            w: size,
            h: size,
        };
        let small = Rect {
            x: w * 56 / 100,
            y: (h * 9 / 100).max(4),
            w: (w * 16 / 100).max(9) / 2 * 2 + 1,
            h: (h * 14 / 100).max(4),
        };
        // the bus: the big chip's upper right pins to the small chip's lower pins
        let bus = ((small.w - 2) / 2).min((size - 4) / 2 / 2).max(1);
        for i in 0..bus {
            let from = (big.x + big.w + 2, big.y + 2 + i * 2);
            let to = (small.x + 1 + i * 2, small.y + small.h + 2);
            b.traces.push(across_then_up(from, to));
        }
        // the lower pins run down to the edge fingers
        let finger_top = h - 5;
        let mut x = big.x + 2;
        while x < big.x + big.w - 1 {
            b.traces.push(along_then_diagonal(
                (x, big.y + big.h + 2),
                (x, finger_top - 1),
            ));
            b.fingers.push(Rect {
                x: x - 1,
                y: finger_top,
                w: 2,
                h: 5,
            });
            x += 4;
        }
        // a crystal above the big chip, on two of its top pins
        if big.y >= 9 {
            let cx = big.x + big.w / 2;
            let crystal = Rect {
                x: cx - 4,
                y: big.y - 8,
                w: 9,
                h: 3,
            };
            for px in [cx - 2, cx + 2] {
                b.traces
                    .push(along_then_diagonal((px, big.y - 3), (px, crystal.y + 3)));
            }
            b.crystal = Some(crystal);
        }
        // resistors in a column right of the small chip, joined to a rail down to a capacitor
        let cap_r = (h / 11).max(3);
        let c1 = (w * 52 / 100, h * 74 / 100, cap_r);
        let c2 = (w * 84 / 100, h * 68 / 100, cap_r);
        b.caps = vec![c1, c2];
        let rail_x = (small.x + small.w + 12).min(w - 6);
        for k in 0..3 {
            let y = small.y + k * 5;
            if y + 2 >= c2.1 - cap_r - 2 {
                break;
            }
            let r = Rect {
                x: rail_x - 8,
                y,
                w: 6,
                h: 2,
            };
            b.traces.push(vec![(r.x + r.w, y), (rail_x, y)]);
            b.resistors.push(r);
        }
        b.traces.push(along_then_diagonal(
            (rail_x, small.y),
            (c2.0, c2.1 - cap_r - 1),
        ));
        // the small chip's last lower pin to a via and on down to the right capacitor's ground
        let via = (small.x + small.w - 1, small.y + small.h + 6);
        b.traces.push(along_then_diagonal(
            (small.x + small.w - 1, small.y + small.h + 2),
            via,
        ));
        b.vias.push(via);
        b.vias.push((big.x + big.w + 6, big.y + big.h + 4));
        b.traces.push(across_then_diagonal(
            (c1.0 + cap_r + 1, c1.1),
            (c2.0 - cap_r - 1, c2.1),
        ));
        // a few small parts by the big chip
        for (sx, sy) in [
            (big.x - 6, big.y + 3),
            (big.x - 6, big.y + big.h - 6),
            (big.x + big.w + 4, big.y + big.h - 1),
        ] {
            if sx >= 3 {
                b.smd.push(Rect {
                    x: sx,
                    y: sy,
                    w: 3,
                    h: 2,
                });
            }
        }
        b.holes = vec![(3, 3), (w - 4, 3), (3, h - 4), (w - 4, h - 4)];
        // the faulting trace: from the big chip's lowest right pin to the left capacitor
        let from = (big.x + big.w + 2, big.y + big.h - 3);
        b.fault = across_then_diagonal(from, (c1.0, c1.1 - cap_r - 1));
        b.gap = b.fault.len() * 2 / 5;
        b.big = Some(big);
        b.small = Some(small);
        self.board = b;
    }

    fn burst(&mut self, rng: &mut Rng) {
        let Some((gx, gy)) = self.break_point() else {
            return;
        };
        let count = if self.motion.calm {
            2
        } else {
            2 + rng.below(2)
        };
        for _ in 0..count {
            // sparks leap a little way up and out, then fall back as they slow
            let angle = rng.range(-2.6, -0.5);
            let speed = rng.range(4.0, 7.0);
            self.sparks.push(Spark {
                x: gx as f32,
                y: gy as f32,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed,
                age: 0.0,
            });
        }
        self.last_burst = self.t;
        self.bursts += 1;
    }

    fn paint_board(&self, canvas: &mut Canvas, c: &Colours) {
        let tall = canvas.height() as i32;
        let b = &self.board;
        for y in 0..tall {
            canvas.fill_row(y, c.board.at(y as f32 / tall.max(1) as f32));
        }
        for &(x, y) in &b.holes {
            disc(canvas, x as f32, y as f32, 2.4, c.copper);
            canvas.fill_rect(x - 1, y - 1, 2, 2, c.hole);
        }
        for trace in &b.traces {
            for &(x, y) in trace {
                canvas.set(x, y, c.trace);
            }
        }
        for &(x, y) in &b.vias {
            disc(canvas, x as f32, y as f32, 1.2, c.copper);
            canvas.set(x, y, c.hole);
        }
        for f in &b.fingers {
            canvas.fill_rect(f.x, f.y, f.w, f.h, c.copper);
        }
        if let Some(chip) = b.big {
            // pins on all four sides, every other pixel, then the body over their inner ends
            for i in (2..chip.w - 1).step_by(2) {
                canvas.fill_rect(chip.x + i, chip.y - 2, 1, 2, c.pins);
                canvas.fill_rect(chip.x + i, chip.y + chip.h, 1, 2, c.pins);
                canvas.fill_rect(chip.x - 2, chip.y + i, 2, 1, c.pins);
                canvas.fill_rect(chip.x + chip.w, chip.y + i, 2, 1, c.pins);
            }
            canvas.fill_rect(chip.x, chip.y, chip.w, chip.h, c.chip);
            canvas.fill_rect(chip.x, chip.y, chip.w, 1, c.bevel);
            canvas.fill_rect(chip.x, chip.y, 1, chip.h, c.bevel);
            // the printed pin one dot and the corner marks round the footprint
            canvas.fill_rect(chip.x + 2, chip.y + 2, 2, 2, c.bevel);
            for (cx, cy, sx, sy) in [
                (chip.x - 4, chip.y - 4, 1, 1),
                (chip.x + chip.w + 3, chip.y - 4, -1, 1),
                (chip.x - 4, chip.y + chip.h + 3, 1, -1),
                (chip.x + chip.w + 3, chip.y + chip.h + 3, -1, -1),
            ] {
                canvas.fill_rect(cx.min(cx + sx * 2), cy, 3, 1, c.silk);
                canvas.fill_rect(cx, cy.min(cy + sy * 2), 1, 3, c.silk);
            }
        }
        if let Some(chip) = b.small {
            for i in (1..chip.w).step_by(2) {
                canvas.fill_rect(chip.x + i, chip.y - 2, 1, 2, c.pins);
                canvas.fill_rect(chip.x + i, chip.y + chip.h, 1, 2, c.pins);
            }
            canvas.fill_rect(chip.x, chip.y, chip.w, chip.h, c.chip);
            canvas.fill_rect(chip.x, chip.y, chip.w, 1, c.bevel);
            // the notch at the pin one end
            canvas.set(chip.x, chip.y + chip.h / 2, c.board.at(0.3));
        }
        if let Some(x) = b.crystal {
            canvas.fill_rect(x.x + 1, x.y, x.w - 2, x.h, c.pins);
            canvas.fill_rect(x.x, x.y + 1, x.w, x.h - 2, c.pins);
            canvas.fill_rect(x.x + 1, x.y + 1, x.w - 2, 1, c.bevel);
        }
        for r in &b.resistors {
            canvas.fill_rect(r.x - 1, r.y, 1, r.h, c.copper);
            canvas.fill_rect(r.x + r.w, r.y, 1, r.h, c.copper);
            canvas.fill_rect(r.x, r.y, r.w, r.h, c.resistor);
            canvas.fill_rect(r.x + 1, r.y, 1, r.h, c.band);
            canvas.fill_rect(r.x + 3, r.y, 1, r.h, c.band);
        }
        for s in &b.smd {
            canvas.fill_rect(s.x, s.y, s.w, s.h, c.resistor);
            canvas.fill_rect(s.x, s.y, 1, s.h, c.copper);
            canvas.fill_rect(s.x + s.w - 1, s.y, 1, s.h, c.copper);
        }
        for &(x, y, r) in &b.caps {
            // a can seen from above: a printed ring, the body, its polarity stripe and the
            // cross scored in its top
            let (x, y, r) = (x as f32, y as f32, r as f32);
            disc(canvas, x, y, r + 1.0, c.silk);
            disc(canvas, x, y, r, c.can.0);
            for py in (y - r) as i32..=(y + r) as i32 {
                for px in (x - r) as i32..=(x + r) as i32 {
                    let (dx, dy) = (px as f32 - x, py as f32 - y);
                    if dx * dx + dy * dy <= r * r && dx < -r * 0.45 {
                        canvas.set(px, py, c.can.1);
                    }
                }
            }
            let arm = (r * 0.5).round() as i32;
            let (cx, cy) = (x.round() as i32, y.round() as i32);
            canvas.fill_rect(cx - arm, cy, 2 * arm + 1, 1, c.stripe);
            canvas.fill_rect(cx, cy - arm, 1, 2 * arm + 1, c.stripe);
        }
    }

    fn paint_fault(&self, canvas: &mut Canvas, c: &Colours) {
        let mut blender = Blender::new(4);
        let b = &self.board;
        let damage = self.damage();
        let Some((gx, gy)) = self.break_point() else {
            return;
        };
        // the scorch spreads round the break with a ragged edge, darkest in the middle, and
        // browns whatever lies under it
        let r = 1.5 + damage * self.scorch_size();
        let reach = r.ceil() as i32 + 2;
        for y in gy - reach..=gy + reach {
            for x in gx - reach..=gx + reach {
                let (dx, dy) = ((x - gx) as f32, (y - gy) as f32);
                let d = (dx * dx + dy * dy).sqrt();
                let rough = r * (0.7 + 0.5 * noise(dy.atan2(dx) * 1.3 + 4.0, 0.5, 62));
                if d > rough {
                    continue;
                }
                let v = d / rough.max(0.1);
                if v < 0.35 {
                    canvas.set(x, y, c.char);
                } else {
                    blender.blend(canvas, x, y, c.scorch.at(1.0 - v), 0.95 - 0.6 * v);
                }
            }
        }
        // a soft glow round the break that swells and fades slowly
        let swell = 0.5
            + 0.5
                * (self.t as f32 * self.motion.pace() * std::f32::consts::TAU / GLOW_PERIOD).sin();
        let glow_r = 2.5 + swell;
        for y in gy - 4..=gy + 4 {
            for x in gx - 4..=gx + 4 {
                let d = (((x - gx) as f32).powi(2) + ((y - gy) as f32).powi(2)).sqrt();
                if d <= glow_r && d > 1.0 {
                    blender.blend(canvas, x, y, c.glow, 0.7 * (1.0 - d / (glow_r + 0.5)));
                }
            }
        }
        // the trace: copper beyond the damage, blackened towards the break, burnt through at it
        let spread = (damage * b.fault.len() as f32 * 0.45) as usize + 2;
        for (i, &(x, y)) in b.fault.iter().enumerate() {
            let from_gap = i.abs_diff(b.gap);
            if from_gap <= 1 {
                continue;
            }
            let colour = if from_gap == 2 {
                c.fault
            } else if from_gap <= spread {
                c.char
            } else {
                c.trace
            };
            canvas.set(x, y, colour);
        }
        canvas.set(gx, gy, c.ember);
        // thin smoke curling up, see through so the board shows under it
        for wisp in &self.wisps {
            let r = 0.6 + wisp.age * 0.45;
            let strength = 0.55 * (1.0 - wisp.age / 4.0).max(0.0);
            let reach = r.ceil() as i32 + 1;
            let (px, py) = (wisp.x.round() as i32, wisp.y.round() as i32);
            for y in py - reach..=py + reach {
                for x in px - reach..=px + reach {
                    let d = ((x as f32 - wisp.x).powi(2) + (y as f32 - wisp.y).powi(2)).sqrt();
                    if d <= r + 0.3 {
                        blender.blend(canvas, x, y, c.smoke, strength);
                    }
                }
            }
        }
        for spark in &self.sparks {
            let (x, y) = (spark.x.round() as i32, spark.y.round() as i32);
            // a young spark is bright with a short tail; an old one cools to an ember
            if spark.age < SPARK_LIFE * 0.5 {
                canvas.set(x, y, c.spark);
                let (tx, ty) = (spark.x - spark.vx * 0.15, spark.y - spark.vy * 0.15);
                canvas.set(tx.round() as i32, ty.round() as i32, c.ember);
            } else {
                canvas.set(x, y, c.ember);
            }
        }
    }
}

impl Scene for ShortCircuit {
    fn name(&self) -> &'static str {
        "short-circuit"
    }

    fn resize(&mut self, width: usize, height: usize, _rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.lay_out();
        self.sparks.clear();
        self.wisps.clear();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let moved = dt * self.motion.speed();
        let pace = self.motion.pace();
        self.fault_t += moved;

        // sparks keep the standard pace at most and half of it when calm
        self.next_spark -= dt * pace;
        if self.next_spark <= 0.0 {
            self.next_spark = SPARK_PERIOD + rng.range(0.0, 1.0);
            if self.t - self.last_burst >= f64::from(SPARK_PERIOD) * 0.9 {
                self.burst(rng);
            }
        }
        for spark in &mut self.sparks {
            spark.age += dt * pace;
            spark.x += spark.vx * dt * pace;
            spark.y += spark.vy * dt * pace;
            // they slow and fall back under their own weight
            spark.vy += 9.0 * dt * pace;
            spark.vx *= 1.0 - 0.8 * dt * pace;
        }
        self.sparks.retain(|s| s.age < SPARK_LIFE);

        self.next_wisp -= moved;
        if self.next_wisp <= 0.0 {
            self.next_wisp = 1.4;
            if let Some((gx, gy)) = self.break_point() {
                self.wisps.push(Wisp {
                    x: gx as f32 + rng.range(-0.8, 0.8),
                    y: gy as f32 - 1.0,
                    age: 0.0,
                });
            }
        }
        let t = self.t;
        for (i, wisp) in self.wisps.iter_mut().enumerate() {
            wisp.age += moved;
            wisp.y -= 1.8 * moved;
            wisp.x += (super::cycle(t, 0.6) + i as f32).sin() * 0.5 * moved;
        }
        self.wisps.retain(|w| w.age < 4.0 && w.y > -3.0);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = Colours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint_board(&mut canvas, &colours);
        self.paint_fault(&mut canvas, &colours);
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
        check_scene("short-circuit");
        check_determinism("short-circuit");
    }

    #[test]
    fn passes_the_pixel_checks() {
        check_pixel_scene("short-circuit", 20.0);
        check_palette(Colours::palette);
    }

    #[test]
    fn the_parts_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            for (a, b, what) in [
                (c.board.at(0.5), c.trace, "the board and the traces"),
                (c.board.at(0.5), c.copper, "the board and the copper"),
                (c.board.at(0.5), c.chip, "the board and the chips"),
                (
                    c.board.at(0.5),
                    c.scorch.at(0.0),
                    "the board and the scorch",
                ),
                (c.trace, c.copper, "the traces and the pads"),
                (c.chip, c.bevel, "a chip and its edge"),
                (c.can.0, c.can.1, "a capacitor and its stripe"),
                (c.char, c.trace, "the burnt trace and the good one"),
            ] {
                assert!(distance(a, b) >= 0.03, "{what} in {}", theme.name);
            }
        }
    }

    #[test]
    fn sparks_keep_their_period_on_every_level() {
        for motion in Motion::ALL {
            let mut rng = Rng::new(4);
            let mut board = ShortCircuit::new(80, 24, motion, &mut rng);
            let mut times = Vec::new();
            let mut last = board.bursts;
            for i in 0..(120 * 20) {
                board.step(0.05, &mut rng);
                if board.bursts != last {
                    times.push(i as f32 * 0.05);
                    last = board.bursts;
                }
                assert!(board.sparks.len() <= 8);
            }
            assert!(times.len() >= 15, "{motion:?}: {}", times.len());
            let floor = if motion.calm { 2.5 } else { 1.4 };
            for w in times.windows(2) {
                assert!(w[1] - w[0] >= floor, "{motion:?}: {w:?}");
            }
        }
    }

    #[test]
    fn the_damage_spreads_then_holds_and_the_trace_stays_broken() {
        let mut rng = Rng::new(4);
        let mut board = ShortCircuit::new(80, 24, Motion::FULL, &mut rng);
        let start = board.damage();
        for _ in 0..(90 * 20) {
            board.step(0.05, &mut rng);
        }
        assert!(start > 0.0 && start < 0.5);
        assert_eq!(board.damage(), 1.0);
        let theme = theme::default_theme();
        let mut frame = Frame::new(80, 24);
        board.draw(&mut frame, theme);
        let c = Colours::new(theme);
        let count = |colour: Rgb| -> usize {
            frame
                .cells()
                .iter()
                .filter_map(|cell| cell.pixel_pair())
                .map(|(a, b)| usize::from(a == colour) + usize::from(b == colour))
                .sum()
        };
        assert!(count(c.fault) >= 2, "{}", count(c.fault));
        assert!(count(c.char) >= 20, "{}", count(c.char));
        assert!(board.board.big.is_some() && board.board.fault.len() > board.board.gap);
    }
}
