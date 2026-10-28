//! Snow: flakes at two depths drift down and settle into a bank along the bottom.

use std::f32::consts::TAU;

use super::count_for;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// How much one flake adds to the bank, in rows.
const FLAKE_DEPTH: f32 = 0.34;

#[derive(Clone, Debug)]
struct Flake {
    x: f32,
    y: f32,
    speed: f32,
    phase: f32,
    near: bool,
}

pub struct Snow {
    width: usize,
    height: usize,
    motion: Motion,
    t: f32,
    flakes: Vec<Flake>,
    /// The depth of settled snow in each column, in rows.
    bank: Vec<f32>,
}

impl Snow {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut snow = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            flakes: Vec::new(),
            bank: Vec::new(),
        };
        snow.resize(width, height, rng);
        snow
    }

    fn new_flake(&self, rng: &mut Rng, from_top: bool) -> Flake {
        let near = rng.chance(0.45);
        let speed = if near {
            rng.range(2.5, 4.0)
        } else {
            rng.range(1.2, 2.0)
        };
        Flake {
            x: rng.range(0.0, self.width as f32),
            y: if from_top {
                -rng.range(0.0, 2.0)
            } else {
                rng.range(-1.0, self.height as f32)
            },
            speed: speed * self.motion.speed(),
            phase: rng.range(0.0, TAU),
            near,
        }
    }

    /// The deepest the bank may get: a third of the screen.
    fn max_depth(&self) -> f32 {
        ((self.height / 3) as f32).max(1.0)
    }

    #[cfg(test)]
    pub(crate) fn bank_depth(&self) -> f32 {
        self.bank.iter().copied().fold(0.0, f32::max)
    }
}

impl Scene for Snow {
    fn name(&self) -> &'static str {
        "snow"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = count_for(self.width, self.height, 14, 6, 400);
        self.flakes = (0..wanted).map(|_| self.new_flake(rng, false)).collect();
        self.bank = vec![0.0; self.width];
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += dt;
        let width = self.width as f32;
        let max_depth = self.max_depth();
        for i in 0..self.flakes.len() {
            let flake = &mut self.flakes[i];
            flake.y += flake.speed * dt;
            let drift = (self.t * 0.7 + flake.phase).sin() * 0.8 + 0.3;
            flake.x = super::wrap(flake.x + drift * self.motion.speed() * dt, width);
            let column = (flake.x as usize).min(self.width - 1);
            let surface = self.height as f32 - 1.0 - self.bank[column];
            if flake.y >= surface {
                if self.bank[column] < max_depth {
                    self.bank[column] += FLAKE_DEPTH;
                }
                // a column that stands more than a row above its neighbour sheds into it
                for neighbour in [column.wrapping_sub(1), column + 1] {
                    if neighbour < self.width && self.bank[column] > self.bank[neighbour] + 1.0 {
                        self.bank[column] -= FLAKE_DEPTH / 2.0;
                        self.bank[neighbour] += FLAKE_DEPTH / 2.0;
                    }
                }
                self.flakes[i] = self.new_flake(rng, true);
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let bank_colour = if theme.dark {
            theme.foreground
        } else {
            theme.grey()
        };
        let bottom = self.height as i32 - 1;
        for (x, depth) in self.bank.iter().enumerate() {
            let full = depth.floor() as i32;
            for i in 0..full {
                frame.put(x as i32, bottom - i, '\u{2588}', Some(bank_colour));
            }
            if depth - full as f32 >= 0.5 {
                frame.put(x as i32, bottom - full, '\u{2584}', Some(bank_colour));
            }
        }
        for flake in &self.flakes {
            let (ch, colour) = if flake.near {
                ('*', theme.foreground)
            } else {
                ('.', theme.muted)
            };
            let y = flake.y.floor() as i32;
            if y >= 0 {
                frame.put(flake.x.floor() as i32, y, ch, Some(colour));
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
        check_scene("snow");
        check_determinism("snow");
    }

    #[test]
    fn flakes_fall_and_settle_into_a_bank() {
        let mut rng = Rng::new(12);
        let mut snow = Snow::new(40, 12, Motion::FULL, &mut rng);
        let early = render(&mut snow, 40, 12, 5);
        assert!(early.to_text().contains('*') && early.to_text().contains('.'));
        for _ in 0..(40 * 20) {
            snow.step(0.05, &mut rng);
        }
        assert!(snow.bank_depth() >= 1.0, "{}", snow.bank_depth());
        let frame = render(&mut snow, 40, 12, 1);
        assert!(frame.row_text(11).contains('\u{2588}'));
    }

    #[test]
    fn the_bank_never_rises_above_a_third_of_the_screen() {
        let mut rng = Rng::new(2);
        let mut snow = Snow::new(20, 8, Motion::CALM, &mut rng);
        for _ in 0..(600 * 20) {
            snow.step(0.05, &mut rng);
        }
        assert!(snow.bank_depth() <= snow.max_depth() + FLAKE_DEPTH);
        assert!(snow.bank_depth() >= 1.0);
    }
}
