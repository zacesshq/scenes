//! The Zacess mark, a page with a folded corner and a band across it on a rounded tile, painted
//! large in solid pixels and floating slowly round the middle of the screen over a calm backdrop:
//! a soft wash of the theme's accent with two broad ribbons of colour flowing gently across it
//! and a quiet halo round the tile.
//!
//! On a dark theme the tile is the theme's background and the page is drawn in the text colour.
//! On a light theme the tile is drawn in the text colour and the page is the background, which
//! keeps the mark's own light page on dark tile. The mark is scaled to the screen, about half
//! its height, so it is the clear subject at every size.
//!
//! The band is kept apart from the page and the tile by rows of background above and below it.
//! The band's colour is close to the page's on some themes and to the tile's on others. With
//! colour off every shape is drawn in one colour, so the gap is what keeps the band visible.
//! The backdrop is a small table of tints held below the non-text floor that the word under the
//! mark still keeps its own floor against.

use super::cycle;
use crate::colour::Rgb;
use crate::frame::{Cell, Frame};
use crate::pixels::{Canvas, IMAGERY_CEILING, Shades};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

const WORD: &str = "Zacess";
/// Lightness steps in each row of the backdrop table.
const LEVELS: usize = 9;
/// The smallest page, in pixels, that still has page above and below the band and its gaps.
const SMALLEST_PAGE: i32 = 8;

/// The mark's size, every measure in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    page_w: i32,
    page_h: i32,
    /// How far the band reaches past the page on each side.
    reach: i32,
    /// The tile's padding round the page, across and down.
    pad_x: i32,
    pad_y: i32,
    /// The radius of the tile's rounded corners.
    radius: i32,
    /// Whether the word fits under the mark.
    word: bool,
}

impl Layout {
    fn with_page(page_h: i32) -> Layout {
        let page_w = ((page_h as f32 * 0.7).round() as i32).max(6);
        let reach = ((page_w as f32 / 7.0).round() as i32).max(1);
        let pad_x = reach + ((page_w as f32 / 8.0).round() as i32).max(1);
        let pad_y = ((page_h as f32 / 7.0).round() as i32).max(2);
        let box_w = page_w + 2 * pad_x;
        Layout {
            page_w,
            page_h,
            reach,
            pad_x,
            pad_y,
            radius: (box_w / 6).clamp(2, 10),
            word: false,
        }
    }

    /// The largest mark that fits `width` columns by `height` rows, about half the height.
    fn fit(width: i32, height: i32) -> Option<Layout> {
        let tall = height * 2;
        let word_room = if height >= 14 { 4 } else { 0 };
        let mut page_h = (tall as f32 * 0.52).round() as i32;
        while page_h >= SMALLEST_PAGE {
            let layout = Layout::with_page(page_h);
            let wide_enough = layout.box_w() + 4 <= width && layout.box_w() * 2 <= width + 8;
            if wide_enough && layout.box_h() + word_room + 2 <= tall {
                let word = word_room > 0 && width >= WORD.len() as i32 + 2;
                return Some(Layout { word, ..layout });
            }
            page_h -= 1;
        }
        None
    }

    fn box_w(self) -> i32 {
        self.page_w + 2 * self.pad_x
    }

    fn box_h(self) -> i32 {
        self.page_h + 2 * self.pad_y
    }

    /// The pixel rows the mark needs, the word included.
    fn total_h(self) -> i32 {
        self.box_h() + if self.word { 4 } else { 0 }
    }

    /// The fold's size. Pixels are square, so the fold is a true diagonal.
    fn fold(self) -> i32 {
        ((self.page_w as f32 * 0.3).round() as i32).clamp(1, (self.page_h - 1).max(1))
    }

    /// The thickness of the gap above and below the band, thicker on a large mark so it still
    /// reads as a cut.
    fn gap(self) -> i32 {
        (self.page_h / 18).max(1)
    }

    /// The band's first row on the page and the row after its last.
    fn band_rows(self) -> (i32, i32) {
        let rows = ((self.page_h as f32 * 0.25).round() as i32).max(2);
        let top = (self.page_h as f32 * 0.465).round() as i32;
        (top, (top + rows).min(self.page_h))
    }
}

/// The mark's colours and the backdrop's table in one theme, every one derived from the theme's
/// own.
#[derive(Clone, Debug)]
struct MarkColours {
    page: Rgb,
    tile: Rgb,
    band: Rgb,
    gap: Rgb,
    word: Rgb,
    /// Three rows of tints, the plain wash and the two ribbons, each from the faintest step to
    /// the strongest.
    table: [[Rgb; LEVELS]; 3],
}

impl MarkColours {
    fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        // a tint towards a hue, held both under the imagery ceiling and where the word keeps
        // its floor on it; both limits lie along the same mix, so the fainter one is the answer
        let soft = |hue: Rgb, strength: f32| {
            let a = shades.tint(hue, strength, IMAGERY_CEILING - 0.4);
            let b = shades.under_text(hue, strength, &[theme.foreground], theme.floor());
            if theme.contrast_of(a) <= theme.contrast_of(b) {
                a
            } else {
                b
            }
        };
        let hues = [theme.accent, theme.cyan(), theme.magenta()];
        let reach = if theme.dark { 0.42 } else { 0.3 };
        let table = hues.map(|hue| {
            std::array::from_fn(|n| soft(hue, 0.03 + reach * n as f32 / (LEVELS - 1) as f32))
        });
        let (page, tile) = if theme.dark {
            (theme.foreground, theme.background)
        } else {
            (theme.background, theme.foreground)
        };
        Self {
            page,
            tile,
            band: Zacess::band_colour(theme),
            gap: theme.background,
            word: theme.foreground,
            table,
        }
    }
}

/// A broad ribbon of colour flowing across the backdrop.
#[derive(Clone, Debug, PartialEq)]
struct Ribbon {
    /// Its middle as a share of the height.
    middle: f32,
    /// How far it swings up and down, as a share of the height.
    swing: f32,
    /// Pixels one rise and fall takes, as a share of the width.
    length: f32,
    phase: f32,
    /// Radians a second the ribbon flows at the standard pace.
    rate: f32,
}

pub struct Zacess {
    width: usize,
    height: usize,
    layout: Option<Layout>,
    speed: f32,
    /// The scene clock, already scaled by the motion setting.
    t: f64,
    phase: (f32, f32),
    ribbons: [Ribbon; 2],
}

impl Zacess {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let ribbon = |rng: &mut Rng, middle: f32, rate: f32| Ribbon {
            middle,
            swing: rng.range(0.06, 0.1),
            length: rng.range(0.25, 0.4),
            phase: rng.range(0.0, std::f32::consts::TAU),
            rate,
        };
        let ribbons = [ribbon(rng, 0.3, 0.16), ribbon(rng, 0.74, -0.12)];
        let mut mark = Self {
            width: 1,
            height: 1,
            layout: None,
            speed: motion.speed(),
            t: 0.0,
            phase: (
                rng.range(0.0, std::f32::consts::TAU),
                rng.range(0.0, std::f32::consts::TAU),
            ),
            ribbons,
        };
        mark.resize(width, height, rng);
        mark
    }

    /// The band's colour on a theme.
    fn band_colour(theme: &ThemeData) -> Rgb {
        if theme.dark {
            theme.bright_yellow()
        } else {
            theme.yellow()
        }
    }

    fn tall(&self) -> f32 {
        self.height as f32 * 2.0
    }

    /// The top left corner of the tile in pixels: the middle of the screen, floated slowly
    /// round a small loop that never reaches an edge.
    fn position(&self) -> (f32, f32) {
        let Some(layout) = self.layout else {
            return (0.0, 0.0);
        };
        let (w, tall) = (self.width as f32, self.tall());
        let (bw, bh) = (layout.box_w() as f32, layout.total_h() as f32);
        let slack_x = ((w - bw) / 2.0).max(0.0);
        let slack_y = ((tall - bh) / 2.0).max(0.0);
        let ax = slack_x.min((w * 0.07).max(2.0)) * 0.9;
        let ay = slack_y.min((tall * 0.06).max(1.5)) * 0.9;
        let x = slack_x + ax * (cycle(self.t, 0.13) + self.phase.0).sin();
        let y = slack_y + ay * (cycle(self.t, 0.19) + self.phase.1).sin();
        (x, y)
    }

    /// The ribbon that shows most at a pixel (counted from one) and how strongly it shows.
    fn ribbon_at(&self, x: i32, y: i32) -> (usize, f32) {
        let (w, tall) = (self.width as f32, self.tall());
        let mut best = (0, 0.0_f32);
        for (i, r) in self.ribbons.iter().enumerate() {
            let flow = cycle(self.t, r.rate) + r.phase;
            let u = x as f32 / (w * r.length).max(4.0);
            let middle = tall
                * (r.middle
                    + r.swing * (u + flow).sin()
                    + r.swing * 0.35 * (u * 2.3 - flow * 0.7).sin());
            let thick = (tall * 0.16).max(2.0);
            let s = (1.0 - (y as f32 - middle).abs() / thick).max(0.0);
            if s > best.1 {
                best = (i + 1, s);
            }
        }
        best
    }

    fn paint(&self, canvas: &mut Canvas, c: &MarkColours, dark: bool) {
        let (wide, tall) = (canvas.width() as i32, canvas.height() as i32);
        let (mx, my) = self.position();
        let layout = self.layout;
        // the halo's middle and size follow the tile
        let (hx, hy, hr) = match layout {
            Some(l) => (
                mx + l.box_w() as f32 / 2.0,
                my + l.box_h() as f32 / 2.0,
                l.box_h() as f32 * 0.95,
            ),
            None => (wide as f32 / 2.0, tall as f32 / 2.0, tall as f32),
        };
        for y in 0..tall {
            // the wash is strongest at the top and fades down the screen
            let wash = 3.0 * (1.0 - y as f32 / tall.max(1) as f32);
            for x in 0..wide {
                let (row, ribbon) = self.ribbon_at(x, y);
                let (dx, dy) = (x as f32 + 0.5 - hx, (y as f32 + 0.5 - hy) * 1.1);
                let d = (dx * dx + dy * dy).sqrt() / hr;
                let halo = (1.0 - d).max(0.0) * 3.0;
                // the halo is light: up the table on a dark theme, back towards the paper on a
                // light one
                let level = wash + ribbon * 4.0 + if dark { halo } else { -halo };
                let level = level.round().clamp(0.0, (LEVELS - 1) as f32) as usize;
                let row = if ribbon > 0.25 { row } else { 0 };
                canvas.set(x, y, c.table[row][level]);
            }
        }
        let Some(layout) = layout else {
            return;
        };
        let bx = mx.round() as i32;
        let by = my.round() as i32;
        let px = bx + layout.pad_x;
        let py = by + layout.pad_y;

        // the tile, its corners rounded
        let (w, h, r) = (layout.box_w(), layout.box_h(), layout.radius);
        for y in 0..h {
            for x in 0..w {
                let cx = (r - 1 - x).max(x - (w - r)).max(0) as f32;
                let cy = (r - 1 - y).max(y - (h - r)).max(0) as f32;
                if cx * cx + cy * cy <= (r as f32 - 0.5).powi(2) {
                    canvas.set(bx + x, by + y, c.tile);
                }
            }
        }

        // the page, with its top right corner cut along the fold's diagonal
        let fold = layout.fold();
        for y in 0..layout.page_h {
            for x in 0..layout.page_w {
                let cut = y < fold && x > layout.page_w - fold + y;
                if !cut {
                    canvas.set(px + x, py + y, c.page);
                }
            }
        }

        // the band across the page and past it, with a gap of background above and below it
        // across the whole tile
        let (top, bottom) = layout.band_rows();
        let gap = layout.gap();
        for k in 0..gap {
            for y in [top - 1 - k, bottom + k] {
                for x in -layout.pad_x..layout.page_w + layout.pad_x {
                    canvas.set(px + x, py + y, c.gap);
                }
            }
        }
        for y in top..bottom {
            for x in -layout.pad_x..layout.page_w + layout.pad_x {
                let inside = x >= -layout.reach && x < layout.page_w + layout.reach;
                canvas.set(px + x, py + y, if inside { c.band } else { c.gap });
            }
        }
    }

    /// The cell row the word sits on, a row clear of the tile.
    fn word_row(&self, layout: Layout) -> i32 {
        (self.position().1.round() as i32 + layout.box_h() + 1) / 2 + 1
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
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt * self.speed);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let c = MarkColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &c, theme.dark);
        canvas.blit(frame);
        match self.layout {
            Some(layout) if layout.word => {
                let row = self.word_row(layout);
                let x0 =
                    self.position().0.round() as i32 + (layout.box_w() - WORD.len() as i32) / 2;
                for (i, ch) in WORD.chars().enumerate() {
                    let x = x0 + i as i32;
                    // the word sits on the backdrop under it, which keeps its floor against it
                    let under = canvas.get(x, row * 2 + 1).unwrap_or(theme.background);
                    let bg = if c.word.contrast(under) >= theme.floor() {
                        under
                    } else {
                        theme.background
                    };
                    frame.put_cell(
                        x,
                        row,
                        Cell {
                            ch,
                            colour: Some(c.word),
                            bg: Some(bg),
                        },
                    );
                }
            }
            Some(_) => {}
            // too small for the mark: the initial alone, held still
            None => frame.put(0, 0, 'Z', Some(theme.foreground)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{
        check_determinism, check_pixel_scene, check_scene, pixels, render,
    };
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("zacess");
        check_determinism("zacess");
        check_pixel_scene("zacess", 6.0);
    }

    fn canvas(mark: &Zacess, width: usize, height: usize, theme: &ThemeData) -> Canvas {
        let mut canvas = Canvas::new(width, height, theme.background);
        mark.paint(&mut canvas, &MarkColours::new(theme), theme.dark);
        canvas
    }

    #[test]
    fn the_mark_fits_a_phone_sized_terminal_and_grows_with_the_screen() {
        let small = Layout::fit(20, 8).unwrap();
        let big = Layout::fit(120, 40).unwrap();
        assert!(small.page_h >= SMALLEST_PAGE && !small.word);
        assert!(big.word);
        // the mark is the subject: about half the height of a full screen
        assert!(big.box_h() * 2 >= 80, "{}", big.box_h());
        assert!(Layout::fit(400, 120).unwrap().page_h > big.page_h);
        assert_eq!(Layout::fit(8, 6), None);
    }

    #[test]
    fn the_backdrop_reads_as_background_and_keeps_the_word_on_its_floor() {
        for theme in theme::all() {
            let c = MarkColours::new(theme);
            for step in c.table.iter().flatten() {
                assert!(theme.contrast_of(*step) < IMAGERY_CEILING, "{}", theme.name);
                assert!(c.word.contrast(*step) >= theme.floor(), "{}", theme.name);
            }
            // the table's ends stay apart, so the wash and the ribbons show
            for row in &c.table {
                assert!(
                    crate::pixels::distance(row[0], row[LEVELS - 1]) >= 0.03,
                    "{}",
                    theme.name
                );
            }
            assert!(theme.contrast_of(c.band) >= theme.floor(), "{}", theme.name);
        }
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
            assert!(frame.to_text().contains("Zacess"));
            let all = pixels(&frame);
            assert!(all.contains(&theme.foreground));
            assert!(all.contains(&Zacess::band_colour(theme)));
            // the fold: the page's top right pixel is cut away, the one below the diagonal kept
            let layout = mark.layout.unwrap();
            let (mx, my) = mark.position();
            let px = mx.round() as i32 + layout.pad_x;
            let py = my.round() as i32 + layout.pad_y;
            let page = MarkColours::new(theme).page;
            let canvas = canvas(&mark, 40, 20, theme);
            assert_ne!(canvas.get(px + layout.page_w - 1, py), Some(page));
            assert_eq!(canvas.get(px, py), Some(page));
            let fold = layout.fold();
            assert_eq!(
                canvas.get(px + layout.page_w - fold, py + fold - 1),
                Some(page)
            );
        }
        let frame = render(&mut mark, 40, 20, 1);
        assert!(pixels(&frame).contains(&dark.foreground));
    }

    #[test]
    fn the_band_never_touches_a_pixel_of_another_shape() {
        for &(width, height) in &[(20, 8), (40, 20), (120, 40), (300, 90)] {
            let mut rng = Rng::new(3);
            let mark = Zacess::new(width, height, Motion::FULL, &mut rng);
            for theme in theme::all() {
                let band = Zacess::band_colour(theme);
                let canvas = canvas(&mark, width, height, theme);
                let mut band_pixels = 0;
                for y in 0..canvas.height() as i32 {
                    for x in 0..canvas.width() as i32 {
                        if canvas.get(x, y) != Some(band) {
                            continue;
                        }
                        band_pixels += 1;
                        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                            if let Some(next) = canvas.get(x + dx, y + dy) {
                                assert!(
                                    next != theme.foreground,
                                    "band touches the text colour at {x},{y} in {} at \
                                     {width}x{height}",
                                    theme.name
                                );
                            }
                        }
                    }
                }
                assert!(band_pixels > 0, "{} at {width}x{height}", theme.name);
            }
        }
    }

    #[test]
    fn without_colour_the_band_is_still_its_own_shape() {
        let mut rng = Rng::new(1);
        let mark = Zacess::new(40, 16, Motion::FULL, &mut rng);
        for theme in theme::all() {
            let canvas = canvas(&mark, 40, 16, theme);
            let layout = mark.layout.unwrap();
            let (top, bottom) = layout.band_rows();
            let (mx, my) = mark.position();
            let py = my.round() as i32 + layout.pad_y;
            let px = mx.round() as i32 + layout.pad_x;
            // with colour off a pixel is drawn when it clears the non-text floor
            let ink = |x: i32, y: i32| {
                canvas
                    .get(x, y)
                    .is_some_and(|p| theme.contrast_of(p) >= IMAGERY_CEILING)
            };
            let band = (px - layout.pad_x..px + layout.page_w + layout.pad_x)
                .filter(|&x| ink(x, py + top))
                .count();
            assert!(band > layout.page_w as usize, "{}", theme.name);
            for gap in [py + top - 1, py + bottom] {
                assert!(
                    (px - layout.pad_x..px + layout.page_w + layout.pad_x).all(|x| !ink(x, gap)),
                    "{}",
                    theme.name
                );
            }
        }
    }

    #[test]
    fn the_mark_floats_round_the_middle_and_stays_inside_the_screen() {
        for &(width, height) in &[(20, 8), (80, 24), (200, 60)] {
            let mut rng = Rng::new(4);
            let mut mark = Zacess::new(width, height, Motion::FULL, &mut rng);
            let start = mark.position();
            let layout = mark.layout.unwrap();
            let mut positions = std::collections::BTreeSet::new();
            for _ in 0..(60 * 20) {
                mark.step(0.05, &mut rng);
                let (x, y) = mark.position();
                assert!(x >= 0.0 && x + layout.box_w() as f32 <= width as f32, "{x}");
                assert!(
                    y >= 0.0 && y + layout.total_h() as f32 <= height as f32 * 2.0,
                    "{y}"
                );
                // never far from the middle
                let middle = x + layout.box_w() as f32 / 2.0;
                assert!((middle - width as f32 / 2.0).abs() <= width as f32 * 0.1 + 1.0);
                positions.insert((x.round() as i32, y.round() as i32));
            }
            assert_ne!(mark.position(), start);
            assert!(positions.len() > 3, "{width}x{height}");
        }
    }
}
