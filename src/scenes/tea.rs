//! Tea: a mug of tea on a table with steam curling up from it, a stack of books and a small
//! plant beside it and, behind, a window with rain or snow falling outside.

use super::{count_for, wrap};
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "A mug of tea with curling steam beside books and a plant, with weather at the window";

pub const CAPTIONS: &[&str] = &[
    "Build passed. Put the kettle on.",
    "A clean build and a warm mug.",
    "Time for a proper cup of tea.",
    "Nothing to fix. Let it brew.",
    "Green build, hot tea, quiet room.",
];

/// Seconds a curl of steam takes to sway from one side to the other and back. Its character
/// follows the sway, so it changes twice a period at most.
pub const SWAY_PERIOD: f32 = 2.4;
/// Seconds a curl of steam lasts.
const STEAM_LIFE: f32 = 5.0;

const MUG: [&str; 5] = [
    " .------. ",
    " |~~~~~~|_ ",
    " |      | )",
    " |      |/ ",
    " `------'  ",
];
const SMALL_MUG: [&str; 3] = [".----.", "|~~~~|)", "`----'"];
const PLANT: [&str; 3] = ["\\|/", " | ", "(_)"];
const LAMP: [&str; 2] = [" /^\\ ", "/___\\"];

#[derive(Clone, Debug)]
struct Puff {
    x: f32,
    y: f32,
    age: f32,
    phase: f32,
}

#[derive(Clone, Debug)]
struct Flake {
    x: f32,
    y: f32,
    speed: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Window {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

pub struct Tea {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    snow: bool,
    puffs: Vec<Puff>,
    since_puff: f32,
    window: Option<Window>,
    weather: Vec<Flake>,
}

impl Tea {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut tea = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            snow: rng.chance(0.5),
            puffs: Vec::new(),
            since_puff: 0.0,
            window: None,
            weather: Vec::new(),
        };
        tea.resize(width, height, rng);
        tea
    }

    fn large(&self) -> bool {
        self.width >= 40 && self.height >= 14
    }

    /// The row of the table top.
    fn table(&self) -> i32 {
        let h = self.height as i32;
        if self.large() { h - 3 } else { h - 2 }
    }

    fn mug_rows(&self) -> &'static [&'static str] {
        if self.large() { &MUG } else { &SMALL_MUG }
    }

    /// The column of the mug's left edge and the column steam rises from.
    fn mug_x(&self) -> (i32, i32) {
        let w = self.mug_rows()[0].len() as i32;
        let left = (self.width as i32 * 3 / 5 - w / 2).max(0);
        (left, left + w / 2)
    }

    fn mug_top(&self) -> i32 {
        self.table() - self.mug_rows().len() as i32
    }

    fn sway_period(&self) -> f32 {
        SWAY_PERIOD / self.motion.pace()
    }

    fn puff_gap(&self) -> f32 {
        let gap = if self.motion.calm { 1.2 } else { 0.6 };
        gap / self.motion.busy()
    }

    fn new_flake(&self, rng: &mut Rng, from_top: bool) -> Flake {
        let Some(w) = self.window else {
            return Flake {
                x: 0.0,
                y: 0.0,
                speed: 1.0,
            };
        };
        let speed = if self.snow {
            rng.range(1.0, 2.0)
        } else {
            rng.range(5.0, 8.0)
        };
        Flake {
            x: rng.range(0.0, (w.width - 2).max(1) as f32),
            y: if from_top {
                0.0
            } else {
                rng.range(0.0, (w.height - 2).max(1) as f32)
            },
            speed: speed * self.motion.speed(),
        }
    }
}

impl Scene for Tea {
    fn name(&self) -> &'static str {
        "tea"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.puffs.clear();
        let w = self.width as i32;
        let h = self.height as i32;
        self.window = if w >= 30 && h >= 12 {
            let width = (w / 3).clamp(12, 40);
            let height = ((self.table() - 3) * 2 / 3).clamp(5, 18);
            Some(Window {
                left: (w / 6).max(1),
                top: 1,
                width,
                height,
            })
        } else {
            None
        };
        let wanted = self.window.map_or(0, |w| {
            self.motion
                .scaled(count_for(w.width as usize, w.height as usize, 9, 3, 120))
        });
        self.weather = (0..wanted).map(|_| self.new_flake(rng, false)).collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        let (_, steam_x) = self.mug_x();
        self.since_puff += dt;
        if self.since_puff >= self.puff_gap() {
            self.since_puff = 0.0;
            self.puffs.push(Puff {
                x: steam_x as f32 + rng.range(-1.5, 1.5),
                y: self.mug_top() as f32,
                age: 0.0,
                phase: rng.range(0.0, 1.0),
            });
        }
        for puff in &mut self.puffs {
            puff.age += dt;
            puff.y -= 1.1 * speed * dt;
        }
        self.puffs.retain(|p| p.age < STEAM_LIFE && p.y >= 0.0);

        if let Some(w) = self.window {
            let inner = (w.height - 2).max(1) as f32;
            let across = (w.width - 2).max(1) as f32;
            for i in 0..self.weather.len() {
                let flake = &mut self.weather[i];
                flake.y += flake.speed * dt;
                if self.snow {
                    flake.x = wrap(flake.x + 0.4 * speed * dt, across);
                }
                if flake.y >= inner {
                    self.weather[i] = self.new_flake(rng, true);
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let w = self.width as i32;
        let h = self.height as i32;

        if let Some(win) = self.window {
            let right = win.left + win.width - 1;
            let bottom = win.top + win.height - 1;
            let mid_x = win.left + win.width / 2;
            let frame_colour = Some(theme.yellow());
            for x in win.left..=right {
                frame.put(x, win.top, '\u{2500}', frame_colour);
                frame.put(x, bottom, '\u{2500}', frame_colour);
            }
            for y in win.top..=bottom {
                frame.put(win.left, y, '\u{2502}', frame_colour);
                frame.put(right, y, '\u{2502}', frame_colour);
                frame.put(mid_x, y, '\u{2502}', frame_colour);
            }
            frame.put(win.left, win.top, '\u{250c}', frame_colour);
            frame.put(right, win.top, '\u{2510}', frame_colour);
            frame.put(win.left, bottom, '\u{2514}', frame_colour);
            frame.put(right, bottom, '\u{2518}', frame_colour);
            frame.put(mid_x, win.top, '\u{252c}', frame_colour);
            frame.put(mid_x, bottom, '\u{2534}', frame_colour);
            // a far roofline sits low in the window, so the weather has somewhere to fall
            let roof_y = bottom - 1;
            for x in win.left + 1..right {
                if x == mid_x {
                    continue;
                }
                let ch = match (x - win.left) % 9 {
                    2 => '/',
                    3 => '\\',
                    _ => '_',
                };
                frame.put(x, roof_y, ch, Some(theme.grey()));
            }
            for flake in &self.weather {
                let x = win.left + 1 + flake.x as i32;
                let y = win.top + 1 + flake.y as i32;
                if x == mid_x || y >= roof_y {
                    continue;
                }
                let (ch, colour) = if self.snow {
                    ('*', theme.foreground)
                } else {
                    ('|', theme.blue())
                };
                frame.put(x, y, ch, Some(colour));
            }
        }

        // a hanging lamp fills the empty corner opposite the window
        let lamp_x = w * 5 / 6;
        if self.large()
            && self
                .window
                .is_some_and(|win| win.left + win.width + 4 < lamp_x)
        {
            let drop = (self.table() / 4).max(1);
            for y in 0..drop {
                frame.put(lamp_x + 2, y, '|', Some(theme.grey()));
            }
            frame.put_sprite(lamp_x, drop, &LAMP, Some(theme.bright_yellow()));
        }

        let table = self.table();
        frame.fill_row(table, '=', Some(theme.yellow()));
        if self.large() {
            for y in table + 1..h {
                frame.put(3, y, '|', Some(theme.yellow()));
                frame.put(w - 4, y, '|', Some(theme.yellow()));
            }
        }

        let (mug_left, _) = self.mug_x();
        let top = self.mug_top();
        frame.put_sprite(mug_left, top, self.mug_rows(), Some(theme.cyan()));
        // the tea itself in a warmer colour than the mug
        for (i, ch) in self.mug_rows()[1].chars().enumerate() {
            if ch == '~' {
                frame.put(mug_left + i as i32, top + 1, '~', Some(theme.yellow()));
            }
        }

        if self.large() {
            let books_x = mug_left - 14;
            let spines = [
                ("=========", theme.red()),
                ("========", theme.blue()),
                ("==========", theme.green()),
            ];
            for (i, (text, colour)) in spines.iter().enumerate() {
                frame.put_str(
                    books_x + (i as i32 % 2),
                    table - 3 + i as i32,
                    text,
                    Some(*colour),
                );
            }
            let plant_x = mug_left + 15;
            if plant_x + 3 < w {
                frame.put_sprite(plant_x, table - 3, &PLANT[..2], Some(theme.green()));
                frame.put_sprite(plant_x, table - 1, &PLANT[2..], Some(theme.red()));
            }
        }

        let period = self.sway_period();
        for puff in &self.puffs {
            let sway = super::cycle(self.t, std::f32::consts::TAU / period)
                + puff.phase * std::f32::consts::TAU;
            let offset = sway.sin() * 1.2;
            let ch = if sway.cos() >= 0.0 { '(' } else { ')' };
            let (ch, colour) = if puff.age > STEAM_LIFE * 0.7 {
                ('.', theme.grey())
            } else {
                (ch, theme.muted)
            };
            frame.put(
                (puff.x + offset).round() as i32,
                puff.y as i32,
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
        check_scene("tea");
        check_determinism("tea");
    }

    #[test]
    fn steam_rises_from_the_mug_and_sways_slowly() {
        let mut rng = Rng::new(5);
        for motion in Motion::ALL {
            let tea = Tea::new(80, 24, motion, &mut rng);
            assert!(tea.sway_period() >= SWAY_PERIOD);
            // a curl changes character twice a period
            assert!(2.0 / tea.sway_period() <= 3.0);
        }
        let mut tea = Tea::new(80, 24, Motion::FULL, &mut rng);
        let frame = render(&mut tea, 80, 24, 60);
        let text = frame.to_text();
        assert!(text.contains("|~~~~~~|"), "{text}");
        let top = tea.mug_top();
        let above: String = (0..top as usize).map(|y| frame.row_text(y)).collect();
        assert!(above.contains('(') || above.contains(')'), "{text}");
    }

    #[test]
    fn the_window_and_the_books_need_room() {
        let mut rng = Rng::new(1);
        let big = render(&mut Tea::new(80, 24, Motion::FULL, &mut rng), 80, 24, 20);
        assert!(big.to_text().contains('\u{250c}'));
        let small = render(&mut Tea::new(20, 8, Motion::FULL, &mut rng), 20, 8, 20);
        assert!(!small.to_text().contains('\u{250c}'));
        assert!(small.to_text().contains("|~~~~|"));
    }
}
