//! Lanterns: paper lanterns rising slowly from a dark lake into a starry sky, at three depths,
//! swaying a little on the air. Each one leaves a reflection on the water that slides down as it
//! climbs. A few more drift on the surface below a far line of trees.

use std::f32::consts::TAU;

use super::{count_for, cycle, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "Paper lanterns rising slowly over a dark lake, with their reflections on the water";

pub const CAPTIONS: &[&str] = &[
    "Every light has lifted off. Build passed.",
    "Let it go. Nothing is wrong.",
    "A clean build, released into the night.",
    "Light and clear all the way up.",
    "No errors to weigh anything down.",
];

/// Seconds for one sway of a lantern from side to side. A sway moves it one column each way, so
/// it changes far less than three times a second.
pub const SWAY_PERIOD: f32 = 7.0;

const NEAR: [&str; 3] = [".-.", "|o|", "'-'"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Depth {
    Far,
    Mid,
    Near,
}

#[derive(Clone, Debug)]
struct Lantern {
    x: f32,
    y: f32,
    rise: f32,
    depth: Depth,
    colour: usize,
    phase: f32,
}

#[derive(Clone, Debug)]
struct Floating {
    x: f32,
    colour: usize,
}

pub struct Lanterns {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    lanterns: Vec<Lantern>,
    floating: Vec<Floating>,
    stars: Vec<(i32, i32, bool)>,
    trees: Vec<char>,
    swell: Vec<(f32, i32)>,
}

impl Lanterns {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut lake = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            lanterns: Vec::new(),
            floating: Vec::new(),
            stars: Vec::new(),
            trees: Vec::new(),
            swell: Vec::new(),
        };
        lake.resize(width, height, rng);
        lake
    }

    /// The first row of water. The far shore's trees stand on the row above it.
    fn shore(&self) -> i32 {
        ((self.height as f32 * 0.62) as i32).clamp(1, (self.height as i32 - 1).max(1))
    }

    fn near_fits(&self) -> bool {
        self.height >= 14 && self.width >= 30
    }

    fn sway_period(&self) -> f32 {
        SWAY_PERIOD / self.motion.pace()
    }

    fn launch(&self, rng: &mut Rng, anywhere: bool) -> Lantern {
        let shore = self.shore() as f32;
        let depth = match rng.below(10) {
            0..=4 => Depth::Far,
            5..=7 => Depth::Mid,
            _ if self.near_fits() => Depth::Near,
            _ => Depth::Mid,
        };
        let rise = match depth {
            Depth::Far => rng.range(0.25, 0.4),
            Depth::Mid => rng.range(0.4, 0.6),
            Depth::Near => rng.range(0.6, 0.9),
        } * self.motion.speed();
        Lantern {
            x: rng.range(1.0, (self.width as f32 - 3.0).max(1.5)),
            y: if anywhere {
                rng.range(-2.0, shore - 1.0)
            } else {
                // a lantern waits below the treeline for a moment before it shows
                shore - 1.0 + rng.range(0.0, 4.0)
            },
            rise,
            depth,
            colour: rng.below(5),
            phase: rng.range(0.0, TAU),
        }
    }

    fn colours(theme: &ThemeData, colour: usize) -> (Rgb, Rgb) {
        // the glow and a dimmer shade for the reflection
        match colour {
            0 => (theme.bright_yellow(), theme.yellow()),
            1 => (theme.yellow(), theme.yellow()),
            2 => (theme.bright_red(), theme.red()),
            3 => (theme.red(), theme.red()),
            _ => (theme.bright_magenta(), theme.magenta()),
        }
    }

    fn sway(&self, phase: f32) -> f32 {
        (cycle(self.t, TAU / self.sway_period()) + phase).sin()
    }
}

impl Scene for Lanterns {
    fn name(&self) -> &'static str {
        "lanterns"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let shore = self.shore();
        let wanted = self
            .motion
            .scaled(count_for(self.width, shore.max(1) as usize, 45, 3, 50));
        self.lanterns = (0..wanted).map(|_| self.launch(rng, true)).collect();
        let floating = count_for(self.width, 1, 25, 1, 6);
        self.floating = (0..floating)
            .map(|_| Floating {
                x: rng.range(0.0, self.width as f32),
                colour: rng.below(5),
            })
            .collect();
        let stars = count_for(self.width, shore.max(1) as usize, 25, 2, 200);
        self.stars = (0..stars)
            .map(|_| {
                (
                    rng.below(self.width) as i32,
                    rng.below((shore - 1).max(1) as usize) as i32,
                    rng.chance(0.2),
                )
            })
            .collect();
        // the far shore: a band of tree tops in two heights
        let mut trees = Vec::with_capacity(self.width);
        while trees.len() < self.width {
            match rng.below(4) {
                0 => trees.extend_from_slice(&['/', '\\']),
                1 => trees.push('^'),
                2 => trees.extend_from_slice(&['A', '^']),
                _ => trees.push('_'),
            }
        }
        trees.truncate(self.width);
        self.trees = trees;
        let water_rows = (self.height as i32 - shore).max(1);
        let swell = count_for(self.width, water_rows as usize, 70, 1, 60);
        self.swell = (0..swell)
            .map(|_| {
                (
                    rng.range(0.0, self.width as f32),
                    shore + rng.below(water_rows as usize) as i32,
                )
            })
            .collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        for i in 0..self.lanterns.len() {
            let lantern = &mut self.lanterns[i];
            lantern.y -= lantern.rise * dt;
            if lantern.y < -4.0 {
                self.lanterns[i] = self.launch(rng, false);
            }
        }
        let width = self.width as f32;
        let drift = 0.15 * self.motion.speed();
        for f in &mut self.floating {
            f.x = wrap(f.x + drift * dt, width);
        }
        for s in &mut self.swell {
            s.0 = wrap(s.0 - drift * 1.5 * dt, width);
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let shore = self.shore();
        let bottom = self.height as i32 - 1;

        for &(x, y, bright) in &self.stars {
            let (ch, colour) = if bright {
                ('+', theme.foreground)
            } else {
                ('.', theme.muted)
            };
            frame.put(x, y, ch, Some(colour));
        }

        for &(x, y) in &self.swell {
            frame.put(x as i32, y, '-', Some(theme.blue()));
        }

        // lanterns in the sky, farthest first; each one's reflection sits as far below the
        // shore as the lantern is above it, squeezed by half
        let mut order: Vec<&Lantern> = self.lanterns.iter().collect();
        order.sort_by_key(|l| l.depth as u8);
        for lantern in order {
            let (glow, dim) = Self::colours(theme, lantern.colour);
            let x = (lantern.x + self.sway(lantern.phase)).round() as i32;
            let y = lantern.y.round() as i32;
            let height_above = shore - 1 - y;
            if height_above >= 0 {
                let ry = shore + height_above / 2;
                if ry <= bottom {
                    let mark = match lantern.depth {
                        Depth::Far => "-",
                        Depth::Mid => "~",
                        Depth::Near => "~-~",
                    };
                    frame.put_str(x, ry, mark, Some(dim));
                }
            }
            // anything below the treeline is still hidden behind it
            let visible = |row: i32| row < shore - 1;
            match lantern.depth {
                Depth::Far => {
                    if visible(y) {
                        frame.put(x, y, 'o', Some(dim));
                    }
                }
                Depth::Mid => {
                    if visible(y) {
                        frame.put_str(x, y, "(o)", Some(glow));
                    }
                }
                Depth::Near => {
                    for (dy, row) in NEAR.iter().enumerate() {
                        let ry = y + dy as i32;
                        if visible(ry) {
                            frame.put_str(x, ry, row, Some(glow));
                        }
                    }
                    if visible(y + 1) {
                        frame.put(x + 1, y + 1, 'o', Some(theme.bright_yellow()));
                    }
                }
            }
        }

        if shore >= 1 {
            for (x, ch) in self.trees.iter().enumerate() {
                frame.put(x as i32, shore - 1, *ch, Some(theme.green()));
            }
        }

        for f in &self.floating {
            let (glow, dim) = Self::colours(theme, f.colour);
            let x = f.x.round() as i32;
            frame.put(x, shore, 'o', Some(glow));
            if shore < bottom {
                frame.put(x, shore + 1, '~', Some(dim));
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
        check_scene("lanterns");
        check_determinism("lanterns");
    }

    #[test]
    fn lanterns_sway_slowly_and_rise() {
        const { assert!(SWAY_PERIOD / 2.0 >= 1.0 / 3.0) };
        let mut rng = Rng::new(3);
        let mut lake = Lanterns::new(80, 24, Motion::FULL, &mut rng);
        assert!(lake.sway_period() >= SWAY_PERIOD);
        let before: Vec<f32> = lake.lanterns.iter().map(|l| l.y).collect();
        lake.step(1.0, &mut rng);
        for (lantern, y) in lake.lanterns.iter().zip(before) {
            assert!(lantern.y < y || y < -3.0);
        }
    }

    #[test]
    fn the_shore_trees_and_lanterns_are_drawn() {
        let mut rng = Rng::new(8);
        let mut lake = Lanterns::new(60, 20, Motion::FULL, &mut rng);
        let frame = render(&mut lake, 60, 20, 60);
        let shore = lake.shore() as usize;
        assert!(frame.row_text(shore - 1).contains('^'));
        let sky: String = (0..shore - 1).map(|y| frame.row_text(y)).collect();
        assert!(sky.contains('o'), "{sky}");
        let small = render(&mut Lanterns::new(20, 8, Motion::CALM, &mut rng), 20, 8, 20);
        assert!(!small.to_text().contains(".-."));
        assert!(small.drawn() > 5);
    }
}
