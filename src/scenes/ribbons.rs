//! Ribbons: a few broad ribbons of contour lines that flow slowly across the screen over a faint
//! grid of dots, each a pair of slow waves in one of the theme's colours.

use std::f32::consts::TAU;

use crate::colour::Rgb;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "Broad ribbons of contour lines flowing slowly across a faint grid of dots";

pub const CAPTIONS: &[&str] = &[
    "Everything flows.",
    "Smooth from end to end.",
    "No knots in this build.",
    "All lines lead to green.",
    "Steady, smooth and clean.",
];

/// The fastest a ribbon's crest travels sideways, in columns a second at the standard speed.
pub const MAX_DRIFT: f32 = 4.0;
/// Columns and rows between the dots of the background grid.
const GRID: (i32, i32) = (8, 4);

#[derive(Clone, Debug)]
struct Ribbon {
    base: f32,
    amplitude: f32,
    /// Wave numbers and angular speeds of the two waves that make the ribbon.
    k: (f32, f32),
    w: (f32, f32),
    phase: (f32, f32),
    colour: usize,
    /// The rows between the ribbon's contour lines.
    spread: f32,
}

pub struct Ribbons {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    ribbons: Vec<Ribbon>,
}

impl Ribbon {
    /// The row of the ribbon's centre at column `x` and time `t`.
    fn y(&self, x: f32, t: f64, speed: f32) -> f32 {
        let a = x * self.k.0 + super::cycle(t, self.w.0 * speed) + self.phase.0;
        let b = x * self.k.1 + super::cycle(t, self.w.1 * speed) + self.phase.1;
        self.base + self.amplitude * (a.sin() * 0.7 + b.sin() * 0.3)
    }
}

fn colours(theme: &ThemeData) -> [Rgb; 5] {
    [
        theme.cyan(),
        theme.magenta(),
        theme.green(),
        theme.yellow(),
        theme.blue(),
    ]
}

impl Ribbons {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut scene = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            ribbons: Vec::new(),
        };
        scene.resize(width, height, rng);
        scene
    }

    /// Draws one contour line across the screen, picking each cell's character from its slope so
    /// the line reads as one smooth curve.
    fn draw_line(
        &self,
        frame: &mut Frame,
        ribbon: &Ribbon,
        offset: f32,
        ch_flat: char,
        colour: Rgb,
    ) {
        let speed = self.motion.speed();
        let mut y = ribbon.y(0.0, self.t, speed) + offset;
        for x in 0..self.width as i32 {
            let next = ribbon.y(x as f32 + 1.0, self.t, speed) + offset;
            let dy = next - y;
            let row = y.round() as i32;
            // a dotted contour stays dotted on its slopes, so only the centre line is drawn in
            // strokes
            let ch = if dy.abs() < 0.35 || ch_flat == '.' {
                ch_flat
            } else if dy.abs() <= 1.3 {
                if dy > 0.0 { '\\' } else { '/' }
            } else {
                '|'
            };
            frame.put(x, row, ch, Some(colour));
            if ch == '|' || (ch == '.' && dy.abs() > 1.3) {
                let to = next.round() as i32;
                for fill in row.min(to) + 1..row.max(to) {
                    frame.put(x, fill, ch, Some(colour));
                }
            }
            y = next;
        }
    }
}

impl Scene for Ribbons {
    fn name(&self) -> &'static str {
        "ribbons"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let h = self.height as f32;
        let w = self.width as f32;
        let count = ((self.height / 9).clamp(1, 4) as f32 * self.motion.busy())
            .round()
            .clamp(1.0, 5.0) as usize;
        let start = rng.below(5);
        self.ribbons = (0..count)
            .map(|i| {
                let k0 = TAU / (w * rng.range(0.9, 1.6)).max(8.0);
                let k1 = k0 * rng.range(1.5, 2.2);
                // the wave speeds are chosen so neither wave's crest travels faster than the
                // drift limit
                let w0 = rng.range(0.4, 1.0) * MAX_DRIFT * k0;
                let w1 = -rng.range(0.2, 0.6) * MAX_DRIFT * k1;
                Ribbon {
                    base: h * (i as f32 + 1.0) / (count as f32 + 1.0),
                    amplitude: (h / (count as f32 + 1.0) * rng.range(0.3, 0.5)).max(0.5),
                    k: (k0, k1),
                    w: (w0, w1),
                    phase: (rng.range(0.0, TAU), rng.range(0.0, TAU)),
                    colour: (start + i) % 5,
                    spread: if h >= 16.0 { 1.0 } else { 0.0 },
                }
            })
            .collect();
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let (gx, gy) = GRID;
        let mut y = gy / 2;
        while y < self.height as i32 {
            let mut x = gx / 2;
            while x < self.width as i32 {
                frame.put(x, y, '\u{b7}', Some(theme.muted));
                x += gx;
            }
            y += gy;
        }
        let palette = colours(theme);
        for ribbon in &self.ribbons {
            let colour = palette[ribbon.colour];
            if ribbon.spread > 0.0 {
                // the outer contours are dotted, so the ribbon's centre stands out by shape too
                self.draw_line(frame, ribbon, -ribbon.spread * 1.5, '.', colour);
                self.draw_line(frame, ribbon, ribbon.spread * 1.5, '.', colour);
            }
            self.draw_line(frame, ribbon, 0.0, '-', colour);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("ribbons");
        check_determinism("ribbons");
    }

    #[test]
    fn ribbons_drift_no_faster_than_the_limit() {
        let mut rng = Rng::new(12);
        for motion in Motion::ALL {
            let scene = Ribbons::new(120, 40, motion, &mut rng);
            assert!(!scene.ribbons.is_empty());
            for ribbon in &scene.ribbons {
                let drift0 = ribbon.w.0.abs() / ribbon.k.0 * motion.speed();
                let drift1 = ribbon.w.1.abs() / ribbon.k.1 * motion.speed();
                let limit = MAX_DRIFT * motion.speed() + 0.001;
                assert!(drift0 <= limit && drift1 <= limit, "{drift0} {drift1}");
            }
        }
    }

    #[test]
    fn every_ribbon_crosses_the_screen() {
        let mut rng = Rng::new(3);
        let frame = render(
            &mut Ribbons::new(80, 24, Motion::FULL, &mut rng),
            80,
            24,
            40,
        );
        for x in [0, 40, 79] {
            let column: String = (0..24)
                .map(|y| frame.get(x, y).map_or(' ', |c| c.ch))
                .collect();
            assert!(column.contains('-') || column.contains('/') || column.contains('\\'));
        }
    }
}
