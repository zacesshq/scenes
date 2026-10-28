//! A campfire: flames that change shape no faster than three times a second, embers that rise
//! and a steady glow on the ground.

use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// The time between two flame shapes. The flame morphs smoothly from one to the next, so this is
/// the fastest anything about it changes: just under three times a second.
pub const FLICKER_PERIOD: f32 = 0.34;
/// The flame shape period under reduced motion.
pub const CALM_FLICKER_PERIOD: f32 = 1.0;

#[derive(Clone, Debug)]
struct Shape {
    /// Per flame row: the centre offset and the width factor.
    rows: Vec<(f32, f32)>,
}

#[derive(Clone, Debug)]
struct Ember {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    age: f32,
    life: f32,
}

pub struct Campfire {
    width: usize,
    height: usize,
    motion: Motion,
    t: f32,
    since_shape: f32,
    previous: Shape,
    next: Shape,
    shapes_rolled: u32,
    embers: Vec<Ember>,
}

impl Campfire {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut fire = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            since_shape: 0.0,
            previous: Shape { rows: Vec::new() },
            next: Shape { rows: Vec::new() },
            shapes_rolled: 0,
            embers: Vec::new(),
        };
        fire.resize(width, height, rng);
        fire
    }

    fn flame_height(&self) -> usize {
        ((self.height as f32 * 0.55) as usize).clamp(2, 14)
    }

    fn base_width(&self) -> usize {
        let w = (self.width / 4).clamp(3, 21);
        if w % 2 == 0 { w - 1 } else { w }
    }

    fn period(&self) -> f32 {
        if self.motion.calm {
            CALM_FLICKER_PERIOD
        } else {
            FLICKER_PERIOD
        }
    }

    fn roll_shape(&self, rng: &mut Rng) -> Shape {
        let rows = (0..self.flame_height())
            .map(|_| (rng.range(-1.2, 1.2), rng.range(0.7, 1.15)))
            .collect();
        Shape { rows }
    }

    /// The shape at this moment, blended between the last two rolled shapes.
    fn shape_now(&self) -> Vec<(f32, f32)> {
        let u = (self.since_shape / self.period()).clamp(0.0, 1.0);
        let u = u * u * (3.0 - 2.0 * u);
        self.previous
            .rows
            .iter()
            .zip(&self.next.rows)
            .map(|(a, b)| (a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u))
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn shapes_rolled(&self) -> u32 {
        self.shapes_rolled
    }
}

impl Scene for Campfire {
    fn name(&self) -> &'static str {
        "campfire"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.previous = self.roll_shape(rng);
        self.next = self.roll_shape(rng);
        self.since_shape = 0.0;
        self.embers.clear();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += dt;
        self.since_shape += dt;
        if self.since_shape >= self.period() {
            self.since_shape -= self.period();
            let fresh = self.roll_shape(rng);
            self.previous = std::mem::replace(&mut self.next, fresh);
            self.shapes_rolled += 1;
        }

        let speed = self.motion.speed();
        let rate = if self.motion.calm { 0.7 } else { 2.0 };
        let centre = self.width as f32 / 2.0;
        let top = self.height as f32 - 2.0 - self.flame_height() as f32;
        if self.embers.len() < 24 && rng.chance(rate * dt) {
            let spread = self.base_width() as f32 / 4.0;
            self.embers.push(Ember {
                x: centre + rng.range(-spread, spread),
                y: top + 1.0,
                vx: rng.range(-0.5, 0.5) * speed,
                vy: -rng.range(1.0, 2.5) * speed,
                age: 0.0,
                life: rng.range(1.5, 3.0),
            });
        }
        for ember in &mut self.embers {
            ember.x += ember.vx * dt;
            ember.y += ember.vy * dt;
            ember.age += dt;
        }
        self.embers.retain(|e| e.age < e.life && e.y >= -1.0);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let ground = self.height as i32 - 1;
        let centre = self.width as f32 / 2.0;
        let base = self.base_width() as f32;

        // the logs and the glow around them on the ground row
        let log_half = (base / 2.0 + 1.0) as i32;
        for dx in -log_half..=log_half {
            let ch = if dx == -log_half {
                '\\'
            } else if dx == log_half {
                '/'
            } else {
                '='
            };
            frame.put(centre as i32 + dx, ground, ch, Some(theme.yellow()));
        }
        for k in 1..=3 {
            let dx = log_half + 1 + k * 2;
            frame.put(centre as i32 - dx, ground, '.', Some(theme.yellow()));
            frame.put(centre as i32 + dx, ground, '.', Some(theme.yellow()));
        }

        let rows = self.shape_now();
        let height = rows.len() as f32;
        for (i, (offset, factor)) in rows.iter().enumerate() {
            let y = ground - 1 - i as i32;
            let narrowing = (1.0 - i as f32 / height).powf(0.8);
            let half = (base / 2.0) * narrowing * factor;
            let row_centre = centre + offset * (i as f32 / height);
            let tip = i as f32 >= height * 0.75;
            let left = (row_centre - half).round() as i32;
            let right = (row_centre + half).round() as i32;
            for x in left..=right {
                let distance = if half < 0.5 {
                    0.0
                } else {
                    ((x as f32 - row_centre) / half).abs()
                };
                let (ch, colour) = if tip {
                    ('^', theme.red())
                } else if distance < 0.35 {
                    ('\u{2593}', theme.bright_yellow())
                } else if distance < 0.7 {
                    ('\u{2592}', theme.yellow())
                } else {
                    ('\u{2591}', theme.red())
                };
                frame.put(x, y, ch, Some(colour));
            }
        }

        for ember in &self.embers {
            let life = ember.age / ember.life;
            let (ch, colour) = if life < 0.4 {
                ('*', theme.bright_yellow())
            } else if life < 0.75 {
                ('.', theme.yellow())
            } else {
                ('.', theme.red())
            };
            frame.put(
                ember.x.round() as i32,
                ember.y.round() as i32,
                ch,
                Some(colour),
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
        check_scene("campfire");
        check_determinism("campfire");
    }

    #[test]
    fn the_fire_sits_on_its_logs_and_sends_up_embers() {
        let mut rng = Rng::new(17);
        let mut fire = Campfire::new(60, 20, Motion::FULL, &mut rng);
        let frame = render(&mut fire, 60, 20, 60);
        let logs = frame.row_text(19);
        assert!(logs.contains("\\====") && logs.contains("/"), "{logs}");
        let text = frame.to_text();
        assert!(text.contains('\u{2593}') && text.contains('^'), "{text}");
        let mut embers_seen = false;
        for _ in 0..100 {
            let frame = render(&mut fire, 60, 20, 1);
            embers_seen |= (0..8).any(|y| frame.row_text(y).contains('*'));
        }
        assert!(embers_seen);
    }

    #[test]
    fn the_flame_changes_shape_no_more_than_three_times_a_second() {
        let mut rng = Rng::new(1);
        let mut fire = Campfire::new(40, 12, Motion::FULL, &mut rng);
        for _ in 0..(10 * 20) {
            fire.step(0.05, &mut rng);
        }
        assert!(fire.shapes_rolled() <= 30, "{}", fire.shapes_rolled());
        assert!(fire.shapes_rolled() >= 25);
        assert!(fire.period() > 1.0 / 3.0);

        let mut calm = Campfire::new(40, 12, Motion::CALM, &mut rng);
        for _ in 0..(10 * 20) {
            calm.step(0.05, &mut rng);
        }
        assert!(calm.shapes_rolled() <= 10, "{}", calm.shapes_rolled());
    }
}
