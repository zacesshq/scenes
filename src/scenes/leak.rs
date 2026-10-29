//! Leak: a pipe across a room with a crack in it, dripping slowly into a bucket. The bucket fills
//! over about a minute, then overflows and a puddle spreads across the floor until it reaches its
//! widest and holds there, while a gauge on the pipe sinks as the pressure falls.

use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::{error_colour, warning_colour};

pub const SUMMARY: &str =
    "A cracked pipe dripping into a bucket that slowly fills and overflows into a spreading puddle";

pub const CAPTIONS: &[&str] = &[
    "Something is leaking. Find the first drip.",
    "A slow leak. Patch the first error and the rest dry up.",
    "The bucket is filling. Time for a closer look.",
    "One crack at a time. Start at the top of the list.",
    "Not a flood yet. Read the first message.",
];

/// Seconds between two drips at the standard pace. A drip is a small moving shape and its
/// splash changes a cell twice within a third of a second, once a drip.
pub const DRIP_PERIOD: f32 = 1.1;
/// How long a splash stays on the water or the floor.
const SPLASH_LENGTH: f32 = 0.3;
/// Drips that fill the bucket, so it takes about a minute at the standard pace.
const DRIPS_TO_FILL: f32 = 50.0;
/// Seconds for the gauge to sink from full to its lowest reading.
const PRESSURE_FALL: f32 = 120.0;

#[derive(Clone, Debug)]
struct Drip {
    y: f32,
    vy: f32,
}

pub struct Leak {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    since_drip: f32,
    drips: Vec<Drip>,
    /// How full the bucket is, from 0 to 1.
    level: f32,
    /// Half the puddle's width in cells, growing once the bucket overflows.
    puddle: f32,
    /// Seconds since the last splash, if one is showing.
    splash: Option<f32>,
}

impl Leak {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut leak = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            since_drip: 0.0,
            drips: Vec::new(),
            level: 0.0,
            puddle: 0.0,
            splash: None,
        };
        leak.resize(width, height, rng);
        leak
    }

    fn floor(&self) -> i32 {
        self.height as i32 - 1
    }

    fn pipe_row(&self) -> i32 {
        ((self.height as i32) / 5).max(1)
    }

    fn leak_x(&self) -> i32 {
        (self.width as i32 * 2 / 5).max(1)
    }

    /// The bucket's outer width and height, the base row included.
    fn bucket_size(&self) -> (i32, i32) {
        let w = (self.width as i32 / 7).clamp(5, 11);
        let w = if w % 2 == 0 { w + 1 } else { w };
        let h = (self.height as i32 / 5).clamp(2, 6);
        (w, h)
    }

    /// The bucket's left column and its top (rim) row.
    fn bucket_origin(&self) -> (i32, i32) {
        let (w, h) = self.bucket_size();
        (self.leak_x() - w / 2, self.floor() - h)
    }

    /// The rows of water inside the bucket, which grow with the level.
    fn water_rows(&self) -> i32 {
        let (_, h) = self.bucket_size();
        (self.level * (h - 1) as f32).round() as i32
    }

    /// The row a drip lands on: the water's surface or else the bucket's base when it is empty.
    fn landing_row(&self) -> i32 {
        let (_, top) = self.bucket_origin();
        let (_, h) = self.bucket_size();
        let base = top + h - 1;
        if self.water_rows() > 0 {
            base - self.water_rows()
        } else {
            base - 1
        }
    }

    fn drip_period(&self) -> f32 {
        DRIP_PERIOD / self.motion.pace()
    }

    fn widest_puddle(&self) -> f32 {
        (self.width as f32 * 0.42).max(4.0)
    }

    fn pressure(&self) -> f32 {
        (1.0 - (self.t as f32 / PRESSURE_FALL)).clamp(0.1, 1.0)
    }
}

impl Scene for Leak {
    fn name(&self) -> &'static str {
        "leak"
    }

    fn resize(&mut self, width: usize, height: usize, _rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        // drips in flight are dropped, the bucket and the puddle are kept
        self.drips.clear();
        self.puddle = self.puddle.min(self.widest_puddle());
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
        self.since_drip += dt;
        if self.since_drip >= self.drip_period() {
            self.since_drip -= self.drip_period();
            self.since_drip = self.since_drip.min(self.drip_period());
            if self.drips.len() < 4 {
                self.drips.push(Drip {
                    y: (self.pipe_row() + 1) as f32,
                    vy: 1.0,
                });
            }
        }
        if let Some(age) = self.splash.as_mut() {
            *age += dt;
            if *age >= SPLASH_LENGTH / self.motion.pace() {
                self.splash = None;
            }
        }
        let landing = self.landing_row() as f32;
        let gravity = 14.0 * self.motion.speed();
        let mut landed = 0;
        for drip in &mut self.drips {
            drip.vy += gravity * dt;
            drip.y += drip.vy * dt;
            if drip.y >= landing {
                landed += 1;
            }
        }
        self.drips.retain(|d| d.y < landing);
        for _ in 0..landed {
            self.splash = Some(0.0);
            if self.level < 1.0 {
                self.level = (self.level + 1.0 / DRIPS_TO_FILL).min(1.0);
            } else {
                // a full bucket spills over the rim, so the puddle spreads a little a drip
                self.puddle = (self.puddle + 0.35).min(self.widest_puddle());
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let width = self.width as i32;
        let floor = self.floor();
        let pipe = self.pipe_row();
        let leak_x = self.leak_x();
        let (bw, bh) = self.bucket_size();
        let (bx, top) = self.bucket_origin();
        let water = Some(theme.bright_blue());
        let deep = Some(theme.blue());

        // the floor, with the puddle drawn over it
        frame.fill_row(floor, '_', Some(theme.muted));
        let centre = bx + bw / 2;
        let half = self.puddle.round() as i32;
        if half > 0 {
            for x in centre - half - bw / 2..=centre + half + bw / 2 {
                let edge = (x - centre).abs() >= half + bw / 2 - 1;
                frame.put(x, floor, if edge { '.' } else { '~' }, water);
            }
        }

        // the pipe: a run along the ceiling and down the right-hand wall, with joints
        let bend = (width * 4 / 5).max(leak_x + bw / 2 + 2);
        for x in 0..bend {
            let ch = if x % 12 == 6 { '#' } else { '=' };
            frame.put(x, pipe, ch, Some(theme.grey()));
        }
        frame.put(bend, pipe, '.', Some(theme.grey()));
        for y in pipe + 1..floor {
            let ch = if (y - pipe) % 6 == 3 { '#' } else { '|' };
            frame.put(bend, y, ch, Some(theme.grey()));
        }
        // the crack, marked by shape as well as colour
        frame.put(leak_x, pipe, 'X', Some(error_colour(theme)));
        if pipe >= 1 && self.width >= 30 {
            frame.put_str(leak_x - 1, pipe - 1, "\\ /", Some(error_colour(theme)));
        }

        // a pressure gauge on the pipe whose needle tips over as the pressure falls
        if pipe >= 1 && self.width >= 24 {
            let gx = (leak_x / 3).max(1);
            let pressure = self.pressure();
            let (needle, colour) = if pressure > 0.66 {
                ('/', theme.foreground)
            } else if pressure > 0.33 {
                ('-', warning_colour(theme))
            } else {
                ('\\', error_colour(theme))
            };
            frame.put(gx - 1, pipe - 1, '(', Some(theme.grey()));
            frame.put(gx, pipe - 1, needle, Some(colour));
            frame.put(gx + 1, pipe - 1, ')', Some(theme.grey()));
        }

        // the bucket and the water in it
        let base = top + bh - 1;
        for y in top..base {
            frame.put(bx, y, '|', Some(theme.foreground));
            frame.put(bx + bw - 1, y, '|', Some(theme.foreground));
        }
        frame.put(bx, base, '\\', Some(theme.foreground));
        frame.put(bx + bw - 1, base, '/', Some(theme.foreground));
        for x in bx + 1..bx + bw - 1 {
            frame.put(x, base, '_', Some(theme.foreground));
        }
        let rows = self.water_rows();
        for i in 0..rows {
            let y = base - 1 - i;
            let surface = i == rows - 1;
            for x in bx + 1..bx + bw - 1 {
                if surface {
                    frame.put(x, y, '~', water);
                } else {
                    frame.put(x, y, theme.faint_shade(), deep);
                }
            }
        }
        if self.level >= 1.0 {
            // water running down the outside of a full bucket
            for y in top..base {
                frame.put(bx + bw, y, ':', water);
            }
        }

        for drip in &self.drips {
            let ch = if drip.vy < 4.0 { '\'' } else { '.' };
            frame.put(leak_x, drip.y.floor() as i32, ch, water);
        }
        if self.splash.is_some() {
            let y = self.landing_row();
            frame.put(leak_x - 1, y - 1, '\'', water);
            frame.put(leak_x + 1, y - 1, '\'', water);
        }

        // a wet floor sign beside the bucket, when there is room for it
        if self.width >= 30 && self.height >= 10 {
            let sx = bx - 9;
            if sx >= 1 {
                frame.put_str(sx, floor - 2, " /\\", Some(warning_colour(theme)));
                frame.put_str(sx, floor - 1, "/!!\\", Some(warning_colour(theme)));
                if self.width >= 50 {
                    frame.put_str(sx - 5, floor - 3, "WET FLOOR", Some(warning_colour(theme)));
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
        check_scene("leak");
        check_determinism("leak");
    }

    #[test]
    fn drips_are_slow_and_never_quicker_on_lively() {
        let mut rng = Rng::new(1);
        for motion in Motion::ALL {
            let leak = Leak::new(80, 24, motion, &mut rng);
            assert!(leak.drip_period() >= DRIP_PERIOD, "{motion:?}");
        }
        const { assert!(SPLASH_LENGTH < DRIP_PERIOD) };
    }

    #[test]
    fn the_bucket_fills_then_the_puddle_spreads_and_holds() {
        let mut rng = Rng::new(4);
        let mut leak = Leak::new(80, 24, Motion::FULL, &mut rng);
        for _ in 0..(30 * 20) {
            leak.step(0.05, &mut rng);
        }
        assert!(leak.level > 0.2 && leak.level < 1.0, "{}", leak.level);
        assert_eq!(leak.puddle, 0.0);
        for _ in 0..(60 * 60 * 20) {
            leak.step(0.05, &mut rng);
        }
        assert_eq!(leak.level, 1.0);
        assert_eq!(leak.puddle, leak.widest_puddle());
        let frame = render(&mut leak, 80, 24, 1);
        assert!(frame.row_text(23).contains('~'));
        assert!(frame.row_text(leak.pipe_row() as usize).contains('X'));
        assert!(frame.to_text().contains("WET FLOOR"));
    }

    #[test]
    fn a_resize_keeps_the_water() {
        let mut rng = Rng::new(4);
        let mut leak = Leak::new(80, 24, Motion::FULL, &mut rng);
        for _ in 0..(40 * 20) {
            leak.step(0.05, &mut rng);
        }
        let level = leak.level;
        leak.resize(20, 8, &mut rng);
        assert_eq!(leak.level, level);
    }
}
