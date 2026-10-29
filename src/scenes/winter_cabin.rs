//! Winter cabin: a log cabin in a snowy clearing at night, its window lit and smoke curling from
//! the chimney, with pines near and far, snow falling gently and a moon over the trees. The light
//! from the window spills onto the snow in front.

use std::f32::consts::TAU;

use super::{count_for, cycle, wrap};
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "A log cabin in the snow at night, with a lit window, chimney smoke, pines and falling snow";

pub const CAPTIONS: &[&str] = &[
    "Warm inside. The build passed.",
    "Snowed in with a clean build.",
    "The fire is lit and the tests are green.",
    "Put the kettle on. Nothing to fix.",
    "Quiet night, tidy code.",
];

/// Seconds a puff of smoke spends at each of its three sizes.
pub const PUFF_STAGE: f32 = 1.2;

const CABIN: [&str; 7] = [
    "          _      ",
    "   ______| |__   ",
    "  /           \\  ",
    " /_____________\\ ",
    "  |  _   [##] |  ",
    "  | | |  [##] |  ",
    "  |_|_|_______|  ",
];
const SMALL_CABIN: [&str; 4] = ["   ___ n  ", " /      \\ ", "/________\\", " |[]  |#| "];
const MOON: [&str; 3] = [" _.", "(  ", " `-"];

#[derive(Clone, Debug)]
struct Pine {
    x: i32,
    /// The row its trunk stands on.
    base: i32,
    size: usize,
    near: bool,
    snow: u64,
}

#[derive(Clone, Debug)]
struct Flake {
    x: f32,
    y: f32,
    speed: f32,
    phase: f32,
    near: bool,
}

#[derive(Clone, Debug)]
struct Puff {
    x: f32,
    y: f32,
    age: f32,
    phase: f32,
}

pub struct WinterCabin {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    pines: Vec<Pine>,
    flakes: Vec<Flake>,
    puffs: Vec<Puff>,
    next_puff: f32,
    stars: Vec<(i32, i32)>,
    drifts: Vec<char>,
}

impl WinterCabin {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut scene = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            pines: Vec::new(),
            flakes: Vec::new(),
            puffs: Vec::new(),
            next_puff: 0.0,
            stars: Vec::new(),
            drifts: Vec::new(),
        };
        scene.resize(width, height, rng);
        scene
    }

    /// The row of the snow's surface.
    fn ground(&self) -> i32 {
        ((self.height as f32 * 0.74) as i32).clamp(1, (self.height as i32 - 1).max(1))
    }

    fn cabin(&self) -> &'static [&'static str] {
        if self.height >= 16 && self.width >= 40 {
            &CABIN
        } else {
            &SMALL_CABIN
        }
    }

    /// The cabin's left column and top row.
    fn cabin_at(&self) -> (i32, i32) {
        let rows = self.cabin();
        let x = (self.width as i32 * 2 / 5 - rows[0].len() as i32 / 2).max(0);
        (x, self.ground() - rows.len() as i32 + 1)
    }

    fn chimney(&self) -> (f32, f32) {
        let (x, y) = self.cabin_at();
        if self.cabin().len() == CABIN.len() {
            ((x + 11) as f32, (y - 1) as f32)
        } else {
            ((x + 7) as f32, (y - 1) as f32)
        }
    }

    fn puff_stage(&self) -> f32 {
        PUFF_STAGE / self.motion.pace()
    }

    fn new_flake(&self, rng: &mut Rng, from_top: bool) -> Flake {
        let near = rng.chance(0.35);
        Flake {
            x: rng.range(0.0, self.width as f32),
            y: if from_top {
                -rng.range(0.0, 2.0)
            } else {
                rng.range(0.0, self.ground() as f32)
            },
            speed: if near {
                rng.range(1.2, 1.8)
            } else {
                rng.range(0.6, 1.0)
            } * self.motion.speed(),
            phase: rng.range(0.0, TAU),
            near,
        }
    }

    /// Half the width of a pine's row: the branches come in tiers of three rows, each tier a
    /// little wider than the last, which reads as a fir rather than a plain triangle.
    fn half_width(row: i32) -> i32 {
        if row == 0 { 0 } else { 1 + row % 3 + row / 3 }
    }

    fn draw_pine(frame: &mut Frame, theme: &ThemeData, pine: &Pine) {
        let colour = if pine.near {
            theme.bright_green()
        } else {
            theme.green()
        };
        let top = pine.base - pine.size as i32;
        frame.put(pine.x, top, '^', Some(colour));
        for i in 1..pine.size as i32 {
            let y = top + i;
            let hw = Self::half_width(i);
            for dx in -hw..=hw {
                let (ch, colour) = if dx == -hw {
                    ('/', colour)
                } else if dx == hw {
                    ('\\', colour)
                } else if (pine.snow >> ((i * 7 + dx + 32).rem_euclid(60))) & 7 == 0 {
                    // a little snow resting on the branches
                    ('.', theme.foreground)
                } else {
                    ('^', colour)
                };
                frame.put(pine.x + dx, y, ch, Some(colour));
            }
        }
        frame.put(pine.x, pine.base, '|', Some(theme.yellow()));
    }
}

impl Scene for WinterCabin {
    fn name(&self) -> &'static str {
        "winter-cabin"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let ground = self.ground();
        let (cabin_x, _) = self.cabin_at();
        let cabin_w = self.cabin()[0].len() as i32;

        // far pines stand along the edge of the clearing; near ones at the sides in the snow
        self.pines.clear();
        let near_size = ((self.height as f32 * 0.45) as usize).clamp(3, 10);
        let has_near = self.width >= 30 && self.height >= 10;
        // far pines keep clear of the near ones, so each near tree keeps its outline
        let near_reach = if has_near {
            Self::half_width(near_size as i32) * 2 + 2
        } else {
            0
        };
        let far_size = ((ground as usize) / 3).clamp(2, 6);
        let mut x = rng.below(4) as i32;
        while x < self.width as i32 {
            let reach = Self::half_width(far_size as i32) + 1;
            let clear_of_cabin = x + reach < cabin_x || x - reach > cabin_x + cabin_w;
            let clear_of_near =
                x - reach > near_reach && x + reach < self.width as i32 - 1 - near_reach;
            if clear_of_cabin && clear_of_near && rng.chance(0.7) {
                self.pines.push(Pine {
                    x,
                    base: ground,
                    size: far_size - rng.below(2),
                    near: false,
                    snow: rng.next_u64(),
                });
            }
            x += Self::half_width(far_size as i32) * 2 + 1 + rng.below(4) as i32;
        }
        if has_near {
            let base = (self.height as i32 - 1).min(ground + 2);
            let reach = Self::half_width(near_size as i32);
            for x in [reach, self.width as i32 - 1 - reach] {
                self.pines.push(Pine {
                    x,
                    base,
                    size: near_size,
                    near: true,
                    snow: rng.next_u64(),
                });
            }
        }

        let flakes = self
            .motion
            .scaled(count_for(self.width, ground.max(1) as usize, 30, 3, 220));
        self.flakes = (0..flakes).map(|_| self.new_flake(rng, false)).collect();
        let sky = (ground - far_size as i32 - 1).max(1);
        let stars = count_for(self.width, sky as usize, 70, 1, 60);
        self.stars = (0..stars)
            .map(|_| (rng.below(self.width) as i32, rng.below(sky as usize) as i32))
            .collect();
        self.drifts = (0..self.width)
            .map(|_| *rng.pick(&['_', '_', '_', '-', '_', '.']))
            .collect();
        self.puffs.clear();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let ground = self.ground() as f32;
        let width = self.width as f32;
        for i in 0..self.flakes.len() {
            let flake = &mut self.flakes[i];
            flake.y += flake.speed * dt;
            let sway = (cycle(self.t, 0.6 * self.motion.speed()) + flake.phase).sin() * 0.6;
            flake.x = wrap(flake.x + sway * self.motion.speed() * dt, width);
            if flake.y >= ground {
                self.flakes[i] = self.new_flake(rng, true);
            }
        }

        let stage = self.puff_stage();
        let speed = self.motion.speed();
        for puff in &mut self.puffs {
            puff.age += dt;
            puff.y -= 0.5 * speed * dt;
            // a light breeze leans the smoke to the right as it climbs
            puff.x += (0.35 + 0.4 * (puff.age * 1.3 + puff.phase).sin()) * speed * dt;
        }
        self.puffs.retain(|p| p.age < stage * 3.0 && p.y >= 0.0);
        self.next_puff -= dt;
        if self.next_puff <= 0.0 {
            self.next_puff = if self.motion.calm { 2.0 } else { 1.1 } / self.motion.busy();
            let (x, y) = self.chimney();
            if y >= 1.0 {
                self.puffs.push(Puff {
                    x,
                    y,
                    age: 0.0,
                    phase: rng.range(0.0, TAU),
                });
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let ground = self.ground();
        for &(x, y) in &self.stars {
            frame.put(x, y, '+', Some(theme.muted));
        }
        if self.width >= 40 && self.height >= 14 {
            frame.put_sprite(
                self.width as i32 - 10,
                1,
                &MOON,
                Some(theme.bright_yellow()),
            );
        }

        for pine in self.pines.iter().filter(|p| !p.near) {
            Self::draw_pine(frame, theme, pine);
        }

        let stage = self.puff_stage();
        for puff in &self.puffs {
            let (ch, colour) = match (puff.age / stage) as usize {
                0 => ('o', theme.foreground),
                1 => ('O', theme.grey()),
                _ => ('.', theme.muted),
            };
            frame.put(
                puff.x.round() as i32,
                puff.y.round() as i32,
                ch,
                Some(colour),
            );
        }

        // the snow's surface and the ground beneath it
        for (x, ch) in self.drifts.iter().enumerate() {
            frame.put(x as i32, ground, *ch, Some(theme.foreground));
        }
        for y in ground + 1..self.height as i32 {
            for x in 0..self.width as i32 {
                let h = (x as u64)
                    .wrapping_mul(0x9E37_79B9)
                    .wrapping_add(y as u64 * 0x85EB_CA6B);
                if (h ^ (h >> 13)) % 19 == 0 {
                    frame.put(x, y, '.', Some(theme.muted));
                }
            }
        }

        // the cabin, solid so the trees and smoke never show through it
        let (cx, cy) = self.cabin_at();
        let rows = self.cabin();
        for (dy, row) in rows.iter().enumerate() {
            let first = row.find(|c| c != ' ').unwrap_or(0);
            let last = row.rfind(|c| c != ' ').unwrap_or(0);
            for dx in first..=last {
                frame.put(cx + dx as i32, cy + dy as i32, ' ', None);
            }
        }
        if rows.len() == CABIN.len() {
            frame.put_sprite(cx, cy, &rows[..4], Some(theme.foreground));
            frame.put_sprite(cx, cy + 4, &rows[4..], Some(theme.yellow()));
            frame.put_str(cx + 10, cy, "_", Some(theme.grey()));
            frame.put_str(cx + 9, cy + 1, "| |", Some(theme.grey()));
            frame.put_str(cx + 10, cy + 4, "##", Some(theme.bright_yellow()));
            frame.put_str(cx + 10, cy + 5, "##", Some(theme.bright_yellow()));
            // the window's light spills onto the snow
            if ground + 1 < self.height as i32 {
                frame.put_str(cx + 8, ground + 1, ". : .", Some(theme.yellow()));
            }
        } else {
            frame.put_sprite(cx, cy, &rows[..3], Some(theme.foreground));
            frame.put_sprite(cx, cy + 3, &rows[3..], Some(theme.yellow()));
            frame.put(cx + 7, cy, 'n', Some(theme.grey()));
            frame.put_str(cx + 2, cy + 3, "[]", Some(theme.bright_yellow()));
        }

        for pine in self.pines.iter().filter(|p| p.near) {
            Self::draw_pine(frame, theme, pine);
        }

        for flake in &self.flakes {
            let (ch, colour) = if flake.near {
                ('*', theme.foreground)
            } else {
                ('.', theme.foreground)
            };
            frame.put(
                flake.x.round() as i32,
                flake.y.floor() as i32,
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
        check_scene("winter-cabin");
        check_determinism("winter-cabin");
    }

    #[test]
    fn the_cabin_window_is_lit_and_smoke_rises() {
        const { assert!(PUFF_STAGE >= 1.0 / 3.0) };
        let mut rng = Rng::new(4);
        let mut scene = WinterCabin::new(80, 24, Motion::FULL, &mut rng);
        let frame = render(&mut scene, 80, 24, 100);
        let text = frame.to_text();
        assert!(text.contains("[##]"), "{text}");
        let (_, chimney_y) = scene.chimney();
        let above: String = (0..chimney_y as usize).map(|y| frame.row_text(y)).collect();
        assert!(above.contains('o') || above.contains('O'), "{above}");
        let theme = crate::theme::default_theme();
        assert!(frame.colours().contains(&theme.bright_yellow()));
    }

    #[test]
    fn snow_never_falls_below_the_ground_and_small_screens_get_a_small_cabin() {
        let mut rng = Rng::new(7);
        let mut scene = WinterCabin::new(20, 8, Motion::LIVELY, &mut rng);
        for _ in 0..600 {
            scene.step(0.05, &mut rng);
            assert!(scene.flakes.iter().all(|f| f.y < scene.ground() as f32));
        }
        let frame = render(&mut scene, 20, 8, 1);
        assert!(frame.to_text().contains("/________\\"));
    }
}
