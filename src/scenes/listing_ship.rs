//! A listing ship: a cargo ship on a grey sea that slowly tips to one side over the first minute
//! while its waterline creeps up past the portholes, then rolls gently at that angle with a
//! lifebuoy and a crate drifting away.

use std::f32::consts::TAU;

use super::{count_for, cycle, wrap};
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::error_colour;

/// Seconds the ship takes to settle at its list at the standard pace.
pub const LIST_SECONDS: f32 = 50.0;
/// The angle the ship settles at, in degrees, bow down to the right.
pub const MAX_LIST: f32 = 8.0;
/// Seconds of one slow roll once it has settled. The roll is small and slow, so no cell of the
/// ship changes more than once or twice a second.
pub const ROLL_PERIOD: f32 = 11.0;
/// How far the ship rolls either way, in degrees.
const ROLL: f32 = 1.5;
/// How many rows the ship sinks as it lists.
const SINK_ROWS: f32 = 1.0;
/// A cell is about twice as tall as it is wide, so a turn is worked out in square units.
const ASPECT: f32 = 2.0;
/// How far the second row of cloud sits from the first, so the two rows never line up.
const CLOUD_OFFSET: f32 = 7.0;

pub const SUMMARY: &str =
    "A cargo ship on a grey sea that slowly lists to one side as its waterline creeps up";

pub const CAPTIONS: &[&str] = &[
    "Taking on water. Plug the first leak.",
    "A list to starboard. Trim it with one fix.",
    "Still afloat. Read the error, then right the ship.",
    "All hands to the first error.",
    "Ships list. Builds recover.",
];

/// The shape of a ship in columns and rows: its length, the depth of its hull, where its
/// bridge sits and how tall it is. The mast's column and height follow.
#[derive(Clone, Copy, Debug)]
struct Hull {
    length: i32,
    depth: i32,
    bridge: (i32, i32),
    bridge_rows: i32,
    mast: i32,
    mast_rows: i32,
}

const LARGE: Hull = Hull {
    length: 36,
    depth: 3,
    bridge: (5, 15),
    bridge_rows: 2,
    mast: 24,
    mast_rows: 4,
};

const SMALL: Hull = Hull {
    length: 16,
    depth: 2,
    bridge: (2, 7),
    bridge_rows: 1,
    mast: 11,
    mast_rows: 2,
};

/// The block character for a cell whose top half, bottom half or both are filled.
fn halves(top: bool, bottom: bool) -> Option<char> {
    match (top, bottom) {
        (true, true) => Some('\u{2588}'),
        (true, false) => Some('\u{2580}'),
        (false, true) => Some('\u{2584}'),
        (false, false) => None,
    }
}

#[derive(Clone, Debug)]
struct Drop {
    x: f32,
    y: f32,
    speed: f32,
}

#[derive(Clone, Debug)]
struct Float {
    x: f32,
    speed: f32,
    sprite: &'static str,
}

pub struct ListingShip {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// Seconds of listing so far, kept across a resize so the ship never rights itself.
    listed: f32,
    floats: Vec<Float>,
    drops: Vec<Drop>,
    clouds: Vec<(f32, usize)>,
}

fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

impl ListingShip {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut ship = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            listed: 0.0,
            floats: Vec::new(),
            drops: Vec::new(),
            clouds: Vec::new(),
        };
        ship.resize(width, height, rng);
        ship
    }

    fn horizon(&self) -> i32 {
        ((self.height as f32 * 0.55) as i32).max(2)
    }

    fn hull(&self) -> Hull {
        if self.width >= 50 && self.height >= 16 {
            LARGE
        } else {
            SMALL
        }
    }

    /// The ship's angle in degrees.
    fn angle(&self) -> f32 {
        let settled = ease(self.listed / LIST_SECONDS);
        let roll = ROLL * settled * cycle(self.t, TAU / ROLL_PERIOD * self.motion.pace()).sin();
        MAX_LIST * settled + roll
    }

    /// How far the ship has sunk, in whole rows.
    fn sink(&self) -> i32 {
        (SINK_ROWS * ease(self.listed / LIST_SECONDS)).round() as i32
    }

    /// The column of the ship's centre: left of the middle on a wide screen, so a panel in the
    /// middle of the watch mode leaves it in view.
    fn centre(&self) -> i32 {
        if self.width >= 70 {
            (self.width as f32 * 0.3) as i32
        } else {
            self.width as i32 / 2
        }
    }

    /// Draws the ship tipped by its angle as a solid silhouette. Everything is worked out in
    /// half rows, drawn with half blocks, so the slope of the deck and the keel steps half a row
    /// at a time and never breaks the shape apart. The stern stays put and the bow goes down,
    /// the way a ship taking on water at the bow settles.
    fn draw_ship(&self, frame: &mut Frame, theme: &ThemeData) {
        let hull = self.hull();
        let surface = self.horizon();
        let left = self.centre() - hull.length / 2;
        // half rows the bow drops for each column along the ship
        let slope = self.angle().to_radians().tan() / ASPECT * 2.0;
        let deck_at =
            |c: i32| 2 * (surface - hull.depth + self.sink()) + (c as f32 * slope).round() as i32;
        let below = |row: i32| row > surface;
        let fill = |frame: &mut Frame, x: i32, from: i32, to: i32, colour, underwater| {
            for row in from.div_euclid(2)..=to.div_euclid(2) {
                let top = (from..=to).contains(&(2 * row));
                let bottom = (from..=to).contains(&(2 * row + 1));
                let Some(ch) = halves(top, bottom) else {
                    continue;
                };
                if below(row) {
                    frame.put(x, row, theme.faint_shade(), Some(underwater));
                    continue;
                }
                // a half block meeting the other half of the bridge above it becomes whole, so
                // the deck never shows a seam of half blocks
                let ch = match (frame.get(x, row).map(|c| c.ch), ch) {
                    (Some('\u{2580}'), '\u{2584}') | (Some('\u{2584}'), '\u{2580}') => '\u{2588}',
                    _ => ch,
                };
                frame.put(x, row, ch, Some(colour));
            }
        };
        // the bridge first, then the hull, which joins it where the two share a cell
        for c in hull.bridge.0..hull.bridge.1 {
            let deck = deck_at(c);
            fill(
                frame,
                left + c,
                deck - 2 * hull.bridge_rows,
                deck - 1,
                theme.muted,
                theme.blue(),
            );
        }
        for c in 0..hull.length {
            let deck = deck_at(c);
            // the keel rises to a rounded stern and a raked bow
            let rise = (2 - c).max(0) + (c - (hull.length - 6)).max(0);
            let keel = deck + 2 * hull.depth - 1 - rise;
            let raised = i32::from(c >= hull.length - 4);
            fill(
                frame,
                left + c,
                deck - raised,
                keel,
                theme.foreground,
                theme.blue(),
            );
        }
        // portholes along the middle of the hull, dark where the water has reached them
        for c in (3..hull.length - 5).step_by(5) {
            let row = (deck_at(c) + 2).div_euclid(2) + i32::from(hull.depth > 2);
            let colour = if below(row) {
                theme.blue()
            } else {
                theme.yellow()
            };
            frame.put(left + c, row, 'o', Some(colour));
        }
        // the mast leans with the ship, its flag at the top
        let base = deck_at(hull.mast).div_euclid(2) - 1;
        let lean = self.angle().to_radians().sin() * ASPECT;
        for k in 0..hull.mast_rows {
            let x = left + hull.mast + (k as f32 * lean).round() as i32;
            let ch = if lean > 0.25 { '/' } else { '|' };
            frame.put(x, base - k, ch, Some(theme.grey()));
            if k == hull.mast_rows - 1 {
                frame.put(x + 1, base - k, '>', Some(error_colour(theme)));
            }
        }
    }
}

impl Scene for ListingShip {
    fn name(&self) -> &'static str {
        "listing-ship"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = self
            .motion
            .scaled(count_for(self.width, self.height, 900, 1, 4));
        let sprites = ["(O)", "[#]", "o", "(O)"];
        self.floats = (0..wanted)
            .map(|i| Float {
                x: rng.range(0.0, self.width as f32),
                speed: rng.range(0.4, 0.9),
                sprite: sprites[i % sprites.len()],
            })
            .collect();
        let drops = self
            .motion
            .scaled(count_for(self.width, self.height, 120, 2, 50));
        let drops = if self.motion.calm { drops / 2 } else { drops };
        self.drops = (0..drops)
            .map(|_| Drop {
                x: rng.range(0.0, self.width as f32),
                y: rng.range(1.0, self.height as f32),
                speed: rng.range(5.0, 8.0),
            })
            .collect();
        self.clouds.clear();
        let mut x = -(rng.below(6) as f32);
        while x < self.width as f32 {
            let len = 5 + rng.below(9);
            self.clouds.push((x, len));
            x += len as f32 + 1.0 + rng.below(4) as f32;
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        self.listed = (self.listed + dt * speed).min(LIST_SECONDS * 2.0);
        let width = self.width as f32;
        for float in &mut self.floats {
            float.x = wrap(float.x + float.speed * speed * dt + 4.0, width + 8.0) - 4.0;
        }
        let horizon = self.horizon() as f32;
        for drop in &mut self.drops {
            drop.y += drop.speed * speed * dt;
            drop.x = wrap(drop.x + 1.2 * speed * dt, width);
            if drop.y >= horizon {
                drop.y = 1.0 - rng.range(0.0, 2.0);
                drop.x = rng.range(0.0, width);
            }
        }
        for cloud in &mut self.clouds {
            cloud.0 = wrap(cloud.0 + 0.25 * speed * dt + 14.0, width + 28.0) - 14.0;
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let horizon = self.horizon();
        let w = self.width as i32;
        // a heavy sky of two cloud rows when there is room for them
        let cloud_rows = if horizon >= 6 {
            2
        } else {
            i32::from(horizon >= 4)
        };
        for row in 0..cloud_rows {
            for &(x, len) in &self.clouds {
                let start = (x + CLOUD_OFFSET * row as f32 * (len % 3) as f32).round() as i32;
                for i in 0..(len as i32 - row * 2).max(2) {
                    frame.put(start + i, row, '~', Some(theme.grey()));
                }
            }
        }

        for drop in &self.drops {
            let y = drop.y.floor() as i32;
            if y >= cloud_rows && y < horizon {
                frame.put(drop.x.round() as i32, y, ':', Some(theme.blue()));
            }
        }

        // a grey sea: the horizon and a slow swell that lengthens towards the viewer
        let drift = self.motion.speed();
        for y in horizon..self.height as i32 {
            let depth = (y - horizon) as f32 / (self.height as f32 - horizon as f32).max(1.0);
            let wavelength = 7.0 + 9.0 * depth;
            for x in 0..w {
                let v = (x as f32 / wavelength * TAU
                    + cycle(self.t, 0.6 * drift / wavelength * TAU)
                    + depth * 7.0)
                    .sin();
                if y == horizon {
                    frame.put(x, y, if v > 0.4 { '~' } else { '-' }, Some(theme.grey()));
                } else if v > 0.7 {
                    frame.put(x, y, '~', Some(theme.grey()));
                } else if v > 0.2 && (x + y) % 2 == 0 {
                    frame.put(x, y, '-', Some(theme.blue()));
                }
            }
        }

        // what has gone over the side drifts away along the surface
        if self.listed > 4.0 {
            for float in &self.floats {
                let colour = if float.sprite == "(O)" {
                    error_colour(theme)
                } else {
                    theme.yellow()
                };
                frame.put_str(float.x.round() as i32, horizon, float.sprite, Some(colour));
            }
        }

        self.draw_ship(frame, theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("listing-ship");
        check_determinism("listing-ship");
    }

    #[test]
    fn the_ship_lists_slowly_then_rolls_gently_at_its_angle() {
        let mut rng = Rng::new(6);
        let mut ship = ListingShip::new(80, 24, Motion::LIVELY, &mut rng);
        assert!(ship.angle().abs() < 0.01);
        let mut last = ship.angle();
        let mut fastest = 0.0_f32;
        let mut angles = Vec::new();
        for _ in 0..(3600 * 20) {
            ship.step(0.05, &mut rng);
            fastest = fastest.max((ship.angle() - last).abs() * 20.0);
            last = ship.angle();
            angles.push(last);
        }
        // under two degrees a second, so the ship's far end moves under a row a second
        assert!(fastest < 2.0, "{fastest} degrees a second");
        for &angle in &angles[angles.len() - 2000..] {
            assert!((MAX_LIST - ROLL - 0.01..=MAX_LIST + ROLL + 0.01).contains(&angle));
        }
        assert_eq!(ship.sink(), SINK_ROWS.round() as i32);
    }

    #[test]
    fn the_ship_is_drawn_with_its_flag_and_portholes() {
        let mut rng = Rng::new(1);
        let mut ship = ListingShip::new(100, 30, Motion::FULL, &mut rng);
        let frame = render(&mut ship, 100, 30, 20);
        let text = frame.to_text();
        assert!(text.contains("|>"), "{text}");
        assert!(text.contains('\u{2588}') && text.contains('o'));
        let small = render(
            &mut ListingShip::new(20, 8, Motion::CALM, &mut rng),
            20,
            8,
            20,
        );
        assert!(small.to_text().contains('>'), "{}", small.to_text());
    }
}
