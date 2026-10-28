//! Waves: a calm sea to the horizon under a sun, with gulls crossing the sky.

use std::f32::consts::TAU;

use super::{count_for, wrap};
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// One wing beat, in seconds. Four wing positions a beat keeps every change under three a second.
const FLAP_PERIOD: f32 = 1.6;

#[derive(Clone, Debug)]
struct Gull {
    x: f32,
    y: i32,
    speed: f32,
    phase: f32,
}

const SUN: [&str; 3] = [" .-. ", "(   )", " `-' "];
const GULL_FRAMES: [&str; 4] = ["\\v/", "-v-", "/v\\", "-v-"];

pub struct Waves {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    gulls: Vec<Gull>,
}

impl Waves {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut waves = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            gulls: Vec::new(),
        };
        waves.resize(width, height, rng);
        waves
    }

    fn horizon(&self) -> i32 {
        ((self.height as f32 * 0.4) as i32).max(1)
    }

    fn flap_period(&self) -> f32 {
        FLAP_PERIOD / self.motion.speed()
    }
}

impl Scene for Waves {
    fn name(&self) -> &'static str {
        "waves"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = count_for(self.width, self.height, 300, 1, 6);
        let horizon = self.horizon();
        self.gulls = (0..wanted)
            .map(|_| {
                let right = rng.chance(0.5);
                Gull {
                    x: rng.range(0.0, self.width as f32),
                    y: rng.below((horizon - 1).max(1) as usize) as i32,
                    speed: if right { 1.0 } else { -1.0 }
                        * rng.range(1.0, 2.0)
                        * self.motion.speed(),
                    phase: rng.range(0.0, 1.0),
                }
            })
            .collect();
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
        let width = self.width as f32;
        for gull in &mut self.gulls {
            gull.x = wrap(gull.x + gull.speed * dt + 4.0, width + 8.0) - 4.0;
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let horizon = self.horizon();
        let rows = (self.height as i32 - horizon).max(1) as f32;
        let speed = self.motion.speed();

        for y in horizon..self.height as i32 {
            let depth = (y - horizon) as f32 / rows;
            let wavelength = 6.0 + 10.0 * depth;
            let drift = (0.8 + 1.2 * depth) * speed;
            let crest = if depth > 0.5 {
                theme.bright_cyan()
            } else {
                theme.cyan()
            };
            for x in 0..self.width as i32 {
                let v = (x as f32 / wavelength * TAU
                    + super::cycle(self.t, drift / wavelength * TAU)
                    + depth * 9.0)
                    .sin();
                if y == horizon {
                    let ch = if v > 0.5 { '~' } else { '-' };
                    frame.put(x, y, ch, Some(theme.cyan()));
                } else if v > 0.9 && depth > 0.6 {
                    frame.put(x, y, '\'', Some(theme.foreground));
                } else if v > 0.5 {
                    frame.put(x, y, '~', Some(crest));
                } else if v > -0.2 {
                    frame.put(x, y, '-', Some(theme.blue()));
                }
            }
        }

        let period = self.flap_period();
        for gull in &self.gulls {
            let beat = ((self.t / f64::from(period) + f64::from(gull.phase)).rem_euclid(1.0) * 4.0)
                as usize;
            frame.put_str(
                gull.x.round() as i32,
                gull.y,
                GULL_FRAMES[beat % 4],
                Some(theme.foreground),
            );
        }

        // the sun is drawn last, so a gull passes behind it
        if self.width >= 16 && horizon >= 3 {
            frame.put_sprite(self.width as i32 - 8, 0, &SUN, Some(theme.bright_yellow()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("waves");
        check_determinism("waves");
    }

    #[test]
    fn the_sea_fills_below_the_horizon_and_gulls_fly_above_it() {
        let mut rng = Rng::new(19);
        let mut waves = Waves::new(60, 20, Motion::FULL, &mut rng);
        let frame = render(&mut waves, 60, 20, 10);
        let horizon = frame.row_text(8);
        assert!(horizon.chars().all(|c| c == '~' || c == '-'), "{horizon}");
        for y in 9..20 {
            assert!(frame.row_text(y).contains('~') || frame.row_text(y).contains('-'));
        }
        assert!(frame.row_text(0).contains(".-."));
        let sky: String = (0..8).map(|y| frame.row_text(y)).collect();
        assert!(sky.contains('v'), "{sky}");
        assert!(waves.flap_period() >= FLAP_PERIOD);
        assert!(waves.flap_period() * 0.25 >= 1.0 / 3.0);
    }

    #[test]
    fn a_tiny_screen_still_shows_a_horizon() {
        let mut rng = Rng::new(2);
        let mut waves = Waves::new(20, 8, Motion::CALM, &mut rng);
        let frame = render(&mut waves, 20, 8, 5);
        assert!(frame.row_text(3).chars().all(|c| c == '~' || c == '-'));
        assert!(frame.row_text(0).contains(".-."));
        let tiny = render(&mut Waves::new(12, 5, Motion::CALM, &mut rng), 12, 5, 5);
        assert!(!tiny.to_text().contains(".-."));
        assert!(tiny.row_text(2).contains('~') || tiny.row_text(2).contains('-'));
    }
}
