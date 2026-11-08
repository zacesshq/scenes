//! Lanterns: paper lanterns rising slowly from a dark lake into a starry sky, painted in solid
//! pixels. They rise at three depths, swaying a little on the air, each glowing brightest at the
//! flame in its open base and casting a soft halo. Each one leaves a reflection on the water that
//! slides down as it climbs. A few more drift on the surface below a far line of pines.
//!
//! A light theme shows the lanterns at dusk instead: a violet blue sky paling to peach over the
//! pines, each lantern bright painted paper with a pale glowing heart and a soft warm halo.
//!
//! Every colour comes from the active theme. The sky, the pines and the water are tints of the
//! background held below the non-text floor, so they read as background; the lanterns keep the
//! theme's own drawing colours, so they stand out as shapes and keep their outline when colour
//! is off.

use std::f32::consts::TAU;

use super::paint;
use super::{count_for, cycle, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "Glowing paper lanterns rising slowly over a dark lake, with their reflections on the water, at dusk in light themes";

pub const CAPTIONS: &[&str] = &[
    "Every light has lifted off. Build passed.",
    "Let it go. Nothing is wrong.",
    "A clean build, released into the night.",
    "Light and clear all the way up.",
    "No errors to weigh anything down.",
];

/// Seconds for one sway of a lantern from side to side. A sway moves it one column each way, so
/// it changes far less than three times a second.
pub const SWAY_PERIOD: f32 = 7.0;

/// The paper colours a dusk lantern leans towards, so it reads as bright painted paper rather
/// than as a dark stain of a theme colour chosen for text.
const PAPER: [Rgb; 5] = [
    Rgb::new(236, 150, 20),
    Rgb::new(238, 106, 28),
    Rgb::new(220, 52, 44),
    Rgb::new(222, 64, 110),
    Rgb::new(186, 58, 168),
];

/// Pixels a second every far lantern rises at the standard pace. They share one pace, so two
/// far lanterns launched apart stay apart.
const FAR_RISE: f32 = 0.65;

/// Steps in the sky and water ramps.
const SKY_STEPS: usize = 10;
const WATER_STEPS: usize = 8;

/// A near lantern: `d` the dim paper at the top, `g` the glowing paper, `b` the brightest paper
/// near the flame and `f` the flame in its open base.
const NEAR: [&str; 7] = [
    ".ddd.", "dgggd", "dgbgg", "dgbbg", ".gbg.", ".gfg.", "..f..",
];
const MID: [&str; 4] = ["ddd", "gbg", "gbg", ".f."];
/// A lantern floating on the water, seen from the side.
const FLOATING: [&str; 3] = [".d.", "gbg", "ggg"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Depth {
    Far,
    Mid,
    Near,
}

impl Depth {
    fn rows(self) -> &'static [&'static str] {
        match self {
            Depth::Far => &["d", "g"],
            Depth::Mid => &MID,
            Depth::Near => &NEAR,
        }
    }

    /// How far apart, in columns, the starting places of two lanterns at these depths stay.
    /// Lanterns at different paces would catch one another up in the same column, so the nearer
    /// ones each keep a column of sky to themselves, wide enough for their sway and most of
    /// their halo. A far lantern keeps clear of every other lantern too where there is room, so
    /// two small lights never sit side by side like a pair of eyes.
    fn spacing(self, other: Depth) -> f32 {
        match (self, other) {
            (Depth::Far, Depth::Far) => 5.0,
            (Depth::Far, Depth::Near) | (Depth::Near, Depth::Far) => 9.0,
            (Depth::Far, Depth::Mid) | (Depth::Mid, Depth::Far) => 7.0,
            (Depth::Near, Depth::Near) => 10.0,
            (Depth::Mid, Depth::Mid) => 6.0,
            _ => 8.0,
        }
    }

    fn halo(self) -> f32 {
        match self {
            Depth::Far => 1.5,
            Depth::Mid => 3.0,
            Depth::Near => 4.5,
        }
    }
}

#[derive(Clone, Debug)]
struct Lantern {
    x: f32,
    /// The top of the lantern, in pixels.
    y: f32,
    /// Pixels a second.
    rise: f32,
    depth: Depth,
    colour: usize,
    phase: f32,
}

#[derive(Clone, Debug)]
struct Floating {
    x: f32,
    /// Pixels below the shore line.
    row: i32,
    colour: usize,
}

/// The lake's colours in one theme, every one derived from the theme's own. Some are kept only
/// so the tests can check the bands against each other.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct LakeColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub pines: Rgb,
    pub water_top: Rgb,
    pub water_deep: Rgb,
    pub swell: Rgb,
    pub star: Rgb,
    pub faint_star: Rgb,
    pub flame: Rgb,
    /// Each lantern colour as its glowing paper and its dim paper.
    pub lanterns: [(Rgb, Rgb); 5],
    /// The halo each lantern casts in the sky, in the order of `lanterns`.
    pub halos: [Rgb; 5],
    /// The paper nearest the flame: mixed from the glow at night, a pale warm light at dusk.
    pub core: Option<Rgb>,
    halo_strength: f32,
    sky: Ramp,
    water: Ramp,
    stars: bool,
    background: Rgb,
}

impl LakeColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        // the night sky lightens towards the horizon on a dark theme; on a light theme it is a
        // dusk sky that pales towards it; the pines stand dark against either
        let (sky_top, sky_low, pines) = if theme.dark {
            (
                shades.tint(theme.info, 0.12, 1.25),
                shades.tint(theme.blue(), 0.38, 1.9),
                shades.tint(theme.green(), 0.12, 1.12),
            )
        } else {
            (
                shades.tint(mix(theme.info, theme.magenta(), 0.3), 0.75, 2.4),
                shades.tint(theme.warning, 0.4, 1.4),
                shades.tint(theme.green(), 0.85, 2.95),
            )
        };
        let (water_top, water_deep) = if theme.dark {
            (
                shades.tint(theme.blue(), 0.3, 1.6),
                shades.tint(theme.blue(), 0.1, 1.1),
            )
        } else {
            (
                shades.tint(theme.blue(), 0.35, 1.9),
                shades.tint(theme.blue(), 0.75, 2.85),
            )
        };
        let swell = if theme.dark {
            mix(water_top, theme.bright_blue(), 0.3)
        } else {
            mix(water_top, theme.background, 0.6)
        };
        let night = [
            (theme.bright_yellow(), theme.yellow()),
            (theme.yellow(), theme.red()),
            (theme.bright_red(), theme.red()),
            (theme.red(), theme.magenta()),
            (theme.bright_magenta(), theme.magenta()),
        ];
        // at dusk each lantern is painted paper only just solid, so it stays bright, with its
        // top a shade deeper
        let dusk_hues = [
            mix(theme.yellow(), PAPER[0], 0.6),
            mix(theme.warning, PAPER[1], 0.6),
            mix(theme.red(), PAPER[2], 0.6),
            mix(theme.red(), PAPER[3], 0.6),
            mix(theme.magenta(), PAPER[4], 0.6),
        ];
        let (lanterns, halos, core) = if theme.dark {
            (night, night.map(|(glow, _)| glow), None)
        } else {
            let paper = dusk_hues.map(|hue| {
                let glow = paint::strong(theme, hue);
                (glow, paint::stronger(theme, glow, 0.3))
            });
            (
                paper,
                dusk_hues.map(|hue| shades.tint(hue, 0.35, 1.3)),
                Some(shades.tint(theme.yellow(), 0.5, 1.3)),
            )
        };
        Self {
            halo_strength: if theme.dark { 0.35 } else { 0.7 },
            halos,
            core,
            sky_top,
            sky_low,
            pines,
            water_top,
            water_deep,
            swell,
            star: theme.foreground,
            faint_star: theme.muted,
            flame: if theme.dark {
                theme.bright_yellow()
            } else {
                shades.tint(theme.yellow(), 0.3, 1.15)
            },
            lanterns,
            sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
            water: Ramp::new(water_top, water_deep, WATER_STEPS),
            stars: theme.dark,
            background: theme.background,
        }
    }

    /// The colours of one lantern's sprite keys.
    fn lantern(&self, colour: usize) -> [(char, Rgb); 4] {
        let (glow, dim) = self.lanterns[colour % self.lanterns.len()];
        [
            ('d', mix(glow, dim, 0.6)),
            ('g', glow),
            ('b', self.core.unwrap_or_else(|| mix(glow, self.flame, 0.5))),
            ('f', self.flame),
        ]
    }
}

pub struct Lanterns {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    lanterns: Vec<Lantern>,
    floating: Vec<Floating>,
    stars: Vec<(i32, i32, bool)>,
    /// How far the pines reach above the shore in each column, in pixels.
    pines: Vec<i32>,
    swell: Vec<(f32, i32)>,
}

impl Lanterns {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut lake = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            lanterns: Vec::new(),
            floating: Vec::new(),
            stars: Vec::new(),
            pines: Vec::new(),
            swell: Vec::new(),
        };
        lake.resize(width, height, rng);
        lake
    }

    /// The first row of water. The far shore's pines stand on the row above it.
    fn shore(&self) -> i32 {
        ((self.height as f32 * 0.62) as i32).clamp(1, (self.height as i32 - 1).max(1))
    }

    /// The first pixel row of water.
    fn shore_px(&self) -> i32 {
        self.shore() * 2
    }

    fn near_fits(&self) -> bool {
        self.height >= 14 && self.width >= 30
    }

    fn sway_period(&self) -> f32 {
        SWAY_PERIOD / self.motion.pace()
    }

    /// A new lantern, kept clear of every other lantern but the one at `replacing`, which it
    /// takes the place of.
    fn launch(&self, rng: &mut Rng, anywhere: bool, replacing: Option<usize>) -> Lantern {
        let shore = self.shore_px() as f32;
        let mut depth = match rng.below(10) {
            0..=4 => Depth::Far,
            5..=7 => Depth::Mid,
            _ if self.near_fits() => Depth::Near,
            _ => Depth::Mid,
        };
        let y = if anywhere {
            rng.range(-4.0, shore - 2.0)
        } else {
            // a lantern waits behind the pines for a moment before it shows
            shore - 2.0 + rng.range(0.0, 8.0)
        };
        let others: Vec<&Lantern> = self
            .lanterns
            .iter()
            .enumerate()
            .filter(|&(i, _)| Some(i) != replacing)
            .map(|(_, l)| l)
            .collect();
        // a nearer lantern needs a column of sky that no other nearer one rises through; a far
        // one keeps clear of the nearer ones where it can and of the other far ones, which all
        // rise at one pace, by distance, so two never drift together
        let clear = |x: f32, depth: Depth| {
            others.iter().all(|o| {
                let apart = (o.x - x).abs() >= depth.spacing(o.depth);
                match (depth, o.depth) {
                    (Depth::Far, Depth::Far) => apart || (o.y - y).abs() >= 4.0,
                    (Depth::Far, _) => apart,
                    (_, Depth::Far) => true,
                    _ => apart,
                }
            })
        };
        let high = (self.width as f32 - 5.0).max(1.5);
        let mut x = rng.range(1.0, high);
        let free = (0..24)
            .map(|_| rng.range(1.0, high))
            .find(|&x| clear(x, depth));
        match free {
            Some(free) => x = free,
            // no column of sky is free, so a nearer lantern rises far off instead, small enough
            // to pass behind the others
            None if depth != Depth::Far => depth = Depth::Far,
            None => {}
        }
        let rise = match depth {
            Depth::Far => FAR_RISE,
            Depth::Mid => rng.range(0.8, 1.2),
            Depth::Near => rng.range(1.2, 1.8),
        } * self.motion.speed();
        Lantern {
            x,
            y,
            rise,
            depth,
            colour: rng.below(5),
            phase: rng.range(0.0, TAU),
        }
    }

    fn sway(&self, phase: f32) -> f32 {
        (cycle(self.t, TAU / self.sway_period()) + phase).sin()
    }

    fn paint(&self, canvas: &mut Canvas, c: &LakeColours) {
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let shore = self.shore_px().min(tall);

        for y in 0..shore {
            canvas.fill_row(y, c.sky.at(y as f32 / shore.max(1) as f32));
        }
        if c.stars {
            for &(x, y, bright) in &self.stars {
                canvas.set(x, y, if bright { c.star } else { c.faint_star });
            }
        }

        // the water, deepening downwards, with a soft dark reflection of the pines along the
        // shore and a few long swell lines drifting across it
        let span = (tall - shore).max(1) as f32;
        for y in shore..tall {
            let depth = (y - shore) as f32 / span;
            canvas.fill_row(y, c.water.at(depth));
        }
        for x in 0..wide {
            let reach = self.pines.get(x as usize).copied().unwrap_or(0) / 3;
            for dy in 0..reach {
                let y = shore + dy;
                let water = c.water.at(dy as f32 / span);
                canvas.set(x, y, mix(water, c.pines, 0.6));
            }
        }
        for &(x, y) in &self.swell {
            let depth = (y - shore) as f32 / span;
            let colour = mix(c.water.at(depth), c.swell, 0.9 - 0.5 * depth);
            let x = x.round() as i32;
            canvas.fill_rect(x, y, 4, 1, colour);
        }

        // lanterns in the sky, farthest first; each one's reflection sits as far below the
        // shore as the lantern is above it, squeezed by half
        let mut order: Vec<&Lantern> = self.lanterns.iter().collect();
        order.sort_by_key(|l| l.depth as u8);
        for lantern in &order {
            let (_, dim) = c.lanterns[lantern.colour % c.lanterns.len()];
            let rows = lantern.depth.rows();
            let wide_sprite = rows[0].len() as i32;
            let x = (lantern.x + self.sway(lantern.phase)).round() as i32;
            let y = lantern.y.round() as i32;
            let above = shore - 1 - (y + rows.len() as i32);
            // only the nearer lanterns are bright enough to leave a streak on the water
            if above >= 0 && lantern.depth != Depth::Far {
                let ry = shore + above / 2;
                // a broken streak straight down, as light on gently moving water looks
                let (length, across) = match lantern.depth {
                    Depth::Far => (2, 1),
                    Depth::Mid => (5, 1),
                    Depth::Near => (8, 2),
                };
                let left = x + (wide_sprite - across) / 2;
                for dy in (0..length).step_by(2) {
                    let yy = ry + dy;
                    if yy >= tall {
                        break;
                    }
                    let water = c.water.at((yy - shore) as f32 / span);
                    let strength = 0.75 - 0.5 * dy as f32 / length as f32;
                    // a reflection is imagery, so it never reaches the ceiling
                    let colour = paint::held_imagery(c.background, water, dim, strength);
                    canvas.fill_rect(left, yy, across, 1, colour);
                }
            }
        }
        for lantern in order {
            let glow = c.halos[lantern.colour % c.halos.len()];
            let rows = lantern.depth.rows();
            let x = (lantern.x + self.sway(lantern.phase)).round() as i32;
            let y = lantern.y.round() as i32;
            let (cx, cy) = (x + rows[0].len() as i32 / 2, y + rows.len() as i32 * 2 / 3);
            // a soft halo in the sky around the lantern, never on the water
            let r = lantern.depth.halo();
            let reach = r.ceil() as i32;
            for dy in -reach - 2..=reach + 2 {
                for dx in -reach..=reach {
                    let (px, py) = (cx + dx, cy + dy);
                    if py >= shore {
                        continue;
                    }
                    let d = ((dx * dx) as f32 + (dy * dy) as f32 * 0.6).sqrt();
                    if d < r {
                        // the halo fades in four steps, which keeps the number of colours small
                        let amount =
                            (c.halo_strength * (1.0 - d / r).powi(2) * 12.0).round() / 12.0;
                        canvas.blend(px, py, glow, amount);
                    }
                }
            }
            let palette = c.lantern(lantern.colour);
            for (dy, row) in rows.iter().enumerate() {
                let py = y + dy as i32;
                if py >= shore {
                    continue;
                }
                for (dx, ch) in row.chars().enumerate() {
                    if let Some(&(_, colour)) = palette.iter().find(|(k, _)| *k == ch) {
                        canvas.set(x + dx as i32, py, colour);
                    }
                }
            }
        }

        // the far pines, drawn over anything still rising from behind them
        for x in 0..wide {
            let reach = self.pines.get(x as usize).copied().unwrap_or(0);
            for dy in 1..=reach {
                canvas.set(x, shore - dy, c.pines);
            }
        }

        for f in &self.floating {
            let x = f.x.round() as i32;
            let y = shore + f.row;
            let (_, dim) = c.lanterns[f.colour % c.lanterns.len()];
            let palette = c.lantern(f.colour);
            // the lantern's light runs down the water below it
            for dy in (4..9).step_by(2) {
                let yy = y + dy;
                if yy < tall {
                    let water = c.water.at((yy - shore) as f32 / span);
                    let light = 0.8 - 0.07 * dy as f32;
                    canvas.set(
                        x + 1,
                        yy,
                        paint::held_imagery(c.background, water, dim, light),
                    );
                }
            }
            canvas.sprite(x, y, &FLOATING, &palette);
        }
    }
}

impl Scene for Lanterns {
    fn name(&self) -> &'static str {
        "lanterns"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let shore = self.shore();
        let wanted = self
            .motion
            .scaled(count_for(self.width, shore.max(1) as usize, 45, 3, 50));
        self.lanterns.clear();
        for _ in 0..wanted {
            let lantern = self.launch(rng, true, None);
            self.lanterns.push(lantern);
        }
        let water_px = (self.height as i32 * 2 - self.shore_px()).max(1);
        let floating = count_for(self.width, 1, 25, 1, 6);
        self.floating = (0..floating)
            .map(|_| Floating {
                x: rng.range(0.0, self.width as f32),
                row: rng.below((water_px - 6).clamp(1, 4) as usize) as i32,
                colour: rng.below(5),
            })
            .collect();
        let sky = (self.shore_px() - 8).max(1);
        let stars = count_for(self.width, (sky / 2).max(1) as usize, 40, 2, 120);
        self.stars = (0..stars)
            .map(|_| {
                (
                    rng.below(self.width) as i32,
                    rng.below(sky as usize) as i32,
                    rng.chance(0.15),
                )
            })
            .collect();
        // the far shore: a band of pine tops of uneven heights, each a narrow point
        let tallest = (self.shore_px() / 3).clamp(2, 9);
        let mut pines = vec![1; self.width];
        let mut x = 0;
        while x < self.width {
            let h = 1 + rng.below(tallest as usize);
            // each pine narrows by two pixels a column, so it comes to a point
            let half = h / 2;
            let first = x.saturating_sub(half);
            let last = (x + half + 1).min(self.width);
            for (at, column) in pines[first..last].iter_mut().enumerate() {
                let off = (first + at) as i32 - x as i32;
                let reach = (h as i32 - 2 * off.abs()).max(1) + 1;
                *column = (*column).max(reach);
            }
            x += 1 + rng.below(3);
        }
        self.pines = pines;
        let swell = count_for(self.width, (water_px / 2).max(1) as usize, 70, 1, 60);
        self.swell = (0..swell)
            .map(|_| {
                (
                    rng.range(0.0, self.width as f32),
                    self.shore_px() + 2 + rng.below(water_px as usize) as i32,
                )
            })
            .collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        for i in 0..self.lanterns.len() {
            let lantern = &mut self.lanterns[i];
            lantern.y -= lantern.rise * dt;
            if lantern.y < -10.0 {
                self.lanterns[i] = self.launch(rng, false, Some(i));
            }
        }
        let width = self.width as f32;
        let drift = 0.15 * self.motion.speed();
        for f in &mut self.floating {
            f.x = wrap(f.x + drift * dt + 3.0, width + 6.0) - 3.0;
        }
        for s in &mut self.swell {
            s.0 = wrap(s.0 - drift * 1.5 * dt, width);
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = LakeColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colour::xterm_colour;

    use crate::pixels::{IMAGERY_CEILING, distance};
    use crate::scenes::testing::{
        apart, check_determinism, check_pixel_scene, check_scene, pixels, render,
    };
    use crate::terminal::pixel_256;
    use crate::theme;

    fn drawn(lake: &Lanterns, width: usize, height: usize, theme: &ThemeData) -> Frame {
        let mut frame = Frame::new(width, height);
        lake.draw(&mut frame, theme);
        frame
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("lanterns");
        check_determinism("lanterns");
        check_pixel_scene("lanterns", 6.0);
    }

    #[test]
    fn lanterns_sway_slowly_and_rise() {
        const { assert!(SWAY_PERIOD / 2.0 >= 1.0 / 3.0) };
        let mut rng = Rng::new(3);
        let mut lake = Lanterns::new(80, 24, Motion::FULL, &mut rng);
        assert!(lake.sway_period() >= SWAY_PERIOD);
        let before: Vec<f32> = lake.lanterns.iter().map(|l| l.y).collect();
        lake.step(1.0, &mut rng);
        for (lantern, y) in lake.lanterns.iter().zip(before) {
            assert!(lantern.y < y || y < -9.0);
        }
        // the fastest lantern climbs under one row a second, so nothing darts about
        assert!(lake.lanterns.iter().all(|l| l.rise <= 1.8));
    }

    #[test]
    fn the_nearer_lanterns_never_share_a_column() {
        for (w, h) in [(40, 14), (80, 24), (120, 34), (200, 50)] {
            for seed in 0..4 {
                let mut rng = Rng::new(seed);
                let mut lake = Lanterns::new(w, h, Motion::LIVELY, &mut rng);
                for _ in 0..(120 * 20) {
                    lake.step(0.05, &mut rng);
                    let lanterns = &lake.lanterns;
                    for (i, a) in lanterns.iter().enumerate() {
                        for b in &lanterns[i + 1..] {
                            if a.depth == Depth::Far || b.depth == Depth::Far {
                                continue;
                            }
                            assert!(
                                (a.x - b.x).abs() >= a.depth.spacing(b.depth),
                                "two lanterns overlap at {w}x{h}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_shore_pines_and_lanterns_are_drawn() {
        let theme = theme::default_theme();
        let c = LakeColours::new(theme);
        let mut rng = Rng::new(8);
        let mut lake = Lanterns::new(60, 20, Motion::FULL, &mut rng);
        let frame = render(&mut lake, 60, 20, 60);
        let shore = lake.shore();
        // the row above the water is the pines' row, with their dark points against the sky
        let pine_row: Vec<Rgb> = (0..60)
            .filter_map(|x| frame.get(x, shore - 1).and_then(|cell| cell.pixel_pair()))
            .map(|(_, bottom)| bottom)
            .collect();
        assert!(pine_row.iter().all(|&p| p == c.pines));
        let sky: Vec<Rgb> = (0..shore - 1)
            .flat_map(|y| (0..60).map(move |x| (x, y)))
            .filter_map(|(x, y)| frame.get(x, y).and_then(|cell| cell.pixel_pair()))
            .flat_map(|(a, b)| [a, b])
            .collect();
        assert!(sky.contains(&c.flame), "no lantern in the sky");
        let small = render(&mut Lanterns::new(20, 8, Motion::CALM, &mut rng), 20, 8, 20);
        assert!(small.drawn() > 5);
        assert!(!Lanterns::new(20, 8, Motion::CALM, &mut rng).near_fits());
    }

    #[test]
    fn the_colours_stay_apart_and_in_their_bands_in_every_theme() {
        for theme in theme::all() {
            let c = LakeColours::new(theme);
            let name = theme.name;
            apart(
                theme,
                c.sky_top,
                c.sky_low,
                0.03,
                "the sky's top and bottom",
            );
            apart(theme, c.pines, c.sky_low, 0.05, "the pines and the sky");
            apart(theme, c.pines, c.water_top, 0.03, "the pines and the water");
            apart(
                theme,
                c.water_top,
                c.water_deep,
                0.03,
                "the shallow and deep water",
            );
            apart(theme, c.water_top, c.swell, 0.03, "the water and its swell");
            assert!(c.water_deep.luminance() < c.water_top.luminance(), "{name}");
            for (what, colour) in [
                ("sky top", c.sky_top),
                ("sky low", c.sky_low),
                ("pines", c.pines),
                ("water", c.water_top),
                ("deep water", c.water_deep),
            ] {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            // at night the paper keeps the text floor; at dusk it is bright paper only just
            // solid, with a pale heart and flame inside it
            let floor = if theme.dark {
                theme.floor()
            } else {
                IMAGERY_CEILING
            };
            for (glow, dim) in c.lanterns {
                assert!(theme.contrast_of(glow) >= floor, "{name}");
                assert!(theme.contrast_of(dim) >= floor, "{name}");
                apart(theme, glow, c.sky_top, 0.1, "a lantern and the sky");
                apart(theme, glow, c.sky_low, 0.1, "a lantern and the low sky");
                apart(theme, glow, c.water_top, 0.1, "a lantern and the water");
            }
            if theme.dark {
                assert!(theme.contrast_of(c.flame) >= theme.floor(), "{name}");
            } else {
                let core = c.core.expect("a dusk heart");
                for (what, colour) in [("flame", c.flame), ("heart", core)] {
                    assert!(
                        theme.contrast_of(colour) < IMAGERY_CEILING,
                        "the {what} in {name}"
                    );
                }
                for halo in c.halos {
                    assert!(theme.contrast_of(halo) < IMAGERY_CEILING, "{name}");
                    apart(theme, halo, c.sky_top, 0.05, "a halo and the sky");
                }
            }
        }
    }

    #[test]
    fn the_256_colour_fallback_keeps_the_bands_apart() {
        for theme in theme::all() {
            let c = LakeColours::new(theme);
            let near = |colour: Rgb| xterm_colour(pixel_256(colour, theme.background));
            let mut rng = Rng::new(8);
            let mut lake = Lanterns::new(80, 24, Motion::FULL, &mut rng);
            for _ in 0..60 {
                lake.step(0.05, &mut rng);
            }
            let frame = drawn(&lake, 80, 24, theme);
            let worst = pixels(&frame)
                .into_iter()
                .map(|colour| distance(colour, near(colour)))
                .fold(0.0_f32, f32::max);
            assert!(worst <= 0.15, "{} is {worst:.3} off", theme.name);
            assert_ne!(near(c.pines), near(c.sky_low), "{}", theme.name);
            assert_ne!(near(c.water_top), near(c.water_deep), "{}", theme.name);
            for (glow, _) in c.lanterns {
                assert_ne!(near(glow), near(c.sky_low), "{}", theme.name);
            }
        }
    }
}
