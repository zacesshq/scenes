//! Ribbons, painted in solid pixels: broad ribbons of silk that fill the screen and flow slowly
//! across it, each following a pair of slow waves and twisting as it goes, so it narrows where it
//! turns and shows its darker back before turning again. The light comes from the upper left, so
//! every fold has a lit face, a sheen along its crest and a shadow on its far side. Each
//! ribbon casts a soft shadow on whatever lies behind it.
//!
//! Every colour comes from the active theme. The backdrop and the cast shadows are tints of the
//! background held below the non-text floor, so they read as background; the ribbons are shaded
//! from the theme's own drawing colours, deepest in the folds and strongest on the lit faces, so
//! they stand out and keep their outline when colour is off.

use std::f32::consts::TAU;

use super::cycle;
use super::paint::Blender;
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "Broad ribbons of shaded silk twisting and flowing slowly across the screen";

pub const CAPTIONS: &[&str] = &[
    "Everything flows.",
    "Smooth from end to end.",
    "No knots in this build.",
    "All lines lead to green.",
    "Steady, smooth and clean.",
];

/// The fastest a ribbon's crest or twist travels sideways, in columns a second at the standard
/// speed.
pub const MAX_DRIFT: f32 = 4.0;
/// Steps of light along each side of a ribbon.
const LIGHT_STEPS: usize = 9;
/// How far a ribbon's shadow falls below and to the right of it, in pixels.
const SHADOW: (i32, i32) = (2, 3);

#[derive(Clone, Debug)]
struct Ribbon {
    /// The centre's resting row, in pixels.
    base: f32,
    amplitude: f32,
    /// Wave numbers and angular speeds of the two waves that make the ribbon.
    k: (f32, f32),
    w: (f32, f32),
    phase: (f32, f32),
    /// Wave number, angular speed and phase of the twist.
    twist: (f32, f32, f32),
    colour: usize,
    /// Half the ribbon's width where it lies flat, in pixels.
    half: f32,
    /// From 0 for the nearest ribbon to 1 for the furthest, which is paler and narrower.
    depth: f32,
}

impl Ribbon {
    /// The pixel row of the ribbon's centre at column `x` and time `t`.
    fn y(&self, x: f32, t: f64, speed: f32) -> f32 {
        let a = x * self.k.0 + cycle(t, self.w.0 * speed) + self.phase.0;
        let b = x * self.k.1 + cycle(t, self.w.1 * speed) + self.phase.1;
        self.base + self.amplitude * (a.sin() * 0.7 + b.sin() * 0.3)
    }

    /// The angle the ribbon has turned to at column `x`: its cosine is 1 flat and facing out,
    /// -1 flat and showing its back and 0 edge on.
    fn angle(&self, x: f32, t: f64, speed: f32) -> f32 {
        x * self.twist.0 + cycle(t, self.twist.1 * speed) + self.twist.2
    }
}

/// The ribbons' colours in one theme, every one derived from the theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct RibbonColours {
    pub back_top: Rgb,
    pub back_low: Rgb,
    pub shadow: Rgb,
    pub bases: [Rgb; 5],
    /// For each ribbon colour, its front from the deepest fold to the sheen, then its back.
    front: Vec<Ramp>,
    back: Vec<Ramp>,
    backdrop: Ramp,
}

impl RibbonColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let bases = [
            theme.cyan(),
            theme.magenta(),
            theme.yellow(),
            theme.blue(),
            theme.green(),
        ];
        let (back_top, back_low) = if theme.dark {
            (
                shades.tint(theme.info, 0.16, 1.25),
                shades.tint(theme.accent, 0.08, 1.12),
            )
        } else {
            (
                shades.tint(theme.info, 0.05, 1.08),
                shades.tint(theme.info, 0.16, 1.3),
            )
        };
        // the shade a ribbon casts is a step deeper into the backdrop on a dark theme and a cool
        // grey on a light one, held well under the ceiling so it never reads as a shape
        let shadow = if theme.dark {
            mix(back_low, theme.background, 0.7)
        } else {
            shades.tint(theme.muted, 0.2, 1.42)
        };
        // a dark theme's fold sinks towards the background and its sheen rises towards the text
        // colour; a light theme's fold deepens and its sheen pales, which is how silk looks in
        // daylight
        let front = bases
            .iter()
            .map(|&c| {
                let (fold, sheen) = if theme.dark {
                    (mix(c, theme.background, 0.62), shades.lighter(c, 0.45))
                } else {
                    (shades.darker(c, 0.45), mix(c, theme.background, 0.62))
                };
                Ramp::new(fold, sheen, LIGHT_STEPS)
            })
            .collect();
        let back = bases
            .iter()
            .map(|&c| {
                let (fold, lit) = if theme.dark {
                    (
                        mix(c, theme.background, 0.8),
                        mix(c, theme.background, 0.35),
                    )
                } else {
                    (shades.darker(c, 0.65), mix(c, theme.background, 0.3))
                };
                Ramp::new(fold, lit, LIGHT_STEPS)
            })
            .collect();
        Self {
            back_top,
            back_low,
            shadow,
            bases,
            front,
            back,
            backdrop: Ramp::new(back_top, back_low, 8),
        }
    }
}

pub struct Ribbons {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    ribbons: Vec<Ribbon>,
}

/// One column of a ribbon as drawn: its centre, its half width and the light on it.
struct Slice {
    centre: f32,
    half: f32,
    front: bool,
    /// The light on the flat of the ribbon here, from 0 in shadow to 1 facing the light.
    light: f32,
}

impl Ribbons {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut scene = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            ribbons: Vec::new(),
        };
        scene.resize(width, height, rng);
        scene
    }

    fn slice(&self, ribbon: &Ribbon, x: f32, speed: f32) -> Slice {
        let centre = ribbon.y(x, self.t, speed);
        let slope = ribbon.y(x + 1.0, self.t, speed) - centre;
        let angle = ribbon.angle(x, self.t, speed);
        let turn = angle.cos();
        // a ribbon edge on is still a thread, so it never breaks in two
        let half = (ribbon.half * turn.abs().sqrt()).max(0.7);
        // the light falls from the upper left: a face turning towards it brightens, a stretch
        // climbing to the right faces it and a stretch falling away is in shade
        let facing = 0.5 + 0.5 * (angle.sin() * 0.8 + turn.abs() * 0.6 - 0.2);
        let light = (facing - slope * 0.35).clamp(0.0, 1.0);
        Slice {
            centre,
            half,
            front: turn >= 0.0,
            light,
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &RibbonColours, theme: &ThemeData) {
        let (wide, tall) = (canvas.width() as i32, canvas.height() as i32);
        let mut blender = Blender::new(4);
        let darkest = Shades::new(theme).darkest();
        for y in 0..tall {
            canvas.fill_row(y, c.backdrop.at(y as f32 / tall.max(1) as f32));
        }
        let speed = self.motion.speed();
        // the furthest ribbon first, so nearer ones lie over it and cast their shade on it
        let mut order: Vec<&Ribbon> = self.ribbons.iter().collect();
        order.sort_by(|a, b| b.depth.total_cmp(&a.depth));
        for ribbon in order {
            let slices: Vec<Slice> = (0..wide)
                .map(|x| self.slice(ribbon, x as f32 + 0.5, speed))
                .collect();
            // the cast shadow, a soft band below and to the right of the ribbon
            for (x, s) in slices.iter().enumerate() {
                let sx = x as i32 + SHADOW.0;
                let top = (s.centre - s.half).floor() as i32 + SHADOW.1;
                let bottom = (s.centre + s.half).ceil() as i32 + SHADOW.1;
                for y in top..=bottom {
                    if let Some(under) = canvas.get(sx, y) {
                        // shade on the backdrop stays imagery; shade on a ribbon further back is
                        // a deeper tone of that ribbon
                        if theme.contrast_of(under) < IMAGERY_CEILING {
                            canvas.set(sx, y, c.shadow);
                        } else {
                            blender.blend(canvas, sx, y, darkest, 0.25);
                        }
                    }
                }
            }
            let front = &c.front[ribbon.colour];
            let back = &c.back[ribbon.colour];
            let far = ribbon.depth * 0.3;
            for (x, s) in slices.iter().enumerate() {
                let x = x as i32;
                let top = (s.centre - s.half).floor() as i32;
                let bottom = (s.centre + s.half).ceil() as i32;
                let ramp = if s.front { front } else { back };
                for py in top..=bottom {
                    let u = (py as f32 + 0.5 - s.centre) / s.half;
                    if u.abs() > 1.05 {
                        continue;
                    }
                    // across the width the silk curls slightly, so the upper half catches the
                    // light and the lower edge rolls into shade, with a sheen just above the
                    // middle where the curl faces the light squarely
                    let curl = (1.0 - u * u).max(0.0).sqrt();
                    let mut level = s.light * (0.55 + 0.45 * curl) - u * 0.18;
                    let sheen = (-((u + 0.3) * 3.0).powi(2)).exp() * s.light * s.light;
                    level += sheen * 0.35;
                    // the very edge is a rim a step deeper than the face beside it, so each
                    // ribbon has a clean outline over whatever is behind it
                    if u.abs() > 0.82 && s.half > 1.5 {
                        level -= 0.22;
                    }
                    let colour = ramp.at(level.clamp(0.0, 1.0));
                    let colour = blender.mix(colour, theme.background, far);
                    canvas.set(x, py, colour);
                }
            }
        }
    }
}

impl Scene for Ribbons {
    fn name(&self) -> &'static str {
        "ribbons"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let h = self.height as f32 * 2.0;
        let w = self.width as f32;
        let count = ((self.height as f32 / 8.0).clamp(1.0, 5.0) * self.motion.busy())
            .round()
            .clamp(1.0, 6.0) as usize;
        let start = rng.below(5);
        // broad enough that the ribbons fill the frame between them, as wide silk does
        let spacing = h / count as f32;
        let half = (spacing * 0.42).clamp(1.5, 14.0);
        self.ribbons = (0..count)
            .map(|i| {
                let k0 = TAU / (w * rng.range(0.9, 1.5)).max(8.0);
                let k1 = k0 * rng.range(1.6, 2.2);
                // the wave speeds are chosen so neither wave's crest travels faster than the
                // drift limit
                let w0 = rng.range(0.4, 1.0) * MAX_DRIFT * k0;
                let w1 = -rng.range(0.2, 0.6) * MAX_DRIFT * k1;
                let kt = TAU / (w * rng.range(0.8, 1.4)).max(8.0);
                let wt = rng.range(0.3, 0.8) * MAX_DRIFT * kt;
                // alternate depths, so neighbours overlap with one clearly in front
                let depth = if i % 2 == 0 { 0.0 } else { rng.range(0.6, 1.0) };
                Ribbon {
                    base: spacing * (i as f32 + 0.5),
                    amplitude: (spacing * rng.range(0.55, 0.8)).max(1.0),
                    k: (k0, k1),
                    w: (w0, w1),
                    phase: (rng.range(0.0, TAU), rng.range(0.0, TAU)),
                    twist: (kt, wt, rng.range(0.0, TAU)),
                    colour: (start + i) % 5,
                    half: half * (1.0 - depth * 0.2),
                    depth,
                }
            })
            .collect();
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = RibbonColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours, theme);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::pixels::distance;
    use crate::scenes::testing::{check_determinism, check_pixel_scene, check_scene, render};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("ribbons");
        check_determinism("ribbons");
        check_pixel_scene("ribbons", 6.0);
    }

    #[test]
    fn ribbons_drift_and_twist_no_faster_than_the_limit() {
        let mut rng = Rng::new(12);
        for motion in Motion::ALL {
            let scene = Ribbons::new(120, 40, motion, &mut rng);
            assert!(!scene.ribbons.is_empty());
            for ribbon in &scene.ribbons {
                let limit = MAX_DRIFT * motion.speed() + 0.001;
                for (w, k) in [
                    (ribbon.w.0, ribbon.k.0),
                    (ribbon.w.1, ribbon.k.1),
                    (ribbon.twist.1, ribbon.twist.0),
                ] {
                    let drift = w.abs() / k * motion.speed();
                    assert!(drift <= limit, "{drift}");
                }
            }
        }
    }

    #[test]
    fn the_ribbons_cross_and_fill_the_screen() {
        let mut rng = Rng::new(3);
        let mut scene = Ribbons::new(80, 24, Motion::FULL, &mut rng);
        let frame = render(&mut scene, 80, 24, 40);
        let c = RibbonColours::new(theme::default_theme());
        let silk = |p: &Rgb| !c.backdrop.steps().contains(p) && *p != c.shadow;
        // every column crosses each ribbon somewhere
        for x in [0, 40, 79] {
            let painted = (0..24)
                .filter_map(|y| frame.get(x, y).and_then(|cell| cell.pixel_pair()))
                .flat_map(|(a, b)| [a, b])
                .filter(silk)
                .count();
            assert!(painted >= scene.ribbons.len(), "column {x}");
        }
        // and between them the ribbons fill a good part of the frame
        let filled = super::super::testing::pixels(&frame)
            .iter()
            .filter(|p| silk(p))
            .count();
        assert!(filled * 3 >= 80 * 48, "only {filled} pixels of ribbon");
    }

    #[test]
    fn the_colours_stay_distinct_and_in_bounds_in_every_theme() {
        for theme in theme::all() {
            let c = RibbonColours::new(theme);
            let name = theme.name;
            for colour in [c.back_top, c.back_low, c.shadow] {
                assert!(theme.contrast_of(colour) < IMAGERY_CEILING, "{name}");
            }
            assert!(distance(c.shadow, c.back_low) >= 0.02, "{name}");
            for (i, base) in c.bases.iter().enumerate() {
                assert!(theme.contrast_of(*base) >= theme.floor(), "{name}");
                assert!(distance(*base, c.back_low) >= 0.1, "{name}");
                // the front and the back of a ribbon read as two sides
                let front = c.front[i].at(0.6);
                let back = c.back[i].at(0.6);
                assert!(distance(front, back) >= 0.05, "{name} ribbon {i}");
                let next = c.bases[(i + 1) % c.bases.len()];
                assert!(
                    distance(*base, next) >= 0.05,
                    "{name} ribbons {i} and the next"
                );
            }
        }
    }
}
