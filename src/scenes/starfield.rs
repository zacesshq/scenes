//! A night sky: stars that brighten and fade slowly, a crescent moon and, now and then, a slow
//! shooting star.

use std::f32::consts::TAU;

use super::count_for;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// The quickest a star may complete one bright to dim cycle, in seconds, so nothing twinkles
/// faster than once every two and a half seconds.
pub const MIN_TWINKLE_PERIOD: f32 = 2.5;

#[derive(Clone, Debug)]
struct Star {
    x: i32,
    y: i32,
    phase: f32,
    period: f32,
    bright: bool,
    tint: usize,
}

#[derive(Clone, Debug)]
struct Shooting {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    age: f32,
}

const MOON: [&str; 3] = [" ,-.", "(   ", " `-'"];

pub struct Starfield {
    width: usize,
    height: usize,
    motion: Motion,
    t: f32,
    stars: Vec<Star>,
    shooting: Option<Shooting>,
    next_shooting: f32,
    shooting_seen: u32,
}

impl Starfield {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut sky = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            stars: Vec::new(),
            shooting: None,
            next_shooting: if motion.calm {
                rng.range(15.0, 30.0)
            } else {
                rng.range(6.0, 16.0)
            },
            shooting_seen: 0,
        };
        sky.resize(width, height, rng);
        sky
    }

    fn moon_fits(&self) -> bool {
        self.width >= 24 && self.height >= 8
    }

    #[cfg(test)]
    pub(crate) fn shooting_seen(&self) -> u32 {
        self.shooting_seen
    }

    #[cfg(test)]
    pub(crate) fn shooting_speed(&self) -> Option<f32> {
        self.shooting.as_ref().map(|s| s.vx.hypot(s.vy))
    }
}

impl Scene for Starfield {
    fn name(&self) -> &'static str {
        "starfield"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = count_for(self.width, self.height, 12, 6, 600);
        let (low, high) = if self.motion.calm {
            (MIN_TWINKLE_PERIOD * 2.0, 10.0)
        } else {
            (MIN_TWINKLE_PERIOD, 6.0)
        };
        self.stars = (0..wanted)
            .map(|i| Star {
                x: rng.below(self.width) as i32,
                y: rng.below(self.height) as i32,
                phase: rng.range(0.0, TAU),
                period: rng.range(low, high),
                bright: rng.chance(0.4),
                tint: i % 7,
            })
            .collect();
        self.shooting = None;
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += dt;
        if let Some(star) = &mut self.shooting {
            star.x += star.vx * dt;
            star.y += star.vy * dt;
            star.age += dt;
            let gone = star.age > 2.5
                || star.x < -6.0
                || star.x > self.width as f32 + 6.0
                || star.y > self.height as f32 + 1.0;
            if gone {
                self.shooting = None;
                self.next_shooting = if self.motion.calm {
                    rng.range(20.0, 40.0)
                } else {
                    rng.range(8.0, 20.0)
                };
            }
        } else if self.width >= 10 {
            self.next_shooting -= dt;
            if self.next_shooting <= 0.0 {
                let speed = self.motion.speed();
                let leftwards = rng.chance(0.5);
                let vx = if leftwards { -14.0 } else { 14.0 } * speed;
                self.shooting = Some(Shooting {
                    x: rng.range(self.width as f32 * 0.25, self.width as f32 * 0.85),
                    y: rng.range(0.0, (self.height as f32 * 0.3).max(1.0)),
                    vx,
                    vy: 5.0 * speed,
                    age: 0.0,
                });
                self.shooting_seen += 1;
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let tints = [theme.bright_yellow(), theme.bright_cyan()];
        for star in &self.stars {
            let level = ((self.t * TAU / star.period + star.phase).sin() + 1.0) / 2.0;
            let (ch, colour) = if star.bright {
                let ch = if level < 0.33 {
                    '.'
                } else if level < 0.66 {
                    '+'
                } else {
                    '*'
                };
                let colour = match star.tint {
                    0 => tints[0],
                    3 => tints[1],
                    _ => theme.foreground,
                };
                (ch, colour)
            } else {
                (if level < 0.5 { '.' } else { ':' }, theme.muted)
            };
            frame.put(star.x, star.y, ch, Some(colour));
        }

        if self.moon_fits() {
            frame.put_sprite(self.width as i32 - 8, 1, &MOON, Some(theme.bright_yellow()));
        }

        if let Some(star) = &self.shooting {
            for k in 1..=5 {
                let back = k as f32 * 0.06;
                let x = (star.x - star.vx * back).round() as i32;
                let y = (star.y - star.vy * back).round() as i32;
                frame.put(x, y, '.', Some(theme.grey()));
            }
            frame.put(
                star.x.round() as i32,
                star.y.round() as i32,
                '*',
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
        check_scene("starfield");
        check_determinism("starfield");
    }

    #[test]
    fn stars_twinkle_slowly_and_the_moon_hangs_top_right() {
        let mut rng = Rng::new(9);
        let mut sky = Starfield::new(60, 20, Motion::FULL, &mut rng);
        assert!(sky.stars.iter().all(|s| s.period >= MIN_TWINKLE_PERIOD));
        let frame = render(&mut sky, 60, 20, 10);
        let text = frame.to_text();
        assert!(text.contains('*') || text.contains('+'));
        assert!(frame.row_text(1).contains(",-."));
        let small = render(
            &mut Starfield::new(20, 8, Motion::FULL, &mut rng),
            20,
            8,
            10,
        );
        assert!(!small.to_text().contains(",-."));
        assert!(small.drawn() >= 6);
    }

    #[test]
    fn a_shooting_star_crosses_the_sky_and_is_slower_when_calm() {
        let mut rng = Rng::new(31);
        let mut sky = Starfield::new(80, 24, Motion::FULL, &mut rng);
        let mut fastest = 0.0_f32;
        for _ in 0..(40 * 20) {
            sky.step(0.05, &mut rng);
            if let Some(speed) = sky.shooting_speed() {
                fastest = fastest.max(speed);
            }
        }
        assert!(sky.shooting_seen() >= 1);
        assert!(fastest > 10.0, "{fastest}");

        let mut calm = Starfield::new(80, 24, Motion::CALM, &mut rng);
        let mut calm_fastest = 0.0_f32;
        for _ in 0..(90 * 20) {
            calm.step(0.05, &mut rng);
            if let Some(speed) = calm.shooting_speed() {
                calm_fastest = calm_fastest.max(speed);
            }
        }
        assert!(calm.shooting_seen() >= 1);
        assert!(calm_fastest < fastest, "{calm_fastest} vs {fastest}");
    }
}
