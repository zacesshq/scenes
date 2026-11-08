//! PCB bring-up: a circuit board on a workbench under an angled lamp, seen from the front and a
//! little above. An oscilloscope stands at the back on the left with its probe lying on the
//! mat, a bench supply stands on the right with its leads clipped to the board and a pegboard
//! wall rises behind. The supply is on: the LEDs along the board light one after another, each
//! with a soft glow, until the status LED at the end of the chain glows a steady green, the
//! "POWER GOOD" legend beside it lights up and the scope settles on a clean square wave. The
//! board holds there for a long while, then the supply is switched off and the chain lights
//! again.
//!
//! The wall, the mat, the board, its copper, the lamp's pool of light and every glow are tints
//! of the background held below the non-text floor. The instruments, the lamp, the probe, the
//! chip and the lit LEDs are solid, so with colour off the lit LEDs show as blocks and a dark
//! one as a gap. The supply's readout and the legend are text on faces cleared to the
//! background, so they keep the theme's full text contrast.
//!
//! The fail twin, `dead-led`, draws the same bench through `Bench` with one LED in the chain
//! that never lights, the probe moving to it and a flat trace on the scope.

use super::paint::{self, disc, held_imagery, tint};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::warning_colour;

pub const SUMMARY: &str = "A circuit board on a lamp-lit bench lighting its LEDs in turn until the status LED glows green, by day in light themes";

pub const CAPTIONS: &[&str] = &[
    "Power good. The build is up.",
    "Every light in the chain is on.",
    "A clean bring-up. All green.",
];

/// Seconds between one LED in the chain lighting and the next, at the standard pace.
pub(crate) const STEP: f32 = 1.4;
/// Seconds after the start before the first LED lights.
pub(crate) const START: f32 = 1.5;
/// Seconds an LED spends glowing faintly before it is fully lit, so it warms up rather than
/// snapping on.
pub(crate) const WARM: f32 = 0.5;
/// Seconds the board holds with every light on before the supply is switched off.
pub(crate) const HOLD: f32 = 45.0;
/// Seconds the lights take to fade when the supply is switched off.
const FADE: f32 = 1.2;
/// Seconds the board stays dark before the chain lights again.
const OFF: f32 = 4.0;
/// Seconds the probe takes to move from the mat to the dark LED.
pub(crate) const PROBE_MOVE: f32 = 7.0;
/// Pixels a second the scope's beam sweeps across its screen at the standard speed.
const SWEEP: f32 = 9.0;

/// Which story the bench tells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Story {
    /// Every LED lights and the board reports power good.
    BringUp,
    /// One LED stays dark and the probe goes to it.
    DeadLed,
}

/// How an LED is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Led {
    Dark,
    /// Warming up or fading out: only a faint glow in the lens.
    Faint,
    Lit,
}

/// A solid tone of `hue` for a lit shape, from 0 for its shaded side to 1 for its highlight.
/// The lit side moves towards the theme's light end on a dark theme and towards the background
/// on a light one, never falling below the solid contrast, so the shape keeps its outline.
pub(crate) fn tone(theme: &ThemeData, hue: Rgb, lit: f32) -> Rgb {
    let lit = lit.clamp(0.0, 1.0);
    let target = f64::from(if theme.dark {
        3.5 + lit * 5.0
    } else {
        3.4 + (1.0 - lit) * 3.2
    });
    let background = theme.background;
    let far = if theme.dark {
        Shades::new(theme).lightest()
    } else {
        Shades::new(theme).darkest()
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

/// The bench's colours in one theme.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct Colours {
    pub wall: Ramp,
    pub hole: Ramp,
    pub mat: Ramp,
    pub lip: Rgb,
    pub shadow: Rgb,
    pub halo: Rgb,
    pub mask: Ramp,
    pub edge: Rgb,
    pub copper: Rgb,
    pub silk: Rgb,
    pub chip: Rgb,
    pub pins: Rgb,
    pub pad: Rgb,
    pub lens: Rgb,
    pub amber: (Rgb, Rgb),
    pub green: (Rgb, Rgb),
    pub amber_glow: (Rgb, Rgb),
    pub green_glow: (Rgb, Rgb),
    pub metal: (Rgb, Rgb, Rgb),
    pub bulb: Rgb,
    pub supply: (Rgb, Rgb, Rgb),
    pub scope: (Rgb, Rgb, Rgb),
    pub bezel: Rgb,
    pub knob: Rgb,
    pub red_post: Rgb,
    pub black_post: Rgb,
    pub graticule: Rgb,
    pub trace: Rgb,
    pub beam: Rgb,
    pub probe: Rgb,
    pub band: Rgb,
    pub tip: Rgb,
    pub cable: Rgb,
    pub face: Rgb,
    pub readout: Rgb,
    pub legend_dim: Rgb,
    pub legend_good: Rgb,
}

impl Colours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let dark = theme.dark;
        let warm = mix(theme.yellow(), theme.foreground, 0.45);
        let board_hue = mix(theme.yellow(), theme.muted, 0.6);
        let mat_hue = mix(theme.blue(), theme.muted, 0.55);
        let mask_hue = mix(theme.green(), theme.cyan(), 0.15);
        // a dark theme is a workshop at night lit by the lamp; a light theme is the same bench
        // by day, a pale wall and mat with the lamp's pool a touch warmer and brighter
        let (wall_dark, wall_lit, hole_dark, hole_lit) = if dark {
            (
                tint(theme, board_hue, 0.12, 1.22),
                tint(theme, mix(board_hue, warm, 0.4), 0.42, 1.95),
                tint(theme, board_hue, 0.03, 1.05),
                tint(theme, mix(board_hue, warm, 0.4), 0.2, 1.4),
            )
        } else {
            (
                tint(theme, board_hue, 0.4, 1.75),
                tint(theme, warm, 0.06, 1.08),
                tint(theme, board_hue, 0.68, 2.5),
                tint(theme, board_hue, 0.36, 1.6),
            )
        };
        let (mat_dark, mat_lit, lip, shadow) = if dark {
            (
                tint(theme, mat_hue, 0.16, 1.3),
                tint(theme, mix(mat_hue, warm, 0.25), 0.52, 2.25),
                tint(theme, theme.muted, 0.06, 1.08),
                tint(theme, mat_hue, 0.05, 1.08),
            )
        } else {
            (
                tint(theme, mat_hue, 0.55, 2.3),
                tint(theme, mix(mat_hue, warm, 0.25), 0.14, 1.18),
                tint(theme, mat_hue, 0.8, 2.85),
                tint(theme, mat_hue, 0.72, 2.7),
            )
        };
        let (mask_dim, mask_lit, edge, copper) = if dark {
            (
                tint(theme, mask_hue, 0.28, 1.5),
                tint(theme, mask_hue, 0.5, 2.05),
                tint(theme, mask_hue, 0.1, 1.18),
                tint(theme, mix(mask_hue, theme.yellow(), 0.35), 0.55, 2.35),
            )
        } else {
            (
                tint(theme, mask_hue, 0.6, 2.4),
                tint(theme, mask_hue, 0.4, 1.8),
                tint(theme, mask_hue, 0.85, 2.9),
                tint(theme, mix(mask_hue, theme.yellow(), 0.35), 0.3, 1.5),
            )
        };
        let mask_mid = mix(mask_dim, mask_lit, 0.6);
        let amber_hue = mix(mix(theme.yellow(), theme.warning, 0.4), theme.red(), 0.22);
        let green_hue = theme.green();
        let glow = |hue: Rgb, t: f32| held_imagery(theme.background, mask_mid, hue, t);
        // a chip is darker than the board in every theme: nearly black on a dark theme, the
        // darkest solid on a light one, where its silver pins are tints instead
        let (chip, pins, silk, lens) = if dark {
            (
                tint(theme, theme.muted, 0.05, 1.07),
                tone(theme, theme.muted, 0.35),
                tint(theme, theme.foreground, 0.5, 2.5),
                tint(theme, mix(amber_hue, theme.muted, 0.6), 0.2, 1.38),
            )
        } else {
            (
                theme.foreground,
                tint(theme, theme.muted, 0.45, 2.1),
                tint(theme, theme.info, 0.02, 1.04),
                tint(theme, mix(amber_hue, theme.muted, 0.6), 0.7, 2.65),
            )
        };
        let metal_hue = mix(theme.muted, theme.foreground, 0.25);
        let supply_hue = mix(theme.muted, theme.red(), 0.12);
        let scope_hue = mix(theme.muted, theme.info, 0.4);
        Self {
            wall: Ramp::new(wall_dark, wall_lit, 8),
            hole: Ramp::new(hole_dark, hole_lit, 8),
            mat: Ramp::new(mat_dark, mat_lit, 7),
            lip,
            shadow,
            halo: held_imagery(theme.background, wall_lit, warm, 0.5),
            mask: Ramp::new(mask_dim, mask_lit, 4),
            edge,
            copper,
            silk,
            chip,
            pins,
            pad: tint(theme, mix(theme.yellow(), theme.warning, 0.3), 0.75, 2.8),
            lens,
            amber: (tone(theme, amber_hue, 0.55), tone(theme, amber_hue, 1.0)),
            green: (tone(theme, green_hue, 0.55), tone(theme, green_hue, 1.0)),
            amber_glow: (glow(amber_hue, 0.55), glow(amber_hue, 0.3)),
            green_glow: (glow(green_hue, 0.6), glow(green_hue, 0.32)),
            metal: (
                tone(theme, metal_hue, 0.05),
                tone(theme, metal_hue, 0.4),
                tone(theme, metal_hue, 0.8),
            ),
            bulb: tone(theme, mix(theme.yellow(), theme.foreground, 0.3), 1.0),
            supply: (
                tone(theme, supply_hue, 0.0),
                tone(theme, supply_hue, 0.25),
                tone(theme, supply_hue, 0.6),
            ),
            scope: (
                tone(theme, scope_hue, 0.05),
                tone(theme, scope_hue, 0.35),
                tone(theme, scope_hue, 0.75),
            ),
            bezel: tone(theme, theme.muted, 0.0),
            knob: tone(theme, metal_hue, 0.95),
            red_post: tone(theme, theme.red(), 0.45),
            black_post: if dark {
                tone(theme, theme.muted, 0.0)
            } else {
                shades.darkest()
            },
            graticule: if dark {
                tint(theme, scope_hue, 0.3, 1.6)
            } else {
                tint(theme, scope_hue, 0.35, 1.7)
            },
            trace: tone(theme, theme.cyan(), 0.5),
            beam: tone(theme, theme.cyan(), 1.0),
            probe: tone(theme, theme.muted, 0.0),
            band: tone(theme, theme.blue(), 0.5),
            tip: tone(theme, metal_hue, 1.0),
            cable: tone(theme, theme.muted, 0.15),
            face: theme.background,
            readout: warning_colour(theme),
            legend_dim: theme.grey(),
            legend_good: theme.green(),
        }
    }
}

/// A rectangle of pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    fn right(&self) -> i32 {
        self.x + self.w
    }

    fn bottom(&self) -> i32 {
        self.y + self.h
    }
}

/// The bench shared by `pcb-bring-up` and `dead-led`: the same layout, colours and timing, with
/// the story deciding which LEDs light and where the probe goes.
pub(crate) struct Bench {
    width: usize,
    height: usize,
    motion: Motion,
    story: Story,
    /// The story's clock, run at the motion pace, so lights never change quicker on lively.
    pub(crate) clock: f32,
    /// The beam's clock, run at the motion speed.
    sweep: f32,
}

impl Bench {
    pub(crate) fn new(width: usize, height: usize, motion: Motion, story: Story) -> Self {
        let mut bench = Self {
            width: width.max(1),
            height: height.max(1),
            motion,
            story,
            clock: 0.0,
            sweep: 0.0,
        };
        // the fail story opens with the chain already lit around its dark LED, so what has
        // gone wrong reads from the first look, as the leak opens on a half full pail
        if story == Story::DeadLed {
            bench.clock = bench.lit_by() - WARM;
        }
        bench
    }

    pub(crate) fn resize(&mut self, width: usize, height: usize) {
        self.width = width.max(1);
        self.height = height.max(1);
    }

    pub(crate) fn step(&mut self, dt: f32) {
        self.clock += dt * self.motion.pace();
        if self.story == Story::BringUp {
            // the loop is wrapped here so a scene left running for days keeps an exact clock
            let cycle = self.cycle();
            if self.clock >= cycle {
                self.clock -= cycle;
            }
        } else {
            self.clock = self.clock.min(self.settled() + 60.0);
        }
        // under reduced motion the beam's spot holds still, so only the story moves
        if !self.motion.calm {
            self.sweep = (self.sweep + dt * self.motion.speed() * SWEEP) % 100_000.0;
        }
    }

    fn wide(&self) -> i32 {
        self.width as i32
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// Whether there is room for the instruments either side of the board.
    fn roomy(&self) -> bool {
        self.width >= 48 && self.height >= 12
    }

    /// The row where the back of the bench meets the wall.
    fn bench_top(&self) -> i32 {
        (self.tall() as f32 * 0.42).round() as i32
    }

    /// The first row of the bench's front edge.
    fn lip(&self) -> i32 {
        self.tall() - (self.tall() / 14).max(1)
    }

    pub(crate) fn board(&self) -> Rect {
        let depth = self.lip() - self.bench_top();
        let share = if self.roomy() { 0.4 } else { 0.72 };
        let w = ((self.wide() as f32 * share).round() as i32).max(8);
        let h = ((depth as f32 * 0.7).round() as i32).clamp(4, (w * 3 / 5).max(4));
        let shift = if self.roomy() { self.wide() / 50 } else { 0 };
        let x = self.wide() / 2 - w / 2 + shift;
        let y = self.bench_top() + ((depth - h) as f32 * 0.42).round() as i32;
        Rect { x, y, w, h }
    }

    /// How many LEDs the chain has before the status LED.
    pub(crate) fn chain(&self) -> usize {
        (self.board().w / 7).clamp(3, 8) as usize
    }

    /// The index of the LED that stays dark in the fail story.
    pub(crate) fn dead(&self) -> usize {
        self.chain() / 2
    }

    fn led_size(&self) -> (i32, i32) {
        if self.board().w >= 44 { (3, 2) } else { (2, 2) }
    }

    /// The top left pixel of each LED, the chain first and the status LED last.
    pub(crate) fn leds(&self) -> Vec<(i32, i32)> {
        let b = self.board();
        let n = self.chain();
        let y = b.y + (b.h as f32 * 0.58).round() as i32;
        // whole pixel spacing, so every LED sits the same distance from the next
        let a = (b.x as f32 + b.w as f32 * 0.1).round() as i32;
        let gap = ((b.w as f32 * 0.5) / (n - 1).max(1) as f32).floor() as i32;
        let mut out: Vec<(i32, i32)> = (0..n).map(|i| (a + i as i32 * gap, y)).collect();
        out.push(((b.x as f32 + b.w as f32 * 0.82).round() as i32, y));
        out
    }

    /// The cell row and first column of the POWER GOOD legend, if the board has room for it.
    fn legend(&self) -> Option<(i32, i32)> {
        let b = self.board();
        let leds = self.leds();
        let &(status, led_y) = leds.last()?;
        let (_, lh) = self.led_size();
        let row = (led_y + lh + 2 + 1) / 2;
        let fits = b.w >= 16 && 2 * row + 1 < b.bottom() - 1 && self.height >= 10;
        if !fits {
            return None;
        }
        let x = (status - 5).min(b.right() - 12).max(b.x + 1);
        Some((row, x))
    }

    /// The seconds of story it takes every LED in the chain and the status LED to light.
    fn lit_by(&self) -> f32 {
        START + self.chain() as f32 * STEP + WARM
    }

    /// The length of the bring-up's loop.
    fn cycle(&self) -> f32 {
        self.lit_by() + HOLD + FADE + OFF
    }

    /// When the fail story has nothing left to change but the beam.
    fn settled(&self) -> f32 {
        self.lit_by() + 0.5 + PROBE_MOVE
    }

    /// How LED `i` shows: the chain lights from the left one `STEP` apart and the status LED
    /// last; in the bring-up the supply switches off after the hold and every LED fades.
    pub(crate) fn led(&self, i: usize) -> Led {
        let n = self.chain();
        let c = self.clock;
        if self.story == Story::DeadLed && (i == self.dead() || i == n) {
            return Led::Dark;
        }
        if self.story == Story::BringUp {
            let off = self.lit_by() + HOLD;
            if c >= off + FADE {
                return Led::Dark;
            }
            if c >= off {
                return Led::Faint;
            }
        }
        let on = START + i as f32 * STEP;
        if c < on {
            Led::Dark
        } else if c < on + WARM {
            Led::Faint
        } else {
            Led::Lit
        }
    }

    /// Whether the board is reporting power good.
    pub(crate) fn good(&self) -> bool {
        self.led(self.chain()) == Led::Lit
    }

    /// How many LEDs are fully lit, for the supply's current reading.
    fn lit_count(&self) -> usize {
        (0..=self.chain())
            .filter(|&i| self.led(i) == Led::Lit)
            .count()
    }

    fn scope(&self) -> Option<Rect> {
        if !self.roomy() {
            return None;
        }
        let base = self.bench_top() + (self.lip() - self.bench_top()) / 5;
        let h = ((self.tall() as f32 * 0.36).round() as i32).max(10);
        let x = (self.wide() as f32 * 0.04).round() as i32;
        let w = (self.wide() as f32 * 0.25).round() as i32;
        Some(Rect {
            x,
            y: base - h,
            w,
            h,
        })
    }

    fn supply(&self) -> Option<Rect> {
        if !self.roomy() {
            return None;
        }
        let base = self.bench_top() + (self.lip() - self.bench_top()) / 5;
        let h = ((self.tall() as f32 * 0.32).round() as i32).max(9);
        let x = (self.wide() as f32 * 0.76).round() as i32;
        let w = (self.wide() as f32 * 0.2).round() as i32;
        Some(Rect {
            x,
            y: base - h,
            w,
            h,
        })
    }

    /// The supply's display: its first cell row and first column, two rows tall.
    fn display(&self) -> Option<(i32, i32)> {
        let s = self.supply()?;
        let row = (s.y + 3 + 1) / 2;
        (s.w >= 10 && 2 * row + 4 <= s.bottom() - 4).then_some((row, s.x + 2))
    }

    /// The scope's screen.
    fn screen(&self) -> Option<Rect> {
        let s = self.scope()?;
        let w = (s.w as f32 * 0.66).round() as i32;
        Some(Rect {
            x: s.x + 2,
            y: s.y + 3,
            w,
            h: s.h - 6,
        })
    }

    /// The scope's input, where the probe's lead leaves it.
    fn input(&self) -> Option<(i32, i32)> {
        let s = self.scope()?;
        Some((s.right() - 3, s.bottom() - 3))
    }

    /// The probe's tip and the end of its handle. It lies on the mat to the right of the board
    /// and in the fail story moves over to touch the dark LED.
    pub(crate) fn probe(&self) -> Option<((f32, f32), (f32, f32))> {
        self.scope()?;
        let b = self.board();
        let reach = (self.wide() as f32 * 0.09).max(7.0);
        let rest_tip = (b.x as f32 - 3.0, (self.lip() - 5) as f32);
        let rest_back = (rest_tip.0 - reach, rest_tip.1 + 2.0);
        if self.story == Story::BringUp {
            return Some((rest_tip, rest_back));
        }
        let (lx, ly) = self.leds()[self.dead()];
        let (_, lh) = self.led_size();
        let to_tip = (lx as f32 - 0.5, (ly + lh) as f32 + 0.5);
        let to_back = (to_tip.0 - reach * 0.8, to_tip.1 + reach * 0.5);
        let k = paint::ease((self.clock - self.lit_by() - 0.5) / PROBE_MOVE);
        let lerp = |a: (f32, f32), b: (f32, f32)| (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k);
        Some((lerp(rest_tip, to_tip), lerp(rest_back, to_back)))
    }

    /// The pool of the lamp's light, its reach and the lamp head's centre.
    #[allow(clippy::type_complexity)]
    fn lamp(&self) -> ((f32, f32), (f32, f32), (f32, f32)) {
        let b = self.board();
        let pool = (b.x as f32 + b.w as f32 * 0.5, b.y as f32 + b.h as f32 * 0.4);
        let reach = (
            b.w as f32 * 0.95,
            (self.lip() - self.bench_top()) as f32 * 0.8,
        );
        let head = (
            b.x as f32 + b.w as f32 * 0.42,
            self.bench_top() as f32 * 0.52,
        );
        (pool, reach, head)
    }

    /// How brightly the lamp lights a point, from 0 in the corners to 1 under it.
    fn light(&self, x: i32, y: i32) -> f32 {
        let (pool, reach, head) = self.lamp();
        if y >= self.bench_top() {
            let dx = (x as f32 - pool.0) / reach.0;
            let dy = (y as f32 - pool.1) / reach.1;
            (1.0 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0).powf(0.8) * 0.9 + 0.1
        } else {
            let dx = x as f32 - head.0;
            let dy = (y as f32 - head.1) * 1.3;
            let d = (dx * dx + dy * dy).sqrt() / (self.wide() as f32 * 0.45).max(8.0);
            (1.0 - d).clamp(0.0, 1.0).powf(1.3) * 0.85 + 0.15
        }
    }

    fn paint_wall(&self, canvas: &mut Canvas, c: &Colours) {
        let top = self.bench_top();
        let pitch = if self.tall() >= 40 { 4 } else { 3 };
        for y in 0..top {
            for x in 0..self.wide() {
                let lit = self.light(x, y);
                // pegboard holes on a fixed grid, offset every other row like real board
                let shift = if (y / pitch) % 2 == 0 { 0 } else { pitch / 2 };
                let hole = self.tall() >= 24 && y % pitch == 1 && (x + shift) % pitch == 1;
                let lit = dither(lit, x, y, 8);
                let colour = if hole { c.hole.at(lit) } else { c.wall.at(lit) };
                canvas.set(x, y, colour);
            }
        }
    }

    fn paint_bench(&self, canvas: &mut Canvas, c: &Colours) {
        let (top, lip) = (self.bench_top(), self.lip());
        for y in top..lip {
            for x in 0..self.wide() {
                canvas.set(x, y, c.mat.at(dither(self.light(x, y), x, y, 7)));
            }
        }
        canvas.fill_rect(0, top, self.wide(), 1, c.shadow);
        canvas.fill_rect(0, lip, self.wide(), self.tall() - lip, c.lip);
    }

    fn paint_shadow(&self, canvas: &mut Canvas, c: &Colours, r: Rect) {
        // a soft shadow falls to the front and right, away from the lamp above and left
        canvas.fill_rect(r.x + 1, r.bottom(), r.w + 1, 1, c.shadow);
        canvas.fill_rect(r.right(), r.y + 2, 1, r.h - 1, c.shadow);
    }

    fn paint_scope(&self, canvas: &mut Canvas, c: &Colours) {
        let (Some(s), Some(screen)) = (self.scope(), self.screen()) else {
            return;
        };
        self.paint_shadow(canvas, c, s);
        canvas.fill_rect(s.x, s.y, s.w, s.h, c.scope.1);
        canvas.fill_rect(s.x, s.y, s.w, 2, c.scope.2);
        canvas.fill_rect(s.right() - 1, s.y + 2, 1, s.h - 2, c.scope.0);
        canvas.fill_rect(s.x, s.bottom() - 1, s.w, 1, c.scope.0);
        // the screen with its bezel and graticule
        canvas.fill_rect(
            screen.x - 1,
            screen.y - 1,
            screen.w + 2,
            screen.h + 2,
            c.bezel,
        );
        canvas.fill_rect(screen.x, screen.y, screen.w, screen.h, c.face);
        let mid = screen.y + screen.h / 2;
        for x in screen.x..screen.right() {
            if (x - screen.x) % 2 == 0 {
                canvas.set(x, mid, c.graticule);
            }
        }
        let mut gx = screen.x + 4;
        while gx < screen.right() - 1 {
            for y in screen.y..screen.bottom() {
                if (y - screen.y) % 2 == 1 {
                    canvas.set(gx, y, c.graticule);
                }
            }
            gx += 5;
        }
        // the trace: a clean square wave once the board is up, otherwise flat on the line
        let swing = (screen.h / 2 - 2).clamp(1, 4);
        let period = 10;
        let good = self.story == Story::BringUp && self.good();
        let level = |x: i32| -> i32 {
            if good {
                if ((x - screen.x) / (period / 2)) % 2 == 0 {
                    mid - swing
                } else {
                    mid + swing
                }
            } else {
                mid
            }
        };
        for x in screen.x + 1..screen.right() - 1 {
            let (y, prev) = (level(x), level(x - 1));
            for yy in y.min(prev)..=y.max(prev) {
                canvas.set(x, yy, c.trace);
            }
        }
        // the beam's bright spot sweeping along the trace
        let span = (screen.w - 2).max(1);
        let bx = screen.x + 1 + (self.sweep as i32).rem_euclid(span);
        canvas.set(bx, level(bx), c.beam);
        // knobs and the input
        let kx = screen.right() + (s.right() - screen.right()) / 2;
        let r = if s.h >= 16 { 1.2 } else { 0.8 };
        let mut ky = s.y + 4;
        while ky < s.bottom() - 6 {
            disc(canvas, kx as f32, ky as f32, r, c.knob);
            ky += 4;
        }
        if let Some((ix, iy)) = self.input() {
            disc(canvas, ix as f32, iy as f32, 1.0, c.metal.2);
            canvas.set(ix, iy, c.bezel);
        }
    }

    fn paint_supply(&self, canvas: &mut Canvas, c: &Colours) {
        let Some(s) = self.supply() else {
            return;
        };
        self.paint_shadow(canvas, c, s);
        canvas.fill_rect(s.x, s.y, s.w, s.h, c.supply.1);
        canvas.fill_rect(s.x, s.y, s.w, 2, c.supply.2);
        canvas.fill_rect(s.right() - 1, s.y + 2, 1, s.h - 2, c.supply.0);
        canvas.fill_rect(s.x, s.bottom() - 1, s.w, 1, c.supply.0);
        if let Some((row, x)) = self.display() {
            canvas.fill_rect(x - 1, 2 * row - 1, 9, 6, c.bezel);
            canvas.fill_rect(x, 2 * row, 7, 4, c.face);
        }
        // a knob for each of volts and amps, then the red and black output posts
        let y = s.bottom() - 4;
        let r = if s.h >= 14 { 1.3 } else { 0.9 };
        disc(canvas, (s.x + 3) as f32, y as f32, r, c.knob);
        disc(canvas, (s.x + 7) as f32, y as f32, r, c.knob);
        let (rx, bx) = self.posts(s);
        disc(canvas, rx as f32, y as f32, 1.0, c.red_post);
        disc(canvas, bx as f32, y as f32, 1.0, c.black_post);
    }

    fn posts(&self, s: Rect) -> (i32, i32) {
        (s.right() - 6, s.right() - 3)
    }

    fn paint_lamp(&self, canvas: &mut Canvas, c: &Colours) {
        if self.height < 8 {
            return;
        }
        let (_, _, head) = self.lamp();
        let b = self.board();
        let top = self.bench_top();
        let base = (b.x + b.w / 10, top + 1);
        let elbow = (base.0 as f32 - self.wide() as f32 * 0.04, top as f32 * 0.12);
        // the clamp and foot at the back of the bench, then the two arms and the spring
        canvas.fill_rect(base.0 - 2, base.1 - 1, 5, 2, c.metal.0);
        canvas.fill_rect(base.0 - 2, base.1 - 1, 5, 1, c.metal.1);
        let thick = if self.tall() >= 50 { 2 } else { 1 };
        paint::line(
            canvas,
            (base.0 as f32, (base.1 - 1) as f32),
            elbow,
            thick,
            c.metal.1,
        );
        paint::line(canvas, elbow, (head.0, head.1 - 2.0), thick, c.metal.1);
        disc(canvas, elbow.0, elbow.1, 1.0, c.metal.2);
        // the shade, a cone opening downwards with the bulb glowing in its mouth
        let size = (self.tall() as f32 / 14.0).clamp(2.0, 6.0);
        let rows = size.round() as i32 + 1;
        for i in 0..rows {
            let y = head.1 as i32 - 2 + i;
            let half = 1.0 + size * 0.9 * (i as f32 / (rows - 1).max(1) as f32);
            let (x0, x1) = (
                (head.0 - half).round() as i32,
                (head.0 + half).round() as i32,
            );
            canvas.fill_rect(x0, y, x1 - x0 + 1, 1, c.metal.1);
            canvas.set(x0, y, c.metal.2);
            canvas.set(x1, y, c.metal.0);
        }
        let mouth = head.1 as i32 - 2 + rows;
        let half = (size * 0.75).round() as i32;
        canvas.fill_rect(head.0.round() as i32 - half, mouth, 2 * half + 1, 1, c.bulb);
    }

    fn paint_board(&self, canvas: &mut Canvas, c: &Colours) {
        let b = self.board();
        let thick = if self.tall() >= 60 { 2 } else { 1 };
        self.paint_shadow(
            canvas,
            c,
            Rect {
                h: b.h + thick,
                ..b
            },
        );
        for y in b.y..b.bottom() {
            for x in b.x..b.right() {
                canvas.set(x, y, c.mask.at(dither(self.light(x, y), x, y, 4)));
            }
        }
        canvas.fill_rect(b.x, b.bottom(), b.w, thick, c.edge);
        // mounting holes in the corners, a plated ring round a dark hole
        if b.w >= 16 && b.h >= 8 {
            for (hx, hy) in [
                (b.x + 2, b.y + 1),
                (b.right() - 3, b.y + 1),
                (b.x + 2, b.bottom() - 2),
                (b.right() - 3, b.bottom() - 2),
            ] {
                canvas.set(hx - 1, hy, c.pad);
                canvas.set(hx + 1, hy, c.pad);
                canvas.set(hx, hy, c.edge);
            }
        }
        let leds = self.leds();
        let (lw, lh) = self.led_size();
        let bus = leds[0].1 - 2;
        let last = leds[leds.len() - 1];
        // the chip, its pins and the bus of traces that feeds the LEDs
        let chip = Rect {
            x: b.x + (b.w / 10).max(2),
            y: b.y + 2,
            w: (b.w / 4).max(4),
            h: (b.h / 5).max(2),
        };
        let chip_fits = chip.bottom() < bus;
        canvas.fill_rect(leds[0].0 + lw / 2, bus, last.0 - leds[0].0 + 1, 1, c.copper);
        if chip_fits {
            let from = chip.y + chip.h / 2;
            canvas.fill_rect(chip.right(), from, 1, bus - from, c.copper);
            canvas.fill_rect(
                chip.right(),
                bus,
                leds[0].0 + lw / 2 - chip.right(),
                1,
                c.copper,
            );
        }
        for &(x, y) in &leds {
            canvas.fill_rect(x + lw / 2, bus, 1, y - bus, c.copper);
        }
        // the supply comes in at a header in the top right corner
        let header = (b.right() - 3, b.y + 2);
        if b.h >= 8 {
            canvas.fill_rect(header.0 + 1, header.1 + 1, 1, bus - header.1 - 1, c.copper);
            canvas.fill_rect(
                last.0 + lw / 2,
                bus,
                header.0 + 2 - last.0 - lw / 2,
                1,
                c.copper,
            );
            canvas.fill_rect(header.0, header.1, 2, 2, c.pad);
            canvas.fill_rect(header.0 - 1, header.1 - 1, 4, 1, c.silk);
        }
        if chip_fits {
            canvas.fill_rect(chip.x, chip.y, chip.w, chip.h, c.chip);
            for x in (chip.x + 1..chip.right() - 1).step_by(2) {
                canvas.set(x, chip.y - 1, c.pins);
                canvas.set(x, chip.bottom(), c.pins);
            }
            canvas.set(chip.x + 1, chip.y, c.silk);
        }
        // on a roomy board a smaller chip sits below the first and a capacitor beside the bus
        if b.h >= 18 && b.w >= 40 {
            let small = Rect {
                x: chip.x + 1,
                y: leds[0].1 + lh + 4,
                w: (chip.w * 2 / 3).max(3),
                h: 2,
            };
            if small.bottom() < b.bottom() - 2 {
                canvas.fill_rect(small.x, small.y, small.w, small.h, c.chip);
                for x in (small.x + 1..small.right() - 1).step_by(2) {
                    canvas.set(x, small.y - 1, c.pins);
                    canvas.set(x, small.bottom(), c.pins);
                }
            }
            let r = (b.h as f32 / 9.0).clamp(1.5, 3.5);
            let (cx, cy) = (
                (chip.right() as f32 + b.w as f32 * 0.14),
                (chip.y as f32 + r),
            );
            if cy + r < bus as f32 - 1.0 {
                disc(canvas, cx, cy, r + 0.6, c.silk);
                // a can seen from the top, silver with its pressed vent
                disc(canvas, cx, cy, r, c.knob);
                canvas.set(cx.round() as i32, cy.round() as i32, c.metal.0);
            }
        }
        // a resistor beside each LED on a big board
        if b.w >= 44 {
            for &(x, y) in &leds {
                canvas.fill_rect(x + lw / 2 - 1, y + lh + 1, 3, 1, c.pad);
            }
        }
        // each LED in its printed outline, then lit, warming or dark
        for (i, &(x, y)) in leds.iter().enumerate() {
            let status = i == leds.len() - 1;
            let state = self.led(i);
            let (body, core) = if status { c.green } else { c.amber };
            let (near, far) = if status { c.green_glow } else { c.amber_glow };
            if state != Led::Dark {
                let (cx, cy) = (x as f32 + lw as f32 / 2.0, y as f32 + lh as f32 / 2.0);
                let reach = if self.tall() >= 40 { 2.6 } else { 2.0 };
                for py in (cy - reach - 1.0) as i32..=(cy + reach + 1.0) as i32 {
                    for px in (cx - reach * 1.4 - 1.0) as i32..=(cx + reach * 1.4 + 1.0) as i32 {
                        let dx = (px as f32 + 0.5 - cx) / 1.15;
                        let dy = py as f32 + 0.5 - cy;
                        let d = (dx * dx + dy * dy).sqrt();
                        let inside = px >= b.x && px < b.right() && py >= b.y && py < b.bottom();
                        if inside && d <= reach {
                            let near_ring = state == Led::Lit && d <= reach * 0.6;
                            canvas.set(px, py, if near_ring { near } else { far });
                        }
                    }
                }
            }
            if state != Led::Lit {
                // the printed marks above and below a lens, as wide as the lens so neighbouring
                // outlines never join into one bar
                canvas.fill_rect(x, y - 1, lw, 1, c.silk);
                canvas.fill_rect(x, y + lh, lw, 1, c.silk);
            }
            let lens = match state {
                Led::Lit => body,
                Led::Faint => near,
                Led::Dark => c.lens,
            };
            canvas.fill_rect(x, y, lw, lh, lens);
            if state == Led::Lit {
                canvas.fill_rect(x, y, lw - 1, 1, core);
            }
        }
        // the legend's face, framed by its printed outline, which glows green with power good
        if let Some((row, x)) = self.legend() {
            let frame = if self.good() { c.green_glow.0 } else { c.silk };
            canvas.fill_rect(x - 1, 2 * row - 1, 12, 4, frame);
            canvas.fill_rect(x, 2 * row, 10, 2, c.face);
        }
    }

    fn paint_leads(&self, canvas: &mut Canvas, c: &Colours) {
        let b = self.board();
        // a match on both rather than a let chain, which needs a newer Rust than the crate's minimum
        if let (Some(s), true) = (self.supply(), b.h >= 8) {
            let (rx, bx) = self.posts(s);
            let y = s.bottom() - 4;
            let header = (b.right() - 3, b.y + 2);
            for (i, (px, colour)) in [(rx, c.red_post), (bx, c.cable)].into_iter().enumerate() {
                let from = (px as f32, (y + 1) as f32);
                let to = ((header.0 + i as i32) as f32, (header.1 + 1) as f32);
                let sag = (from.0 + to.0) / 2.0;
                let low = from.1.max(to.1) + 3.0 + i as f32;
                curve(canvas, from, (sag, low), to, colour);
            }
        }
        if let (Some((tip, back)), Some((ix, iy))) = (self.probe(), self.input()) {
            // the lead sags along the mat in front of the board to the scope's input
            let from = (back.0 - 1.0, back.1 + 0.5);
            let mid = ((from.0 + ix as f32) / 2.0, (from.1 + iy as f32) / 2.0 + 2.0);
            curve(canvas, from, mid, (ix as f32, iy as f32), c.cable);
            // the probe's body, a band near its grip and the bright tip
            let (dx, dy) = (tip.0 - back.0, tip.1 - back.1);
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            let (ux, uy) = (dx / len, dy / len);
            let neck = (tip.0 - ux * 2.0, tip.1 - uy * 2.0);
            let thick = if self.tall() >= 40 { 2 } else { 1 };
            paint::line(canvas, back, neck, thick, c.band);
            let band = (back.0 + ux * len * 0.55, back.1 + uy * len * 0.55);
            paint::line(
                canvas,
                band,
                (band.0 + ux * 1.5, band.1 + uy * 1.5),
                thick,
                c.probe,
            );
            paint::line(canvas, neck, tip, 1, c.tip);
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        self.paint_wall(canvas, c);
        self.paint_bench(canvas, c);
        self.paint_scope(canvas, c);
        self.paint_supply(canvas, c);
        self.paint_lamp(canvas, c);
        self.paint_board(canvas, c);
        self.paint_leads(canvas, c);
    }

    pub(crate) fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let c = Colours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &c);
        canvas.blit(frame);
        if let Some((row, x)) = self.display() {
            let amps = 0.04 * self.lit_count() as f32;
            paint::label(frame, x + 1, row, "5.00V", c.readout);
            paint::label(frame, x + 1, row + 1, &format!("{amps:.2}A"), c.readout);
        }
        if let Some((row, x)) = self.legend() {
            let colour = if self.good() {
                c.legend_good
            } else {
                c.legend_dim
            };
            paint::label(frame, x, row, "POWER GOOD", colour);
        }
    }
}

/// `t` nudged by an ordered 2 by 2 pattern fixed to the pixel, so the boundary between two steps
/// of a `steps` long ramp breaks into a fine checker instead of a hard ring.
fn dither(t: f32, x: i32, y: i32, steps: usize) -> f32 {
    let order = [0.0, 0.5, 0.75, 0.25][((x & 1) + 2 * (y & 1)) as usize];
    t + (order - 0.375) / (steps.max(2) - 1) as f32
}

/// A quadratic curve of one pixel lines from `a` through the pull of `b` to `c`.
fn curve(canvas: &mut Canvas, a: (f32, f32), b: (f32, f32), c: (f32, f32), colour: Rgb) {
    let steps = 24;
    let at = |t: f32| {
        let u = 1.0 - t;
        (
            u * u * a.0 + 2.0 * u * t * b.0 + t * t * c.0,
            u * u * a.1 + 2.0 * u * t * b.1 + t * t * c.1,
        )
    };
    let mut last = at(0.0);
    for i in 1..=steps {
        let next = at(i as f32 / steps as f32);
        canvas.line(
            (last.0.round() as i32, last.1.round() as i32),
            (next.0.round() as i32, next.1.round() as i32),
            colour,
        );
        last = next;
    }
}

pub struct PcbBringUp {
    bench: Bench,
}

impl PcbBringUp {
    pub fn new(width: usize, height: usize, motion: Motion, _rng: &mut Rng) -> Self {
        Self {
            bench: Bench::new(width, height, motion, Story::BringUp),
        }
    }
}

impl Scene for PcbBringUp {
    fn name(&self) -> &'static str {
        "pcb-bring-up"
    }

    fn resize(&mut self, width: usize, height: usize, _rng: &mut Rng) {
        self.bench.resize(width, height);
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.bench.step(dt);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        self.bench.draw(frame, theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixels::IMAGERY_CEILING;
    use crate::scenes::testing::{apart, check_determinism, check_pixel_scene, check_scene};
    use crate::scenes::testing::{pixels, render};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("pcb-bring-up");
        check_determinism("pcb-bring-up");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("pcb-bring-up", 25.0);
    }

    #[test]
    fn leds_light_in_order_no_faster_than_the_step_and_end_on_power_good() {
        for motion in Motion::ALL {
            let mut bench = Bench::new(80, 24, motion, Story::BringUp);
            let n = bench.chain();
            let mut first_lit: Vec<Option<f32>> = vec![None; n + 1];
            let mut t = 0.0_f32;
            while !bench.good() {
                bench.step(0.05);
                t += 0.05;
                for (i, slot) in first_lit.iter_mut().enumerate() {
                    if slot.is_none() && bench.led(i) == Led::Lit {
                        *slot = Some(t);
                    }
                }
                assert!(t < 120.0, "never reached power good with {motion:?}");
            }
            let times: Vec<f32> = first_lit.iter().map(|s| s.expect("lit")).collect();
            for w in times.windows(2) {
                assert!(w[1] - w[0] >= STEP - 0.06, "{motion:?}: {times:?}");
            }
            // power good holds steady for the whole hold
            for _ in 0..(30 * 20) {
                bench.step(0.05);
                assert!(bench.good(), "{motion:?}");
            }
        }
    }

    #[test]
    fn the_status_led_and_legend_glow_green_with_power_good() {
        let theme = theme::default_theme();
        let c = Colours::new(theme);
        let mut rng = Rng::new(1);
        let mut scene = PcbBringUp::new(100, 30, Motion::FULL, &mut rng);
        let frame = render(&mut scene, 100, 30, 25 * 20);
        assert!(scene.bench.good());
        let all = pixels(&frame);
        for (what, colour) in [
            ("status LED", c.green.0),
            ("lit chain", c.amber.0),
            ("glow", c.green_glow.0),
            ("probe", c.probe),
            ("supply", c.supply.1),
            ("scope", c.scope.1),
        ] {
            assert!(all.contains(&colour), "no {what}");
        }
        let text = frame.to_text();
        assert!(text.contains("POWER GOOD") && text.contains("5.00V"));
        // after the hold the supply goes off and the chain starts again from dark
        let mut bench = Bench::new(80, 24, Motion::FULL, Story::BringUp);
        let off = bench.lit_by() + HOLD + FADE + 0.5;
        while bench.clock < off {
            bench.step(0.05);
        }
        assert!((0..=bench.chain()).all(|i| bench.led(i) == Led::Dark));
    }

    #[test]
    fn the_layers_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            apart(
                theme,
                c.mask.at(0.5),
                c.mat.at(0.5),
                0.03,
                "the board and the mat",
            );
            apart(theme, c.wall.at(0.5), c.hole.at(0.5), 0.02, "the pegboard");
            apart(
                theme,
                c.lens,
                c.mask.at(0.5),
                0.03,
                "a dark LED and the board",
            );
            apart(
                theme,
                c.amber.0,
                c.green.0,
                0.08,
                "the chain and the status LED",
            );
            apart(
                theme,
                c.supply.1,
                c.scope.1,
                0.03,
                "the supply and the scope",
            );
            for colour in c
                .wall
                .steps()
                .iter()
                .chain(c.hole.steps())
                .chain(c.mat.steps())
                .chain(c.mask.steps())
                .chain([
                    &c.lens,
                    &c.halo,
                    &c.copper,
                    &c.amber_glow.0,
                    &c.green_glow.0,
                    &c.graticule,
                    &c.shadow,
                    &c.lip,
                ])
            {
                assert!(
                    theme.contrast_of(*colour) < IMAGERY_CEILING,
                    "{} in {}",
                    colour.hex(),
                    theme.name
                );
            }
            for colour in [
                c.amber.0, c.green.0, c.probe, c.metal.1, c.supply.1, c.scope.1,
            ] {
                assert!(
                    theme.contrast_of(colour) >= paint::SOLID - 0.01,
                    "{} in {}",
                    colour.hex(),
                    theme.name
                );
            }
        }
    }
}
