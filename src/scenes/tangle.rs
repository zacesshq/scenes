//! Tangle: a patch panel along the top with cables hanging straight down from its ports. Over
//! the first minute they wander across each other, loop and knot, until they hang in a snarl
//! that sways slowly, with a warning tag on one of them and coils heaped on the floor.

use std::f32::consts::PI;

use super::{count_for, cycle};
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::warning_colour;

/// Seconds the cables take to tangle at the standard pace.
pub const TANGLE_SECONDS: f32 = 45.0;
/// Radians a second the snarl sways by. It moves the cables a fraction of a cell a second, so a
/// cell changes as a cable drifts across it, never by flicking back and forth.
const SWAY_RATE: f32 = 0.35;

pub const SUMMARY: &str =
    "Cables hanging from a patch panel that slowly wander, cross and knot into a snarl";

pub const CAPTIONS: &[&str] = &[
    "A few crossed wires. Trace the first one back.",
    "Untangle one cable at a time.",
    "Follow the error to the port it came from.",
    "Knotted, not cut. It comes apart with patience.",
    "Labels help. So does the first error.",
];

#[derive(Clone, Debug)]
struct Cable {
    port: f32,
    /// Where the bottom end wanders to once fully tangled, as an offset from the port.
    reach: f32,
    amplitude: f32,
    waves: f32,
    phase: f32,
    colour: usize,
    knot: Option<f32>,
}

pub struct Tangle {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// Seconds of tangling so far, kept across a resize so the cables never straighten.
    tangled: f32,
    cables: Vec<Cable>,
    tagged: usize,
}

fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

impl Tangle {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut tangle = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            tangled: 0.0,
            cables: Vec::new(),
            tagged: 0,
        };
        tangle.resize(width, height, rng);
        tangle
    }

    /// The top row the cables hang from, under the panel.
    fn top(&self) -> i32 {
        2
    }

    /// The row the cables end on, just above the floor.
    fn bottom(&self) -> i32 {
        (self.height as i32 - 2).max(self.top())
    }

    fn amount(&self) -> f32 {
        ease(self.tangled / TANGLE_SECONDS)
    }

    /// Where a cable crosses a row.
    fn x_at(&self, cable: &Cable, y: i32) -> f32 {
        let span = (self.bottom() - self.top()).max(1) as f32;
        let s = ((y - self.top()) as f32 / span).clamp(0.0, 1.0);
        let amount = self.amount();
        let sway = cycle(self.t, SWAY_RATE * self.motion.speed());
        let drift = cable.reach * amount * ease(s);
        let wave = cable.amplitude
            * amount
            * (PI * s).sin()
            * (s * PI * cable.waves + cable.phase + sway).sin();
        (cable.port + drift + wave).clamp(0.0, (self.width as f32 - 1.0).max(0.0))
    }
}

impl Scene for Tangle {
    fn name(&self) -> &'static str {
        "tangle"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = self
            .motion
            .scaled(count_for(self.width, self.height, 220, 3, 14))
            .min((self.width / 5).max(1));
        let gap = self.width as f32 / wanted as f32;
        let spread = (self.width as f32 / 4.0).max(2.0);
        let w = self.width as f32;
        self.cables = (0..wanted)
            .map(|i| {
                let port = gap * (i as f32 + 0.5);
                // the end wanders across its neighbours but always lands inside the screen
                let low = 2.0 - port;
                let reach = rng
                    .range(-spread, spread)
                    .clamp(low, (w - 3.0 - port).max(low));
                let room = port.min(w - 1.0 - port).max(1.0);
                Cable {
                    port,
                    reach,
                    amplitude: rng.range(1.5, 2.0 + w / 40.0).min(room),
                    waves: rng.range(1.0, 2.5),
                    phase: rng.range(0.0, 2.0 * PI),
                    colour: i % 6,
                    knot: rng.chance(0.5).then(|| rng.range(0.35, 0.75)),
                }
            })
            .collect();
        self.tagged = wanted / 2;
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
        self.tangled = (self.tangled + dt * self.motion.speed()).min(TANGLE_SECONDS * 2.0);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let w = self.width as i32;
        let colours = [
            theme.blue(),
            theme.green(),
            theme.magenta(),
            theme.cyan(),
            theme.yellow(),
            theme.red(),
        ];
        let top = self.top();
        let bottom = self.bottom();

        // the panel and its ports
        frame.fill_row(0, '=', Some(theme.grey()));
        if self.height >= 4 {
            for cable in &self.cables {
                let x = cable.port.round() as i32;
                frame.put_str(x - 1, 1, "[ ]", Some(theme.muted));
                frame.put(x, 1, 'o', Some(colours[cable.colour]));
            }
        }
        if self.height as i32 - 1 > top {
            frame.fill_row(self.height as i32 - 1, '_', Some(theme.grey()));
        }

        for cable in &self.cables {
            let colour = Some(colours[cable.colour]);
            for y in top..=bottom {
                let here = self.x_at(cable, y);
                let next = self.x_at(cable, y + 1);
                let (a, b) = (here.round() as i32, next.round() as i32);
                let dx = next - here;
                // a steep run is drawn as a short level stretch, so the cable never breaks
                if (b - a).abs() > 1 && y < bottom {
                    let step = (b - a).signum();
                    let mut x = a;
                    while x != b - step {
                        frame.put(x, y, '_', colour);
                        x += step;
                    }
                    frame.put(b - step, y, if step > 0 { '\\' } else { '/' }, colour);
                } else {
                    let ch = if dx.abs() < 0.3 || y == bottom {
                        '|'
                    } else if dx > 0.0 {
                        '\\'
                    } else {
                        '/'
                    };
                    frame.put(a, y, ch, colour);
                }
            }
            // a knot where the cable has looped back on itself
            if let Some(at) = cable.knot.filter(|_| self.amount() > 0.6) {
                let y = top + ((bottom - top) as f32 * at).round() as i32;
                frame.put(self.x_at(cable, y).round() as i32, y, '@', colour);
            }
            // the loose end lies coiled on the floor
            let end = self.x_at(cable, bottom).round() as i32;
            if self.amount() > 0.3 && bottom > top + 1 {
                frame.put_str(end - 1, bottom, "(@)", colour);
            }
        }

        // the warning tag hangs off one cable a third of the way down
        if let Some(cable) = self
            .cables
            .get(self.tagged)
            .filter(|_| w >= 16 && bottom - top >= 4)
        {
            let y = top + (bottom - top) / 3;
            let x = self.x_at(cable, y).round() as i32;
            let tag = if w >= 60 { "-[! tangled]" } else { "-[!]" };
            let left = if x + tag.chars().count() as i32 >= w {
                x - tag.chars().count() as i32
            } else {
                x + 1
            };
            frame.put_str(left, y, tag, Some(warning_colour(theme)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("tangle");
        check_determinism("tangle");
    }

    #[test]
    fn cables_hang_straight_then_tangle_and_stay_on_screen() {
        let mut rng = Rng::new(3);
        let mut tangle = Tangle::new(80, 24, Motion::FULL, &mut rng);
        let start = render(&mut tangle, 80, 24, 1);
        // at first every cable hangs straight down from its port
        for cable in &tangle.cables {
            let x = cable.port.round() as i32;
            assert_eq!(start.get(x, 10).map(|c| c.ch), Some('|'));
        }
        for _ in 0..(3600 * 20) {
            tangle.step(0.05, &mut rng);
        }
        assert!(tangle.amount() > 0.99);
        let frame = render(&mut tangle, 80, 24, 1);
        let text = frame.to_text();
        assert!(text.contains('@') && text.contains("[! tangled]") || text.contains("[!]"));
        for cable in &tangle.cables {
            for y in tangle.top()..=tangle.bottom() {
                let x = tangle.x_at(cable, y);
                assert!((0.0..80.0).contains(&x));
            }
        }
    }

    #[test]
    fn the_tangle_grows_slowly() {
        let mut rng = Rng::new(8);
        let mut tangle = Tangle::new(80, 24, Motion::LIVELY, &mut rng);
        let mut last = tangle.amount();
        for _ in 0..(120 * 20) {
            tangle.step(0.05, &mut rng);
            // never more than a thirtieth of the whole tangle in a second
            assert!((tangle.amount() - last) * 20.0 < 1.0 / 30.0 * 1.5);
            last = tangle.amount();
        }
    }
}
