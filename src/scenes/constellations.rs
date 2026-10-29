//! Constellations: a night sky of faint stars where, one group at a time, a few brighter stars
//! are slowly joined by lines into a figure, named, held for a while and then faded away.

use super::count_for;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "Stars that are slowly joined into named figures one group at a time, then fade";

pub const CAPTIONS: &[&str] = &[
    "Everything connects.",
    "A clean build, written in the stars.",
    "Every point joined up.",
    "The sky checks out.",
    "Nothing out of place up there.",
];

/// Cells a second a line is drawn at, at the standard speed.
const DRAW_SPEED: f32 = 6.0;
/// Seconds a finished figure is held before it fades.
const HOLD: f32 = 6.0;
/// Seconds between the steps of a fade. A line changes colour once a step, so a cell changes at
/// most once in this long.
pub const FADE_STEP: f32 = 1.2;
/// Seconds of plain sky between two figures.
const REST: f32 = 2.5;

/// Names of our own for the figures.
const NAMES: [&str; 10] = [
    "The Kettle",
    "The Lantern",
    "The Heron",
    "The Compass",
    "The Little Bridge",
    "The Key",
    "The Kite",
    "The Teapot",
    "The Anchor",
    "The Open Book",
];

#[derive(Clone, Debug)]
struct Figure {
    stars: Vec<(i32, i32)>,
    /// Pairs of star indices joined by a line.
    lines: Vec<(usize, usize)>,
    name: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    /// Lines being drawn: cells drawn so far.
    Drawing(f32),
    /// The whole figure shown, for seconds so far.
    Holding(f32),
    /// Fading, for seconds so far.
    Fading(f32),
    /// Plain sky, for seconds so far.
    Resting(f32),
}

pub struct Constellations {
    width: usize,
    height: usize,
    motion: Motion,
    faint: Vec<(i32, i32)>,
    figures: Vec<Figure>,
    current: usize,
    stage: Stage,
    shown: u32,
}

/// The cells of a straight line between two stars, one cell a step as Bresenham's line walks
/// it, each drawn with the character for the step that reached it: `|` straight down, `/` or
/// `\\` on a diagonal, `_` along the flat of a sloping line and `-` on a level one. The stars
/// themselves are left out and the cells run from `a` to `b`, the order a line is drawn in.
fn line_cells(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32, char)> {
    // the walk always goes downwards, so the flat cells of each row sit on the far side of its
    // diagonal, where a `_` meets the end of the `/` or `\\` without a gap
    let downwards = b.1 >= a.1;
    let (from, to) = if downwards { (a, b) } else { (b, a) };
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let sx = dx.signum();
    let (ax, ay) = (dx.abs(), -dy);
    let flat = if dy == 0 { '-' } else { '_' };
    let mut cells = Vec::new();
    let (mut x, mut y) = from;
    let mut err = ax + ay;
    while (x, y) != to {
        let doubled = 2 * err;
        let mut moved_x = false;
        let mut moved_y = false;
        if doubled >= ay {
            err += ay;
            x += sx;
            moved_x = true;
        }
        if doubled <= ax {
            err += ax;
            y += 1;
            moved_y = true;
        }
        if (x, y) == to {
            break;
        }
        let ch = match (moved_x, moved_y) {
            (true, true) if sx > 0 => '\\',
            (true, true) => '/',
            (false, true) => '|',
            _ => flat,
        };
        cells.push((x, y, ch));
    }
    if !downwards {
        cells.reverse();
    }
    cells
}

impl Constellations {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut sky = Self {
            width: 1,
            height: 1,
            motion,
            faint: Vec::new(),
            figures: Vec::new(),
            current: 0,
            stage: Stage::Resting(REST),
            shown: 0,
        };
        sky.resize(width, height, rng);
        sky
    }

    fn fade_step(&self) -> f32 {
        FADE_STEP / self.motion.pace()
    }

    fn make_figure(rng: &mut Rng, left: i32, top: i32, w: i32, h: i32) -> Figure {
        let count = 4 + rng.below(4);
        let mut stars: Vec<(i32, i32)> = Vec::new();
        for _ in 0..count * 4 {
            if stars.len() == count {
                break;
            }
            let p = (
                left + rng.below(w.max(1) as usize) as i32,
                top + rng.below(h.max(1) as usize) as i32,
            );
            // stars too close together would merge into one blob of line
            if stars
                .iter()
                .all(|s| (s.0 - p.0).abs() >= 4 || (s.1 - p.1).abs() >= 2)
            {
                stars.push(p);
            }
        }
        // each new star joins the nearest one already in the figure, which keeps it one shape
        let mut lines = Vec::new();
        for i in 1..stars.len() {
            let nearest = (0..i)
                .min_by_key(|&j| {
                    let dx = (stars[i].0 - stars[j].0) / 2;
                    let dy = stars[i].1 - stars[j].1;
                    dx * dx + dy * dy
                })
                .unwrap_or(0);
            lines.push((nearest, i));
        }
        Figure {
            stars,
            lines,
            name: NAMES[rng.below(NAMES.len())],
        }
    }

    fn line_length(&self, figure: &Figure) -> usize {
        figure
            .lines
            .iter()
            .map(|&(a, b)| line_cells(figure.stars[a], figure.stars[b]).len())
            .sum()
    }

    fn next_figure(&mut self, rng: &mut Rng) {
        if self.figures.len() > 1 {
            let mut next = rng.below(self.figures.len() - 1);
            if next >= self.current {
                next += 1;
            }
            self.current = next;
        }
        self.stage = Stage::Drawing(0.0);
        self.shown += 1;
    }

    #[cfg(test)]
    pub(crate) fn shown(&self) -> u32 {
        self.shown
    }
}

impl Scene for Constellations {
    fn name(&self) -> &'static str {
        "constellations"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = self
            .motion
            .scaled(count_for(self.width, self.height, 25, 6, 400));
        self.faint = (0..wanted)
            .map(|_| (rng.below(self.width) as i32, rng.below(self.height) as i32))
            .collect();
        // the sky is split into boxes, one figure each, so no two figures overlap
        let box_w = (self.width as i32 / 3).clamp(10, 36).min(self.width as i32);
        let box_h = (self.height as i32 / 2)
            .clamp(5, 14)
            .min(self.height as i32);
        self.figures.clear();
        let mut top = 0;
        while top + box_h <= self.height as i32 {
            let mut left = 0;
            while left + box_w <= self.width as i32 {
                self.figures.push(Self::make_figure(
                    rng,
                    left + 1,
                    top + 1,
                    box_w - 2,
                    box_h - 3,
                ));
                left += box_w;
            }
            top += box_h;
        }
        if self.figures.is_empty() {
            let figure = Self::make_figure(
                rng,
                0,
                0,
                self.width as i32,
                (self.height as i32 - 1).max(1),
            );
            self.figures.push(figure);
        }
        self.current = rng.below(self.figures.len());
        self.stage = Stage::Drawing(0.0);
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        let Some(figure) = self.figures.get(self.current) else {
            return;
        };
        let total = self.line_length(figure) as f32;
        let fade = self.fade_step();
        self.stage = match self.stage {
            Stage::Drawing(done) => {
                let done = done + DRAW_SPEED * self.motion.speed() * dt;
                if done >= total {
                    Stage::Holding(0.0)
                } else {
                    Stage::Drawing(done)
                }
            }
            Stage::Holding(t) if t + dt >= HOLD / self.motion.pace() => Stage::Fading(0.0),
            Stage::Holding(t) => Stage::Holding(t + dt),
            Stage::Fading(t) if t + dt >= 3.0 * fade => Stage::Resting(0.0),
            Stage::Fading(t) => Stage::Fading(t + dt),
            Stage::Resting(t) if t + dt >= REST => {
                self.next_figure(rng);
                return;
            }
            Stage::Resting(t) => Stage::Resting(t + dt),
        };
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        for &(x, y) in &self.faint {
            frame.put(x, y, '.', Some(theme.muted));
        }
        // the stars of every figure stay in the sky, so a figure is joined from stars already seen
        for figure in &self.figures {
            for &(x, y) in &figure.stars {
                frame.put(x, y, '+', Some(theme.foreground));
            }
        }
        let Some(figure) = self.figures.get(self.current) else {
            return;
        };
        let fade = self.fade_step();
        let (line_colour, drawn) = match self.stage {
            Stage::Drawing(done) => (Some(theme.cyan()), done as usize),
            Stage::Holding(_) => (Some(theme.bright_cyan()), usize::MAX),
            Stage::Fading(t) if t < fade => (Some(theme.cyan()), usize::MAX),
            Stage::Fading(t) if t < 2.0 * fade => (Some(theme.grey()), usize::MAX),
            Stage::Fading(_) | Stage::Resting(_) => (None, 0),
        };
        if let Some(colour) = line_colour {
            let mut left = drawn;
            'lines: for &(a, b) in &figure.lines {
                for (x, y, ch) in line_cells(figure.stars[a], figure.stars[b]) {
                    if left == 0 {
                        break 'lines;
                    }
                    left -= 1;
                    frame.put(x, y, ch, Some(colour));
                }
            }
            for &(x, y) in &figure.stars {
                frame.put(x, y, '*', Some(theme.bright_yellow()));
            }
        }
        // the name shows whenever the figure does, brightest while it is held, below the
        // figure where there is room and above it otherwise
        let name_colour = match self.stage {
            Stage::Holding(_) => Some(theme.foreground),
            Stage::Drawing(_) => Some(theme.muted),
            Stage::Fading(t) if t < 2.0 * fade => Some(theme.muted),
            _ => None,
        };
        let length = figure.name.len() as i32;
        if let Some(colour) = name_colour.filter(|_| length <= self.width as i32) {
            let lowest = figure.stars.iter().map(|s| s.1).max().unwrap_or(0);
            let highest = figure.stars.iter().map(|s| s.1).min().unwrap_or(0);
            let row = if lowest + 1 < self.height as i32 {
                lowest + 1
            } else {
                highest - 1
            };
            let centre =
                figure.stars.iter().map(|s| s.0).sum::<i32>() / figure.stars.len().max(1) as i32;
            let left = (centre - length / 2).clamp(0, self.width as i32 - length);
            if row >= 0 {
                frame.put_str(left, row, figure.name, Some(colour));
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
        check_scene("constellations");
        check_determinism("constellations");
    }

    #[test]
    fn figures_are_drawn_named_and_faded_one_at_a_time() {
        let mut rng = Rng::new(6);
        let mut sky = Constellations::new(100, 30, Motion::LIVELY, &mut rng);
        assert!(sky.fade_step() >= FADE_STEP);
        let mut named = false;
        for _ in 0..(120 * 20) {
            sky.step(0.05, &mut rng);
            let mut frame = Frame::new(100, 30);
            sky.draw(&mut frame, crate::theme::default_theme());
            named |= NAMES.iter().any(|n| frame.to_text().contains(n));
        }
        assert!(named);
        assert!(sky.shown() >= 2, "{}", sky.shown());
    }

    #[test]
    fn lines_follow_their_slope() {
        assert!(line_cells((0, 0), (8, 0)).iter().all(|c| c.2 == '-'));
        assert!(line_cells((0, 0), (0, 5)).iter().all(|c| c.2 == '|'));
        assert!(line_cells((0, 0), (4, 4)).iter().all(|c| c.2 == '\\'));
        assert!(line_cells((0, 4), (4, 0)).iter().all(|c| c.2 == '/'));
        assert_eq!(line_cells((0, 0), (1, 0)).len(), 0);
        // one cell a step, so a line never doubles up on a row or a column
        for (a, b) in [
            ((0, 0), (9, 3)),
            ((2, 7), (14, 1)),
            ((5, 0), (1, 9)),
            ((0, 0), (6, 4)),
        ] {
            let cells = line_cells(a, b);
            let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs()) as usize;
            assert_eq!(cells.len(), steps - 1, "{a:?} {b:?}");
            for pair in cells.windows(2) {
                let dx = (pair[1].0 - pair[0].0).abs();
                let dy = (pair[1].1 - pair[0].1).abs();
                assert!(dx <= 1 && dy <= 1 && dx + dy > 0, "{pair:?}");
            }
        }
        // a sloping line runs along the bottom of its cells between its steps, whichever way
        // it is drawn
        assert!(line_cells((0, 0), (9, 3)).iter().any(|c| c.2 == '_'));
        assert!(line_cells((0, 3), (9, 0)).iter().any(|c| c.2 == '_'));
        let back: Vec<_> = line_cells((9, 3), (0, 0)).into_iter().rev().collect();
        assert_eq!(back, line_cells((0, 0), (9, 3)));
        assert_eq!(line_cells((0, 0), (9, 3))[0].0, 1);
    }

    #[test]
    fn a_small_sky_still_has_a_figure() {
        let mut rng = Rng::new(2);
        let frame = render(
            &mut Constellations::new(20, 8, Motion::CALM, &mut rng),
            20,
            8,
            100,
        );
        assert!(frame.to_text().contains('+') || frame.to_text().contains('*'));
    }
}
