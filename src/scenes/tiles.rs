//! Tiles, painted in solid pixels: a calm grid of small pages, each with the folded corner of
//! the Zacess mark. Some are filled in the theme's colours, lit along their top and left edges
//! and shaded along the others, with the fold turned back in a paler shade; the rest are left as
//! a faint outline. One tile at a time, every few seconds, takes a new colour or settles back to
//! its outline.
//!
//! Every colour comes from the active theme. The backdrop and the empty outlines are tints of
//! the background held below the non-text floor, so they read as background; the filled tiles
//! keep the theme's own drawing colours, so they stand out and keep their outline when colour is
//! off.

use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "A grid of painted folded pages in the theme's colours, one tile changing at a time";

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
/// Pixels between tiles across and down.
const GAP_X: i32 = 2;
const GAP_Y: i32 = 2;

/// The tiles' colours in one theme, every one derived from the theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct TileColours {
    pub back_top: Rgb,
    pub back_low: Rgb,
    pub outline: Rgb,
    pub inside: Rgb,
    pub fills: [Rgb; 10],
    back: Ramp,
}

impl TileColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let (back_top, back_low) = if theme.dark {
            (shades.tint(theme.accent, 0.08, 1.12), theme.background)
        } else {
            (shades.tint(theme.accent, 0.12, 1.2), theme.background)
        };
        Self {
            back_top,
            back_low,
            outline: shades.tint(theme.muted, 0.45, 2.2),
            inside: shades.tint(theme.muted, 0.08, 1.1),
            fills: palette(theme),
            back: Ramp::new(back_top, back_low, 10),
        }
    }
}

/// The colour roles a tile can take, in a fixed order.
fn palette(theme: &ThemeData) -> [Rgb; 10] {
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

pub struct Tiles {
    width: usize,
    height: usize,
    motion: Motion,
    /// A tile's width and height in pixels and the size of its fold.
    tile_w: i32,
    tile_h: i32,
    fold: i32,
    columns: i32,
    rows: i32,
    left: i32,
    top: i32,
    /// The colour each tile is filled with, `None` for an outline.
    colours: Vec<Option<usize>>,
    since_change: f32,
    changes: u32,
}

impl Tiles {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut tiles = Self {
            width: 1,
            height: 1,
            motion,
            tile_w: 4,
            tile_h: 4,
            fold: 1,
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

    /// Whether a pixel of a tile, from its top left, lies in the cut corner above the fold.
    fn cut(&self, dx: i32, dy: i32) -> bool {
        let from_right = self.tile_w - 1 - dx;
        dy + from_right < self.fold
    }

    /// Whether a pixel of a tile lies on the fold flap: the corner turned back below the cut.
    fn flap(&self, dx: i32, dy: i32) -> bool {
        let from_right = self.tile_w - 1 - dx;
        !self.cut(dx, dy) && from_right < self.fold && dy < self.fold
    }

    fn paint_filled(&self, canvas: &mut Canvas, theme: &ThemeData, x0: i32, y0: i32, base: Rgb) {
        let shades = Shades::new(theme);
        let lit = shades.lighter(base, 0.3);
        let shade = shades.darker(base, 0.3);
        let flap = shades.lighter(base, 0.55);
        let (tw, th) = (self.tile_w, self.tile_h);
        for dy in 0..th {
            for dx in 0..tw {
                if self.cut(dx, dy) {
                    continue;
                }
                let colour = if self.flap(dx, dy) {
                    flap
                } else if dy == 0 || dx == 0 {
                    lit
                } else if dy == th - 1 || dx == tw - 1 {
                    shade
                } else {
                    base
                };
                canvas.set(x0 + dx, y0 + dy, colour);
            }
        }
        // the fold's own shadow, along its lower edge
        if self.fold >= 2 {
            canvas.set(x0 + tw - self.fold - 1, y0 + self.fold - 1, shade);
        }
    }

    fn paint_empty(&self, canvas: &mut Canvas, c: &TileColours, x0: i32, y0: i32) {
        let (tw, th) = (self.tile_w, self.tile_h);
        for dy in 0..th {
            for dx in 0..tw {
                if self.cut(dx, dy) {
                    continue;
                }
                let from_right = tw - 1 - dx;
                let edge = dy == 0 || dx == 0 || dy == th - 1 || dx == tw - 1;
                // the fold's diagonal closes the outline across the cut corner
                let diagonal = dy + from_right == self.fold;
                let colour = if edge || diagonal {
                    c.outline
                } else {
                    c.inside
                };
                canvas.set(x0 + dx, y0 + dy, colour);
            }
        }
    }
}

impl Scene for Tiles {
    fn name(&self) -> &'static str {
        "tiles"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let (w, h) = (self.width as i32, self.height as i32 * 2);
        (self.tile_w, self.tile_h, self.fold) = if self.width >= 120 && self.height >= 40 {
            (10, 10, 3)
        } else if self.width >= 40 && self.height >= 12 {
            (6, 6, 2)
        } else {
            (4, 4, 1)
        };
        // a margin all round keeps the grid off the edges of the screen
        let (margin_x, margin_y) = if self.width >= 40 && self.height >= 12 {
            (2, 2)
        } else {
            (0, 0)
        };
        self.columns = ((w - 2 * margin_x + GAP_X) / (self.tile_w + GAP_X)).max(1);
        self.rows = ((h - 2 * margin_y + GAP_Y) / (self.tile_h + GAP_Y)).max(1);
        let used_w = self.columns * (self.tile_w + GAP_X) - GAP_X;
        let used_h = self.rows * (self.tile_h + GAP_Y) - GAP_Y;
        self.left = ((w - used_w) / 2).max(0);
        // tiles start on an even pixel row, so a tile's top and bottom edges line up with
        // whole cells and each change touches the fewest cells
        self.top = ((h - used_h) / 2).max(0) & !1;
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
        let c = TileColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        let tall = canvas.height() as i32;
        for y in 0..tall {
            canvas.fill_row(y, c.back.at(y as f32 / tall.max(1) as f32));
        }
        let (tw, th) = (self.tile_w, self.tile_h);
        for row in 0..self.rows {
            for column in 0..self.columns {
                let i = (row * self.columns + column) as usize;
                let x0 = self.left + column * (tw + GAP_X);
                let y0 = self.top + row * (th + GAP_Y);
                match self.colours.get(i).copied().flatten() {
                    Some(fill) => self.paint_filled(&mut canvas, theme, x0, y0, c.fills[fill]),
                    None => self.paint_empty(&mut canvas, &c, x0, y0),
                }
            }
        }
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::pixels::{IMAGERY_CEILING, distance};
    use crate::scenes::testing::{
        check_determinism, check_pixel_scene, check_scene, pixels, render,
    };
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("tiles");
        check_determinism("tiles");
        check_pixel_scene("tiles", 6.0);
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
        let c = TileColours::new(theme::default_theme());
        let all = pixels(&frame);
        assert!(c.fills.iter().any(|f| all.contains(f)));
        assert!(all.contains(&c.outline));
        assert!(Tiles::new(20, 8, Motion::FULL, &mut rng).columns >= 3);
        // the grid never runs off the screen
        for (w, h) in [(20, 8), (80, 24), (200, 60)] {
            let t = Tiles::new(w, h, Motion::FULL, &mut rng);
            assert!(t.left + t.columns * (t.tile_w + GAP_X) - GAP_X <= w as i32);
            assert!(t.top + t.rows * (t.tile_h + GAP_Y) - GAP_Y <= 2 * h as i32);
        }
    }

    #[test]
    fn every_tile_has_a_folded_corner() {
        let mut rng = Rng::new(2);
        let tiles = Tiles::new(80, 24, Motion::FULL, &mut rng);
        let (tw, f) = (tiles.tile_w, tiles.fold);
        assert!(tiles.cut(tw - 1, 0));
        assert!(!tiles.cut(0, 0) && !tiles.cut(tw - 1, f));
        assert!(f == 1 || tiles.flap(tw - 2, f - 1));
    }

    #[test]
    fn the_colours_stay_distinct_and_in_bounds_in_every_theme() {
        for theme in theme::all() {
            let c = TileColours::new(theme);
            let name = theme.name;
            for (what, colour) in [
                ("backdrop", c.back_top),
                ("outline", c.outline),
                ("inside", c.inside),
            ] {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            assert!(distance(c.outline, c.inside) >= 0.05, "{name}");
            assert!(distance(c.inside, c.back_low) >= 0.01, "{name}");
            for fill in c.fills {
                assert!(theme.contrast_of(fill) >= theme.floor(), "{name}");
                assert!(distance(fill, c.outline) >= 0.08, "{name}");
            }
        }
    }
}
