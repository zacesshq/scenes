//! Blossom: a flowering branch reaching in from the corner of the sky, letting petals go on a
//! spring breeze. The petals tumble slowly down past far hills to a stream that carries them away
//! and a small bird sits on the branch, looking one way and then the other now and then.

use std::f32::consts::TAU;

use super::{count_for, cycle};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "A flowering branch letting petals go on a breeze, down to a stream that carries them off";

pub const CAPTIONS: &[&str] = &[
    "Everything is in bloom.",
    "A fresh build, freshly green.",
    "Spring cleaning complete.",
    "Not a petal out of place.",
    "The build passed. Let it drift.",
];

/// Seconds a falling petal shows each side before it turns over.
pub const PETAL_TURN: f32 = 0.7;
/// Seconds the bird looks one way before it turns its head.
pub const BIRD_LOOK: f32 = 6.0;

const TURNS: [char; 2] = ['\'', ','];

#[derive(Clone, Debug)]
struct Petal {
    x: f32,
    y: f32,
    fall: f32,
    phase: f32,
    age: f32,
    colour: u8,
    /// Set once the petal has landed on the stream and is floating.
    floating: bool,
}

pub struct Blossom {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// The branch and its twigs as cells, with a character for each.
    wood: Vec<(i32, i32, char)>,
    /// Flowers as cells, with a character and a colour.
    flowers: Vec<(i32, i32, char, u8)>,
    petals: Vec<Petal>,
    next_petal: f32,
    hills: Vec<i32>,
    bank: Vec<char>,
    bird: Option<(i32, i32)>,
}

impl Blossom {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut scene = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            wood: Vec::new(),
            flowers: Vec::new(),
            petals: Vec::new(),
            next_petal: 0.0,
            hills: Vec::new(),
            bank: Vec::new(),
            bird: None,
        };
        scene.resize(width, height, rng);
        scene
    }

    /// The first row of the stream. On a short screen the stream is just the bottom row.
    fn stream(&self) -> i32 {
        if self.height >= 10 {
            self.height as i32 - 2
        } else {
            self.height as i32 - 1
        }
    }

    fn turn_period(&self) -> f32 {
        PETAL_TURN / self.motion.pace()
    }

    fn look_period(&self) -> f32 {
        BIRD_LOOK / self.motion.pace()
    }

    fn colour(theme: &ThemeData, colour: u8) -> Rgb {
        match colour {
            0 => theme.bright_magenta(),
            1 => theme.magenta(),
            2 => theme.foreground,
            // the flower's centre
            _ => theme.bright_yellow(),
        }
    }

    /// The branch's row at a column.
    fn branch_row(&self, x: f32, droop: f32) -> i32 {
        (1.0 + x * droop + (x * 0.17).sin() * 0.8).round() as i32
    }
}

impl Scene for Blossom {
    fn name(&self) -> &'static str {
        "blossom"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let reach = ((self.width as f32 * 0.68) as i32).max(4);
        // the branch droops more on a tall screen, so it fills the corner either way
        let droop = (self.height as f32 * 0.35 / reach as f32).clamp(0.05, 0.3);
        self.wood.clear();
        self.flowers.clear();
        let bird_x = (reach as f32 * 0.45) as i32;
        self.bird = if self.width >= 30 && self.height >= 10 {
            Some((bird_x, self.branch_row(bird_x as f32, droop) - 1))
        } else {
            None
        };
        let mut last = self.branch_row(0.0, droop);
        let mut tips = Vec::new();
        let mut next_twig = 4 + rng.below(4) as i32;
        for x in 0..reach {
            let y = self.branch_row(x as f32, droop);
            // a step down is drawn on the row the line arrives at, so the line stays joined
            let ch = match y.cmp(&last) {
                std::cmp::Ordering::Greater => '\\',
                std::cmp::Ordering::Less => '/',
                std::cmp::Ordering::Equal => '_',
            };
            self.wood.push((x, y, ch));
            last = y;
            if x == next_twig && x + 3 < reach {
                next_twig = x + 6 + rng.below(5) as i32;
                if self.bird.is_some_and(|(bx, _)| (x - bx).abs() < 4) {
                    continue;
                }
                // a short twig up or down with a flower at its end, left out where the flower
                // would not fit above the stream
                let up = y >= 5 && rng.chance(0.5);
                let tip = if up { (x + 3, y - 4) } else { (x + 3, y + 4) };
                if tip.1 < 1 || tip.1 > self.stream() - 3 {
                    continue;
                }
                for i in 1..=2 {
                    if up {
                        self.wood.push((x + i, y - i, '/'));
                    } else {
                        self.wood.push((x + i, y + i, '\\'));
                    }
                }
                tips.push(tip);
            }
        }
        tips.push((reach + 1, last));
        // each flower is a bright centre with four petals around it, kept apart from the next
        let mut placed: Vec<(i32, i32)> = Vec::new();
        for (tx, ty) in tips {
            if ty < 1
                || ty > self.stream() - 3
                || placed
                    .iter()
                    .any(|&(px, py)| (px - tx).abs() < 4 && (py - ty).abs() < 3)
            {
                continue;
            }
            placed.push((tx, ty));
            let colour = rng.below(3) as u8;
            self.flowers.push((tx, ty, 'o', 3));
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                self.flowers.push((tx + dx, ty + dy, '*', colour));
            }
        }
        let horizon = (self.height as f32 * 0.66) as i32;
        let p = rng.range(0.0, TAU);
        self.hills = (0..self.width)
            .map(|x| horizon + ((x as f32 * 0.08 + p).sin() * 1.3).round() as i32)
            .collect();
        self.bank = (0..self.width)
            .map(|_| *rng.pick(&['"', ',', '_', '_', 'o', '_', '"']))
            .collect();
        self.petals.clear();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        let stream = self.stream() as f32;
        let gust = 0.8 + 0.5 * cycle(self.t, 0.09).sin();
        for petal in &mut self.petals {
            petal.age += dt;
            if petal.floating {
                petal.x += 1.0 * speed * dt;
                continue;
            }
            petal.y += petal.fall * dt;
            let sway = (petal.age * 1.6 + petal.phase).sin();
            petal.x += (gust + sway) * speed * dt;
            if petal.y >= stream {
                petal.y = stream
                    + if self.height >= 10 {
                        (petal.phase > 3.0) as i32 as f32
                    } else {
                        0.0
                    };
                petal.floating = true;
            }
        }
        let width = self.width as f32;
        self.petals.retain(|p| p.x < width + 1.0);

        self.next_petal -= dt;
        if self.next_petal <= 0.0 {
            self.next_petal = if self.motion.calm {
                rng.range(1.6, 3.0)
            } else {
                rng.range(0.6, 1.4) / self.motion.busy()
            };
            // a sparse drift, so each petal can be followed on its way down
            let limit = count_for(self.width, self.height, 150, 2, 16);
            if self.petals.len() < limit && !self.flowers.is_empty() {
                let &(x, y, _, colour) = rng.pick(&self.flowers);
                let colour = colour.min(2);
                self.petals.push(Petal {
                    x: x as f32,
                    y: y as f32,
                    fall: rng.range(0.5, 0.9) * speed,
                    phase: rng.range(0.0, TAU),
                    age: 0.0,
                    colour,
                    floating: false,
                });
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let stream = self.stream();
        let w = self.width as i32;

        if self.height >= 10 {
            for (x, &y) in self.hills.iter().enumerate() {
                if y < stream - 1 {
                    frame.put(x as i32, y, '_', Some(theme.blue()));
                }
            }
        }

        // the bank and the stream
        if stream >= 1 {
            for (x, ch) in self.bank.iter().enumerate() {
                let colour = if *ch == 'o' {
                    theme.grey()
                } else {
                    theme.green()
                };
                frame.put(x as i32, stream - 1, *ch, Some(colour));
            }
        }
        // the stream runs to the right, its ripples drifting with it
        let drift = cycle(self.t, 0.8 * self.motion.speed() / 9.0 * TAU);
        for y in stream..self.height as i32 {
            for x in 0..w {
                let v = (x as f32 / 9.0 * TAU - drift + y as f32 * 2.1).sin()
                    + 0.5 * (x as f32 / 4.0 * TAU - drift * 2.0 + y as f32).sin();
                if v > 0.9 {
                    frame.put(x, y, '~', Some(theme.cyan()));
                } else if v > 0.3 {
                    frame.put(x, y, '-', Some(theme.blue()));
                }
            }
        }
        // a meadow between the hills and the bank, a few tufts at a time
        if self.height >= 10 {
            for (x, &top) in self.hills.iter().enumerate() {
                for y in top + 1..stream - 1 {
                    let h = (x as u64)
                        .wrapping_mul(0x9E37_79B9)
                        .wrapping_add(y as u64 * 0x85EB_CA6B);
                    if (h ^ (h >> 11)) % 23 == 0 {
                        let ch = if h % 2 == 0 { ',' } else { '"' };
                        frame.put(x as i32, y, ch, Some(theme.green()));
                    }
                }
            }
        }

        for &(x, y, ch) in &self.wood {
            frame.put(x, y, ch, Some(theme.yellow()));
        }
        for &(x, y, ch, colour) in &self.flowers {
            frame.put(x, y, ch, Some(Self::colour(theme, colour)));
        }
        if let Some((x, y)) = self.bird {
            let looking_left = (self.t / f64::from(self.look_period())) as u64 % 2 == 0;
            let bird = if looking_left { "<o)" } else { "(o>" };
            frame.put_str(x, y, bird, Some(theme.bright_cyan()));
        }

        let period = self.turn_period();
        for petal in &self.petals {
            let ch = if petal.floating {
                '.'
            } else if self.motion.calm {
                '\''
            } else {
                TURNS[((petal.age / period) as usize + petal.colour as usize) % TURNS.len()]
            };
            frame.put(
                petal.x.round() as i32,
                petal.y.floor() as i32,
                ch,
                Some(Self::colour(theme, petal.colour)),
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
        check_scene("blossom");
        check_determinism("blossom");
    }

    #[test]
    fn petals_turn_slowly_and_the_bird_looks_rarely() {
        const { assert!(PETAL_TURN >= 1.0 / 3.0) };
        const { assert!(BIRD_LOOK >= 1.0 / 3.0) };
        for motion in Motion::ALL {
            let mut rng = Rng::new(1);
            let scene = Blossom::new(60, 20, motion, &mut rng);
            assert!(scene.turn_period() >= PETAL_TURN);
            assert!(scene.look_period() >= BIRD_LOOK);
        }
    }

    #[test]
    fn petals_fall_to_the_stream_and_float_away() {
        let mut rng = Rng::new(10);
        let mut scene = Blossom::new(80, 24, Motion::FULL, &mut rng);
        for _ in 0..(60 * 20) {
            scene.step(0.05, &mut rng);
        }
        assert!(scene.petals.iter().any(|p| p.floating));
        for petal in &scene.petals {
            assert!(petal.y <= 23.0);
        }
        let frame = render(&mut scene, 80, 24, 1);
        let text = frame.to_text();
        assert!(text.contains("*o*") && (text.contains("<o)") || text.contains("(o>")));
        assert!(frame.row_text(22).contains('~') || frame.row_text(22).contains('-'));
        let small = render(&mut Blossom::new(20, 8, Motion::CALM, &mut rng), 20, 8, 20);
        assert!(small.to_text().contains("*o*"));
    }
}
