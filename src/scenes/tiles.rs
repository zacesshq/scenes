//! Tiles: a calm grid of small pages, each with the folded corner of the Zacess mark. Some are
//! filled in the theme's colours and the rest are left as a faint shade. One tile at a time,
//! every few seconds, takes a new colour or settles back to its outline.

use crate::frame::Frame;
use crate::glyphs;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "A grid of small folded pages in the theme's colours, one tile changing at a time";

pub const CAPTIONS: &[&str] = &[
    "Every tile in its place.",
    "A tidy grid and a tidy build.",
    "Clean, clear and readable.",
    "All squared away.",
    "Nothing out of line.",
];

/// The most seconds between two tile changes, at the standard pace, on a small grid.
pub const CHANGE_PERIOD: f32 = 3.0;
/// The fewest seconds between two tile changes, on a large grid. A tile is a small part of a
/// large screen, so even this keeps every large change well under three a second.
pub const MIN_CHANGE_GAP: f32 = 0.7;
const GAP_X: i32 = 2;
const GAP_Y: i32 = 1;

pub struct Tiles {
    width: usize,
    height: usize,
    motion: Motion,
    tile_w: i32,
    tile_h: i32,
    columns: i32,
    rows: i32,
    left: i32,
    top: i32,
    /// The colour each tile is filled with, `None` for an outline.
    colours: Vec<Option<usize>>,
    since_change: f32,
    changes: u32,
}

/// The colour roles a tile can take, in a fixed order.
fn palette(theme: &ThemeData) -> [crate::colour::Rgb; 10] {
    [
        theme.accent,
        theme.cyan(),
        theme.magenta(),
        theme.green(),
        theme.yellow(),
        theme.blue(),
        theme.bright_cyan(),
        theme.keyword(),
        theme.string(),
        theme.function(),
    ]
}

impl Tiles {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut tiles = Self {
            width: 1,
            height: 1,
            motion,
            tile_w: 4,
            tile_h: 3,
            columns: 0,
            rows: 0,
            left: 0,
            top: 0,
            colours: Vec::new(),
            since_change: 0.0,
            changes: 0,
        };
        tiles.resize(width, height, rng);
        tiles
    }

    /// Seconds between two changes: fewer on a large grid, so it does not look still.
    fn change_gap(&self) -> f32 {
        let count = self.colours.len().max(1) as f32;
        (CHANGE_PERIOD * 40.0 / count).clamp(MIN_CHANGE_GAP, CHANGE_PERIOD) / self.motion.pace()
    }

    fn change_one(&mut self, rng: &mut Rng) {
        if self.colours.is_empty() {
            return;
        }
        let filled = self.colours.iter().filter(|c| c.is_some()).count();
        let share = filled as f32 / self.colours.len() as f32;
        let i = rng.below(self.colours.len());
        self.colours[i] = match self.colours[i] {
            // the grid drifts back towards about a third filled, so it never becomes a wall of
            // solid colour or empties
            Some(_) if share > 0.4 || rng.chance(0.3) => None,
            Some(old) => Some((old + 1 + rng.below(8)) % 10),
            None => Some(rng.below(10)),
        };
        self.changes += 1;
    }
}

impl Scene for Tiles {
    fn name(&self) -> &'static str {
        "tiles"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let (w, h) = (self.width as i32, self.height as i32);
        (self.tile_w, self.tile_h) = if w >= 120 && h >= 40 { (6, 4) } else { (4, 3) };
        // a margin all round keeps the grid off the edges of the screen
        let (margin_x, margin_y) = if w >= 40 && h >= 12 { (2, 1) } else { (0, 0) };
        self.columns = ((w - 2 * margin_x + GAP_X) / (self.tile_w + GAP_X)).max(1);
        self.rows = ((h - 2 * margin_y + GAP_Y) / (self.tile_h + GAP_Y)).max(1);
        let used_w = self.columns * (self.tile_w + GAP_X) - GAP_X;
        let used_h = self.rows * (self.tile_h + GAP_Y) - GAP_Y;
        self.left = ((w - used_w) / 2).max(0);
        self.top = ((h - used_h) / 2).max(0);
        let count = (self.columns * self.rows) as usize;
        self.colours = (0..count)
            .map(|_| rng.chance(0.3).then(|| rng.below(10)))
            .collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.since_change += dt;
        let gap = self.change_gap();
        if self.since_change >= gap {
            self.since_change = (self.since_change - gap).min(gap);
            self.change_one(rng);
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = palette(theme);
        let line = Some(theme.grey());
        // the fold is the page's own colour on its lower left, leaving the corner cut; without
        // the triangle the corner is simply left off
        let fold = glyphs::fold_dark();
        let (tw, th) = (self.tile_w, self.tile_h);
        for row in 0..self.rows {
            for column in 0..self.columns {
                let i = (row * self.columns + column) as usize;
                let x0 = self.left + column * (tw + GAP_X);
                let y0 = self.top + row * (th + GAP_Y);
                if let Some(c) = self.colours.get(i).copied().flatten() {
                    let colour = Some(colours[c]);
                    for dy in 0..th {
                        for dx in 0..tw {
                            if dy == 0 && dx == tw - 1 {
                                if let Some(f) = fold {
                                    frame.put(x0 + dx, y0, f, colour);
                                }
                            } else {
                                frame.put(x0 + dx, y0 + dy, '\u{2588}', colour);
                            }
                        }
                    }
                    continue;
                }
                // an empty tile is the outline of a page, its folded corner a dot
                for dx in 1..tw - 1 {
                    frame.put(x0 + dx, y0, '\u{2500}', line);
                    frame.put(x0 + dx, y0 + th - 1, '\u{2500}', line);
                }
                for dy in 1..th - 1 {
                    frame.put(x0, y0 + dy, '\u{2502}', line);
                    frame.put(x0 + tw - 1, y0 + dy, '\u{2502}', line);
                }
                frame.put(x0, y0, '\u{250c}', line);
                frame.put(x0 + tw - 1, y0, '.', line);
                frame.put(x0, y0 + th - 1, '\u{2514}', line);
                frame.put(x0 + tw - 1, y0 + th - 1, '\u{2518}', line);
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
        check_scene("tiles");
        check_determinism("tiles");
    }

    #[test]
    fn one_tile_changes_at_a_time_and_never_quickly() {
        let mut rng = Rng::new(4);
        for (w, h) in [(20, 8), (80, 24), (200, 60)] {
            for motion in Motion::ALL {
                let mut tiles = Tiles::new(w, h, motion, &mut rng);
                assert!(tiles.change_gap() >= MIN_CHANGE_GAP);
                let mut changed_at_once = 0;
                for _ in 0..(30 * 20) {
                    let before = tiles.colours.clone();
                    tiles.step(0.05, &mut rng);
                    let differ = before
                        .iter()
                        .zip(&tiles.colours)
                        .filter(|(a, b)| a != b)
                        .count();
                    changed_at_once = changed_at_once.max(differ);
                }
                assert!(changed_at_once <= 1);
                assert!(tiles.changes as f32 <= 30.0 / MIN_CHANGE_GAP + 1.0);
            }
        }
    }

    #[test]
    fn the_grid_fits_and_stays_about_a_third_filled() {
        let mut rng = Rng::new(9);
        let mut tiles = Tiles::new(80, 24, Motion::LIVELY, &mut rng);
        for _ in 0..(600 * 20) {
            tiles.step(0.05, &mut rng);
        }
        let filled = tiles.colours.iter().filter(|c| c.is_some()).count();
        let share = filled as f32 / tiles.colours.len() as f32;
        assert!((0.15..0.6).contains(&share), "{share}");
        let frame = render(&mut tiles, 80, 24, 1);
        assert!(frame.to_text().contains('\u{2588}'));
        assert!(Tiles::new(20, 8, Motion::FULL, &mut rng).columns >= 3);
    }
}
