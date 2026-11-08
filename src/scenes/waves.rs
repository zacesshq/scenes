//! Waves: a calm sea to the horizon under a sun, painted in solid pixels, with gulls crossing the
//! sky.
//!
//! The sky, the sea and its swell are tints of the background held below the non-text floor, so
//! they read as background. The sun and the gulls keep the theme's own drawing colours, so they
//! stand out as shapes and keep their outline when colour is off.

use std::f32::consts::TAU;

use super::{count_for, cycle, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// One wing beat, in seconds. Four wing positions a beat keeps every change under three a second.
const FLAP_PERIOD: f32 = 1.6;

#[derive(Clone, Debug)]
struct Gull {
    x: f32,
    /// The top pixel row of the gull.
    y: i32,
    speed: f32,
    phase: f32,
}

/// A gull's four wing positions at pixel resolution: up, level, down and level again.
const GULL_FRAMES: [[&str; 3]; 4] = [
    ["w.....w", ".w...w.", "..wbw.."],
    [".ww.ww.", "w..b..w", "......."],
    [".......", ".wwbww.", "w.....w"],
    [".ww.ww.", "w..b..w", "......."],
];
const GULL_WIDTH: f32 = 7.0;

/// Steps in the depth and swell ramps, few enough to keep the escape output small.
const DEPTH_STEPS: usize = 10;
const SWELL_STEPS: usize = 4;

/// The colours of the waves in one theme, every one derived from the theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct WavesColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub glow: Rgb,
    pub far: Rgb,
    pub near: Rgb,
    pub crest: Rgb,
    pub sun: Rgb,
    /// The ring around the sun on a light theme, where the sun is a pale disc in a warm halo.
    pub halo: Option<Rgb>,
    pub gull: Rgb,
    pub gull_body: Rgb,
    sky: Ramp,
    /// For each depth step, the swell from the water to the crest.
    swell: Vec<Ramp>,
    /// For each depth step, the water in a trough's shadow.
    shadow: Vec<Rgb>,
}

impl WavesColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        // a dark theme's sky brightens towards the horizon; a light one's pales towards it
        let (sky_top, sky_low) = if theme.dark {
            (
                shades.tint(theme.info, 0.18, 1.45),
                shades.tint(theme.info, 0.45, 2.1),
            )
        } else {
            (
                shades.tint(theme.info, 0.42, 2.0),
                shades.tint(theme.info, 0.12, 1.3),
            )
        };
        let glow = shades.tint(theme.warning, 0.55, 2.5);
        // the far sea is hazy and the near sea deeper, on dark and light themes alike
        let (far, near) = if theme.dark {
            (
                shades.tint(theme.blue(), 0.6, 2.6),
                shades.tint(theme.blue(), 0.25, 1.4),
            )
        } else {
            (
                shades.tint(theme.blue(), 0.28, 1.8),
                shades.tint(theme.blue(), 0.75, 2.8),
            )
        };
        // a crest is the lightest the sea gets while it still reads as background
        let crest = if theme.dark {
            shades.tint(theme.bright_cyan(), 0.75, IMAGERY_CEILING - 0.05)
        } else {
            mix(far, theme.background, 0.75)
        };
        let depth = Ramp::new(far, near, DEPTH_STEPS);
        let swell = depth
            .steps()
            .iter()
            .map(|&base| Ramp::new(base, crest, SWELL_STEPS))
            .collect();
        let shadow = depth
            .steps()
            .iter()
            .map(|&base| mix(base, shades.darkest(), 0.18))
            .collect();
        Self {
            sky_top,
            sky_low,
            glow,
            far,
            near,
            crest,
            // a daylit sky on a light theme has a pale sun, never a dark disc
            sun: if theme.dark {
                theme.bright_yellow()
            } else {
                theme.background
            },
            halo: (!theme.dark).then(|| mix(theme.background, glow, 0.6)),
            gull: theme.foreground,
            gull_body: theme.bright(),
            sky: Ramp::new(sky_top, sky_low, DEPTH_STEPS),
            swell,
            shadow,
        }
    }

    fn step(depth: f32) -> usize {
        ((depth.clamp(0.0, 1.0) * (DEPTH_STEPS - 1) as f32).round() as usize).min(DEPTH_STEPS - 1)
    }

    /// The water at a depth from 0 at the horizon to 1 at the bottom, lifted by a crest from 0
    /// to 1.
    fn water(&self, depth: f32, lift: f32) -> Rgb {
        self.swell[Self::step(depth)].at(lift)
    }

    fn trough(&self, depth: f32) -> Rgb {
        self.shadow[Self::step(depth)]
    }
}

pub struct Waves {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    gulls: Vec<Gull>,
}

impl Waves {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut waves = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            gulls: Vec::new(),
        };
        waves.resize(width, height, rng);
        waves
    }

    /// The pixel row of the horizon.
    fn horizon(&self) -> i32 {
        ((self.height as f32 * 0.4) as i32).max(1) * 2
    }

    fn flap_period(&self) -> f32 {
        FLAP_PERIOD / self.motion.pace()
    }

    fn sun_fits(&self) -> bool {
        self.width >= 16 && self.horizon() >= 6
    }

    /// The sun's centre and radius in pixels.
    fn sun(&self) -> (f32, f32, f32) {
        let horizon = self.horizon() as f32;
        let r = (horizon * 0.16).clamp(1.5, 4.5);
        (self.width as f32 - 6.0 - r, (horizon * 0.4).max(r + 1.0), r)
    }

    fn paint(&self, canvas: &mut Canvas, colours: &WavesColours) {
        let horizon = self.horizon();
        let tall = canvas.height() as i32;
        let wide = canvas.width() as i32;
        let speed = self.motion.speed();

        // the sky, warming to a glow over the last few rows above the horizon
        for y in 0..horizon.min(tall) {
            let t = y as f32 / horizon.max(1) as f32;
            let colour = if t > 0.8 {
                mix(colours.sky_low, colours.glow, (t - 0.8) * 2.0)
            } else {
                colours.sky.at(t / 0.8)
            };
            canvas.fill_row(y, colour);
        }

        // a still sun with a soft edge: it never changes, so it can never flash
        let (sun_x, sun_y, sun_r) = self.sun();
        if self.sun_fits() {
            let reach = sun_r as i32 + 2;
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    let (px, py) = (sun_x.round() as i32 + dx, sun_y.round() as i32 + dy);
                    let d = ((px as f32 - sun_x).powi(2) + (py as f32 - sun_y).powi(2)).sqrt();
                    if d > sun_r && d <= sun_r + 0.9 {
                        match colours.halo {
                            Some(halo) => canvas.set(px, py, halo),
                            None => canvas.blend(px, py, colours.sun, 0.2),
                        }
                    }
                }
            }
        }

        // the sea: lines of swell that crowd together towards the horizon and roll slowly in,
        // each broken into streaks that lengthen towards the viewer, so the water reads as
        // reaching away into the distance; under the sun the streaks catch more of the light
        let span = (tall - horizon).max(1) as f32;
        let lines = (span / 3.0).max(1.0);
        let roll = cycle(self.t, 0.12 * speed * TAU) / TAU;
        let drift = cycle(self.t, 0.35 * speed);
        for y in horizon..tall {
            let depth = (y - horizon) as f32 / span;
            let u = depth.sqrt() * lines + roll;
            let band = u.floor();
            let within = u - band;
            let streak = 3.0 + 12.0 * depth;
            let path = 1.5 + depth * 12.0;
            for x in 0..wide {
                let xf = x as f32;
                let along = (xf / streak + band * 2.3 + drift * (0.5 + depth)).sin()
                    + 0.5 * (xf / (streak * 0.37) - band * 1.7).sin();
                let under_sun = self.sun_fits() && (xf - sun_x).abs() < path;
                let lit = if under_sun { -0.4 } else { 0.25 };
                let colour = if y == horizon {
                    // the horizon line always catches the light
                    colours.water(0.0, 0.67)
                } else if within < 0.3 && along > lit {
                    let lift = 0.34 + (along - lit) * if under_sun { 0.8 } else { 0.5 };
                    colours.water(depth, lift.min(1.0))
                } else if within > 0.55 && within < 0.8 && along < -0.3 {
                    colours.trough(depth)
                } else {
                    colours.water(depth, 0.0)
                };
                canvas.set(x, y, colour);
            }
        }

        let period = f64::from(self.flap_period());
        for gull in &self.gulls {
            let beat = ((self.t / period + f64::from(gull.phase)).rem_euclid(1.0) * 4.0) as usize;
            canvas.sprite(
                gull.x.round() as i32,
                gull.y,
                &GULL_FRAMES[beat % 4],
                &[('w', colours.gull), ('b', colours.gull_body)],
            );
        }

        // the sun is painted last, so a gull passes behind it
        if self.sun_fits() {
            canvas.disc(sun_x, sun_y, sun_r, colours.sun);
        }
    }
}

impl Scene for Waves {
    fn name(&self) -> &'static str {
        "waves"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = count_for(self.width, self.height, 300, 1, 6);
        let horizon = self.horizon();
        self.gulls = (0..wanted)
            .map(|_| {
                let right = rng.chance(0.5);
                Gull {
                    x: rng.range(0.0, self.width as f32),
                    y: rng.below((horizon - 4).max(1) as usize) as i32,
                    speed: if right { 1.0 } else { -1.0 }
                        * rng.range(1.0, 2.0)
                        * self.motion.speed(),
                    phase: rng.range(0.0, 1.0),
                }
            })
            .collect();
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
        let width = self.width as f32;
        for gull in &mut self.gulls {
            gull.x = wrap(
                gull.x + gull.speed * dt + GULL_WIDTH,
                width + 2.0 * GULL_WIDTH,
            ) - GULL_WIDTH;
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = WavesColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixels::distance;
    use crate::scenes::testing::{check_determinism, check_pixel_scene, check_scene, pixels};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("waves");
        check_determinism("waves");
        check_pixel_scene("waves", 6.0);
    }

    fn drawn(waves: &Waves, width: usize, height: usize, theme: &ThemeData) -> Frame {
        let mut frame = Frame::new(width, height);
        waves.draw(&mut frame, theme);
        frame
    }

    #[test]
    fn the_bands_stay_distinct_and_in_order_in_every_theme() {
        for theme in theme::all() {
            let c = WavesColours::new(theme);
            let name = theme.name;
            for (a, b, what) in [
                (c.sky_top, c.sky_low, "the sky's top and bottom"),
                (c.sky_low, c.far, "the sky and the sea"),
                (c.far, c.near, "the far and near sea"),
                (c.far, c.crest, "the sea and its crests"),
                (c.glow, c.sky_low, "the glow and the sky"),
            ] {
                assert!(distance(a, b) >= 0.03, "{what} collapse in {name}");
            }
            assert!(c.near.luminance() < c.far.luminance(), "{name}");
            for (what, colour) in [
                ("sky top", c.sky_top),
                ("sky low", c.sky_low),
                ("glow", c.glow),
                ("far", c.far),
                ("near", c.near),
                ("crest", c.crest),
            ] {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            let sun = theme.dark.then_some(c.sun);
            for colour in [c.gull, c.gull_body].into_iter().chain(sun) {
                assert!(theme.contrast_of(colour) >= theme.floor(), "{name}");
                assert!(distance(colour, c.far) >= 0.1, "{name}");
                assert!(distance(colour, c.sky_top) >= 0.1, "{name}");
            }
            // a light theme's pale sun still stands clear of the sky around it
            if let Some(halo) = c.halo {
                assert!(distance(c.sun, c.sky.at(0.5)) >= 0.05, "{name}");
                assert!(distance(halo, c.sky.at(0.5)) >= 0.02, "{name}");
            }
        }
    }

    #[test]
    fn the_sea_fills_below_the_horizon_and_gulls_fly_above_it() {
        let theme = theme::default_theme();
        let c = WavesColours::new(theme);
        let mut rng = Rng::new(19);
        let mut waves = Waves::new(60, 20, Motion::FULL, &mut rng);
        for _ in 0..10 {
            waves.step(0.05, &mut rng);
        }
        let frame = drawn(&waves, 60, 20, theme);
        let horizon = waves.horizon();
        assert_eq!(horizon, 16);
        let (top, _) = frame.get(0, 0).unwrap().pixel_pair().unwrap();
        assert_eq!(top, c.sky_top);
        let mut sky = Vec::new();
        for y in 0..horizon / 2 {
            for x in 0..60 {
                let (a, b) = frame.get(x, y).unwrap().pixel_pair().unwrap();
                sky.extend([a, b]);
            }
        }
        assert!(sky.contains(&c.sun));
        assert!(sky.contains(&c.gull), "no gull in the sky");
        for y in horizon / 2..20 {
            for x in 0..60 {
                let (a, b) = frame.get(x, y).unwrap().pixel_pair().unwrap();
                for p in [a, b] {
                    assert!(p != c.sky_top && p != c.gull, "{x},{y}");
                }
            }
        }
        assert!(waves.flap_period() >= FLAP_PERIOD);
        assert!(waves.flap_period() * 0.25 >= 1.0 / 3.0);
    }

    #[test]
    fn a_tiny_screen_still_shows_a_horizon() {
        let theme = theme::default_theme();
        let c = WavesColours::new(theme);
        let line = c.water(0.0, 0.67);
        let mut rng = Rng::new(2);
        let waves = Waves::new(20, 8, Motion::CALM, &mut rng);
        let frame = drawn(&waves, 20, 8, theme);
        assert!(pixels(&frame).contains(&c.sun));
        let (top, _) = frame
            .get(5, waves.horizon() / 2)
            .unwrap()
            .pixel_pair()
            .unwrap();
        assert_eq!(top, line);
        let tiny = Waves::new(12, 5, Motion::CALM, &mut rng);
        let frame = drawn(&tiny, 12, 5, theme);
        assert!(!pixels(&frame).contains(&c.sun));
        assert!(pixels(&frame).contains(&line));
    }
}
