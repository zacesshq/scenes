//! Constellations, painted in solid pixels: a night sky that deepens upwards, with a faint band
//! of the galaxy across it and stars of several brightnesses, where one group at a time a few
//! brighter stars are slowly joined by lines into a figure, named, held for a while and then
//! faded away.
//!
//! A light theme shows the same sky at the blue hour, deep blue overhead with the stars pale and
//! white, the figure's lines drawn over them and a dark line of wooded hills along the bottom.
//! The hills close a dark sky too, as a soft silhouette.
//!
//! Every colour comes from the active theme. The sky, the galaxy and the faint stars are tints of
//! the background held below the non-text floor, so they read as background; the figure stars,
//! the lines and the name keep the theme's own drawing colours. The name is text, so it keeps
//! the theme's floor against the sky behind it too.

use super::{count_for, paint};
use crate::colour::Rgb;
use crate::frame::{Cell, Frame};
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "Stars in a painted night sky over wooded hills, joined into named figures one group at a time, at the blue hour in light themes";

pub const CAPTIONS: &[&str] = &[
    "Everything connects.",
    "A clean build, written in the stars.",
    "Every point joined up.",
    "The sky checks out.",
    "Nothing out of place up there.",
];

/// Pixels a second a line is drawn at, at the standard speed.
const DRAW_SPEED: f32 = 9.0;
/// Seconds a finished figure is held before it fades.
const HOLD: f32 = 6.0;
/// Seconds between the steps of a fade. A line changes colour once a step, so a pixel changes at
/// most once in this long.
pub const FADE_STEP: f32 = 1.2;
/// Seconds of plain sky between two figures.
const REST: f32 = 2.5;
/// Steps in the sky ramp.
const SKY_STEPS: usize = 8;

/// Names made up for the figures.
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
    /// Star positions in pixels.
    stars: Vec<(i32, i32)>,
    /// Pairs of star indices joined by a line.
    lines: Vec<(usize, usize)>,
    name: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    /// Lines being drawn: pixels drawn so far.
    Drawing(f32),
    /// The whole figure shown, for seconds so far.
    Holding(f32),
    /// Fading, for seconds so far.
    Fading(f32),
    /// Plain sky, for seconds so far.
    Resting(f32),
}

/// The pixels of a straight line between two stars, one pixel a step, from `a` to `b` with the
/// stars themselves left out.
fn line_pixels(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let steps = dx.abs().max(dy.abs());
    (1..steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            (
                (a.0 as f32 + dx as f32 * t).round() as i32,
                (a.1 as f32 + dy as f32 * t).round() as i32,
            )
        })
        .collect()
}

/// The sky's colours in one theme, every one derived from the theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct SkyColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub galaxy: Rgb,
    pub dim_star: Rgb,
    pub faint_star: Rgb,
    pub star: Rgb,
    pub glow: Rgb,
    pub figure_star: Rgb,
    pub drawing: Rgb,
    pub held: Rgb,
    pub fading: Rgb,
    pub hill: Rgb,
    sky: Ramp,
}

/// Pure white, the stars at the blue hour: a touch above an off-white background, so they stay
/// imagery while showing against the deep blue.
const WHITE: Rgb = Rgb::new(255, 255, 255);

impl SkyColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        // a dark sky lifts towards the horizon; a light one is a deep blue overhead that pales
        // towards the horizon, like the last light of the evening
        let (sky_top, sky_low) = if theme.dark {
            (
                shades.tint(theme.info, 0.1, 1.12),
                shades.tint(theme.info, 0.35, 1.5),
            )
        } else {
            (
                shades.tint(theme.info, 0.9, 2.6),
                shades.tint(mix(theme.info, theme.magenta(), 0.3), 0.3, 1.4),
            )
        };
        // the galaxy is a paler band in either case: lit dust on a dark sky, thin haze on a
        // deep blue one
        let galaxy_hue = mix(theme.info, theme.magenta(), 0.35);
        let galaxy = if theme.dark {
            shades.tint(galaxy_hue, 0.3, 1.4)
        } else {
            mix(sky_top, mix(WHITE, galaxy_hue, 0.2), 0.25)
        };
        let faint_star = if theme.dark {
            shades.tint(shades.lighter(sky_top, 0.5), 1.0, IMAGERY_CEILING - 0.2)
        } else {
            mix(WHITE, sky_top, 0.45)
        };
        // at night the stars are the text colour's own light; at the blue hour they are white
        // imagery and the hills carry the outline
        let (star, figure_star, hill) = if theme.dark {
            (
                theme.foreground,
                theme.bright_yellow(),
                mix(theme.background, theme.green(), 0.05),
            )
        } else {
            (
                WHITE,
                mix(WHITE, theme.yellow(), 0.15),
                paint::strong(
                    theme,
                    shades.tint(mix(theme.info, theme.green(), 0.5), 1.0, 2.9),
                ),
            )
        };
        Self {
            sky_top,
            sky_low,
            galaxy,
            dim_star: mix(sky_top, faint_star, 0.5),
            faint_star,
            star,
            glow: mix(sky_top, star, 0.3),
            hill,
            figure_star,
            drawing: theme.cyan(),
            held: theme.bright_cyan(),
            fading: theme.grey(),
            sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
        }
    }

    /// The plain sky at a pixel row out of `tall`.
    fn sky_at(&self, y: i32, tall: i32) -> Rgb {
        self.sky.at((y as f32 / tall.max(1) as f32).powf(1.4))
    }
}

pub struct Constellations {
    width: usize,
    height: usize,
    motion: Motion,
    /// Faint stars by pixel and brightness, 0 for the dimmest.
    faint: Vec<(i32, i32, u8)>,
    figures: Vec<Figure>,
    current: usize,
    stage: Stage,
    shown: u32,
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

    /// A figure with its stars in a box of pixels.
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
            // stars too close together would merge into one blob of light
            if stars
                .iter()
                .all(|s| (s.0 - p.0).abs() >= 5 || (s.1 - p.1).abs() >= 5)
            {
                stars.push(p);
            }
        }
        // each new star joins the nearest one already in the figure, which keeps it one shape
        let mut lines = Vec::new();
        for i in 1..stars.len() {
            let nearest = (0..i)
                .min_by_key(|&j| {
                    let dx = stars[i].0 - stars[j].0;
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
            .map(|&(a, b)| line_pixels(figure.stars[a], figure.stars[b]).len())
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

    /// How many cell rows the hills along the bottom take, none on a short screen.
    fn hill_rows(&self) -> i32 {
        if self.height >= 12 {
            (self.height as i32 / 7).max(2)
        } else {
            0
        }
    }

    /// A low line of wooded hills along the bottom, with a small pine here and there on the
    /// crest.
    fn paint_hills(&self, canvas: &mut Canvas, c: &SkyColours) {
        let rows = self.hill_rows();
        if rows == 0 {
            return;
        }
        let tall = canvas.height() as i32;
        let line = (tall - rows * 2) as f32 + 2.0;
        for x in 0..canvas.width() as i32 {
            let xf = x as f32;
            let swell = 1.6 * (xf / 11.0 + 0.7).sin() + 0.9 * (xf / 4.3 + 2.1).sin();
            let mut top = (line + swell).round() as i32;
            let pine = |at: i32| paint::hash(at, 0, 23) % 7 == 0;
            if pine(x) {
                top -= 4;
            } else if pine(x - 1) || pine(x + 1) {
                top -= 2;
            }
            canvas.fill_rect(x, top, 1, tall - top, c.hill);
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &SkyColours) {
        let (w, h) = (canvas.width() as i32, canvas.height() as i32);
        for y in 0..h {
            canvas.fill_row(y, c.sky_at(y, h));
        }
        // a faint band of the galaxy, thinning at its edges and broken up by grain
        for y in 0..h {
            for x in 0..w {
                let along = (x as f32 * 0.5 + y as f32 * 0.8) / (w.max(1) as f32 * 0.5) - 0.9;
                let band = (-along * along * 14.0).exp();
                let hash =
                    (x as u32).wrapping_mul(0x9e37_79b1) ^ (y as u32).wrapping_mul(0x85eb_ca77);
                let grain = ((hash >> 13) % 100) as f32 / 100.0;
                let level = band * (0.85 + 0.25 * grain) - 0.35;
                if level > 0.0 {
                    let base = c.sky_at(y, h);
                    let step = ((level * 1.6).min(1.0) * 3.0).round() / 3.0;
                    canvas.set(x, y, mix(base, c.galaxy, step));
                }
            }
        }
        for &(x, y, level) in &self.faint {
            canvas.set(x, y, if level == 0 { c.dim_star } else { c.faint_star });
        }
        // the stars of every figure stay in the sky, so a figure is joined from stars already
        // seen; each has a soft cross of glow around it
        for figure in &self.figures {
            for &(x, y) in &figure.stars {
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    canvas.set(x + dx, y + dy, c.glow);
                }
                canvas.set(x, y, c.star);
            }
        }
        self.paint_hills(canvas, c);
        let Some(figure) = self.figures.get(self.current) else {
            return;
        };
        let fade = self.fade_step();
        let (line_colour, drawn) = match self.stage {
            Stage::Drawing(done) => (Some(c.drawing), done as usize),
            Stage::Holding(_) => (Some(c.held), usize::MAX),
            Stage::Fading(t) if t < fade => (Some(c.drawing), usize::MAX),
            Stage::Fading(t) if t < 2.0 * fade => (Some(c.fading), usize::MAX),
            Stage::Fading(_) | Stage::Resting(_) => (None, 0),
        };
        let Some(colour) = line_colour else {
            return;
        };
        let mut left = drawn;
        'lines: for &(a, b) in &figure.lines {
            let (from, to) = (figure.stars[a], figure.stars[b]);
            for (x, y) in line_pixels(from, to) {
                if left == 0 {
                    break 'lines;
                }
                left -= 1;
                // the line stops short of each star, so the star keeps its own shape
                let near = |s: (i32, i32)| (s.0 - x).abs() <= 1 && (s.1 - y).abs() <= 1;
                if !near(from) && !near(to) {
                    canvas.set(x, y, colour);
                }
            }
        }
        for &(x, y) in &figure.stars {
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                canvas.set(x + dx, y + dy, mix(c.figure_star, c.sky_top, 0.45));
            }
            canvas.set(x, y, c.figure_star);
        }
    }

    /// Writes the current figure's name as text below it where there is room and above it
    /// otherwise, on the sky behind it when the theme's floor holds there.
    fn label(&self, frame: &mut Frame, canvas: &Canvas, theme: &ThemeData, c: &SkyColours) {
        let Some(figure) = self.figures.get(self.current) else {
            return;
        };
        let fade = self.fade_step();
        let wanted = match self.stage {
            Stage::Holding(_) => theme.foreground,
            Stage::Drawing(_) => theme.muted,
            Stage::Fading(t) if t < 2.0 * fade => theme.muted,
            _ => return,
        };
        let length = figure.name.len() as i32;
        if length > self.width as i32 || figure.stars.is_empty() {
            return;
        }
        let lowest = figure.stars.iter().map(|s| s.1).max().unwrap_or(0) / 2;
        let highest = figure.stars.iter().map(|s| s.1).min().unwrap_or(0) / 2;
        let row = if lowest + 1 < self.height as i32 {
            lowest + 1
        } else {
            highest - 1
        };
        if row < 0 {
            return;
        }
        let centre =
            figure.stars.iter().map(|s| s.0).sum::<i32>() / figure.stars.len().max(1) as i32;
        let left = (centre - length / 2).clamp(0, self.width as i32 - length);
        let tall = canvas.height() as i32;
        let sky = c.sky_at(2 * row + 1, tall);
        let floor = theme.floor();
        // the name sits on the plain sky when the text keeps the floor there, switching to the
        // stronger text colour if that is what it takes; failing both it sits on the theme's
        // own background
        let (colour, back) = if wanted.contrast(sky) >= floor {
            (wanted, sky)
        } else if theme.foreground.contrast(sky) >= floor {
            (theme.foreground, sky)
        } else {
            (wanted, theme.background)
        };
        for (i, ch) in figure.name.chars().enumerate() {
            frame.put_cell(
                left + i as i32,
                row,
                Cell {
                    ch,
                    colour: (ch != ' ').then_some(colour),
                    bg: Some(back),
                },
            );
        }
    }
}

impl Scene for Constellations {
    fn name(&self) -> &'static str {
        "constellations"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let tall = self.height * 2;
        let wanted = self
            .motion
            .scaled(count_for(self.width, self.height, 14, 6, 600));
        self.faint = (0..wanted)
            .map(|_| {
                (
                    rng.below(self.width) as i32,
                    rng.below(tall) as i32,
                    u8::from(rng.chance(0.35)),
                )
            })
            .collect();
        // the sky is split into boxes of cells, one figure each, so no two figures overlap;
        // each box keeps its bottom row free for the name
        let box_w = (self.width as i32 / 3).clamp(10, 36).min(self.width as i32);
        // the figures keep clear of the hills along the bottom
        let open = self.height as i32 - self.hill_rows();
        let box_h = (open / 2).clamp(5, 14).min(open.max(1));
        self.figures.clear();
        let mut top = 0;
        while top + box_h <= open {
            let mut left = 0;
            while left + box_w <= self.width as i32 {
                self.figures.push(Self::make_figure(
                    rng,
                    left + 1,
                    2 * top + 2,
                    box_w - 2,
                    2 * (box_h - 2) - 2,
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
                (2 * self.height as i32 - 2).max(1),
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
        let colours = SkyColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
        self.label(frame, &canvas, theme, &colours);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::pixels::distance;
    use crate::scenes::testing::{
        check_determinism, check_pixel_scene, check_scene, pixels, render,
    };
    use crate::theme;

    fn drawn(sky: &Constellations, width: usize, height: usize, theme: &ThemeData) -> Frame {
        let mut frame = Frame::new(width, height);
        sky.draw(&mut frame, theme);
        frame
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("constellations");
        check_determinism("constellations");
        check_pixel_scene("constellations", 6.0);
    }

    #[test]
    fn figures_are_drawn_named_and_faded_one_at_a_time() {
        let mut rng = Rng::new(6);
        let mut sky = Constellations::new(100, 30, Motion::LIVELY, &mut rng);
        assert!(sky.fade_step() >= FADE_STEP);
        let mut named = false;
        for _ in 0..(120 * 20) {
            sky.step(0.05, &mut rng);
            let frame = drawn(&sky, 100, 30, theme::default_theme());
            named |= NAMES.iter().any(|n| frame.to_text().contains(n));
        }
        assert!(named);
        assert!(sky.shown() >= 2, "{}", sky.shown());
    }

    #[test]
    fn lines_are_one_pixel_a_step_and_leave_the_stars_out() {
        assert_eq!(line_pixels((0, 0), (1, 0)).len(), 0);
        assert!(line_pixels((0, 0), (8, 0)).iter().all(|p| p.1 == 0));
        assert!(line_pixels((0, 0), (0, 5)).iter().all(|p| p.0 == 0));
        for (a, b) in [
            ((0, 0), (9, 3)),
            ((2, 7), (14, 1)),
            ((5, 0), (1, 9)),
            ((0, 0), (6, 4)),
        ] {
            let pixels = line_pixels(a, b);
            let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs()) as usize;
            assert_eq!(pixels.len(), steps - 1, "{a:?} {b:?}");
            assert!(!pixels.contains(&a) && !pixels.contains(&b));
            for pair in pixels.windows(2) {
                let dx = (pair[1].0 - pair[0].0).abs();
                let dy = (pair[1].1 - pair[0].1).abs();
                assert!(dx <= 1 && dy <= 1 && dx + dy > 0, "{pair:?}");
            }
        }
    }

    #[test]
    fn the_name_keeps_the_floor_on_the_sky_in_every_theme() {
        for theme in theme::all() {
            let mut rng = Rng::new(6);
            let mut sky = Constellations::new(100, 30, Motion::FULL, &mut rng);
            let mut named = 0;
            for step in 0..(40 * 20) {
                sky.step(0.05, &mut rng);
                if step % 10 != 0 {
                    continue;
                }
                let frame = drawn(&sky, 100, 30, theme);
                for cell in frame.cells().iter().filter(|c| !c.is_pixel()) {
                    let fg = cell.colour.unwrap_or(theme.foreground);
                    let bg = cell.bg.unwrap_or(theme.background);
                    assert!(fg.contrast(bg) >= theme.floor(), "{}", theme.name);
                    named += 1;
                }
            }
            assert!(named > 0, "no name in {}", theme.name);
        }
    }

    #[test]
    fn the_colours_stay_distinct_and_in_bounds_in_every_theme() {
        for theme in theme::all() {
            let c = SkyColours::new(theme);
            let name = theme.name;
            for (what, colour) in [
                ("sky top", c.sky_top),
                ("sky low", c.sky_low),
                ("galaxy", c.galaxy),
                ("faint star", c.faint_star),
            ] {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            assert!(distance(c.sky_top, c.sky_low) >= 0.03, "{name}");
            assert!(distance(c.faint_star, c.sky_top) >= 0.05, "{name}");
            assert!(distance(c.galaxy, c.sky_top) >= 0.02, "{name}");
            // at night every star and line keeps the floor; at the blue hour the stars are
            // white imagery and the lines and the hills keep it
            let solid = if theme.dark {
                vec![c.star, c.figure_star, c.drawing, c.held, c.fading]
            } else {
                for colour in [c.star, c.figure_star] {
                    assert!(theme.contrast_of(colour) < IMAGERY_CEILING, "{name}");
                    assert!(distance(colour, c.sky_top) >= 0.1, "{name}");
                }
                assert!(theme.contrast_of(c.hill) >= IMAGERY_CEILING, "{name}");
                vec![c.drawing, c.held, c.fading]
            };
            for colour in solid {
                assert!(theme.contrast_of(colour) >= theme.floor(), "{name}");
                assert!(distance(colour, c.sky_top) >= 0.1, "{name}");
                assert!(distance(colour, c.sky_low) >= 0.1, "{name}");
            }
            // the held lines and the fading ones read as different steps
            assert!(distance(c.held, c.fading) >= 0.05, "{name}");
        }
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
        let c = SkyColours::new(theme::default_theme());
        let all = pixels(&frame);
        assert!(all.contains(&c.star) || all.contains(&c.figure_star));
    }
}
