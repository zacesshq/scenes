//! Code rain: columns of characters fall down the screen, each in one of the theme's syntax
//! colours with a bright head, over a softly shaded backdrop painted in pixels.
//!
//! The characters are text, so they keep the theme's floor against the backdrop under them: the
//! backdrop is the strongest tint every text colour still clears. Each column's head throws a
//! faint glow on the cell below it and its tail fades away into the backdrop in pixels; the glow
//! and the fade are tints held below the non-text floor, so they read as background.

use crate::colour::Rgb;
use crate::frame::{Cell, Frame};
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

const GLYPHS: &[u8] = b"01ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz<>{}[]()=+-*/%#@$&!?;:";
const BACKDROP_STEPS: usize = 6;

#[derive(Clone, Debug)]
struct Column {
    /// The row of the leading character, which may be above or below the screen.
    head: f32,
    speed: f32,
    length: usize,
    chars: Vec<char>,
    colour: usize,
    active: bool,
    wait: f32,
}

/// The fastest a column falls under reduced motion, in rows a second.
pub const CALM_MAX_SPEED: f32 = 3.0;

/// The colours of the code rain in one theme, every one derived from the theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct RainColours {
    pub head: Rgb,
    pub glow: Rgb,
    /// The backdrop from the top of the screen to the bottom, every step safe under text.
    backdrop: Ramp,
    /// For each syntax colour, its tail's two fading steps.
    fades: Vec<[Rgb; 2]>,
}

impl RainColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let mut text: Vec<Rgb> = theme.syntax.to_vec();
        text.push(theme.bright());
        let floor = theme.floor();
        let top = shades.under_text(theme.comment(), 0.3, &text, floor);
        let backdrop = Ramp::new(top, theme.background, BACKDROP_STEPS);
        let fades = theme
            .syntax
            .iter()
            .map(|&colour| {
                [
                    shades.tint(colour, 0.5, IMAGERY_CEILING - 0.3),
                    shades.tint(colour, 0.25, 1.6),
                ]
            })
            .collect();
        Self {
            head: theme.bright(),
            glow: shades.tint(theme.bright(), 0.35, 2.0),
            backdrop,
            fades,
        }
    }

    /// The backdrop at a pixel row, `t` from 0 at the top to 1 at the bottom.
    fn backdrop(&self, t: f32) -> Rgb {
        self.backdrop.at(t)
    }
}

pub struct CodeRain {
    width: usize,
    height: usize,
    motion: Motion,
    columns: Vec<Column>,
}

impl CodeRain {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut rain = Self {
            width: 1,
            height: 1,
            motion,
            columns: Vec::new(),
        };
        rain.resize(width, height, rng);
        rain
    }

    /// The longest pause before a column falls again: short screens refill sooner.
    fn wait_cap(&self) -> f32 {
        (self.height as f32 / 5.0).clamp(1.5, 8.0)
    }

    fn glyph(rng: &mut Rng) -> char {
        *rng.pick(GLYPHS) as char
    }

    fn start(&self, column: &mut Column, rng: &mut Rng, from_top: bool) {
        column.length = 4 + rng.below((self.height / 2).max(3));
        // rows a second scale with the height, so a short screen is not a blur and a tall one
        // is not a crawl
        let scale = (self.height as f32 / 24.0).clamp(0.5, 1.5);
        // under reduced motion a column advances at most three rows a second, so no cell of
        // its trail shows a new character more than three times a second
        column.speed = if self.motion.calm {
            rng.range(1.5, CALM_MAX_SPEED)
        } else {
            rng.range(4.0, 10.0) * scale
        };
        column.chars = (0..column.length).map(|_| Self::glyph(rng)).collect();
        column.colour = rng.below(9);
        column.head = if from_top {
            -1.0
        } else {
            rng.range(0.0, self.height as f32)
        };
        column.active = true;
    }

    /// The backdrop colour under a cell row: the bottom pixel's, which is what text sits on.
    fn row_backdrop(&self, c: &RainColours, row: i32) -> Rgb {
        let tall = (self.height * 2) as f32;
        c.backdrop((row * 2 + 1) as f32 / tall)
    }
}

impl Scene for CodeRain {
    fn name(&self) -> &'static str {
        "code-rain"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.columns = (0..self.width)
            .map(|_| {
                let mut column = Column {
                    head: 0.0,
                    speed: 0.0,
                    length: 0,
                    chars: Vec::new(),
                    colour: 0,
                    active: false,
                    wait: rng.range(0.0, 4.0),
                };
                // about half the columns are already falling, so the first frame is not empty
                if rng.chance(0.5) {
                    self.start(&mut column, rng, false);
                }
                column
            })
            .collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        let height = self.height as f32;
        let mutation = if self.motion.calm { 0.0 } else { 1.0 * dt };
        let wait_cap = self.wait_cap();
        for i in 0..self.columns.len() {
            if self.columns[i].active {
                let column = &mut self.columns[i];
                column.head += column.speed * dt;
                if mutation > 0.0 {
                    for ch in &mut column.chars {
                        if rng.chance(mutation) {
                            *ch = Self::glyph(rng);
                        }
                    }
                }
                if column.head - column.length as f32 > height {
                    column.active = false;
                    column.wait = rng.range(0.3, wait_cap);
                }
            } else {
                self.columns[i].wait -= dt;
                if self.columns[i].wait <= 0.0 {
                    let mut column = self.columns[i].clone();
                    self.start(&mut column, rng, true);
                    self.columns[i] = column;
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let c = RainColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        let tall = canvas.height() as i32;
        for y in 0..tall {
            canvas.fill_row(y, c.backdrop(y as f32 / tall.max(1) as f32));
        }
        // the pixel parts of each column: the fading tail and the glow ahead of the head
        for (x, column) in self.columns.iter().enumerate() {
            if !column.active {
                continue;
            }
            let x = x as i32;
            let head = column.head.floor() as i32;
            let [near, far] = c.fades[column.colour % c.fades.len()];
            let tail = head - column.length as i32 + 1;
            canvas.fill_rect(x, (tail + 1) * 2, 1, 2, near);
            canvas.set(x, tail * 2 + 1, far);
            canvas.set(x, (head + 1) * 2, c.glow);
        }
        canvas.blit(frame);

        // the characters, on the backdrop of their own row
        for (x, column) in self.columns.iter().enumerate() {
            if !column.active {
                continue;
            }
            let colour = theme.syntax[column.colour % theme.syntax.len()];
            let head = column.head.floor() as i32;
            for (i, ch) in column.chars.iter().enumerate() {
                let y = head - i as i32;
                // the last two characters have faded into the pixels painted above
                if y < 0 || y >= self.height as i32 || i + 2 >= column.length {
                    continue;
                }
                let colour = if i == 0 { c.head } else { colour };
                frame.put_cell(
                    x as i32,
                    y,
                    Cell {
                        ch: *ch,
                        colour: Some(colour),
                        bg: Some(self.row_backdrop(&c, y)),
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_pixel_scene, check_scene, render};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("code-rain");
        check_determinism("code-rain");
        check_pixel_scene("code-rain", 6.0);
    }

    #[test]
    fn the_backdrop_keeps_every_text_colour_on_its_floor_in_every_theme() {
        for theme in theme::all() {
            let c = RainColours::new(theme);
            for t in [0.0, 0.3, 0.6, 1.0] {
                let backdrop = c.backdrop(t);
                assert!(
                    theme.contrast_of(backdrop) < IMAGERY_CEILING,
                    "{}",
                    theme.name
                );
                for &colour in theme.syntax.iter().chain([&c.head]) {
                    assert!(
                        colour.contrast(backdrop) >= theme.floor(),
                        "{} on {} in {}",
                        colour.hex(),
                        backdrop.hex(),
                        theme.name
                    );
                }
            }
            for [near, far] in &c.fades {
                assert!(theme.contrast_of(*near) < IMAGERY_CEILING, "{}", theme.name);
                assert!(theme.contrast_of(*far) < IMAGERY_CEILING, "{}", theme.name);
            }
            assert!(
                theme.contrast_of(c.glow) < IMAGERY_CEILING,
                "{}",
                theme.name
            );
        }
    }

    #[test]
    fn columns_fall_in_syntax_colours_with_a_bright_head() {
        let mut rng = Rng::new(3);
        let mut rain = CodeRain::new(60, 20, Motion::FULL, &mut rng);
        let frame = render(&mut rain, 60, 20, 20);
        assert!(frame.drawn() > 30);
        let theme = theme::default_theme();
        let colours = frame.colours();
        assert!(colours.contains(&theme.bright()));
        assert!(
            colours.iter().filter(|c| theme.syntax.contains(c)).count() >= 2,
            "{colours:?}"
        );
        // the characters sit on the backdrop, never on a hole cut out of it
        let text = frame
            .cells()
            .iter()
            .filter(|cell| !cell.is_pixel() && cell.ch != ' ')
            .count();
        assert!(text > 30);
        assert!(
            frame
                .cells()
                .iter()
                .filter(|cell| !cell.is_pixel())
                .all(|cell| cell.bg.is_some())
        );
        let heads_before: Vec<f32> = rain.columns.iter().map(|c| c.head).collect();
        rain.step(0.5, &mut rng);
        let moved = rain
            .columns
            .iter()
            .zip(&heads_before)
            .filter(|(c, before)| c.active && c.head > **before)
            .count();
        assert!(moved > 10, "{moved}");
    }

    #[test]
    fn calm_trails_change_a_cell_at_most_three_times_a_second() {
        use crate::scenes::testing::max_changes_per_second;
        let theme = crate::theme::default_theme();
        for &(width, height) in &[(20, 8), (80, 24), (120, 40)] {
            let mut rng = Rng::new(6);
            let mut rain = CodeRain::new(width, height, Motion::CALM, &mut rng);
            let frames: Vec<Frame> = (0..(20 * 12))
                .map(|_| {
                    rain.step(1.0 / 12.0, &mut rng);
                    let mut frame = Frame::new(width, height);
                    rain.draw(&mut frame, theme);
                    frame
                })
                .collect();
            let most = max_changes_per_second(&frames, 12);
            assert!(most <= 3, "{most} at {width}x{height}");
        }
    }

    #[test]
    fn calm_columns_never_change_their_characters() {
        let mut rng = Rng::new(5);
        let mut rain = CodeRain::new(30, 10, Motion::CALM, &mut rng);
        let before: Vec<(bool, f32, Vec<char>)> = rain
            .columns
            .iter()
            .map(|c| (c.active, c.head, c.chars.clone()))
            .collect();
        for _ in 0..10 {
            rain.step(0.05, &mut rng);
        }
        let mut compared = 0;
        for (column, (active, head, chars)) in rain.columns.iter().zip(&before) {
            // only a column that kept falling, not one that finished and started afresh
            if *active && column.active && column.head > *head {
                assert_eq!(&column.chars, chars);
                compared += 1;
            }
        }
        assert!(compared > 5, "{compared}");
    }
}
