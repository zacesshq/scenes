//! The Zacess mark, a page with a folded corner and a band across it, drifting slowly and
//! bouncing gently off the edges.
//!
//! On a dark theme the terminal background stands in for the mark's dark tile, so the page is
//! drawn in the text colour. On a light theme the tile is drawn and the page is left as the
//! background, which keeps the mark's own light page on dark tile.

use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// Page sizes (columns, rows) from the largest to the smallest.
const PAGE_SIZES: [(i32, i32); 6] = [(15, 11), (11, 8), (9, 7), (7, 5), (5, 4), (3, 3)];
const WORD: &str = "Zacess";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    page_w: i32,
    page_h: i32,
    /// How far the band reaches past the page on each side.
    reach: i32,
    /// The tile's padding around the page, columns and rows.
    pad_x: i32,
    pad_y: i32,
    /// Whether the word fits under the mark.
    word: bool,
}

impl Layout {
    fn fit(width: i32, height: i32) -> Option<Layout> {
        for &(page_w, page_h) in &PAGE_SIZES {
            let reach = ((page_w as f32 / 6.0).round() as i32).max(1);
            let layout = Layout {
                page_w,
                page_h,
                reach,
                pad_x: reach + 1,
                pad_y: 1,
                word: false,
            };
            if layout.box_w() + 2 <= width && layout.box_h() < height {
                let word = height >= layout.box_h() + 3 && width >= WORD.len() as i32 + 2;
                return Some(Layout { word, ..layout });
            }
        }
        None
    }

    fn box_w(self) -> i32 {
        self.page_w + 2 * self.pad_x
    }

    fn box_h(self) -> i32 {
        self.page_h + 2 * self.pad_y
    }

    /// The rows the drifting box needs, the word included.
    fn total_h(self) -> i32 {
        self.box_h() + if self.word { 2 } else { 0 }
    }

    fn fold(self) -> i32 {
        ((self.page_w as f32 * 0.3).round() as i32).clamp(1, (self.page_h - 1).max(1))
    }

    fn band_rows(self) -> (i32, i32) {
        let rows = ((self.page_h as f32 * 0.25).round() as i32).max(1);
        let top = (self.page_h as f32 * 0.465).round() as i32;
        (top, (top + rows).min(self.page_h))
    }
}

pub struct Zacess {
    width: usize,
    height: usize,
    layout: Option<Layout>,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

impl Zacess {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut mark = Self {
            width: 1,
            height: 1,
            layout: None,
            x: 0.0,
            y: 0.0,
            vx: 1.4 * motion.speed(),
            vy: 0.5 * motion.speed(),
        };
        if rng.chance(0.5) {
            mark.vx = -mark.vx;
        }
        if rng.chance(0.5) {
            mark.vy = -mark.vy;
        }
        mark.resize(width, height, rng);
        mark
    }

    #[cfg(test)]
    pub(crate) fn position(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    fn draw_mark(&self, frame: &mut Frame, theme: &ThemeData, layout: Layout) {
        let bx = self.x.round() as i32;
        let by = self.y.round() as i32;
        let px = bx + layout.pad_x;
        let py = by + layout.pad_y;
        let fold = layout.fold();
        let band = if theme.dark {
            theme.bright_yellow()
        } else {
            theme.yellow()
        };
        let block = '\u{2588}';

        if !theme.dark {
            // the tile, with its four corners left off so it reads as rounded
            for y in 0..layout.box_h() {
                for x in 0..layout.box_w() {
                    let corner =
                        (x == 0 || x == layout.box_w() - 1) && (y == 0 || y == layout.box_h() - 1);
                    if !corner {
                        frame.put(bx + x, by + y, block, Some(theme.foreground));
                    }
                }
            }
        }

        for y in 0..layout.page_h {
            for x in 0..layout.page_w {
                let diagonal = y < fold && x == layout.page_w - fold + y;
                let cut = y < fold && x > layout.page_w - fold + y;
                if theme.dark {
                    if diagonal {
                        frame.put(px + x, py + y, '\u{25e3}', Some(theme.muted));
                    } else if !cut {
                        frame.put(px + x, py + y, block, Some(theme.foreground));
                    }
                } else if diagonal {
                    frame.put(px + x, py + y, '\u{25e5}', Some(theme.foreground));
                } else if !cut {
                    frame.put(px + x, py + y, ' ', None);
                }
            }
        }

        let (top, bottom) = layout.band_rows();
        for y in top..bottom {
            for x in -layout.reach..layout.page_w + layout.reach {
                frame.put(px + x, py + y, block, Some(band));
            }
        }

        if layout.word {
            let x = bx + (layout.box_w() - WORD.len() as i32) / 2;
            frame.put_str(x, by + layout.box_h() + 1, WORD, Some(theme.foreground));
        }
    }
}

impl Scene for Zacess {
    fn name(&self) -> &'static str {
        "zacess"
    }

    fn resize(&mut self, width: usize, height: usize, _rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.layout = Layout::fit(self.width as i32, self.height as i32);
        if let Some(layout) = self.layout {
            let max_x = (self.width as f32 - layout.box_w() as f32).max(0.0);
            let max_y = (self.height as f32 - layout.total_h() as f32).max(0.0);
            self.x = self.x.clamp(0.0, max_x);
            self.y = self.y.clamp(0.0, max_y);
        }
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        let Some(layout) = self.layout else {
            return;
        };
        let max_x = (self.width as f32 - layout.box_w() as f32).max(0.0);
        let max_y = (self.height as f32 - layout.total_h() as f32).max(0.0);
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        if self.x < 0.0 {
            self.x = 0.0;
            self.vx = self.vx.abs();
        } else if self.x > max_x {
            self.x = max_x;
            self.vx = -self.vx.abs();
        }
        if self.y < 0.0 {
            self.y = 0.0;
            self.vy = self.vy.abs();
        } else if self.y > max_y {
            self.y = max_y;
            self.vy = -self.vy.abs();
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        match self.layout {
            Some(layout) => self.draw_mark(frame, theme, layout),
            // too small for the mark: the initial alone, held still
            None => frame.put(0, 0, 'Z', Some(theme.foreground)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("zacess");
        check_determinism("zacess");
    }

    #[test]
    fn the_mark_fits_a_phone_sized_terminal_and_grows_with_the_screen() {
        assert_eq!(
            Layout::fit(20, 8).map(|l| (l.page_w, l.page_h)),
            Some((7, 5))
        );
        assert_eq!(
            Layout::fit(120, 40).map(|l| (l.page_w, l.page_h)),
            Some((15, 11))
        );
        assert!(Layout::fit(120, 40).unwrap().word);
        assert!(!Layout::fit(20, 8).unwrap().word);
        assert_eq!(Layout::fit(4, 3), None);
    }

    #[test]
    fn the_page_band_and_fold_are_drawn_in_both_appearances() {
        let mut rng = Rng::new(1);
        let mut mark = Zacess::new(40, 20, Motion::FULL, &mut rng);
        let dark = theme::by_id("deep-contrast-dark").unwrap();
        let light = theme::by_id("deep-contrast-light").unwrap();
        for theme in [dark, light] {
            let mut frame = Frame::new(40, 20);
            mark.draw(&mut frame, theme);
            let text = frame.to_text();
            assert!(text.contains('\u{2588}'), "{text}");
            assert!(text.contains("Zacess"), "{text}");
            let band_colour = if theme.dark {
                theme.bright_yellow()
            } else {
                theme.yellow()
            };
            assert!(frame.colours().contains(&band_colour));
            let fold = if theme.dark { '\u{25e3}' } else { '\u{25e5}' };
            assert!(text.contains(fold), "{text}");
        }
        let frame = render(&mut mark, 40, 20, 1);
        assert!(frame.colours().contains(&dark.muted));
    }

    #[test]
    fn the_mark_drifts_and_stays_inside_the_screen() {
        let mut rng = Rng::new(4);
        let mut mark = Zacess::new(20, 8, Motion::FULL, &mut rng);
        let start = mark.position();
        let layout = mark.layout.unwrap();
        let mut positions = std::collections::BTreeSet::new();
        for _ in 0..(60 * 20) {
            mark.step(0.05, &mut rng);
            let (x, y) = mark.position();
            assert!(x >= 0.0 && x + layout.box_w() as f32 <= 20.0, "{x}");
            assert!(y >= 0.0 && y + layout.total_h() as f32 <= 8.0, "{y}");
            positions.insert((x.round() as i32, y.round() as i32));
        }
        assert_ne!(mark.position(), start);
        assert!(positions.len() > 10);
    }
}
