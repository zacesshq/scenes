//! Fireflies: a dark meadow with lights that wander and pulse slowly.

use std::f32::consts::TAU;

use super::count_for;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// The quickest pulse, in seconds per cycle, well under one flash a second.
pub const MIN_PULSE_PERIOD: f32 = 2.5;

#[derive(Clone, Debug)]
struct Fly {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    phase: f32,
    period: f32,
}

pub struct Fireflies {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    flies: Vec<Fly>,
    grass: Vec<char>,
    stalks: Vec<(usize, char)>,
}

impl Fireflies {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut meadow = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            flies: Vec::new(),
            grass: Vec::new(),
            stalks: Vec::new(),
        };
        meadow.resize(width, height, rng);
        meadow
    }

    /// The last row fireflies may reach: just above the grass.
    fn sky_bottom(&self) -> f32 {
        (self.height as f32 - 2.0).max(0.0)
    }

    fn top_speed(&self) -> f32 {
        if self.motion.calm { 0.7 } else { 1.5 }
    }
}

impl Scene for Fireflies {
    fn name(&self) -> &'static str {
        "fireflies"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = count_for(self.width, self.height, 45, 3, 40);
        let (low, high) = if self.motion.calm {
            (MIN_PULSE_PERIOD * 2.0, 9.0)
        } else {
            (MIN_PULSE_PERIOD, 5.0)
        };
        let top_speed = self.top_speed();
        self.flies = (0..wanted)
            .map(|_| Fly {
                x: rng.range(0.0, self.width as f32),
                y: rng.range(0.0, self.sky_bottom()),
                vx: rng.range(-top_speed, top_speed),
                vy: rng.range(-top_speed, top_speed) * 0.5,
                phase: rng.range(0.0, TAU),
                period: rng.range(low, high),
            })
            .collect();
        self.grass = (0..self.width)
            .map(|_| *rng.pick(&[',', '.', '\'', '|', '"', ',', '.', ' ']))
            .collect();
        self.stalks.clear();
        for x in 0..self.width {
            if rng.chance(0.12) {
                self.stalks.push((x, *rng.pick(&['|', 'Y', '\\', '/'])));
            }
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let top_speed = self.top_speed();
        let width = self.width as f32;
        let bottom = self.sky_bottom();
        for fly in &mut self.flies {
            fly.vx = (fly.vx + rng.range(-2.0, 2.0) * dt).clamp(-top_speed, top_speed);
            fly.vy = (fly.vy + rng.range(-1.0, 1.0) * dt).clamp(-top_speed * 0.5, top_speed * 0.5);
            fly.x += fly.vx * dt;
            fly.y += fly.vy * dt;
            if fly.x < 0.0 {
                fly.x = 0.0;
                fly.vx = fly.vx.abs();
            } else if fly.x > width - 1.0 {
                fly.x = (width - 1.0).max(0.0);
                fly.vx = -fly.vx.abs();
            }
            if fly.y < 0.0 {
                fly.y = 0.0;
                fly.vy = fly.vy.abs();
            } else if fly.y > bottom {
                fly.y = bottom;
                fly.vy = -fly.vy.abs();
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let ground = self.height as i32 - 1;
        for (x, ch) in self.grass.iter().enumerate() {
            if *ch != ' ' {
                frame.put(x as i32, ground, *ch, Some(theme.green()));
            }
        }
        if ground >= 1 {
            for &(x, ch) in &self.stalks {
                frame.put(x as i32, ground - 1, ch, Some(theme.green()));
            }
        }
        for fly in &self.flies {
            let level = ((super::cycle(self.t, TAU / fly.period) + fly.phase).sin() + 1.0) / 2.0;
            let (ch, colour) = if level < 0.25 {
                continue;
            } else if level < 0.5 {
                ('.', theme.yellow())
            } else if level < 0.8 {
                ('o', theme.yellow())
            } else {
                ('*', theme.bright_yellow())
            };
            frame.put(fly.x.round() as i32, fly.y.round() as i32, ch, Some(colour));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("fireflies");
        check_determinism("fireflies");
    }

    #[test]
    fn lights_pulse_slowly_over_a_grassy_bottom_row() {
        let mut rng = Rng::new(6);
        let mut meadow = Fireflies::new(60, 20, Motion::FULL, &mut rng);
        assert!(meadow.flies.iter().all(|f| f.period >= MIN_PULSE_PERIOD));
        let mut glyphs = std::collections::BTreeSet::new();
        for _ in 0..60 {
            let frame = render(&mut meadow, 60, 20, 1);
            for y in 0..18 {
                for ch in frame.row_text(y).chars() {
                    if ch != ' ' {
                        glyphs.insert(ch);
                    }
                }
            }
        }
        assert!(glyphs.contains(&'*') && glyphs.contains(&'o'), "{glyphs:?}");
        let frame = render(&mut meadow, 60, 20, 1);
        assert!(frame.row_text(19).trim().len() > 20);
    }

    #[test]
    fn flies_stay_above_the_grass_and_inside_the_screen() {
        let mut rng = Rng::new(14);
        let mut meadow = Fireflies::new(20, 8, Motion::CALM, &mut rng);
        for _ in 0..(60 * 20) {
            meadow.step(0.05, &mut rng);
            for fly in &meadow.flies {
                assert!((0.0..20.0).contains(&fly.x), "{}", fly.x);
                assert!((0.0..=6.0).contains(&fly.y), "{}", fly.y);
                assert!(fly.vx.abs() <= 0.7);
            }
        }
    }
}
