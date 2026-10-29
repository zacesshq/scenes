//! Flood: a street of houses under a grey sky where the water rises slowly over the first minute,
//! then settles into a gentle swell with debris drifting along the surface and a warning sign
//! standing half in the water.

use std::f32::consts::TAU;

use super::{count_for, cycle, wrap};
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::warning_colour;

/// Seconds the water takes to reach its highest level at the standard pace.
pub const RISE_SECONDS: f32 = 45.0;
/// Seconds of one slow swell of the risen water. The surface row moves at most twice a swell.
pub const SWELL_PERIOD: f32 = 12.0;
/// How far the swell lifts and lowers the water, in rows.
const SWELL_ROWS: f32 = 0.35;

pub const SUMMARY: &str =
    "A street where the water rises slowly past the doors, with debris drifting on the surface";

pub const CAPTIONS: &[&str] = &[
    "The water is rising. Bail out the first error.",
    "Wet feet, not a lost house. Start with the top error.",
    "Sandbags first: one fix at a time.",
    "High water in the build. It goes down as fast.",
    "Keep your head above the error list.",
];

const DEBRIS: [&str; 6] = ["[#]", "(=)", "===", "[_]", "\\_/", "o"];

#[derive(Clone, Debug)]
struct House {
    x: i32,
    width: i32,
    height: i32,
}

#[derive(Clone, Debug)]
struct Debris {
    x: f32,
    speed: f32,
    sprite: usize,
}

#[derive(Clone, Debug)]
struct Drop {
    x: f32,
    y: f32,
    speed: f32,
}

pub struct Flood {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// Seconds of rising so far, kept across a resize so the water never drains away.
    risen: f32,
    houses: Vec<House>,
    sign_x: Option<i32>,
    debris: Vec<Debris>,
    drops: Vec<Drop>,
    clouds: Vec<(f32, usize)>,
}

/// Eases from 0 to 1 and holds, so the rise slows as it nears the top.
fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

impl Flood {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut flood = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            risen: 0.0,
            houses: Vec::new(),
            sign_x: None,
            debris: Vec::new(),
            drops: Vec::new(),
            clouds: Vec::new(),
        };
        flood.resize(width, height, rng);
        flood
    }

    fn ground(&self) -> i32 {
        self.height as i32 - 1
    }

    /// The most rows the water rises: past the doors and the lower windows, always leaving the
    /// roofs and the sky clear.
    fn max_level(&self) -> f32 {
        let lowest = self.houses.iter().map(|h| h.height).min().unwrap_or(3) as f32;
        (lowest * 0.5)
            .min(self.height as f32 * 0.4)
            .clamp(1.0, (self.height as f32 - 4.0).max(1.0))
    }

    /// The water's height above the ground, in rows, before rounding.
    fn level(&self) -> f32 {
        let rise = ease(self.risen / RISE_SECONDS);
        let swell =
            SWELL_ROWS * rise * cycle(self.t, TAU / SWELL_PERIOD * self.motion.pace()).sin();
        (self.max_level() * rise + swell).max(0.0)
    }

    /// The row of the water's surface. `None` before it has risen a row.
    fn surface(&self) -> Option<i32> {
        let rows = self.level().round() as i32;
        (rows >= 1).then(|| (self.ground() - rows).max(1))
    }
}

impl Scene for Flood {
    fn name(&self) -> &'static str {
        "flood"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let w = self.width as i32;
        let tallest = ((self.height as f32 * 0.55) as i32).clamp(4, 22);
        let narrow = self.width < 40;
        self.sign_x = (self.width >= 24 && self.height >= 10).then_some(w / 5);
        // the sign stands in a clearing of its own, so it never sits across a house
        let clearing = self.sign_x.map(|x| (x - 4, x + 4));
        self.houses.clear();
        let mut x = rng.below(3) as i32;
        while x < w {
            let width = if narrow {
                6 + rng.below(3) as i32
            } else {
                9 + rng.below(6) as i32
            };
            if let Some((left, right)) = clearing {
                if x <= right && x + width > left {
                    x = right + 1;
                    continue;
                }
            }
            let height = (tallest - rng.below((tallest / 3).max(1) as usize + 1) as i32).max(4);
            self.houses.push(House { x, width, height });
            x += width + 1 + rng.below(3) as i32;
        }
        let wanted = self
            .motion
            .scaled(count_for(self.width, self.height, 500, 2, 8));
        self.debris = (0..wanted)
            .map(|i| Debris {
                x: rng.range(0.0, self.width as f32),
                speed: rng.range(0.5, 1.4) * if i % 2 == 0 { 1.0 } else { -1.0 },
                sprite: rng.below(DEBRIS.len()),
            })
            .collect();
        let drops = self
            .motion
            .scaled(count_for(self.width, self.height, 90, 3, 60));
        let drops = if self.motion.calm { drops / 2 } else { drops };
        self.drops = (0..drops)
            .map(|_| Drop {
                x: rng.range(0.0, self.width as f32),
                y: rng.range(1.0, self.height as f32),
                speed: rng.range(5.0, 8.0),
            })
            .collect();
        self.clouds.clear();
        let mut cx = -(rng.below(6) as f32);
        while cx < self.width as f32 {
            let len = 4 + rng.below(8);
            self.clouds.push((cx, len));
            cx += len as f32 + 1.0 + rng.below(5) as f32;
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        self.risen = (self.risen + dt * speed).min(RISE_SECONDS * 2.0);
        let width = self.width as f32;
        for item in &mut self.debris {
            item.x = wrap(item.x + item.speed * speed * dt + 4.0, width + 8.0) - 4.0;
        }
        for cloud in &mut self.clouds {
            cloud.0 = wrap(cloud.0 + 0.3 * speed * dt + 12.0, width + 24.0) - 12.0;
        }
        let floor = self.surface().unwrap_or(self.ground()) as f32;
        for drop in &mut self.drops {
            drop.y += drop.speed * speed * dt;
            drop.x = wrap(drop.x + 0.8 * speed * dt, width);
            if drop.y >= floor {
                drop.y = 1.0 - rng.range(0.0, 2.0);
                drop.x = rng.range(0.0, width);
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let ground = self.ground();
        let w = self.width as i32;
        let surface = self.surface();
        let under = |y: i32| surface.is_some_and(|s| y > s);

        // the sky: one row of cloud and a light drizzle
        if self.height >= 6 {
            for &(x, len) in &self.clouds {
                for i in 0..len as i32 {
                    frame.put(x.round() as i32 + i, 0, '~', Some(theme.grey()));
                }
            }
        }
        for drop in &self.drops {
            let y = drop.y.floor() as i32;
            if y >= 1 && surface.is_none_or(|s| y < s) && y < ground {
                frame.put(drop.x.round() as i32, y, ':', Some(theme.blue()));
            }
        }

        // the houses, drawn in blue where the water has reached them
        for house in &self.houses {
            let top = ground - house.height;
            let right = house.x + house.width - 1;
            for y in top.max(0)..ground {
                let colour = |normal| Some(if under(y) { theme.blue() } else { normal });
                let row = y - top;
                if row == 0 {
                    for x in house.x + 1..right {
                        frame.put(x, y, '_', colour(theme.red()));
                    }
                } else if row == 1 {
                    frame.put(house.x, y, '/', colour(theme.red()));
                    frame.put(right, y, '\\', colour(theme.red()));
                } else {
                    frame.put(house.x, y, '|', colour(theme.muted));
                    frame.put(right, y, '|', colour(theme.muted));
                    let door = house.x + house.width / 2;
                    if y >= ground - 2 {
                        frame.put(door - 1, y, '|', colour(theme.muted));
                        frame.put(door + 1, y, '|', colour(theme.muted));
                    } else if row % 2 == 0 {
                        let mut wx = house.x + 2;
                        while wx + 1 < right {
                            frame.put_str(wx, y, "[]", colour(theme.yellow()));
                            wx += 4;
                        }
                    }
                }
            }
        }
        if ground >= 1 {
            frame.fill_row(ground, '_', Some(theme.muted));
        }

        // the warning sign, its post going under as the water rises
        if let Some(x) = self.sign_x {
            let top = ground - 5;
            let colour = |y: i32| {
                Some(if under(y) {
                    theme.blue()
                } else {
                    warning_colour(theme)
                })
            };
            frame.put_str(x - 1, top, "/!\\", colour(top));
            frame.put_str(x - 2, top + 1, "/___\\", colour(top + 1));
            for y in top + 2..ground {
                let post = if under(y) { theme.blue() } else { theme.muted };
                frame.put(x, y, '|', Some(post));
            }
        }

        // the water: a drifting surface line and a sparse swell below it
        let Some(surface) = surface else {
            return;
        };
        let drift = cycle(self.t, 0.9 * self.motion.speed());
        for x in 0..w {
            let v = (x as f32 * 0.55 + drift).sin();
            let ch = if v > 0.3 { '~' } else { '-' };
            frame.put(x, surface, ch, Some(theme.cyan()));
        }
        // below the surface only a scattering of swell marks, so the drowned houses still read
        for y in surface + 1..ground {
            let depth = (y - surface) as f32;
            for x in 0..w {
                let v = (x as f32 * 0.31 - drift * 0.7 + depth * 2.3).sin()
                    * (x as f32 * 0.13 + depth).cos();
                if v > 0.8 && frame.get(x, y).is_some_and(|c| c.ch == ' ') {
                    frame.put(x, y, '~', Some(theme.blue()));
                }
            }
        }
        if ground > surface {
            frame.fill_row(ground, '_', Some(theme.blue()));
        }
        for item in &self.debris {
            let colour = match item.sprite % 3 {
                0 => theme.yellow(),
                1 => theme.foreground,
                _ => theme.magenta(),
            };
            frame.put_str(
                item.x.round() as i32,
                surface,
                DEBRIS[item.sprite],
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
        check_scene("flood");
        check_determinism("flood");
    }

    #[test]
    fn the_water_rises_slowly_then_holds_below_the_roofs() {
        let mut rng = Rng::new(4);
        let mut flood = Flood::new(80, 24, Motion::FULL, &mut rng);
        assert_eq!(flood.surface(), None);
        for _ in 0..(10 * 20) {
            flood.step(0.05, &mut rng);
        }
        let early = flood.surface().unwrap_or(flood.ground());
        assert!(early >= flood.ground() - 3, "{early}");
        // a simulated hour
        let mut rows = Vec::new();
        for _ in 0..(3600 * 20) {
            flood.step(0.05, &mut rng);
            rows.push(flood.surface().expect("risen"));
        }
        let top = flood.ground() - flood.max_level().round() as i32;
        for &row in &rows[rows.len() - 1200..] {
            assert!((top - 1..=top + 1).contains(&row), "{row} against {top}");
            assert!(row >= 2);
        }
        let frame = render(&mut flood, 80, 24, 1);
        assert!(frame.to_text().contains("/!\\"));
    }

    #[test]
    fn the_surface_row_moves_at_most_twice_a_swell() {
        let mut rng = Rng::new(9);
        let mut flood = Flood::new(60, 20, Motion::LIVELY, &mut rng);
        for _ in 0..(120 * 20) {
            flood.step(0.05, &mut rng);
        }
        let mut changes = 0;
        let mut last = flood.surface();
        let seconds = SWELL_PERIOD * 5.0;
        for _ in 0..((seconds * 20.0) as usize) {
            flood.step(0.05, &mut rng);
            if flood.surface() != last {
                changes += 1;
                last = flood.surface();
            }
        }
        assert!(changes <= 2 * 5 + 1, "{changes}");
    }

    #[test]
    fn a_resize_keeps_the_water_risen() {
        let mut rng = Rng::new(2);
        let mut flood = Flood::new(80, 24, Motion::FULL, &mut rng);
        for _ in 0..(90 * 20) {
            flood.step(0.05, &mut rng);
        }
        flood.resize(40, 12, &mut rng);
        flood.step(0.05, &mut rng);
        assert!(flood.surface().is_some());
    }
}
