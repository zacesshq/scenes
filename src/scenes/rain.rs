//! Rain: drops at two depths, ripples where they land and, unless motion is reduced, a distant
//! flash of lightning every few seconds at most.

use super::{count_for, wrap};
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// The shortest gap between two flashes, whatever the random numbers say.
pub const MIN_LIGHTNING_GAP: f32 = 5.0;
/// How long one flash stays on screen.
const FLASH_LENGTH: f32 = 0.12;
const RIPPLE_LENGTH: f32 = 0.6;

#[derive(Clone, Debug)]
struct Drop {
    x: f32,
    y: f32,
    speed: f32,
    near: bool,
}

#[derive(Clone, Debug)]
struct Ripple {
    x: i32,
    age: f32,
}

#[derive(Clone, Debug)]
struct Bolt {
    cells: Vec<(i32, i32, char)>,
    age: f32,
}

pub struct Rain {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    drops: Vec<Drop>,
    ripples: Vec<Ripple>,
    clouds: Vec<(f32, usize)>,
    bolt: Option<Bolt>,
    next_bolt: f32,
    /// Seconds since the last bolt appeared. Every strike waits for this to reach the minimum
    /// gap, whatever else resets, so a resize can never bring a bolt early.
    since_strike: f32,
    flashes: u32,
}

impl Rain {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut rain = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            drops: Vec::new(),
            ripples: Vec::new(),
            clouds: Vec::new(),
            bolt: None,
            next_bolt: rng.range(MIN_LIGHTNING_GAP, 14.0),
            since_strike: 0.0,
            flashes: 0,
        };
        rain.resize(width, height, rng);
        rain
    }

    fn new_drop(&self, rng: &mut Rng, from_top: bool) -> Drop {
        let near = rng.chance(0.4);
        let speed = if near {
            rng.range(14.0, 20.0)
        } else {
            rng.range(8.0, 12.0)
        };
        Drop {
            x: rng.range(0.0, self.width as f32),
            y: if from_top {
                -rng.range(0.0, 3.0)
            } else {
                rng.range(-1.0, self.height as f32)
            },
            speed: speed * self.motion.speed(),
            near,
        }
    }

    fn ground(&self) -> i32 {
        self.height as i32 - 1
    }

    fn make_bolt(&self, rng: &mut Rng) -> Option<Bolt> {
        if self.width < 7 || self.height < 6 {
            return None;
        }
        let length = ((self.height / 3) as i32).clamp(2, 7);
        let mut x = rng.range(3.0, self.width as f32 - 3.0) as i32;
        let mut cells = Vec::new();
        for y in 1..=length {
            let dx = rng.below(3) as i32 - 1;
            let ch = match dx {
                -1 => '/',
                1 => '\\',
                _ => '|',
            };
            x += dx;
            cells.push((x, y, ch));
        }
        Some(Bolt { cells, age: 0.0 })
    }

    #[cfg(test)]
    pub(crate) fn flashes(&self) -> u32 {
        self.flashes
    }

    #[cfg(test)]
    pub(crate) fn flashing(&self) -> bool {
        self.bolt.is_some()
    }
}

impl Scene for Rain {
    fn name(&self) -> &'static str {
        "rain"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = count_for(self.width, self.height, 10, 6, 400);
        self.drops = (0..wanted).map(|_| self.new_drop(rng, false)).collect();
        self.ripples.clear();
        if self.bolt.take().is_some() {
            self.next_bolt = self.next_bolt.max(MIN_LIGHTNING_GAP);
        }
        self.clouds.clear();
        let mut x = -(rng.below(6) as f32);
        while x < self.width as f32 {
            let len = 3 + rng.below(6);
            self.clouds.push((x, len));
            x += len as f32 + 2.0 + rng.below(8) as f32;
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        self.since_strike += dt;
        let speed = self.motion.speed();
        let width = self.width as f32;
        let ground = self.ground();

        for cloud in &mut self.clouds {
            cloud.0 = wrap(cloud.0 + 0.4 * speed * dt + 12.0, width + 24.0) - 12.0;
        }

        for i in 0..self.drops.len() {
            let drop = &mut self.drops[i];
            drop.y += drop.speed * dt;
            drop.x = wrap(drop.x + 1.0 * speed * dt, width);
            if drop.y >= ground as f32 {
                let x = drop.x.round() as i32;
                let near = drop.near;
                self.drops[i] = self.new_drop(rng, true);
                if near && self.ripples.len() < 60 {
                    self.ripples.push(Ripple { x, age: 0.0 });
                }
            }
        }

        for ripple in &mut self.ripples {
            ripple.age += dt * speed;
        }
        self.ripples.retain(|r| r.age < RIPPLE_LENGTH);

        if self.motion.calm {
            self.bolt = None;
            return;
        }
        if let Some(bolt) = &mut self.bolt {
            bolt.age += dt;
            if bolt.age >= FLASH_LENGTH {
                self.bolt = None;
                self.next_bolt = rng.range(MIN_LIGHTNING_GAP, 15.0);
            }
        } else {
            self.next_bolt -= dt;
            if self.next_bolt <= 0.0 && self.since_strike >= MIN_LIGHTNING_GAP {
                match self.make_bolt(rng) {
                    Some(bolt) => {
                        self.bolt = Some(bolt);
                        self.since_strike = 0.0;
                        self.flashes += 1;
                    }
                    None => self.next_bolt = MIN_LIGHTNING_GAP,
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let ground = self.ground();
        if ground >= 1 {
            for &(x, len) in &self.clouds {
                for i in 0..len as i32 {
                    frame.put(x.round() as i32 + i, 0, '~', Some(theme.grey()));
                }
            }
            frame.fill_row(ground, '_', Some(theme.muted));
        }

        for drop in &self.drops {
            let (ch, colour) = if drop.near {
                ('|', theme.bright_blue())
            } else {
                (':', theme.blue())
            };
            let y = drop.y.floor() as i32;
            if y >= 1 || ground == 0 {
                frame.put(drop.x.round() as i32, y.min(ground - 1), ch, Some(colour));
            }
        }

        for ripple in &self.ripples {
            let radius = (ripple.age / (RIPPLE_LENGTH / 3.0)) as i32;
            let colour = Some(theme.cyan());
            if radius == 0 {
                frame.put(ripple.x, ground, 'o', colour);
            } else {
                frame.put(ripple.x - radius, ground, '(', colour);
                frame.put(ripple.x + radius, ground, ')', colour);
            }
        }

        if let Some(bolt) = &self.bolt {
            for &(x, y, ch) in &bolt.cells {
                frame.put(x, y, ch, Some(theme.bright_yellow()));
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
        check_scene("rain");
        check_determinism("rain");
    }

    #[test]
    fn drops_fall_and_ripples_land_on_the_ground_row() {
        let mut rng = Rng::new(8);
        let mut rain = Rain::new(60, 20, Motion::FULL, &mut rng);
        let frame = render(&mut rain, 60, 20, 40);
        let text = frame.to_text();
        assert!(text.contains('|') && text.contains(':'), "{text}");
        let ground = frame.row_text(19);
        assert!(ground.contains('_'));
        assert!(
            ground.contains('(') || ground.contains('o'),
            "no ripple: {ground}"
        );
        assert!(frame.row_text(0).contains('~'));
    }

    #[test]
    fn lightning_never_strikes_under_reduced_motion() {
        let mut rng = Rng::new(1);
        let mut rain = Rain::new(80, 24, Motion::CALM, &mut rng);
        for _ in 0..(300 * 20) {
            rain.step(0.05, &mut rng);
            assert!(!rain.flashing());
        }
        assert_eq!(rain.flashes(), 0);
    }

    #[test]
    fn lightning_keeps_at_least_five_seconds_between_flashes() {
        let mut rng = Rng::new(21);
        let mut rain = Rain::new(80, 24, Motion::FULL, &mut rng);
        let mut t = 0.0_f32;
        let mut starts = Vec::new();
        let mut was_flashing = false;
        for _ in 0..(300 * 20) {
            rain.step(0.05, &mut rng);
            t += 0.05;
            if rain.flashing() && !was_flashing {
                starts.push(t);
            }
            was_flashing = rain.flashing();
        }
        assert!(
            starts.len() >= 10,
            "only {} flashes in five minutes",
            starts.len()
        );
        for pair in starts.windows(2) {
            assert!(pair[1] - pair[0] >= MIN_LIGHTNING_GAP - 0.01, "{pair:?}");
        }
        assert_eq!(starts.len() as u32, rain.flashes());
    }

    #[test]
    fn resizing_never_brings_lightning_early() {
        let mut rng = Rng::new(21);
        let mut rain = Rain::new(80, 24, Motion::FULL, &mut rng);
        let mut t = 0.0_f32;
        let mut starts = Vec::new();
        let mut was_flashing = false;
        // a window edge dragged for five minutes: a new size on every frame
        for frame in 0..(300 * 20) {
            let width = 60 + (frame % 40);
            let height = 20 + (frame % 7);
            rain.resize(width, height, &mut rng);
            rain.step(0.05, &mut rng);
            t += 0.05;
            if rain.flashing() && !was_flashing {
                starts.push(t);
            }
            was_flashing = rain.flashing();
        }
        assert!(!starts.is_empty());
        for pair in starts.windows(2) {
            assert!(pair[1] - pair[0] >= MIN_LIGHTNING_GAP - 0.01, "{pair:?}");
        }
    }

    #[test]
    fn a_tiny_screen_gets_no_bolt() {
        let mut rng = Rng::new(4);
        let mut rain = Rain::new(6, 4, Motion::FULL, &mut rng);
        for _ in 0..(60 * 20) {
            rain.step(0.05, &mut rng);
        }
        assert_eq!(rain.flashes(), 0);
    }
}
