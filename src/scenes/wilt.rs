//! Wilt: potted plants on a windowsill in front of a window full of hard, hot sun. Over the
//! first minute their stems bend and droop until the flowers hang over the rims, the green dries
//! to straw from the tips down, the leaves fall one by one onto the sill and the soil in each
//! pot cracks.
//!
//! The wall, the window and the sill are tints of the background held below the non-text floor.
//! The pots, the stems, the leaves and the sun are solid, so with colour off the drooping shapes
//! still read.

use std::f32::consts::TAU;

use super::cycle;
use super::paint::{self, ease, gradient, shaded, solid, unit};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// Seconds the plants take to wilt at the standard pace.
pub const WILT_SECONDS: f32 = 50.0;
/// Seconds of one slow sway of the drooping stems once they have wilted.
pub const SWAY_PERIOD: f32 = 9.0;
/// How far the furthest bend reaches from upright, in radians.
const MAX_BEND: f32 = 2.6;
/// Pixels a second a falling leaf drops.
const FALL_SPEED: f32 = 3.0;
/// A pot's width at its rim and its height, in pixels.
const POT_WIDTH: i32 = 9;
const POT_HEIGHT: i32 = 6;
/// The pixel rows of the sill under the pots.
const SILL_ROWS: i32 = 3;

pub const SUMMARY: &str =
    "Potted plants on a sunny sill whose stems droop and dry, leaves fall and soil cracks";

pub const CAPTIONS: &[&str] = &[
    "The build needs watering. Start with the first error.",
    "Drooping, not dead. One fix perks it up.",
    "Dry soil. A little care brings it back.",
    "Every plant wilts sometimes. So does every build.",
    "Prune the first error and see what grows.",
];

#[derive(Clone, Debug)]
struct Leaf {
    /// How far along the stem it grows, from 0 to 1.
    at: f32,
    right: bool,
    /// How wilted the plant is when this leaf lets go.
    falls_at: f32,
    /// Where it is once it has let go. `landed` says whether it has come to rest.
    falling: Option<(f32, f32)>,
    landed: bool,
}

#[derive(Clone, Debug)]
struct Plant {
    x: i32,
    /// The stem's length in pixels.
    length: usize,
    /// 1 to droop to the right, -1 to the left.
    side: f32,
    /// How far this plant bends compared with the others.
    droop: f32,
    leaves: Vec<Leaf>,
    /// When each crack in the soil opens, by how wilted the plant is.
    cracks: Vec<f32>,
    flower: usize,
}

/// The windowsill's colours in one theme.
struct Colours {
    wall: Ramp,
    sky: Ramp,
    glow: Rgb,
    frame: Rgb,
    sill: (Rgb, Rgb, Rgb),
    sun: Rgb,
    /// The heart of each fresh flower.
    centre: Rgb,
    pot: (Rgb, Rgb, Rgb),
    soil: Rgb,
    crack: Rgb,
    /// From fresh green to dry straw.
    stem: Ramp,
    flowers: [Rgb; 4],
    dead: Rgb,
}

impl Colours {
    fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let warm = mix(theme.warning, theme.foreground, 0.35);
        let (wall_top, wall_low, sky_top, sky_low, frame, sill) = if theme.dark {
            (
                shades.tint(theme.muted, 0.1, 1.2),
                shades.tint(warm, 0.18, 1.4),
                shades.tint(warm, 0.42, 2.2),
                shades.tint(warm, 0.62, 2.8),
                shades.tint(theme.muted, 0.3, 1.75),
                shades.tint(warm, 0.38, 2.1),
            )
        } else {
            (
                shades.tint(theme.muted, 0.3, 1.6),
                shades.tint(warm, 0.35, 1.8),
                shades.tint(warm, 0.18, 1.25),
                shades.tint(warm, 0.06, 1.08),
                shades.tint(theme.muted, 0.6, 2.5),
                shades.tint(warm, 0.55, 2.4),
            )
        };
        let straw = solid(
            theme,
            mix(theme.yellow(), theme.muted, 0.4),
            shades.darkest(),
            0.2,
        );
        let (_, sill_lit, _) = (0, mix(sill, frame, 0.4), 0);
        Self {
            wall: Ramp::new(wall_top, wall_low, 6),
            sky: Ramp::new(sky_top, sky_low, 8),
            // on a light theme the sun is a white glare in a warm glow, never a dark disc
            glow: if theme.dark {
                shades.tint(theme.bright_yellow(), 0.7, 2.95)
            } else {
                shades.tint(theme.warning, 0.4, 1.6)
            },
            frame,
            sill: (mix(sill, wall_top, 0.5), sill, sill_lit),
            sun: if theme.dark {
                theme.bright_yellow()
            } else {
                theme.background
            },
            centre: theme.bright_yellow(),
            pot: shaded(theme, theme.red()),
            soil: shades.tint(mix(theme.red(), theme.muted, 0.5), 0.3, 1.6),
            crack: straw,
            stem: Ramp::new(theme.green(), straw, 5),
            flowers: [theme.magenta(), theme.yellow(), theme.red(), theme.cyan()],
            dead: solid(theme, theme.muted, shades.darkest(), 0.35),
        }
    }
}

pub struct Wilt {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// Seconds of wilting so far, kept across a resize so the plants never revive.
    wilted: f32,
    plants: Vec<Plant>,
}

impl Wilt {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut wilt = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            wilted: 0.0,
            plants: Vec::new(),
        };
        wilt.resize(width, height, rng);
        wilt
    }

    fn amount(&self) -> f32 {
        ease(self.wilted / WILT_SECONDS)
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The first pixel row of the sill the pots stand on.
    fn sill(&self) -> i32 {
        (self.tall() - SILL_ROWS).max(1)
    }

    /// The pixel row of a pot's rim, which holds its soil.
    fn rim(&self) -> i32 {
        self.sill() - POT_HEIGHT
    }

    /// The window's top and bottom pixel rows.
    fn window(&self) -> (i32, i32) {
        (2, (self.sill() - 3).max(3))
    }

    /// Points along a stem from the soil to its tip, in half pixel steps so a bent stem never
    /// skips a pixel. Each point carries the stem's heading there.
    fn stem(&self, plant: &Plant) -> Vec<(f32, f32, f32)> {
        let amount = self.amount();
        let sway = 0.08 * amount * cycle(self.t, TAU / SWAY_PERIOD * self.motion.pace()).sin();
        let steps = plant.length * 2;
        let mut x = plant.x as f32;
        let mut y = (self.rim() - 1) as f32;
        let mut points = vec![(x, y, 0.0)];
        for k in 1..=steps {
            let s = k as f32 / steps as f32;
            let bend = plant.side * (MAX_BEND * plant.droop * amount * s * s + sway * s);
            x += bend.sin() * 0.5;
            y -= bend.cos() * 0.5;
            points.push((x, y, bend));
        }
        points
    }

    /// How dry a point along a stem is, from 0 to 1: the tip dries first.
    fn dryness(&self, along: f32) -> f32 {
        (self.amount() * 1.7 - (1.0 - along) * 0.7).clamp(0.0, 1.0)
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let sill = self.sill();
        let rim = self.rim();
        gradient(canvas, &c.wall, 0, sill);

        // the window: a hot pale sky behind two panes, the sun high in one corner
        let (win_top, win_bottom) = self.window();
        let (left, right) = ((wide / 10).max(1), (wide - wide / 10 - 1).max(2));
        if win_bottom - win_top >= 6 && right - left >= 10 {
            let span = (win_bottom - win_top).max(1) as f32;
            for y in win_top..win_bottom {
                let colour = c.sky.at((y - win_top) as f32 / span);
                for x in left..=right {
                    canvas.set(x, y, colour);
                }
            }
            let (sx, sy) = (
                right - (right - left) / 5,
                win_top + (win_bottom - win_top) / 3,
            );
            let r = ((win_bottom - win_top) as f32 / 7.0).clamp(1.5, 5.0);
            paint::disc(canvas, sx as f32, sy as f32, r + 2.0, c.glow);
            paint::disc(canvas, sx as f32, sy as f32, r, c.sun);
            // the frame and its glazing bars
            for x in left - 1..=right + 1 {
                canvas.set(x, win_top - 1, c.frame);
                canvas.set(x, win_bottom, c.frame);
            }
            let middle = (left + right) / 2;
            for y in win_top - 1..=win_bottom {
                for x in [left - 1, middle, right + 1] {
                    canvas.set(x, y, c.frame);
                }
            }
            for x in left..=right {
                canvas.set(x, (win_top + win_bottom) / 2, c.frame);
            }
        }

        // the sill: a lit front edge, the top and a shadow under it
        let (shadow, top, lit) = c.sill;
        for y in sill..tall {
            let colour = match y - sill {
                0 => lit,
                1 => top,
                _ => shadow,
            };
            canvas.fill_row(y, colour);
        }

        for plant in &self.plants {
            let stem = self.stem(plant);
            let last = stem.len().max(2) - 1;
            for (i, &(x, y, _)) in stem.iter().enumerate() {
                let along = i as f32 / last as f32;
                let colour = c.stem.at(self.dryness(along));
                if (y.round() as i32) < rim {
                    canvas.set(x.round() as i32, y.round() as i32, colour);
                    // the lower half of the stem is two pixels thick
                    if along < 0.5 {
                        canvas.set(x.round() as i32 + 1, y.round() as i32, colour);
                    }
                }
            }
            for leaf in &plant.leaves {
                if leaf.falling.is_some() || leaf.landed {
                    continue;
                }
                let i = ((stem.len() - 1) as f32 * leaf.at) as usize;
                let (x, y, _) = stem[i];
                let dry = self.dryness(leaf.at);
                let colour = c.stem.at(dry);
                let dx = if leaf.right { 1 } else { -1 };
                // a fresh leaf reaches up and out; a dry one hangs
                let lift = if dry < 0.5 { -1 } else { 1 };
                let (x, y) = (x.round() as i32, y.round() as i32);
                for (px, py) in [(x + dx, y), (x + 2 * dx, y + lift), (x + 3 * dx, y + lift)] {
                    if py < rim {
                        canvas.set(px, py, colour);
                    }
                }
                if y < rim && dry < 0.5 {
                    canvas.set(x + 2 * dx, y, colour);
                }
            }
            // the flower at the tip fades from a bloom to a dry head as it droops
            let (tx, ty, _) = stem[last];
            let (tx, ty) = (tx.round() as i32, ty.round() as i32);
            let amount = self.amount();
            let petal = if amount < 0.55 {
                c.flowers[plant.flower % c.flowers.len()]
            } else {
                c.dead
            };
            let centre = if amount < 0.8 { c.centre } else { c.dead };
            if ty < rim - 1 {
                let petals: &[(i32, i32)] = if amount < 0.35 {
                    &[
                        (0, -1),
                        (-1, 0),
                        (1, 0),
                        (0, 1),
                        (-1, -1),
                        (1, -1),
                        (-1, 1),
                        (1, 1),
                    ]
                } else if amount < 0.75 {
                    &[(-1, 0), (1, 0), (0, 1), (-1, 1), (1, 1)]
                } else {
                    &[(0, 1)]
                };
                for &(dx, dy) in petals {
                    canvas.set(tx + dx, ty + dy, petal);
                }
                canvas.set(tx, ty, centre);
            }

            // the pot: a rim a pixel wider than the body, which narrows towards its foot
            let (dark, body, light) = c.pot;
            let left = plant.x - POT_WIDTH / 2;
            for x in left - 1..=left + POT_WIDTH {
                canvas.set(x, rim, light);
                canvas.set(x, rim + 1, body);
            }
            for row in 2..POT_HEIGHT {
                let inset = row / 3;
                for x in left + inset..left + POT_WIDTH - inset {
                    let colour = if x == left + inset {
                        light
                    } else if x >= left + POT_WIDTH - inset - 2 {
                        dark
                    } else {
                        body
                    };
                    canvas.set(x, rim + row, colour);
                }
            }
            // the soil inside the rim, cracking open as it dries
            for (i, &opens) in plant.cracks.iter().enumerate() {
                let x = left + 1 + i as i32;
                let colour = if amount >= opens { c.crack } else { c.soil };
                canvas.set(x, rim, colour);
            }
            canvas.set(plant.x, rim, c.stem.at(self.dryness(0.0)));
        }
        // leaves on their way down and on the sill
        for plant in &self.plants {
            for leaf in &plant.leaves {
                if let Some((x, y)) = leaf.falling {
                    let colour = c.stem.at(1.0);
                    let (x, y) = (x.round() as i32, y.round() as i32);
                    canvas.set(x, y, colour);
                    canvas.set(x + 1, y, colour);
                }
            }
        }
    }
}

impl Scene for Wilt {
    fn name(&self) -> &'static str {
        "wilt"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let gap = if self.width < 40 { 10 } else { 16 };
        let count = ((self.width as i32 - 2) / gap).max(1) as usize;
        let room = (self.rim() - 4).max(2) as usize;
        let tallest = ((room as f32 * 0.8) as usize).clamp(3, 34);
        let spare = self.width as i32 - gap * count as i32;
        self.plants = (0..count)
            .map(|i| {
                let length = tallest - rng.below((tallest / 3).max(1));
                let first = rng.below(3);
                let leaves = (3..length.saturating_sub(2))
                    .skip(first)
                    .step_by(3)
                    .enumerate()
                    .map(|(n, k)| Leaf {
                        at: k as f32 / length as f32,
                        right: n % 2 == 0,
                        falls_at: rng.range(0.35, 0.95),
                        falling: None,
                        landed: false,
                    })
                    .collect();
                Plant {
                    x: spare / 2 + gap * i as i32 + gap / 2,
                    length,
                    side: if rng.chance(0.5) { 1.0 } else { -1.0 },
                    droop: rng.range(0.8, 1.05),
                    leaves,
                    cracks: (0..POT_WIDTH - 2)
                        .map(|k| {
                            if unit(k, i as i32, 0x0c1a) < 0.4 {
                                rng.range(0.25, 1.0)
                            } else {
                                2.0
                            }
                        })
                        .collect(),
                    flower: i,
                }
            })
            .collect();
        // leaves that have already fallen land again on the new sill at once
        let amount = self.amount();
        let floor = (self.sill() - 1) as f32;
        for plant in &mut self.plants {
            for leaf in &mut plant.leaves {
                if amount >= leaf.falls_at {
                    leaf.landed = true;
                    leaf.falling = Some((plant.x as f32 + rng.range(-7.0, 7.0), floor));
                }
            }
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        self.wilted = (self.wilted + dt * speed).min(WILT_SECONDS * 2.0);
        let amount = self.amount();
        let floor = (self.sill() - 1) as f32;
        let stems: Vec<Vec<(f32, f32, f32)>> = self.plants.iter().map(|p| self.stem(p)).collect();
        for (plant, stem) in self.plants.iter_mut().zip(stems) {
            for leaf in &mut plant.leaves {
                if leaf.landed {
                    continue;
                }
                match &mut leaf.falling {
                    None if amount >= leaf.falls_at => {
                        let i = ((stem.len() - 1) as f32 * leaf.at) as usize;
                        let (x, y, _) = stem[i];
                        leaf.falling = Some((x + if leaf.right { 2.0 } else { -3.0 }, y));
                    }
                    Some((x, y)) => {
                        *y += FALL_SPEED * speed * dt;
                        *x += rng.range(-1.2, 1.2) * speed * dt;
                        if *y >= floor {
                            *y = floor;
                            leaf.landed = true;
                        }
                    }
                    None => {}
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = Colours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixels::IMAGERY_CEILING;
    use crate::scenes::testing::{apart, check_pixel_scene, pixels};
    use crate::scenes::testing::{check_determinism, check_scene, render};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("wilt");
        check_determinism("wilt");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("wilt", 40.0);
    }

    #[test]
    fn stems_droop_leaves_fall_and_land_on_the_sill() {
        let theme = theme::default_theme();
        let c = Colours::new(theme);
        let mut rng = Rng::new(10);
        let mut wilt = Wilt::new(80, 24, Motion::FULL, &mut rng);
        let tips: Vec<f32> = wilt
            .plants
            .iter()
            .map(|p| wilt.stem(p).last().unwrap().1)
            .collect();
        let fresh = pixels(&render(&mut wilt, 80, 24, 1));
        assert!(fresh.contains(&c.flowers[0]) && fresh.contains(&c.stem.at(0.0)));
        for _ in 0..(3600 * 20) {
            wilt.step(0.05, &mut rng);
        }
        for (plant, tip) in wilt.plants.iter().zip(tips) {
            let now = wilt.stem(plant).last().unwrap().1;
            assert!(now > tip + 4.0, "the tip only moved from {tip} to {now}");
            for leaf in &plant.leaves {
                assert!(leaf.landed);
                let (x, y) = leaf.falling.expect("fallen");
                assert_eq!(y, (wilt.sill() - 1) as f32);
                assert!(x > -4.0 && x < 84.0);
            }
        }
        let late = pixels(&render(&mut wilt, 80, 24, 1));
        assert!(late.contains(&c.crack) && late.contains(&c.dead));
        assert!(!late.contains(&c.stem.at(0.0)), "every stem has dried");
        const { assert!(SWAY_PERIOD / 4.0 > 1.0 / 3.0) };
    }

    #[test]
    fn the_layers_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            apart(
                theme,
                c.sky.at(0.0),
                c.wall.at(0.0),
                0.03,
                "the window and the wall",
            );
            apart(
                theme,
                c.sill.1,
                c.wall.at(1.0),
                0.03,
                "the sill and the wall",
            );
            apart(
                theme,
                c.stem.at(0.0),
                c.stem.at(1.0),
                0.06,
                "fresh and dry stems",
            );
            apart(theme, c.pot.1, c.sill.1, 0.1, "the pot and the sill");
            apart(theme, c.sun, c.glow, 0.05, "the sun and its glow");
            apart(theme, c.glow, c.sky.at(0.3), 0.03, "the glow and the sky");
            for colour in c.stem.steps().iter().chain([&c.pot.1, &c.dead]) {
                assert!(
                    theme.contrast_of(*colour) >= paint::SOLID - 0.01,
                    "{}",
                    theme.name
                );
            }
            for colour in c
                .wall
                .steps()
                .iter()
                .chain(c.sky.steps())
                .chain([&c.glow, &c.soil])
            {
                assert!(
                    theme.contrast_of(*colour) < IMAGERY_CEILING,
                    "{}",
                    theme.name
                );
            }
        }
    }
}
