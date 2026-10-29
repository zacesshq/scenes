//! City night: a skyline of near and far buildings under a moon, where one window at a time
//! lights up or goes dark, a river along the bottom and, on a screen with room, a bridge that a
//! train crosses now and then.

use super::{count_for, wrap};
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "A night skyline where windows light and darken one at a time, a moon, a river and a passing train";

pub const CAPTIONS: &[&str] = &[
    "The city sleeps. So do the errors.",
    "Every light in the right window.",
    "A clean build after dark.",
    "Quiet streets and a green build.",
    "The last train home is on time.",
];

/// Seconds between two windows changing, at the standard pace. Only one window changes at a
/// time, so no cell changes in place more than once in this long.
pub const WINDOW_PERIOD: f32 = 1.2;
/// Columns a second the train crosses at.
const TRAIN_SPEED: f32 = 9.0;
const MOON: [&str; 3] = [" _.", "(  ", " `'"];
const CAR: &str = "[o o o]";

#[derive(Clone, Debug)]
struct Building {
    x: i32,
    width: i32,
    height: i32,
    antenna: bool,
}

#[derive(Clone, Debug)]
struct Train {
    x: f32,
    speed: f32,
    cars: usize,
}

pub struct CityNight {
    width: usize,
    height: usize,
    motion: Motion,
    far: Vec<Building>,
    near: Vec<Building>,
    /// The windows of the near buildings and whether each is lit.
    windows: Vec<(i32, i32, bool)>,
    stars: Vec<(i32, i32)>,
    since_window: f32,
    train: Option<Train>,
    next_train: f32,
    trains_seen: u32,
    window_changes: u32,
}

impl CityNight {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut city = Self {
            width: 1,
            height: 1,
            motion,
            far: Vec::new(),
            near: Vec::new(),
            windows: Vec::new(),
            stars: Vec::new(),
            since_window: 0.0,
            train: None,
            next_train: rng.range(4.0, 12.0),
            trains_seen: 0,
            window_changes: 0,
        };
        city.resize(width, height, rng);
        city
    }

    fn has_river(&self) -> bool {
        self.height >= 10
    }

    fn has_bridge(&self) -> bool {
        self.height >= 14 && self.width >= 30
    }

    /// The row the buildings stand on.
    fn street(&self) -> i32 {
        let h = self.height as i32;
        if self.has_bridge() {
            h - 4
        } else if self.has_river() {
            h - 2
        } else {
            h - 1
        }
    }

    fn window_period(&self) -> f32 {
        WINDOW_PERIOD / self.motion.pace()
    }

    fn skyline(rng: &mut Rng, width: i32, low: i32, high: i32, narrow: i32) -> Vec<Building> {
        let mut out = Vec::new();
        let mut x = -(rng.below(3) as i32);
        while x < width {
            let w = narrow + rng.below(6) as i32;
            let h = low + rng.below((high - low).max(1) as usize) as i32;
            out.push(Building {
                x,
                width: w,
                height: h,
                antenna: rng.chance(0.2),
            });
            x += w + rng.below(3) as i32;
        }
        out
    }

    fn train_cells(&self, train: &Train) -> String {
        let mut text = String::new();
        if train.speed > 0.0 {
            for _ in 0..train.cars {
                text.push_str(CAR);
                text.push('-');
            }
            text.push_str("[o o]>");
        } else {
            text.push_str("<[o o]");
            for _ in 0..train.cars {
                text.push('-');
                text.push_str(CAR);
            }
        }
        text
    }

    #[cfg(test)]
    pub(crate) fn trains_seen(&self) -> u32 {
        self.trains_seen
    }
}

impl Scene for CityNight {
    fn name(&self) -> &'static str {
        "city-night"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let w = self.width as i32;
        let street = self.street();
        let room = (street - 1).max(1);
        // the far skyline stands taller than the near one, so it shows above and between it
        self.far = Self::skyline(rng, w, (room / 2).max(2), (room * 9 / 10).max(3), 4);
        self.near = Self::skyline(rng, w, (room / 3).max(3), (room * 7 / 10).max(4), 5);
        self.windows.clear();
        for b in &self.near {
            let top = street - b.height + 1;
            // windows sit on every other column and row inside the walls, so the wall shows
            // between them
            let mut y = top + 1;
            while y < street {
                let mut x = b.x + 1;
                while x < b.x + b.width - 1 {
                    self.windows.push((x, y, rng.chance(0.4)));
                    x += 2;
                }
                y += 2;
            }
        }
        let sky = (street - 1).max(1) as usize;
        let wanted = count_for(self.width, sky, 50, 2, 80);
        self.stars = (0..self.motion.scaled(wanted))
            .map(|_| (rng.below(self.width) as i32, rng.below(sky) as i32))
            .collect();
        self.train = None;
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.since_window += dt;
        let period = self.window_period();
        if self.since_window >= period {
            self.since_window = (self.since_window - period).min(period);
            if !self.windows.is_empty() {
                let i = rng.below(self.windows.len());
                self.windows[i].2 = !self.windows[i].2;
                self.window_changes += 1;
            }
        }

        if !self.has_bridge() {
            return;
        }
        let width = self.width as f32;
        if let Some(train) = &mut self.train {
            train.x += train.speed * dt;
            let length = (train.cars * (CAR.len() + 1) + 6) as f32;
            if train.x > width + length || train.x < -2.0 * length {
                self.train = None;
                self.next_train = if self.motion.calm {
                    rng.range(30.0, 60.0)
                } else {
                    rng.range(15.0, 35.0)
                };
            }
        } else {
            self.next_train -= dt;
            if self.next_train <= 0.0 {
                let right = rng.chance(0.5);
                let cars = 2 + rng.below(3);
                let length = (cars * (CAR.len() + 1) + 6) as f32;
                let speed = TRAIN_SPEED * self.motion.speed();
                self.train = Some(Train {
                    x: if right { -length } else { width },
                    speed: if right { speed } else { -speed },
                    cars,
                });
                self.trains_seen += 1;
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let w = self.width as i32;
        let h = self.height as i32;
        let street = self.street();

        for &(x, y) in &self.stars {
            frame.put(x, y, '.', Some(theme.muted));
        }
        if self.width >= 30 && self.height >= 12 {
            frame.put_sprite(w / 6, 1, &MOON, Some(theme.bright_yellow()));
        }

        // the far skyline is a flat shaded silhouette, so the near buildings stand in front of it
        let shade = theme.faint_shade();
        for b in &self.far {
            let top = street - b.height + 1;
            for y in top.max(0)..=street {
                for x in b.x..b.x + b.width {
                    frame.put(x, y, shade, Some(theme.grey()));
                }
            }
        }

        for b in &self.near {
            let top = street - b.height + 1;
            let right = b.x + b.width - 1;
            for y in top + 1..=street {
                for x in b.x..=right {
                    let wall = x == b.x || x == right;
                    frame.put(x, y, if wall { '|' } else { ' ' }, Some(theme.muted));
                }
            }
            frame.put(b.x, top, '.', Some(theme.muted));
            for x in b.x + 1..right {
                frame.put(x, top, '-', Some(theme.muted));
            }
            frame.put(right, top, '.', Some(theme.muted));
            if b.antenna && top >= 2 {
                let x = b.x + b.width / 2;
                frame.put(x, top - 1, '|', Some(theme.grey()));
                frame.put(x, top - 2, '.', Some(theme.red()));
            }
        }
        for &(x, y, lit) in &self.windows {
            if lit {
                frame.put(x, y, '\u{2584}', Some(theme.yellow()));
            } else {
                frame.put(x, y, '.', Some(theme.grey()));
            }
        }

        if self.has_bridge() {
            let deck = h - 3;
            frame.fill_row(deck, '=', Some(theme.muted));
            let mut x = 4;
            while x < w {
                frame.put(x, deck + 1, '|', Some(theme.grey()));
                x += 9;
            }
            if let Some(train) = &self.train {
                let text = self.train_cells(train);
                let left = train.x.round() as i32;
                for (i, ch) in text.chars().enumerate() {
                    let colour = if ch == 'o' {
                        theme.bright_yellow()
                    } else {
                        theme.foreground
                    };
                    frame.put(left + i as i32, deck - 1, ch, Some(colour));
                }
            }
        } else {
            frame.fill_row(street, '_', Some(theme.muted));
        }

        if self.has_river() {
            let river = h - 1;
            for x in 0..w {
                let ch = if wrap(x as f32 * 0.37, 3.0) < 1.0 {
                    '~'
                } else {
                    '-'
                };
                frame.put(x, river, ch, Some(theme.blue()));
            }
            // some lit windows leave a short reflection on the water
            for &(x, _, lit) in &self.windows {
                if lit && x % 3 == 0 {
                    frame.put(x, river, '~', Some(theme.yellow()));
                }
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
        check_scene("city-night");
        check_determinism("city-night");
    }

    #[test]
    fn one_window_changes_at_a_time_and_never_quickly() {
        let mut rng = Rng::new(4);
        for motion in Motion::ALL {
            let mut city = CityNight::new(80, 24, motion, &mut rng);
            assert!(city.window_period() >= WINDOW_PERIOD);
            let before: Vec<bool> = city.windows.iter().map(|w| w.2).collect();
            city.step(0.05, &mut rng);
            let after: Vec<bool> = city.windows.iter().map(|w| w.2).collect();
            let changed = before.iter().zip(&after).filter(|(a, b)| a != b).count();
            assert!(changed <= 1);
            for _ in 0..(60 * 20) {
                city.step(0.05, &mut rng);
            }
            assert!(city.window_changes as f32 <= 60.0 / WINDOW_PERIOD + 1.0);
            assert!(city.window_changes > 10);
        }
    }

    #[test]
    fn a_train_crosses_the_bridge_and_small_screens_have_none() {
        let mut rng = Rng::new(9);
        let mut city = CityNight::new(80, 24, Motion::FULL, &mut rng);
        let mut seen = false;
        for _ in 0..(60 * 20) {
            city.step(0.05, &mut rng);
            let mut frame = Frame::new(80, 24);
            city.draw(&mut frame, crate::theme::default_theme());
            seen |= frame.row_text(20).contains("[o o]");
        }
        assert!(seen && city.trains_seen() >= 1);
        let frame = render(
            &mut CityNight::new(80, 24, Motion::FULL, &mut rng),
            80,
            24,
            5,
        );
        assert!(frame.row_text(21).contains('='));
        let mut small = CityNight::new(20, 8, Motion::FULL, &mut rng);
        for _ in 0..(60 * 20) {
            small.step(0.05, &mut rng);
        }
        assert_eq!(small.trains_seen(), 0);
    }
}
