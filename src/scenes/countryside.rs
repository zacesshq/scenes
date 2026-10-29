//! Countryside: a steam train crossing open country, seen from beside the line. The view keeps
//! pace with the train, so the land slides past in layers: far hills slowest, then fields and
//! hedgerows, then the trees by the line, the track and a fence right in front. Every layer is
//! worked out from where it is in the world, so the country never repeats and needs no memory.

use std::f32::consts::TAU;

use super::{cycle, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "A steam train crossing open country, with hills, fields, trees and a fence sliding past";

pub const CAPTIONS: &[&str] = &[
    "Right on time. Everything passed.",
    "All aboard a clean build.",
    "Full steam ahead.",
    "The line is clear all the way.",
    "No delays on this service.",
];

/// Columns a second the ground beside the track slides past at the standard pace.
pub const TRACK_SPEED: f32 = 2.5;
/// Seconds a puff of smoke spends at each of its three sizes, so it changes slowly as it drifts.
pub const PUFF_STAGE: f32 = 1.0;

const LOCO: [&str; 3] = [" __   _    ", "|[]|_| |__\\", "'o--o--o-o'"];
const CARRIAGE: [&str; 3] = ["._________.", "|[] [] [] |", "'-o-----o-'"];
const SUN: [&str; 3] = ["\\ | /", "- O -", "/ | \\"];
const CLOUD: [&str; 2] = ["  .-~~-.  ", "(_______)"];
const PINE: [&str; 4] = ["  /\\  ", " /  \\ ", "/____\\", "  ||  "];
const OAK: [&str; 3] = [" .--. ", "(    )", " `||' "];
const TALL_PINE: [&str; 6] = [
    "   /\\   ",
    "  /  \\  ",
    "  /  \\  ",
    " /    \\ ",
    "/______\\",
    "   ||   ",
];
const TALL_OAK: [&str; 5] = [" .-~~-. ", "(      )", "(      )", " `-..-' ", "   ||   "];

#[derive(Clone, Debug)]
struct Puff {
    /// Position in the air, which moves back past the train at the train's own speed.
    x: f32,
    y: f32,
    age: f32,
}

pub struct Countryside {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// How far the train has travelled, in columns of the nearest layer.
    travelled: f64,
    puffs: Vec<Puff>,
    next_puff: f32,
    clouds: Vec<(f32, i32)>,
}

impl Countryside {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut country = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            travelled: 0.0,
            puffs: Vec::new(),
            next_puff: 0.0,
            clouds: Vec::new(),
        };
        country.resize(width, height, rng);
        country
    }

    fn hash(a: i64, b: i64) -> u64 {
        let mut z = (a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (b as u64).wrapping_mul(0xD1B5_4A32_D192_ED03);
        z = (z ^ (z >> 29)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z ^ (z >> 32)
    }

    /// The row the rails run along.
    fn rails(&self) -> i32 {
        self.height as i32 - 2
    }

    /// The row of the far hills' crest at a world column.
    fn hill_top(&self, wx: f32) -> i32 {
        let h = self.height as f32;
        let wave = (wx * 0.07).sin() + 0.6 * (wx * 0.023 + 1.3).sin();
        let amp = (h * 0.06).max(1.0);
        ((h * 0.4 + amp * wave).round() as i32).clamp(1, (self.rails() - 4).max(1))
    }

    fn carriages(&self) -> usize {
        let room = (self.width as f32 * 0.7) as usize;
        (room.saturating_sub(LOCO[0].len()) / (CARRIAGE[0].len() + 1)).min(4)
    }

    fn train_width(&self) -> i32 {
        (LOCO[0].len() + self.carriages() * (CARRIAGE[0].len() + 1)) as i32
    }

    /// The train's left edge: near the middle, easing forward and back a little so it looks
    /// like it is working rather than painted on.
    fn train_x(&self) -> i32 {
        let centre = (self.width as i32 - self.train_width()) / 2;
        let ease = cycle(self.t, TAU / 60.0).sin() * self.width as f32 * 0.06;
        centre + ease.round() as i32
    }

    fn chimney(&self) -> (f32, f32) {
        let loco = self.train_x() + self.train_width() - LOCO[0].len() as i32;
        ((loco + 6) as f32, (self.rails() - 4) as f32)
    }

    fn puff_stage(&self) -> f32 {
        PUFF_STAGE / self.motion.pace()
    }

    fn offset(&self, factor: f64) -> f64 {
        self.travelled * factor
    }

    /// Draws something placed at a world column on a layer moving at `factor`.
    fn layer_x(&self, wx: i64, factor: f64) -> i32 {
        (wx as f64 - self.offset(factor)).round() as i32
    }

    fn train_colour(theme: &ThemeData, i: usize) -> Rgb {
        if i % 2 == 0 {
            theme.bright_red()
        } else {
            theme.bright_blue()
        }
    }
}

impl Scene for Countryside {
    fn name(&self) -> &'static str {
        "countryside"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.puffs.clear();
        let clouds = (self.width / 30).clamp(1, 6);
        let sky = (self.height / 5).max(1);
        self.clouds = (0..clouds)
            .map(|_| (rng.range(0.0, self.width as f32), rng.below(sky) as i32))
            .collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = TRACK_SPEED * self.motion.speed();
        self.travelled += f64::from(speed * dt);
        let width = self.width as f32;
        for cloud in &mut self.clouds {
            cloud.0 = wrap(
                cloud.0 - 0.15 * self.motion.speed() * dt + 12.0,
                width + 24.0,
            ) - 12.0;
        }

        let stage = self.puff_stage();
        for puff in &mut self.puffs {
            puff.age += dt;
            puff.x -= speed * 0.6 * dt;
            puff.y -= 0.6 * self.motion.speed() * dt;
        }
        self.puffs.retain(|p| p.age < stage * 3.0 && p.y > -1.0);
        self.next_puff -= dt;
        if self.next_puff <= 0.0 && self.rails() >= 5 {
            self.next_puff = if self.motion.calm { 1.6 } else { 0.8 } / self.motion.busy();
            let (x, y) = self.chimney();
            self.puffs.push(Puff {
                x: x + rng.range(-0.3, 0.3),
                y,
                age: 0.0,
            });
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let w = self.width as i32;
        let rails = self.rails();

        if self.height >= 14 && self.width >= 40 {
            frame.put_sprite(w - 9, 1, &SUN, Some(theme.bright_yellow()));
        }
        for &(x, y) in &self.clouds {
            frame.put_sprite(x.round() as i32, y, &CLOUD, Some(theme.muted));
        }

        // the far hills, slowest of all, with a few small trees on the crest
        let hill_factor = 0.15;
        let hill_offset = self.offset(hill_factor) as f32;
        for x in 0..w {
            let wx = x as f32 + hill_offset;
            let here = self.hill_top(wx);
            let left = self.hill_top(wx - 1.0);
            let right = self.hill_top(wx + 1.0);
            let ch = if right < here && left >= here {
                '/'
            } else if left < here && right >= here {
                '\\'
            } else {
                '_'
            };
            frame.put(x, here, ch, Some(theme.blue()));
            if Self::hash(wx.floor() as i64, 1) % 13 == 0 && here >= 1 && ch == '_' {
                frame.put(x, here - 1, '^', Some(theme.green()));
            }
        }

        // fields between the hills and the line, each row sliding a little faster than the
        // one above it, so the ground seems to stretch away
        let near_top = rails - 4;
        let crest = (self.height as f32 * 0.46) as i32 + 1;
        if near_top > crest {
            for y in crest..near_top {
                let depth = (y - crest) as f64 / (near_top - crest).max(1) as f64;
                let factor = 0.25 + 0.4 * depth;
                let offset = self.offset(factor);
                // every third row is a line of hedgerows with gaps between the fields
                let hedges = (y - crest) % 3 == 1;
                for x in 0..w {
                    let wx = (x as f64 + offset).floor() as i64;
                    if hedges {
                        let field = wx.div_euclid(9);
                        if Self::hash(field, i64::from(y)) % 5 < 2 && wx.rem_euclid(9) != 0 {
                            frame.put(x, y, '"', Some(theme.green()));
                        }
                        continue;
                    }
                    let h = Self::hash(wx, i64::from(y));
                    if h % 29 == 0 {
                        let ch = [',', '.', '\''][(h >> 8) as usize % 3];
                        frame.put(x, y, ch, Some(theme.green()));
                    }
                }
            }
        }

        // trees by the line, standing on the far side of the track
        let tree_factor = 0.7;
        let start = self.offset(tree_factor).floor() as i64 - 8;
        for wx in start..start + w as i64 + 16 {
            let h = Self::hash(wx, 7);
            if h % 9 != 0 {
                continue;
            }
            let x = self.layer_x(wx, tree_factor);
            let (sprite, colour): (&[&str], Rgb) = match (h % 2 == 0, self.height) {
                (true, 18..) => (&TALL_PINE, theme.green()),
                (false, 18..) => (&TALL_OAK, theme.bright_green()),
                (true, 12..) => (&PINE, theme.green()),
                _ => (&OAK, theme.bright_green()),
            };
            let top = rails - sprite.len() as i32;
            frame.put_sprite(x, top, sprite, Some(colour));
            let trunk = sprite[sprite.len() - 1].find('|').unwrap_or(0) as i32;
            frame.put_str(x + trunk, rails - 1, "||", Some(theme.yellow()));
        }

        let stage = self.puff_stage();
        for puff in &self.puffs {
            let (ch, colour) = match (puff.age / stage) as usize {
                0 => ('O', theme.foreground),
                1 => ('o', theme.foreground),
                _ => ('.', theme.muted),
            };
            frame.put(
                puff.x.round() as i32,
                puff.y.round() as i32,
                ch,
                Some(colour),
            );
        }

        // the train
        if rails >= 3 {
            let mut x = self.train_x();
            // the train is solid, so the trees behind never show between its lines
            for dx in 0..self.train_width() {
                let row0 = dx < self.train_width() - LOCO[0].len() as i32;
                for y in rails - if row0 { 3 } else { 2 }..rails {
                    frame.put(x + dx, y, ' ', None);
                }
            }
            for i in 0..self.carriages() {
                let colour = Self::train_colour(theme, i);
                frame.put_sprite(x, rails - 3, &CARRIAGE, Some(colour));
                frame.put_str(x + 1, rails - 2, "[] [] []", Some(theme.bright_yellow()));
                frame.put(
                    x + CARRIAGE[0].len() as i32,
                    rails - 1,
                    '-',
                    Some(theme.grey()),
                );
                x += CARRIAGE[0].len() as i32 + 1;
            }
            frame.put_sprite(x, rails - 3, &LOCO, Some(theme.foreground));
            frame.put_str(x + 1, rails - 2, "[]", Some(theme.bright_yellow()));
        }

        // the rails with their sleepers, then the fence nearest of all
        if rails >= 0 {
            let offset = self.offset(1.0);
            for x in 0..w {
                let wx = (x as f64 + offset).floor() as i64;
                let ch = if wx.rem_euclid(3) == 0 { '+' } else { '=' };
                frame.put(x, rails, ch, Some(theme.grey()));
            }
        }
        let fence = rails + 1;
        if fence < self.height as i32 && rails >= 1 {
            let offset = self.offset(1.4);
            for x in 0..w {
                let wx = (x as f64 + offset).floor() as i64;
                let ch = if wx.rem_euclid(7) == 0 { '|' } else { '-' };
                frame.put(x, fence, ch, Some(theme.yellow()));
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
        check_scene("countryside");
        check_determinism("countryside");
    }

    #[test]
    fn the_land_slides_past_but_the_train_stays_in_view() {
        const { assert!(PUFF_STAGE >= 1.0 / 3.0) };
        let mut rng = Rng::new(1);
        let mut country = Countryside::new(100, 30, Motion::LIVELY, &mut rng);
        for _ in 0..(120 * 20) {
            country.step(0.05, &mut rng);
            let x = country.train_x();
            assert!(x >= 0 && x + country.train_width() <= 100, "{x}");
        }
        assert!(country.travelled > 100.0);
        let frame = render(&mut country, 100, 30, 10);
        let text = frame.to_text();
        assert!(text.contains("|[]|") && text.contains("[] [] []"), "{text}");
        assert!(frame.row_text(28).contains('+'));
        assert!(frame.row_text(29).contains('|'));
    }

    #[test]
    fn a_small_screen_gets_just_the_engine() {
        let mut rng = Rng::new(2);
        let mut country = Countryside::new(20, 8, Motion::CALM, &mut rng);
        assert_eq!(country.carriages(), 0);
        let frame = render(&mut country, 20, 8, 10);
        assert!(frame.to_text().contains("|[]|"));
    }
}
