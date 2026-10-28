//! A campfire: flames whose rows change shape one at a time, each no more than once every 0.6
//! seconds, embers that rise and a steady glow on the ground.

use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// The time between two shapes of one flame row. A row switches straight from one shape to the
/// next, with no blend that could cross a shade boundary several times on the way, so no flame
/// cell changes more than once in this time: under twice a second.
pub const FLICKER_PERIOD: f32 = 0.6;
/// The flame row period under reduced motion.
pub const CALM_FLICKER_PERIOD: f32 = 1.5;

/// One row of the flame: where its centre leans, how wide it is and how long since it changed.
#[derive(Clone, Debug)]
struct FlameRow {
    offset: f32,
    factor: f32,
    since: f32,
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
    t: f64,
    rows: Vec<FlameRow>,
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
            rows: Vec::new(),
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

    fn roll_row(row: &mut FlameRow, rng: &mut Rng) {
        row.offset = rng.range(-1.2, 1.2);
        row.factor = rng.range(0.7, 1.15);
    }

    /// Draws the flame alone, without the logs or the embers.
    fn draw_flame(&self, frame: &mut Frame, theme: &ThemeData) {
        let ground = self.height as i32 - 1;
        let centre = self.width as f32 / 2.0;
        let base = self.base_width() as f32;
        let height = self.rows.len() as f32;
        for (i, row) in self.rows.iter().enumerate() {
            let y = ground - 1 - i as i32;
            let narrowing = (1.0 - i as f32 / height).powf(0.8);
            let half = (base / 2.0) * narrowing * row.factor;
            let row_centre = centre + row.offset * (i as f32 / height);
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
                    (theme.faint_shade(), theme.red())
                };
                frame.put(x, y, ch, Some(colour));
            }
        }
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
        // the rows already burning keep their shape and their place in the cycle; only rows the
        // new size adds are rolled, each starting at its own point so they never change together
        let wanted = self.flame_height();
        let period = self.period();
        self.rows.truncate(wanted);
        while self.rows.len() < wanted {
            let mut row = FlameRow {
                offset: 0.0,
                factor: 1.0,
                since: rng.range(0.0, period),
            };
            Self::roll_row(&mut row, rng);
            self.rows.push(row);
        }
        let (w, h) = (self.width as f32, self.height as f32);
        self.embers.retain(|e| e.x >= 0.0 && e.x < w && e.y < h);
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let period = self.period();
        for row in &mut self.rows {
            row.since += dt;
            if row.since >= period {
                row.since = (row.since - period).min(period);
                Self::roll_row(row, rng);
                self.shapes_rolled += 1;
            }
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

        self.draw_flame(frame, theme);

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
    use crate::scenes::testing::{check_determinism, check_scene, max_changes_per_second, render};

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
    fn no_flame_cell_changes_more_than_twice_a_second() {
        for (motion, fps, limit) in [(Motion::FULL, 30, 2), (Motion::CALM, 12, 1)] {
            for &(width, height) in &[(20, 8), (80, 24), (120, 40)] {
                let mut rng = Rng::new(1);
                let mut fire = Campfire::new(width, height, motion, &mut rng);
                let theme = crate::theme::default_theme();
                let frames: Vec<Frame> = (0..(30 * fps))
                    .map(|_| {
                        fire.step(1.0 / fps as f32, &mut rng);
                        let mut frame = Frame::new(width, height);
                        fire.draw_flame(&mut frame, theme);
                        frame
                    })
                    .collect();
                let most = max_changes_per_second(&frames, fps as usize);
                assert!(
                    most <= limit,
                    "{most} changes a second at {width}x{height} ({motion:?})"
                );
            }
        }
    }

    #[test]
    fn each_row_takes_a_new_shape_once_a_period() {
        let mut rng = Rng::new(1);
        let mut fire = Campfire::new(40, 12, Motion::FULL, &mut rng);
        let rows = fire.rows.len() as f32;
        for _ in 0..(10 * 20) {
            fire.step(0.05, &mut rng);
        }
        let per_row = fire.shapes_rolled() as f32 / rows;
        assert!((15.0..=17.0).contains(&per_row), "{per_row}");
        assert!(fire.period() >= 0.5);
        let calm = Campfire::new(40, 12, Motion::CALM, &mut rng);
        assert!(calm.period() >= 1.0);
    }

    #[test]
    fn a_resize_keeps_the_flame_rows_it_still_has() {
        let mut rng = Rng::new(5);
        let mut fire = Campfire::new(80, 24, Motion::FULL, &mut rng);
        let before: Vec<f32> = fire.rows.iter().map(|r| r.offset).collect();
        fire.resize(81, 24, &mut rng);
        let after: Vec<f32> = fire.rows.iter().map(|r| r.offset).collect();
        assert_eq!(before, after);
    }
}
