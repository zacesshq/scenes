//! Code rain: columns of characters fall down the screen, each in one of the theme's syntax
//! colours with a bright head.

use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

const GLYPHS: &[u8] = b"01ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz<>{}[]()=+-*/%#@$&!?;:";

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
        for (x, column) in self.columns.iter().enumerate() {
            if !column.active {
                continue;
            }
            let colour = theme.syntax[column.colour % theme.syntax.len()];
            let head = column.head.floor() as i32;
            for (i, ch) in column.chars.iter().enumerate() {
                let y = head - i as i32;
                if y < 0 || y >= self.height as i32 {
                    continue;
                }
                let colour = if i == 0 {
                    theme.bright()
                } else if i + 2 >= column.length {
                    theme.grey()
                } else {
                    colour
                };
                frame.put(x as i32, y, *ch, Some(colour));
            }
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
        check_scene("code-rain");
        check_determinism("code-rain");
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
