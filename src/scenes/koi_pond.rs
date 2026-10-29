//! Koi pond: a garden pond seen from above, with a few large koi gliding across it on slow curves,
//! their tails swaying as they swim, lily pads they slip beneath and now and then a ripple
//! spreading where something touched the surface.

use std::f32::consts::TAU;

use super::{count_for, cycle, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "A garden pond from above, with koi gliding under lily pads and slow ripples";

pub const CAPTIONS: &[&str] = &[
    "Still water, clean build.",
    "The koi have nothing to report.",
    "Not a ripple of trouble.",
    "Everything compiles. Feed the fish.",
    "Calm on the surface and underneath.",
];

/// Seconds each ring of a ripple stays before it widens. A ripple cell changes at most once a
/// stage, so well under twice a second.
pub const RIPPLE_STAGE: f32 = 0.6;
/// Seconds a koi's tail holds each of its two positions as it sways.
pub const TAIL_SWAY: f32 = 1.2;
/// The ripple's rings, from the touch outwards.
const RIPPLE_STAGES: usize = 4;

const PAD_LARGE: [&str; 3] = [" ,--. ", "(   <)", " `--' "];
const PAD_SMALL: [&str; 1] = ["(_<)"];

/// A koi seen from above, facing right, with its tail spread and then closed. The body's spots
/// and eye are drawn over it in their own colours.
const KOI_RIGHT: [[&str; 3]; 2] = [
    ["\\ .-~~-.  ", " >(     o)", "/ `-~~-'  "],
    ["  .-~~-.  ", "=>(     o)", "  `-~~-'  "],
];
const KOI_LEFT: [[&str; 3]; 2] = [
    ["  .-~~-. /", "(o     )< ", "  `-~~-' \\"],
    ["  .-~~-.  ", "(o     )<=", "  `-~~-'  "],
];
/// A small koi for a small screen, one row high.
const SMALL_RIGHT: [&str; 2] = ["><(*o)", "=<(*o)"];
const SMALL_LEFT: [&str; 2] = ["(o*)><", "(o*)>="];
const KOI_WIDTH: f32 = 10.0;

#[derive(Clone, Debug)]
struct Koi {
    x: f32,
    /// The row the koi's curve is centred on.
    base: f32,
    /// How far and how slowly it curves up and down as it swims.
    swing: f32,
    swing_rate: f32,
    phase: f32,
    speed: f32,
    /// Which of the colour pairs it wears.
    coat: usize,
    /// Where its spots sit along the body.
    spots: [i32; 2],
}

#[derive(Clone, Debug)]
struct Pad {
    x: i32,
    y: i32,
    large: bool,
    flower: bool,
}

#[derive(Clone, Debug)]
struct Ripple {
    x: i32,
    y: i32,
    age: f32,
}

pub struct KoiPond {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    koi: Vec<Koi>,
    pads: Vec<Pad>,
    flecks: Vec<(f32, i32)>,
    ripples: Vec<Ripple>,
    next_ripple: f32,
    rim: Vec<char>,
    tufts: Vec<i32>,
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
            flecks: Vec::new(),
            ripples: Vec::new(),
            next_ripple: rng.range(1.0, 3.0),
            rim: Vec::new(),
            tufts: Vec::new(),
        };
        pond.resize(width, height, rng);
        pond
    }

    /// The first and last rows of open water.
    fn water(&self) -> (f32, f32) {
        if self.height >= 5 {
            (1.0, self.height as f32 - 2.0)
        } else {
            (0.0, self.height as f32 - 1.0)
        }
    }

    /// Whether the large koi fit: each needs three rows and room to curve.
    fn large(&self) -> bool {
        self.width >= 40 && self.height >= 12
    }

    fn ripple_stage(&self) -> f32 {
        RIPPLE_STAGE / self.motion.pace()
    }

    fn tail_period(&self) -> f32 {
        TAIL_SWAY / self.motion.pace()
    }

    fn spawn_koi(&self, rng: &mut Rng, anywhere: bool) -> Koi {
        let (top, bottom) = self.water();
        let rows = if self.large() { 3.0 } else { 1.0 };
        let room = (bottom - top - rows + 1.0).max(0.0);
        let swing = if self.large() {
            rng.range(1.0, (room / 3.0).clamp(1.0, 4.0))
        } else {
            (room / 3.0).min(1.0)
        };
        let base = top + swing + rng.range(0.0, (room - 2.0 * swing).max(0.0));
        let right = rng.chance(0.5);
        let speed = rng.range(1.2, 2.2) * self.motion.speed();
        let width = self.width as f32;
        Koi {
            x: if anywhere {
                rng.range(0.0, width)
            } else if right {
                -KOI_WIDTH
            } else {
                width
            },
            base,
            swing,
            swing_rate: rng.range(0.25, 0.45),
            phase: rng.range(0.0, TAU),
            speed: if right { speed } else { -speed },
            coat: rng.below(4),
            spots: [3 + rng.below(2) as i32, 5 + rng.below(2) as i32],
        }
    }

    fn row_of(&self, koi: &Koi) -> f32 {
        let curve = (cycle(self.t, koi.swing_rate * self.motion.speed()) + koi.phase).sin();
        koi.base + koi.swing * curve
    }

    fn coat(theme: &ThemeData, coat: usize) -> (Rgb, Rgb) {
        match coat {
            0 => (theme.bright_red(), theme.foreground),
            1 => (theme.bright_yellow(), theme.red()),
            2 => (theme.foreground, theme.bright_red()),
            _ => (theme.yellow(), theme.foreground),
        }
    }
}

impl Scene for KoiPond {
    fn name(&self) -> &'static str {
        "koi-pond"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = self
            .motion
            .scaled(count_for(self.width, self.height, 500, 1, 7));
        self.koi = (0..wanted).map(|_| self.spawn_koi(rng, true)).collect();

        let (top, bottom) = self.water();
        self.pads.clear();
        let pad_count = count_for(self.width, self.height, 300, 1, 10);
        for _ in 0..pad_count * 4 {
            if self.pads.len() >= pad_count {
                break;
            }
            let large = self.width >= 30 && bottom - top >= 4.0 && rng.chance(0.6);
            let (w, h) = if large { (6, 3) } else { (4, 1) };
            if self.width < w + 2 || bottom - top < h as f32 {
                break;
            }
            let x = rng.below(self.width - w) as i32;
            let y = top as i32 + rng.below((bottom - top) as usize + 2 - h) as i32;
            let clash = self.pads.iter().any(|p| {
                let (pw, ph) = if p.large { (6, 3) } else { (4, 1) };
                x < p.x + pw + 3
                    && p.x < x + w as i32 + 3
                    && y < p.y + ph + 1
                    && p.y < y + h as i32 + 1
            });
            if !clash {
                self.pads.push(Pad {
                    x,
                    y,
                    large,
                    flower: large && rng.chance(0.5),
                });
            }
        }

        let flecks = count_for(self.width, self.height, 120, 2, 80);
        self.flecks = (0..flecks)
            .map(|_| {
                (
                    rng.range(0.0, self.width as f32),
                    rng.range(top, bottom + 1.0) as i32,
                )
            })
            .collect();
        self.ripples.clear();
        // the rim is a run of rounded stones with the odd pebble between them
        self.rim.clear();
        while self.rim.len() < self.width {
            self.rim.push('(');
            for _ in 0..1 + rng.below(3) {
                self.rim.push('_');
            }
            self.rim.push(')');
            if rng.chance(0.3) {
                self.rim.push(if rng.chance(0.5) { 'o' } else { '.' });
            }
        }
        self.rim.truncate(self.width);
        // the far bank is a plain line of grass with a tuft here and there
        let mut x = 3 + rng.below(6) as i32;
        self.tufts.clear();
        while x < self.width as i32 {
            self.tufts.push(x);
            x += 9 + rng.below(10) as i32;
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let width = self.width as f32;
        for i in 0..self.koi.len() {
            let koi = &mut self.koi[i];
            koi.x += koi.speed * dt;
            if koi.x > width + 1.0 || koi.x < -KOI_WIDTH - 1.0 {
                // a koi that has crossed the pond comes back in from either side
                self.koi[i] = self.spawn_koi(rng, false);
            }
        }

        let current = 0.2 * self.motion.speed();
        for fleck in &mut self.flecks {
            fleck.0 = wrap(fleck.0 + current * dt, width);
        }

        let stage = self.ripple_stage();
        for ripple in &mut self.ripples {
            ripple.age += dt;
        }
        self.ripples
            .retain(|r| r.age < stage * RIPPLE_STAGES as f32);
        self.next_ripple -= dt;
        if self.next_ripple <= 0.0 {
            self.next_ripple = if self.motion.calm {
                rng.range(5.0, 10.0)
            } else {
                rng.range(2.5, 6.0) / self.motion.busy()
            };
            let (top, bottom) = self.water();
            if self.ripples.len() < 4 && self.width >= 10 {
                self.ripples.push(Ripple {
                    x: rng.range(3.0, width - 3.0) as i32,
                    y: rng.range(top, bottom + 0.99) as i32,
                    age: 0.0,
                });
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let (top, bottom) = self.water();
        let ground = self.height as i32 - 1;
        if self.height >= 5 {
            frame.fill_row(0, '_', Some(theme.green()));
            for &x in &self.tufts {
                frame.put_str(x, 0, "\\|/", Some(theme.bright_green()));
            }
            for (x, ch) in self.rim.iter().enumerate() {
                frame.put(x as i32, ground, *ch, Some(theme.grey()));
            }
        }

        for &(x, y) in &self.flecks {
            frame.put(x as i32, y, '~', Some(theme.blue()));
        }

        let stage = self.ripple_stage();
        for ripple in &self.ripples {
            let ring = (ripple.age / stage) as usize;
            let colour = Some(if ring < 2 { theme.cyan() } else { theme.muted });
            let (x, y) = (ripple.x, ripple.y);
            match ring {
                0 => frame.put(x, y, 'o', colour),
                1 => frame.put_str(x - 1, y, "( )", colour),
                2 => {
                    frame.put(x - 2, y, '(', colour);
                    frame.put(x + 2, y, ')', colour);
                    if (y - 1) as f32 >= top {
                        frame.put_str(x - 1, y - 1, "._.", colour);
                    }
                    if (y + 1) as f32 <= bottom {
                        frame.put_str(x - 1, y + 1, "'-'", colour);
                    }
                }
                _ => {
                    frame.put(x - 4, y, '.', colour);
                    frame.put(x + 4, y, '.', colour);
                }
            }
        }

        let sway = (self.t / f64::from(self.tail_period())) as usize;
        for (i, koi) in self.koi.iter().enumerate() {
            let (main, spot) = Self::coat(theme, koi.coat);
            let frame_no = if self.motion.calm { 0 } else { (sway + i) % 2 };
            let right = koi.speed > 0.0;
            let x = koi.x.round() as i32;
            let y = self.row_of(koi).round() as i32;
            if self.large() {
                let rows = if right {
                    &KOI_RIGHT[frame_no]
                } else {
                    &KOI_LEFT[frame_no]
                };
                // the body is solid, so the water's flecks never show through the fish
                for dy in 0..3 {
                    frame.put_str(x + 3, y + dy, "    ", None);
                }
                frame.put_sprite(x, y, rows, Some(main));
                for &s in &koi.spots {
                    let sx = if right { x + s } else { x + 9 - s };
                    frame.put(sx, y + 1, '*', Some(spot));
                    frame.put(sx, y + (s % 2) * 2, '~', Some(spot));
                }
            } else {
                let rows = if right { &SMALL_RIGHT } else { &SMALL_LEFT };
                frame.put_str(x, y, rows[frame_no], Some(main));
                let sx = if right { x + 3 } else { x + 2 };
                frame.put(sx, y, '*', Some(spot));
            }
        }

        // the pads are drawn after the koi so a fish slips underneath them
        for pad in &self.pads {
            if pad.large {
                frame.put_sprite(pad.x, pad.y, &PAD_LARGE, Some(theme.green()));
                // the pad's middle is solid leaf, so a koi below it is hidden
                frame.put_str(pad.x + 1, pad.y + 1, "   ", None);
                if pad.flower {
                    frame.put(pad.x + 2, pad.y + 1, '*', Some(theme.bright_magenta()));
                    frame.put(pad.x + 3, pad.y + 1, '.', Some(theme.magenta()));
                }
            } else {
                frame.put_sprite(pad.x, pad.y, &PAD_SMALL, Some(theme.bright_green()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("koi-pond");
        check_determinism("koi-pond");
    }

    #[test]
    fn ripples_widen_slowly_and_tails_sway_slowly() {
        const { assert!(RIPPLE_STAGE >= 1.0 / 3.0) };
        const { assert!(TAIL_SWAY >= 1.0 / 3.0) };
        for motion in Motion::ALL {
            let mut rng = Rng::new(1);
            let pond = KoiPond::new(60, 20, motion, &mut rng);
            assert!(pond.ripple_stage() >= RIPPLE_STAGE);
            assert!(pond.tail_period() >= TAIL_SWAY);
        }
    }

    #[test]
    fn koi_stay_in_the_water_and_pads_are_drawn() {
        let mut rng = Rng::new(4);
        let mut pond = KoiPond::new(60, 20, Motion::LIVELY, &mut rng);
        for _ in 0..(120 * 20) {
            pond.step(0.05, &mut rng);
            for koi in &pond.koi {
                let y = pond.row_of(koi);
                assert!((1.0..=16.5).contains(&y), "{y}");
            }
        }
        let frame = render(&mut pond, 60, 20, 1);
        let text = frame.to_text();
        assert!(text.contains("<)"), "{text}");
        assert!(text.contains(".-~~-."), "{text}");
        assert!(frame.row_text(19).contains('_'));
        let small = render(&mut KoiPond::new(20, 8, Motion::CALM, &mut rng), 20, 8, 20);
        assert!(small.drawn() > 10);
    }
}
