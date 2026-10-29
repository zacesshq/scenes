//! Blackout: a city skyline at night whose lit windows go dark one at a time over about a minute
//! and a half, beside a pole with a snapped power line. A few windows keep a candle and a handful
//! stay lit on their own power, so the city is quiet but never empty. Once dark it stays dark,
//! with clouds drifting past the moon.

use super::wrap;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::{error_colour, warning_colour};

pub const SUMMARY: &str =
    "A night skyline whose windows go dark one by one beside a snapped power line, leaving candles";

pub const CAPTIONS: &[&str] = &[
    "The lights went out. Start with the first error.",
    "Power is down. One fix brings it back.",
    "Dark for now. The cause is in the list below.",
    "A candle and the compiler output. Good enough.",
    "Lights out. Retrace the last change.",
];

/// Seconds for the whole city to go dark at the standard pace.
pub const DARKEN_SECONDS: f32 = 90.0;
/// The shortest time between two windows going dark. Each window changes once, so this keeps
/// the change to a trickle even on a very wide screen.
pub const MIN_WINDOW_GAP: f32 = 0.25;
/// Seconds after a window goes dark before its candle is lit.
const CANDLE_DELAY: f32 = 5.0;

#[derive(Clone, Copy, Debug)]
struct Window {
    x: i32,
    y: i32,
    /// Whether it was lit when the scene began.
    lit: bool,
    /// Its place in the order the lights go out.
    rank: usize,
    /// Whether someone lights a candle there once it is dark.
    candle: bool,
    /// Whether it stays lit on its own power.
    generator: bool,
    /// The second shade of yellow, so the windows are not all one colour.
    warm: bool,
}

#[derive(Clone, Copy, Debug)]
struct Building {
    x: i32,
    width: i32,
    top: i32,
}

pub struct Blackout {
    width: usize,
    height: usize,
    motion: Motion,
    buildings: Vec<Building>,
    windows: Vec<Window>,
    stars: Vec<(i32, i32)>,
    clouds: Vec<(f32, i32, usize)>,
    /// How many windows have gone dark, in rank order.
    darkened: usize,
    /// When each window went dark, by rank, in seconds of scene time.
    dark_since: Vec<f32>,
    since_last: f32,
    t: f32,
}

impl Blackout {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut city = Self {
            width: 1,
            height: 1,
            motion,
            buildings: Vec::new(),
            windows: Vec::new(),
            stars: Vec::new(),
            clouds: Vec::new(),
            darkened: 0,
            dark_since: Vec::new(),
            since_last: 0.0,
            t: 0.0,
        };
        city.resize(width, height, rng);
        city
    }

    fn ground(&self) -> i32 {
        self.height as i32 - 1
    }

    /// How many windows were lit at the start, which sets the gap between two going dark.
    fn lit_count(&self) -> usize {
        self.windows.iter().filter(|w| w.lit).count()
    }

    fn gap(&self) -> f32 {
        let lit = self.lit_count().max(1) as f32;
        (DARKEN_SECONDS / lit).max(MIN_WINDOW_GAP) / self.motion.pace()
    }

    /// The left edge of the skyline, leaving room for the pole on a wide enough screen.
    fn skyline_start(&self) -> i32 {
        if self.width >= 40 { 8 } else { 0 }
    }

    fn is_dark(&self, window: &Window) -> bool {
        window.lit && !window.generator && window.rank < self.darkened
    }
}

impl Scene for Blackout {
    fn name(&self) -> &'static str {
        "blackout"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let ground = self.ground();
        let sky = (ground as f32).max(1.0);
        self.buildings.clear();
        let mut x = self.skyline_start();
        while x < self.width as i32 {
            let w = 5 + rng.below(6) as i32;
            let tall = rng.range(0.35, 0.8);
            let top = (ground - (sky * tall) as i32).max(2.min(ground));
            self.buildings.push(Building { x, width: w, top });
            x += w + rng.below(2) as i32;
        }
        let mut windows = Vec::new();
        for b in &self.buildings {
            let mut y = b.top + 2;
            while y < ground - 1 {
                let mut wx = b.x + 2;
                while wx < b.x + b.width - 1 {
                    windows.push(Window {
                        x: wx,
                        y,
                        lit: rng.chance(0.65),
                        rank: 0,
                        candle: rng.chance(0.08),
                        generator: rng.chance(0.03),
                        warm: rng.chance(0.4),
                    });
                    wx += 2;
                }
                y += 2;
            }
        }
        // a new order for the lights, with the windows already dark kept dark
        let mut order: Vec<usize> = (0..windows.len()).collect();
        for i in (1..order.len()).rev() {
            order.swap(i, rng.below(i + 1));
        }
        let mut rank = 0;
        for &i in &order {
            if windows[i].lit && !windows[i].generator {
                windows[i].rank = rank;
                rank += 1;
            } else {
                windows[i].rank = usize::MAX;
            }
        }
        let fraction = if self.dark_since.is_empty() {
            0.0
        } else {
            self.darkened as f32 / self.dark_since.len() as f32
        };
        self.darkened = (fraction * rank as f32).round() as usize;
        // candles in windows already dark appear at once rather than all together later
        self.dark_since = vec![self.t - CANDLE_DELAY; self.darkened];
        self.dark_since.resize(rank, f32::MAX);
        self.windows = windows;
        self.stars = (0..super::count_for(self.width, self.height, 60, 2, 60))
            .map(|_| {
                (
                    rng.below(self.width) as i32,
                    rng.below((ground as usize / 3).max(1)) as i32,
                )
            })
            .collect();
        let clouds = self.motion.scaled((self.width / 30).max(1));
        self.clouds = (0..clouds)
            .map(|_| {
                (
                    rng.range(0.0, self.width as f32),
                    rng.below((ground as usize / 4).max(1)) as i32,
                    6 + rng.below(8),
                )
            })
            .collect();
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += dt;
        let width = self.width as f32;
        let speed = self.motion.speed();
        for cloud in &mut self.clouds {
            cloud.0 = wrap(cloud.0 + 0.5 * speed * dt + 16.0, width + 32.0) - 16.0;
        }
        self.since_last += dt;
        let gap = self.gap();
        if self.since_last >= gap && self.darkened < self.dark_since.len() {
            self.since_last = 0.0;
            self.dark_since[self.darkened] = self.t;
            self.darkened += 1;
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let ground = self.ground();
        for &(x, y) in &self.stars {
            frame.put(x, y, '.', Some(theme.muted));
        }
        // the moon, behind the clouds
        let moon_x = self.width as i32 * 2 / 3;
        if self.width >= 30 && self.height >= 12 {
            frame.put_sprite(moon_x, 1, &[" _", "( ", " `"], Some(theme.foreground));
        }
        for &(x, y, len) in &self.clouds {
            for i in 0..len as i32 {
                frame.put(x.round() as i32 + i, y + 1, '~', Some(theme.grey()));
            }
        }

        let outline = Some(theme.grey());
        for b in &self.buildings {
            for x in b.x..b.x + b.width {
                frame.put(x, b.top, '_', outline);
            }
            for y in b.top + 1..ground {
                frame.put(b.x, y, '|', outline);
                frame.put(b.x + b.width - 1, y, '|', outline);
                for x in b.x + 1..b.x + b.width - 1 {
                    frame.put(x, y, ' ', None);
                }
            }
        }
        for window in &self.windows {
            if !window.lit {
                frame.put(window.x, window.y, '.', Some(theme.muted));
            } else if self.is_dark(window) {
                let since = self.dark_since[window.rank];
                if window.candle && self.t - since >= CANDLE_DELAY {
                    frame.put(window.x, window.y, 'i', Some(warning_colour(theme)));
                } else {
                    frame.put(window.x, window.y, '.', Some(theme.muted));
                }
            } else {
                let colour = if window.warm {
                    theme.yellow()
                } else {
                    theme.bright_yellow()
                };
                frame.put(window.x, window.y, '#', Some(colour));
            }
        }
        frame.fill_row(ground, '_', Some(theme.muted));

        // the pole with its snapped line, hanging down towards the street
        if self.width >= 40 && self.height >= 10 {
            let top = (ground - (self.height as i32) / 2).max(1);
            let px = 3;
            frame.put_str(px - 2, top, "-+-", Some(theme.foreground));
            for y in top + 1..ground {
                frame.put(px - 1, y, '|', Some(theme.foreground));
            }
            let mut y = top;
            let mut x = px + 1;
            while y < ground - 2 && x < self.skyline_start() {
                frame.put(x, y, '\\', Some(warning_colour(theme)));
                y += 1;
                x += 1;
            }
            frame.put(x, y, 'x', Some(error_colour(theme)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, max_changes_per_second};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("blackout");
        check_determinism("blackout");
    }

    #[test]
    fn windows_go_dark_one_at_a_time_and_stay_dark() {
        let mut rng = Rng::new(6);
        let mut city = Blackout::new(100, 30, Motion::LIVELY, &mut rng);
        let lit = |c: &Blackout| c.windows.iter().filter(|w| w.lit && !c.is_dark(w)).count();
        let start = lit(&city);
        assert!(start > 20, "{start}");
        let mut last = city.darkened;
        for _ in 0..(10 * 60 * 20) {
            city.step(0.05, &mut rng);
            assert!(city.darkened - last <= 1);
            last = city.darkened;
        }
        assert_eq!(city.darkened, city.dark_since.len());
        let generators = city.windows.iter().filter(|w| w.lit && w.generator).count();
        assert_eq!(lit(&city), generators);
        assert!(city.gap() >= MIN_WINDOW_GAP);
    }

    #[test]
    fn no_cell_changes_more_than_once_a_second_while_the_lights_go_out() {
        let mut rng = Rng::new(2);
        let mut city = Blackout::new(80, 24, Motion::LIVELY, &mut rng);
        let mut frames = Vec::new();
        for _ in 0..(30 * 20) {
            city.step(0.05, &mut rng);
            let mut frame = Frame::new(80, 24);
            city.draw(&mut frame, crate::theme::default_theme());
            // the clouds move, so only the windows are counted
            for y in 0..24 {
                for x in 0..80 {
                    if frame.get(x, y).is_some_and(|c| c.ch == '~') {
                        frame.put(x, y, ' ', None);
                    }
                }
            }
            frames.push(frame);
        }
        assert!(max_changes_per_second(&frames, 20) <= 2);
    }

    #[test]
    fn a_resize_keeps_how_dark_the_city_is() {
        let mut rng = Rng::new(9);
        let mut city = Blackout::new(80, 24, Motion::FULL, &mut rng);
        for _ in 0..(45 * 20) {
            city.step(0.05, &mut rng);
        }
        let before = city.darkened as f32 / city.dark_since.len() as f32;
        city.resize(120, 40, &mut rng);
        let after = city.darkened as f32 / city.dark_since.len() as f32;
        assert!((before - after).abs() < 0.05, "{before} {after}");
    }
}
