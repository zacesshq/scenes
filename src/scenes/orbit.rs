//! Orbit: a banded planet lit from the left, with a ring around it and a small moon on a slow
//! orbit that passes in front of the planet and behind it, among distant stars.

use std::f32::consts::TAU;

use super::count_for;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "A ringed planet among distant stars with a small moon on a slow orbit around it";

pub const CAPTIONS: &[&str] = &[
    "Everything in its orbit.",
    "A stable build, a stable orbit.",
    "Clear of debris. Green across the board.",
    "Ground control reports all good.",
    "Steady as a moon.",
];

/// Seconds the moon takes to go round once at the standard speed. Tens of seconds, so it drifts
/// rather than spins.
pub const ORBIT_PERIOD: f32 = 40.0;
/// Seconds the storm on the planet's face takes to cross it once.
const SPOT_PERIOD: f32 = 60.0;
/// Width of a terminal cell over its height, roughly: a circle is drawn twice as wide as tall.
const ASPECT: f32 = 2.0;

pub struct Orbit {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    stars: Vec<(i32, i32, bool)>,
}

impl Orbit {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut orbit = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            stars: Vec::new(),
        };
        orbit.resize(width, height, rng);
        orbit
    }

    /// The planet's radius in rows.
    fn radius(&self) -> f32 {
        (self.height as f32 * 0.28)
            .min(self.width as f32 * 0.13)
            .max(1.0)
    }

    fn centre(&self) -> (f32, f32) {
        (self.width as f32 / 2.0, self.height as f32 / 2.0 - 0.5)
    }

    /// The moon's angle, which grows by one turn per orbit period.
    fn moon_angle(&self) -> f32 {
        super::cycle(self.t, TAU / ORBIT_PERIOD * self.motion.speed())
    }

    /// The moon's position and whether it is in front of the planet.
    fn moon(&self) -> (f32, f32, bool) {
        let (cx, cy) = self.centre();
        let r = self.radius();
        let angle = self.moon_angle();
        let a = (r * ASPECT * 2.3)
            .min(self.width as f32 / 2.0 - 2.0)
            .max(r * ASPECT + 2.0);
        let b = r * 0.75;
        (
            cx + a * angle.cos(),
            cy + b * angle.sin(),
            angle.sin() > 0.0,
        )
    }

    fn inside_planet(&self, x: f32, y: f32) -> bool {
        let (cx, cy) = self.centre();
        let r = self.radius();
        let dx = (x - cx) / ASPECT;
        let dy = y - cy;
        dx * dx + dy * dy <= r * r
    }

    /// The ring's half widths: across in columns and up and down in rows.
    fn ring_axes(&self) -> (f32, f32) {
        let r = self.radius();
        (r * ASPECT * 1.75, (r * 0.45).max(1.0))
    }

    /// The ring's row at column `x` on its far half (`front` false) or its near half.
    fn ring_y(&self, x: f32, front: bool) -> f32 {
        let (cx, cy) = self.centre();
        let (a, b) = self.ring_axes();
        let u = ((x - cx) / a).clamp(-1.0, 1.0);
        let fy = b * (1.0 - u * u).sqrt();
        if front { cy + fy } else { cy - fy }
    }

    /// Draws one half of the ring as a single continuous curve: flat stretches as `-` or `_`
    /// by where the curve sits in the cell, slopes as `/` or `\` and the two ends as `(` and
    /// `)`. The far half is hidden where the planet covers it.
    fn draw_ring(&self, frame: &mut Frame, theme: &ThemeData, front: bool) {
        let (cx, cy) = self.centre();
        let (a, _) = self.ring_axes();
        let colour = Some(theme.muted);
        let left = (cx - a).round() as i32;
        let right = (cx + a).round() as i32;
        let put = |frame: &mut Frame, x: i32, y: i32, ch: char| {
            if front || !self.inside_planet(x as f32, y as f32) {
                frame.put(x, y, ch, colour);
            }
        };
        for x in left..=right {
            if x == left || x == right {
                // an end fills the rows between the two halves one column in, so it joins
                // them without a gap or a doubled corner
                let inner = if x == left {
                    x as f32 + 1.0
                } else {
                    x as f32 - 1.0
                };
                let back = self.ring_y(inner, false).round() as i32;
                let near = self.ring_y(inner, true).round() as i32;
                let middle = cy.round() as i32;
                let ch = if x == left { '(' } else { ')' };
                let (from, to) = if front {
                    (middle, (near - 1).max(middle))
                } else {
                    (back + 1, middle - 1)
                };
                for row in from..=to {
                    put(frame, x, row, ch);
                }
                continue;
            }
            let xf = x as f32;
            let y = self.ring_y(xf, front);
            let before = self.ring_y(xf - 0.5, front);
            let after = self.ring_y(xf + 0.5, front);
            let dy = after - before;
            if dy.abs() >= 0.6 {
                let ch = if dy < 0.0 { '/' } else { '\\' };
                let from = before.round() as i32;
                let to = after.round() as i32;
                for row in from.min(to)..=from.max(to) {
                    put(frame, x, row, ch);
                }
                continue;
            }
            let row = y.round();
            let off = y - row;
            if off > 0.22 {
                put(frame, x, row as i32, '_');
            } else if off < -0.22 {
                put(frame, x, row as i32 - 1, '_');
            } else {
                put(frame, x, row as i32, '-');
            }
        }
    }

    /// Draws the planet in shades rather than solid blocks: three levels of light across its
    /// day side, alternate bands a level fainter, a line of dots along the terminator and only
    /// a faint rim on its night side, so even a large planet is never one bright slab.
    fn draw_planet(&self, frame: &mut Frame, theme: &ThemeData) {
        let (cx, cy) = self.centre();
        let r = self.radius();
        let bands = [theme.yellow(), theme.bright_yellow(), theme.red()];
        let spot_angle =
            super::cycle(self.t, TAU / SPOT_PERIOD * self.motion.speed()) / TAU * 2.0 - 1.0;
        let faint = theme.faint_shade();
        let shades = [faint, '\u{2592}', '\u{2593}'];
        let top = (cy - r).floor() as i32;
        let bottom = (cy + r).ceil() as i32;
        let left = (cx - r * ASPECT).floor() as i32;
        let right = (cx + r * ASPECT).ceil() as i32;
        for y in top..=bottom {
            for x in left..=right {
                let nx = (x as f32 - cx) / (r * ASPECT);
                let ny = (y as f32 - cy) / r;
                let d = nx * nx + ny * ny;
                if d > 1.0 {
                    continue;
                }
                let nz = (1.0 - d).sqrt();
                // light comes from the upper left, so the right side falls into night
                let light = -0.6 * nx - 0.3 * ny + 0.75 * nz;
                let band = ((ny + 1.0) * 3.0) as usize;
                let spot = (nx - spot_angle * 0.8).abs() < 0.18 && (ny - 0.25).abs() < 0.16;
                if light < 0.0 {
                    // the night side shows only its rim, so the planet keeps its round outline
                    if d > 0.88 {
                        frame.put(x, y, faint, Some(theme.grey()));
                    }
                    continue;
                }
                if light < 0.08 {
                    frame.put(x, y, ':', Some(theme.grey()));
                    continue;
                }
                let level = if light > 0.6 {
                    2
                } else if light > 0.3 {
                    1
                } else {
                    0
                };
                // every other band sits a level fainter, which draws the stripes by shape too
                let level = if band % 2 == 1 {
                    level.max(1) - 1
                } else {
                    level
                };
                let colour = if spot {
                    theme.red()
                } else {
                    bands[band % bands.len()]
                };
                frame.put(x, y, shades[level], Some(colour));
            }
        }
    }
}

impl Scene for Orbit {
    fn name(&self) -> &'static str {
        "orbit"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = self
            .motion
            .scaled(count_for(self.width, self.height, 30, 4, 300));
        self.stars = (0..wanted)
            .map(|_| {
                (
                    rng.below(self.width) as i32,
                    rng.below(self.height) as i32,
                    rng.chance(0.2),
                )
            })
            .collect();
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        for &(x, y, bright) in &self.stars {
            if bright {
                frame.put(x, y, '+', Some(theme.foreground));
            } else {
                frame.put(x, y, '.', Some(theme.muted));
            }
        }
        let (mx, my, moon_front) = self.moon();
        let moon_char = if self.radius() >= 4.0 { 'O' } else { 'o' };
        let moon_hidden = !moon_front && self.inside_planet(mx.round(), my.round());
        if !moon_front && !moon_hidden {
            frame.put(
                mx.round() as i32,
                my.round() as i32,
                moon_char,
                Some(theme.bright()),
            );
        }
        self.draw_ring(frame, theme, false);
        self.draw_planet(frame, theme);
        self.draw_ring(frame, theme, true);
        if moon_front {
            frame.put(
                mx.round() as i32,
                my.round() as i32,
                moon_char,
                Some(theme.bright()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("orbit");
        check_determinism("orbit");
    }

    #[test]
    fn the_moon_orbits_slowly_and_passes_behind_the_planet() {
        const { assert!(ORBIT_PERIOD >= 20.0) };
        let mut rng = Rng::new(1);
        let mut orbit = Orbit::new(80, 24, Motion::LIVELY, &mut rng);
        let mut in_front = 0;
        let mut behind = 0;
        let mut last = orbit.moon_angle();
        for _ in 0..(60 * 20) {
            orbit.step(0.05, &mut rng);
            let angle = orbit.moon_angle();
            let turned = (angle - last).rem_euclid(TAU);
            // even lively, the moon moves well under a tenth of a turn a second
            assert!(turned * 20.0 < TAU / 10.0, "{turned}");
            last = angle;
            if orbit.moon().2 {
                in_front += 1;
            } else {
                behind += 1;
            }
        }
        assert!(in_front > 100 && behind > 100);
    }

    #[test]
    fn the_planet_ring_and_stars_are_drawn_at_every_size() {
        let mut rng = Rng::new(2);
        for (w, h) in [(20, 8), (80, 24), (200, 60)] {
            let frame = render(&mut Orbit::new(w, h, Motion::FULL, &mut rng), w, h, 10);
            let text = frame.to_text();
            // the planet is drawn in shades, never as one solid slab
            assert!(
                text.contains('\u{2593}') && text.contains('\u{2592}'),
                "{text}"
            );
            assert!(!text.contains('\u{2588}'), "{text}");
            // the moon can sit over one end of the ring, never both
            assert!(text.contains('(') || text.contains(')'), "{text}");
            assert!(text.contains('-') || text.contains('/'), "{text}");
        }
    }
}
