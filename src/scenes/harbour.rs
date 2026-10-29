//! Harbour: a small harbour town at night. Lamps along the quay throw long reflections that
//! shimmer slowly on the water, moored boats sway at their masts as the water laps their hulls, a
//! lighthouse keeps a steady lamp on the breakwater and now and then a ferry crosses the far water.

use std::f32::consts::TAU;

use super::{count_for, cycle};
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "A harbour town at night, with moored boats, quay lamps and their reflections";

pub const CAPTIONS: &[&str] = &[
    "Everything is moored and secure.",
    "Safe in harbour. The build passed.",
    "Calm water, clean build.",
    "The lamps are lit and nothing is adrift.",
    "All hands report green.",
];

/// Seconds between two changes of a reflection's shimmer. Each reflection cell changes at most
/// once a step, so well under twice a second.
pub const SHIMMER_PERIOD: f32 = 0.8;
/// Seconds for one sway of a mast, which passes through four positions on the way.
pub const SWAY_PERIOD: f32 = 4.0;

const LIGHTHOUSE: [&str; 5] = ["  _  ", " [o] ", " | | ", " |=| ", "_|_|_"];
const FERRY: [&str; 2] = ["  _|_  ", "\\:::::/"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hull {
    Sail,
    Cabin,
    Rowing,
}

impl Hull {
    fn rows(self) -> &'static [&'static str] {
        match self {
            Hull::Sail => &["   |   ", "   |   ", "___|___", "\\_____/"],
            Hull::Cabin => &["  _    ", " |o|__ ", "\\_____/"],
            Hull::Rowing => &["\\_=_/"],
        }
    }

    fn width(self) -> i32 {
        self.rows()[self.rows().len() - 1].len() as i32
    }
}

#[derive(Clone, Debug)]
struct Boat {
    x: i32,
    /// The row of the hull's bottom.
    y: i32,
    hull: Hull,
    colour: usize,
    phase: f32,
}

#[derive(Clone, Debug)]
struct Building {
    x: i32,
    width: i32,
    height: i32,
    windows: Vec<(i32, i32)>,
}

pub struct Harbour {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    stars: Vec<(i32, i32, bool)>,
    buildings: Vec<Building>,
    lamps: Vec<i32>,
    boats: Vec<Boat>,
    ferry_x: f32,
    ferry_wait: f32,
}

impl Harbour {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut harbour = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            stars: Vec::new(),
            buildings: Vec::new(),
            lamps: Vec::new(),
            boats: Vec::new(),
            ferry_x: -10.0,
            ferry_wait: rng.range(2.0, 10.0),
        };
        harbour.resize(width, height, rng);
        harbour
    }

    /// The quay's row: the sky and the town are above it, the water below.
    fn quay(&self) -> i32 {
        ((self.height as f32 * 0.42) as i32).max(2)
    }

    fn lighthouse_fits(&self) -> bool {
        self.width >= 50 && self.quay() >= 6
    }

    fn ferry_fits(&self) -> bool {
        self.width >= 40 && self.height as i32 - self.quay() >= 8
    }

    fn shimmer_period(&self) -> f32 {
        SHIMMER_PERIOD / self.motion.pace()
    }

    fn sway_period(&self) -> f32 {
        SWAY_PERIOD / self.motion.pace()
    }

    /// The step of the shimmer, so every reflection changes on the same slow beat.
    fn shimmer_step(&self) -> u64 {
        (self.t / f64::from(self.shimmer_period())) as u64
    }

    /// The mast top for a sway from -1 to 1, through four positions.
    fn mast_top(&self, phase: f32) -> char {
        if self.motion.calm {
            return '|';
        }
        let lean = (cycle(self.t, TAU / self.sway_period()) + phase).sin();
        if lean > 0.6 {
            '/'
        } else if lean < -0.6 {
            '\\'
        } else {
            '|'
        }
    }

    fn hash(a: i64, b: i64) -> u64 {
        let mut z =
            (a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (b as u64).wrapping_add(0x632B_E59B);
        z = (z ^ (z >> 29)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z ^ (z >> 32)
    }
}

impl Scene for Harbour {
    fn name(&self) -> &'static str {
        "harbour"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let quay = self.quay();
        let w = self.width as i32;

        // the town: a row of buildings of one to three storeys along the quay
        self.buildings.clear();
        let tallest = (quay - 3).clamp(1, 4);
        let right_edge = if self.lighthouse_fits() { w - 10 } else { w };
        let mut x = rng.below(3) as i32;
        while x < right_edge - 3 {
            let width = 4 + rng.below(5) as i32;
            let height = 1 + rng.below(tallest as usize) as i32;
            let mut windows = Vec::new();
            for wy in 1..height {
                for wx in (1..width - 1).step_by(2) {
                    if rng.chance(0.35) {
                        windows.push((wx, wy));
                    }
                }
            }
            if height == 1 && rng.chance(0.6) {
                windows.push((width / 2, 0));
            }
            self.buildings.push(Building {
                x,
                width,
                height,
                windows,
            });
            x += width + rng.below(3) as i32;
        }

        let sky_rows = (quay - tallest - 1).max(0);
        let stars = count_for(self.width, sky_rows.max(1) as usize, 30, 2, 120);
        self.stars = if sky_rows > 0 {
            (0..stars)
                .map(|_| {
                    (
                        rng.below(self.width) as i32,
                        rng.below(sky_rows as usize) as i32,
                        rng.chance(0.25),
                    )
                })
                .collect()
        } else {
            Vec::new()
        };

        let spacing = if self.width >= 60 { 14 } else { 9 };
        self.lamps = (0..)
            .map(|i| 3 + i * spacing)
            .take_while(|&x| x < right_edge - 2)
            .collect();

        // boats sit in the water, the larger ones further out
        self.boats.clear();
        let water_top = quay + 2;
        let bottom = self.height as i32 - 1;
        let wanted = self
            .motion
            .scaled(count_for(self.width, self.height, 260, 1, 8));
        for _ in 0..wanted * 4 {
            if self.boats.len() >= wanted {
                break;
            }
            let roomy = bottom - water_top >= 4;
            let hull = match rng.below(3) {
                0 if roomy => Hull::Sail,
                1 if roomy => Hull::Cabin,
                _ => Hull::Rowing,
            };
            let rows = hull.rows().len() as i32;
            let lowest = bottom - 1;
            let highest = water_top + rows - 1;
            if lowest < highest || w < hull.width() + 2 {
                continue;
            }
            let y = highest + rng.below((lowest - highest + 1) as usize) as i32;
            let x = rng.below((w - hull.width()) as usize) as i32;
            let clash = self.boats.iter().any(|b| {
                let top = b.y - b.hull.rows().len() as i32;
                x < b.x + b.hull.width() + 2
                    && b.x < x + hull.width() + 2
                    && y > top - 1
                    && b.y > y - rows - 1
            });
            if !clash {
                self.boats.push(Boat {
                    x,
                    y,
                    hull,
                    colour: rng.below(3),
                    phase: rng.range(0.0, TAU),
                });
            }
        }
        // nearer boats are drawn last so they sit in front
        self.boats.sort_by_key(|b| b.y);
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        if !self.ferry_fits() {
            return;
        }
        if self.ferry_wait > 0.0 {
            self.ferry_wait -= dt;
            return;
        }
        self.ferry_x += 0.8 * self.motion.speed() * dt;
        if self.ferry_x > self.width as f32 + 1.0 {
            self.ferry_x = -(FERRY[1].len() as f32);
            self.ferry_wait = rng.range(8.0, 25.0);
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let quay = self.quay();
        let w = self.width as i32;
        let bottom = self.height as i32 - 1;

        for &(x, y, bright) in &self.stars {
            let (ch, colour) = if bright {
                ('+', theme.foreground)
            } else {
                ('.', theme.muted)
            };
            frame.put(x, y, ch, Some(colour));
        }
        if self.width >= 40 && quay >= 7 {
            frame.put_sprite(4, 0, &[" ,-", "(  ", " `-"], Some(theme.bright_yellow()));
        }

        // the town
        for building in &self.buildings {
            // the town stands back behind the promenade, where the lamps are
            let top = quay - 1 - building.height;
            for dx in 0..building.width {
                frame.put(building.x + dx, top - 1, '_', Some(theme.grey()));
            }
            for dy in 0..building.height {
                frame.put(building.x, top + dy, '|', Some(theme.grey()));
                frame.put(
                    building.x + building.width - 1,
                    top + dy,
                    '|',
                    Some(theme.grey()),
                );
            }
            for &(wx, wy) in &building.windows {
                frame.put(building.x + wx, top + wy, ':', Some(theme.yellow()));
            }
        }

        // the quay edge and its lamps
        frame.fill_row(quay, '=', Some(theme.grey()));
        for &x in &self.lamps {
            frame.put(x, quay - 1, 'o', Some(theme.bright_yellow()));
        }

        if self.lighthouse_fits() {
            let x = w - 8;
            let top = quay - LIGHTHOUSE.len() as i32;
            frame.put_sprite(x, top, &LIGHTHOUSE, Some(theme.foreground));
            frame.put(x + 2, top + 1, 'o', Some(theme.bright_yellow()));
            frame.put(x + 1, top + 3, '=', Some(theme.red()));
            frame.put(x + 3, top + 3, '=', Some(theme.red()));
        }

        // the water is mostly dark and still: a few long, low swell lines on every third row
        let speed = self.motion.speed();
        let drift = self.t * f64::from(0.4 * speed);
        for y in (quay + 2..=bottom).step_by(3) {
            let offset = (drift * if y % 2 == 0 { 1.0 } else { -0.7 }).floor() as i64;
            for x in 0..w {
                let wx = i64::from(x) + offset;
                // a swell line is a run of a few cells, with long gaps of still water between
                let run = wx.div_euclid(14);
                let at = wx.rem_euclid(14);
                if Self::hash(run, i64::from(y)) % 3 == 0 && (2..6).contains(&at) {
                    let ch = if at == 2 || at == 5 { '-' } else { '~' };
                    frame.put(x, y, ch, Some(theme.blue()));
                }
            }
        }

        // each lamp's reflection: a straight column of light down the water, broken here and
        // there on a slow beat, with a short glint either side near the top
        let step = self.shimmer_step();
        let length = ((bottom - quay) * 2 / 3).max(1);
        let light = |x: i32, length: i32, seed: i64, frame: &mut Frame| {
            for dy in 0..length {
                let y = quay + 1 + dy;
                let h = Self::hash(seed * 31 + i64::from(dy), step as i64);
                if dy > 1 && h % 5 == 0 {
                    continue;
                }
                let ch = if h % 3 == 0 { ':' } else { '|' };
                let colour = if dy < length / 2 {
                    theme.bright_yellow()
                } else {
                    theme.yellow()
                };
                frame.put(x, y, ch, Some(colour));
                if dy < length / 3 && (h >> 4) % 4 == 0 {
                    frame.put(x - 1, y, '-', Some(theme.yellow()));
                    frame.put(x + 1, y, '-', Some(theme.yellow()));
                }
            }
        };
        for &x in &self.lamps {
            light(x, length, i64::from(x), frame);
        }
        if self.lighthouse_fits() {
            light(w - 6, length + 1, 999, frame);
        }

        if self.ferry_fits() && self.ferry_wait <= 0.0 {
            let x = self.ferry_x.round() as i32;
            let y = quay + 1;
            frame.put_sprite(x, y, &FERRY, Some(theme.foreground));
            frame.put_str(x + 1, y + 1, ":::::", Some(theme.bright_yellow()));
        }

        for boat in &self.boats {
            let rows = boat.hull.rows();
            let top = boat.y - rows.len() as i32 + 1;
            let colour = match boat.colour {
                0 => theme.foreground,
                1 => theme.bright_cyan(),
                _ => theme.bright_red(),
            };
            // the hull's own row is cleared, so the swell never shows through the boat
            frame.put_str(boat.x, boat.y, rows[rows.len() - 1], None);
            frame.put_sprite(boat.x, top, rows, Some(colour));
            match boat.hull {
                Hull::Sail => {
                    let mast = boat.x + 3;
                    frame.put(mast, top, self.mast_top(boat.phase), Some(colour));
                    frame.put(mast, top - 1, '.', Some(theme.bright_yellow()));
                }
                Hull::Cabin => {
                    frame.put(boat.x + 2, top + 1, 'o', Some(theme.bright_yellow()));
                }
                Hull::Rowing => {}
            }
            // the water laps either side of the hull, rising and falling with the sway
            if boat.y < bottom {
                let lap = if self.motion.calm {
                    0
                } else {
                    ((cycle(self.t, TAU / self.sway_period()) + boat.phase).sin() > 0.0) as i32
                };
                let under = boat.y + 1;
                frame.put(boat.x - 1 + lap, under, '~', Some(theme.cyan()));
                frame.put(
                    boat.x + boat.hull.width() - lap,
                    under,
                    '~',
                    Some(theme.cyan()),
                );
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
        check_scene("harbour");
        check_determinism("harbour");
    }

    #[test]
    fn the_shimmer_and_the_sway_stay_slow() {
        const { assert!(SHIMMER_PERIOD >= 1.0 / 3.0) };
        // four mast positions a sway, so under one change a second
        const { assert!(SWAY_PERIOD / 4.0 >= 1.0 / 3.0) };
        for motion in Motion::ALL {
            let mut rng = Rng::new(2);
            let harbour = Harbour::new(80, 24, motion, &mut rng);
            assert!(harbour.shimmer_period() >= SHIMMER_PERIOD);
            assert!(harbour.sway_period() >= SWAY_PERIOD);
        }
    }

    #[test]
    fn the_quay_lamps_and_their_reflections_are_drawn() {
        let mut rng = Rng::new(6);
        let mut harbour = Harbour::new(80, 24, Motion::FULL, &mut rng);
        let frame = render(&mut harbour, 80, 24, 40);
        let quay = harbour.quay() as usize;
        assert!(frame.row_text(quay).chars().all(|c| c == '='));
        assert!(frame.row_text(quay - 1).contains('o'));
        assert!(frame.to_text().contains("[o]"));
        let theme = crate::theme::default_theme();
        let reflected = (quay + 1..24).any(|y| {
            (0..80).any(|x| {
                frame.get(x, y as i32).and_then(|c| c.colour) == Some(theme.bright_yellow())
            })
        });
        assert!(reflected);
        let small = render(&mut Harbour::new(20, 8, Motion::CALM, &mut rng), 20, 8, 20);
        assert!(small.row_text(3).chars().all(|c| c == '='));
        assert!(!small.to_text().contains("[o]"));
    }
}
