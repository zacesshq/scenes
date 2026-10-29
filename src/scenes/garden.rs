//! Garden: a bed where seeds sprout, stems grow row by row and flowers bud and bloom over a
//! couple of minutes, with butterflies and a drifting cloud. When the season ends the petals fall,
//! the stems settle back into the soil and the bed starts again.
//!
//! Every plant's state is worked out from the garden's clock rather than stored, so a resize keeps
//! the season where it was.

use std::f32::consts::TAU;

use super::wrap;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "Seeds that sprout, grow and bloom over a couple of minutes, with butterflies, before the bed starts again";

pub const CAPTIONS: &[&str] = &[
    "Everything is growing nicely.",
    "A clean build. Watch it bloom.",
    "No weeds in this bed.",
    "Green shoots all round.",
    "Tended and thriving.",
];

/// Seconds one season takes, from seed to bloom and back, at the standard pace.
pub const SEASON: f32 = 110.0;
/// Seconds a butterfly's wings take to open and close. Each half is one change, two a period.
pub const FLUTTER_PERIOD: f32 = 1.2;
/// The share of the season by which every flower has bloomed and when petals start to fall.
const FALL_START: f32 = 0.84;
const SETTLE_START: f32 = 0.92;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Tulip,
    Daisy,
    Sunflower,
    Bell,
    Fern,
    Rose,
}

const KINDS: [Kind; 6] = [
    Kind::Tulip,
    Kind::Daisy,
    Kind::Sunflower,
    Kind::Bell,
    Kind::Fern,
    Kind::Rose,
];

#[derive(Clone, Debug)]
struct Plant {
    x: i32,
    kind: Kind,
    full: i32,
    /// When it sprouts and how long it grows for, as shares of the season.
    start: f32,
    grow: f32,
    /// Which side its first leaf grows on.
    leaf: bool,
}

#[derive(Clone, Debug)]
struct Butterfly {
    centre: f32,
    reach: f32,
    rate: f32,
    phase: f32,
    row: f32,
}

pub struct Garden {
    width: usize,
    height: usize,
    motion: Motion,
    /// The season clock, in seconds of the standard pace.
    season: f64,
    t: f64,
    plants: Vec<Plant>,
    butterflies: Vec<Butterfly>,
    cloud: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    Seed,
    Growing(i32),
    Bud,
    Bloom,
    /// Petals falling, for seconds so far.
    Falling(f32),
    Settling(i32),
}

const SUN: [&str; 3] = [" .-. ", "(   )", " `-' "];
const CLOUD: [&str; 2] = [" .--.  ", "(    `)"];

impl Garden {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut garden = Self {
            width: 1,
            height: 1,
            motion,
            // the first season opens part grown, so a passing build is never greeted by a bare bed
            season: f64::from(SEASON) * 0.35,
            t: 0.0,
            plants: Vec::new(),
            butterflies: Vec::new(),
            cloud: 0.0,
        };
        garden.resize(width, height, rng);
        garden
    }

    /// The row plants grow from.
    fn ground(&self) -> i32 {
        self.height as i32 - 2
    }

    /// How far through the season the garden is, from 0 to 1.
    fn progress(&self) -> f32 {
        (self.season / f64::from(SEASON)).rem_euclid(1.0) as f32
    }

    fn stage(&self, plant: &Plant) -> Stage {
        let p = self.progress();
        if p < plant.start {
            return Stage::Seed;
        }
        if p >= SETTLE_START {
            let left = 1.0 - (p - SETTLE_START) / (1.0 - SETTLE_START);
            return Stage::Settling((plant.full as f32 * left).floor() as i32);
        }
        if p >= FALL_START {
            return Stage::Falling((p - FALL_START) * SEASON);
        }
        let grown = (p - plant.start) / plant.grow;
        if grown < 1.0 {
            Stage::Growing((plant.full as f32 * grown).floor() as i32)
        } else if grown < 1.25 {
            Stage::Bud
        } else {
            Stage::Bloom
        }
    }

    fn flutter_period(&self) -> f32 {
        FLUTTER_PERIOD / self.motion.pace()
    }

    fn draw_stem(&self, frame: &mut Frame, theme: &ThemeData, plant: &Plant, height: i32) {
        let ground = self.ground();
        let green = Some(theme.green());
        for k in 1..=height {
            let y = ground - k + 1;
            if plant.kind == Kind::Fern {
                // a fern is fronds all the way up, narrowing to a tip
                if k == height && height > 1 {
                    frame.put(plant.x, y, '^', green);
                } else {
                    frame.put(plant.x, y, '|', green);
                    if k > 1 {
                        frame.put(plant.x - 1, y, '\\', green);
                        frame.put(plant.x + 1, y, '/', green);
                    }
                }
                continue;
            }
            frame.put(plant.x, y, '|', green);
            if k % 3 == 2 && k < height {
                let left = (k / 3 % 2 == 0) == plant.leaf;
                if left {
                    frame.put(plant.x - 1, y, '\\', green);
                } else {
                    frame.put(plant.x + 1, y, '/', green);
                }
            }
        }
    }

    fn draw_head(&self, frame: &mut Frame, theme: &ThemeData, plant: &Plant, open: bool) {
        let y = self.ground() - plant.full;
        let x = plant.x;
        if !open {
            let colour = match plant.kind {
                Kind::Fern => return,
                _ => theme.green(),
            };
            frame.put(x, y, 'o', Some(colour));
            return;
        }
        match plant.kind {
            Kind::Tulip => frame.put_str(x - 1, y, "(V)", Some(theme.red())),
            Kind::Daisy => {
                frame.put_str(x - 1, y, "-@-", Some(theme.foreground));
                frame.put(x, y, '@', Some(theme.yellow()));
            }
            Kind::Sunflower => {
                frame.put_str(x - 1, y, "{@}", Some(theme.bright_yellow()));
                frame.put(x, y, '@', Some(theme.red()));
            }
            Kind::Bell => {
                frame.put(x - 1, y + 1, 'o', Some(theme.blue()));
                frame.put(x + 1, y + 2, 'o', Some(theme.blue()));
                frame.put(x, y, '^', Some(theme.green()));
            }
            Kind::Rose => frame.put(x, y, '@', Some(theme.magenta())),
            Kind::Fern => {}
        }
    }
}

impl Scene for Garden {
    fn name(&self) -> &'static str {
        "garden"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let room = (self.ground() - 2).max(1);
        let gap = if self.width < 40 { 4 } else { 5 };
        let gap = ((gap as f32 / self.motion.busy()).round() as i32).max(4);
        self.plants.clear();
        let mut x = 2 + rng.below(3) as i32;
        while x < self.width as i32 - 2 {
            let kind = *rng.pick(&KINDS);
            let reach = if kind == Kind::Sunflower {
                rng.range(0.5, 0.8)
            } else {
                rng.range(0.2, 0.55)
            };
            self.plants.push(Plant {
                x,
                kind,
                full: ((room as f32 * reach) as i32).clamp(1, 16).min(room),
                start: rng.range(0.0, 0.2),
                grow: rng.range(0.3, 0.45),
                leaf: rng.chance(0.5),
            });
            x += gap + rng.below(3) as i32;
        }
        let wanted = if self.height >= 10 {
            self.motion.scaled(1 + self.width / 50).min(4)
        } else {
            0
        };
        let sky = (self.height as f32 * 0.35).max(1.0);
        self.butterflies = (0..wanted)
            .map(|_| Butterfly {
                centre: rng.range(0.2, 0.8) * self.width as f32,
                reach: rng.range(0.1, 0.25) * self.width as f32,
                rate: rng.range(0.05, 0.09),
                phase: rng.range(0.0, TAU),
                row: rng.range(1.0, sky),
            })
            .collect();
        self.cloud = rng.range(0.0, self.width as f32);
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
        self.season += f64::from(dt * self.motion.pace());
        let width = self.width as f32;
        self.cloud = wrap(
            self.cloud + 0.6 * self.motion.speed() * dt + 8.0,
            width + 16.0,
        ) - 8.0;
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let w = self.width as i32;
        let h = self.height as i32;
        let ground = self.ground();

        if h >= 12 && w >= 30 {
            frame.put_sprite(w - 8, 0, &SUN, Some(theme.bright_yellow()));
            frame.put_sprite(self.cloud.round() as i32, 1, &CLOUD, Some(theme.grey()));
        }

        // soil in a fixed pattern of characters with tufts of grass, so it reads as earth
        // rather than a line
        for x in 0..w {
            let (ch, colour) = match (x * 7 + 3) % 11 {
                0 => ('"', theme.green()),
                1..=3 => (':', theme.yellow()),
                _ => ('.', theme.yellow()),
            };
            frame.put(x, h - 1, ch, Some(colour));
        }

        // a picket fence at the back gives the bed depth on a screen with room for it
        if h >= 12 {
            for x in 0..w {
                let post = x % 6 == 0;
                frame.put(
                    x,
                    ground - 1,
                    if post { '|' } else { '-' },
                    Some(theme.grey()),
                );
                if post {
                    frame.put(x, ground - 2, '^', Some(theme.grey()));
                }
            }
        }

        for plant in &self.plants {
            match self.stage(plant) {
                Stage::Seed => {}
                Stage::Growing(0) | Stage::Settling(0) => {
                    frame.put(plant.x, ground, ',', Some(theme.green()));
                }
                Stage::Growing(height) | Stage::Settling(height) => {
                    self.draw_stem(frame, theme, plant, height);
                }
                Stage::Bud => {
                    self.draw_stem(frame, theme, plant, plant.full);
                    self.draw_head(frame, theme, plant, false);
                }
                Stage::Bloom => {
                    self.draw_stem(frame, theme, plant, plant.full);
                    self.draw_head(frame, theme, plant, true);
                }
                Stage::Falling(since) => {
                    self.draw_stem(frame, theme, plant, plant.full);
                    if plant.kind == Kind::Fern {
                        continue;
                    }
                    // three petals drift down from where the flower was, swaying as they go
                    let top = (ground - plant.full) as f32;
                    for j in 0..3 {
                        let fall = since * self.motion.speed() * 1.3 - j as f32 * 0.8;
                        if fall < 0.0 {
                            continue;
                        }
                        let y = (top + fall).min(ground as f32 + 0.5);
                        let x = plant.x as f32 + (fall * 0.9 + j as f32 * 2.1).sin() * 1.5;
                        let colour = match plant.kind {
                            Kind::Tulip => theme.red(),
                            Kind::Sunflower | Kind::Daisy => theme.yellow(),
                            Kind::Bell => theme.blue(),
                            _ => theme.magenta(),
                        };
                        frame.put(x.round() as i32, y.round() as i32, '.', Some(colour));
                    }
                }
            }
        }

        let period = f64::from(self.flutter_period());
        for (i, fly) in self.butterflies.iter().enumerate() {
            let a = super::cycle(self.t, fly.rate * TAU * self.motion.speed()) + fly.phase;
            let x = fly.centre + fly.reach * a.sin();
            let y = fly.row + 1.5 * (a * 2.3).sin();
            let open = ((self.t / period) + i as f64 * 0.37).rem_euclid(1.0) < 0.5;
            let wings = if open { "}{" } else { ")(" };
            let colour = if i % 2 == 0 {
                theme.magenta()
            } else {
                theme.bright_cyan()
            };
            frame.put_str(x.round() as i32, y.round() as i32, wings, Some(colour));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("garden");
        check_determinism("garden");
    }

    #[test]
    fn a_season_grows_blooms_and_starts_again() {
        let mut rng = Rng::new(8);
        let mut garden = Garden::new(80, 24, Motion::FULL, &mut rng);
        let mut bloomed = false;
        let mut fell = false;
        let mut restarted = false;
        let mut was_late = false;
        for _ in 0..((SEASON as usize + 10) * 20) {
            garden.step(0.05, &mut rng);
            let stages: Vec<Stage> = garden.plants.iter().map(|p| garden.stage(p)).collect();
            bloomed |= stages.contains(&Stage::Bloom);
            fell |= stages.iter().any(|s| matches!(s, Stage::Falling(_)));
            if was_late && garden.progress() < 0.1 {
                restarted = true;
            }
            was_late = garden.progress() > 0.9;
        }
        assert!(bloomed && fell && restarted);
    }

    #[test]
    fn stems_grow_slowly_and_wings_beat_slowly() {
        let mut rng = Rng::new(8);
        for motion in Motion::ALL {
            let garden = Garden::new(200, 60, motion, &mut rng);
            assert!(garden.flutter_period() >= FLUTTER_PERIOD);
            for plant in &garden.plants {
                // seconds a row of stem takes to grow, at the fastest pace
                let per_row = plant.grow * SEASON / plant.full as f32;
                assert!(per_row >= 1.0, "{per_row}");
            }
        }
    }
}
