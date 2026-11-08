//! Orbit, painted in solid pixels: a banded gas giant lit from the upper left, shaded from its
//! bright limb to a dark night side, with a storm drifting across its face, a broad ring with a
//! gap in it that passes behind the planet and in front of it, a small cratered moon on a slow
//! orbit and distant stars over a faint band of haze.
//!
//! A light theme shows the planet as a bright illustration on a pale sky: mid tones that are
//! only just solid rather than the theme's darkest inks, a pale icy ring and a few soft blue grey
//! stars.
//!
//! Every colour comes from the active theme. The haze and the faint stars are tints of the
//! background held below the non-text floor, so they read as background; the planet, its ring
//! and the moon are shaded from the theme's own drawing colours, so they stand out as shapes and
//! keep their outline when colour is off.

use std::f32::consts::TAU;

use super::{count_for, cycle, paint};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, distance, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "A shaded, ringed planet among distant stars with a small moon on a slow orbit around it, on a pale sky in light themes";

pub const CAPTIONS: &[&str] = &[
    "Everything in its orbit.",
    "A stable build, a stable orbit.",
    "Clear of debris. Green across the board.",
    "Ground control reports all good.",
    "Steady as a moon.",
];

/// Seconds the moon takes to go round once at the standard speed. Tens of seconds, so it drifts
/// rather than spins.
pub const ORBIT_PERIOD: f32 = 40.0;
/// Seconds the storm on the planet's face takes to cross it once.
const SPOT_PERIOD: f32 = 60.0;
/// How far the ring and the moon's orbit lean towards the viewer: the ratio of an ellipse's
/// height to its width.
const TILT: f32 = 0.3;
/// The ring's inner and outer edge and its gap, in planet radii.
const RING_INNER: f32 = 1.35;
const RING_OUTER: f32 = 2.15;
const GAP: (f32, f32) = (1.72, 1.8);
/// Steps of light across a band, from the night side to full sun.
const LIGHT_STEPS: usize = 8;

/// The direction the sunlight comes from: the upper left and towards the viewer.
fn light() -> (f32, f32, f32) {
    let (x, y, z) = (-0.62_f32, -0.45_f32, 0.64_f32);
    let len = (x * x + y * y + z * z).sqrt();
    (x / len, y / len, z / len)
}

/// The orbit's colours in one theme, every one derived from the theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct OrbitColours {
    pub haze: Ramp,
    pub faint_star: Rgb,
    pub star: Rgb,
    pub bands: [Rgb; 4],
    pub spot: Rgb,
    pub ring: Rgb,
    pub ring_gap: Rgb,
    pub moon: Rgb,
    pub crater: Rgb,
    /// Each band's ramp from its night side to full light.
    lit: Vec<Ramp>,
    spot_lit: Ramp,
    ring_lit: Ramp,
    moon_lit: Ramp,
}

impl OrbitColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let pale = mix(theme.yellow(), theme.muted, 0.4);
        // a light theme with a 7 to 1 floor can swap both yellows for its muted ink, which
        // leaves the pale band nothing to differ from, so it takes the palette's own yellow
        let pale = if distance(pale, theme.yellow()) < 0.03
            || distance(pale, theme.bright_yellow()) < 0.03
        {
            mix(theme.ansi[3], theme.muted, 0.4)
        } else {
            pale
        };
        let bands = [
            theme.yellow(),
            mix(theme.yellow(), theme.red(), 0.45),
            theme.bright_yellow(),
            pale,
        ];
        let spot = theme.red();
        if !theme.dark {
            return Self::daylight(theme, bands, spot);
        }
        let ring = mix(theme.muted, theme.yellow(), 0.3);
        // the night side sinks most of the way into space
        let night = |c: Rgb| shades.darker(c, 0.85);
        let ramp = |c: Rgb| Ramp::new(night(c), c, LIGHT_STEPS);
        let moon = theme.grey();
        Self {
            haze: Ramp::new(
                theme.background,
                shades.tint(mix(theme.magenta(), theme.info, 0.5), 0.25, 1.22),
                4,
            ),
            faint_star: shades.tint(theme.foreground, 0.35, 2.2),
            star: theme.foreground,
            bands,
            spot,
            ring,
            ring_gap: shades.darker(ring, 0.6),
            moon,
            crater: shades.darker(moon, 0.35),
            lit: bands.iter().map(|&c| ramp(c)).collect(),
            spot_lit: ramp(spot),
            ring_lit: Ramp::new(shades.darker(ring, 0.5), ring, LIGHT_STEPS - 2),
            moon_lit: ramp(moon),
        }
    }

    /// A light theme's planet: each colour lifted to a mid tone that is only just solid, so the
    /// planet reads as a bright, coloured world, with its night side a deeper shade of the same.
    fn daylight(theme: &ThemeData, bands: [Rgb; 4], spot: Rgb) -> Self {
        let shades = Shades::new(theme);
        // each colour leans towards a warm gas giant hue, so the planet stays bright and coloured
        // rather than a muddy step towards the text colour
        let lift = |c: Rgb, hue: Rgb| paint::strong(theme, mix(c, hue, 0.7));
        let mut i = 0;
        let bands = bands.map(|c| {
            let lifted = lift(c, GIANT[i]);
            i += 1;
            lifted
        });
        let spot = lift(spot, Rgb::new(206, 84, 58));
        let night = |c: Rgb| mix(c, theme.foreground, 0.45);
        let ramp = |c: Rgb| Ramp::new(night(c), c, LIGHT_STEPS);
        // the ring is pale ice, lighter than the planet it crosses
        let ring = shades.tint(mix(theme.muted, theme.yellow(), 0.4), 1.0, 1.9);
        let moon = paint::strong(theme, shades.tint(theme.grey(), 1.0, 2.9));
        Self {
            haze: Ramp::new(
                theme.background,
                shades.tint(mix(theme.magenta(), theme.info, 0.5), 0.3, 1.25),
                4,
            ),
            faint_star: shades.tint(theme.info, 0.3, 1.35),
            star: shades.tint(theme.info, 0.6, 1.9),
            bands,
            spot,
            ring,
            ring_gap: shades.tint(theme.info, 0.15, 1.15),
            moon,
            crater: night(moon),
            lit: bands.iter().map(|&c| ramp(c)).collect(),
            spot_lit: ramp(spot),
            ring_lit: Ramp::new(mix(ring, theme.foreground, 0.2), ring, LIGHT_STEPS - 2),
            moon_lit: ramp(moon),
        }
    }
}

/// The hues a light theme's planet bands lean towards: gold, rust, pale gold and a cool grey
/// brown, the last far enough from the gold to keep its own band.
const GIANT: [Rgb; 4] = [
    Rgb::new(222, 166, 72),
    Rgb::new(206, 118, 64),
    Rgb::new(232, 190, 110),
    Rgb::new(160, 146, 150),
];

pub struct Orbit {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// Stars by pixel, bright or faint.
    stars: Vec<(i32, i32, bool)>,
}

impl Orbit {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut orbit = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            stars: Vec::new(),
        };
        orbit.resize(width, height, rng);
        orbit
    }

    fn tall(&self) -> f32 {
        self.height as f32 * 2.0
    }

    /// The planet's radius in pixels.
    fn radius(&self) -> f32 {
        (self.tall() * 0.27).min(self.width as f32 * 0.17).max(1.5)
    }

    /// The planet's centre in pixels.
    fn centre(&self) -> (f32, f32) {
        (self.width as f32 / 2.0, self.tall() / 2.0)
    }

    /// The moon's angle, which grows by one turn per orbit period.
    fn moon_angle(&self) -> f32 {
        cycle(self.t, TAU / ORBIT_PERIOD * self.motion.speed())
    }

    fn moon_radius(&self) -> f32 {
        (self.radius() * 0.2).clamp(1.0, 4.5)
    }

    /// The moon's centre in pixels and whether it is in front of the planet.
    fn moon(&self) -> (f32, f32, bool) {
        let (cx, cy) = self.centre();
        let r = self.radius();
        let angle = self.moon_angle();
        let a = (r * 2.7)
            .min(self.width as f32 / 2.0 - self.moon_radius() - 1.0)
            .max(r + self.moon_radius() + 2.0);
        let b = a * TILT;
        (
            cx + a * angle.cos(),
            cy + b * angle.sin(),
            angle.sin() > 0.0,
        )
    }

    /// Whether a pixel centre is on the planet's disc.
    fn on_planet(&self, x: f32, y: f32) -> bool {
        let (cx, cy) = self.centre();
        let r = self.radius();
        (x - cx).powi(2) + (y - cy).powi(2) <= r * r
    }

    fn paint_background(&self, canvas: &mut Canvas, c: &OrbitColours) {
        let (w, h) = (canvas.width() as i32, canvas.height() as i32);
        // a faint diagonal band of haze that thins out at its edges, so it reads as distant dust
        for y in 0..h {
            for x in 0..w {
                let along = (x as f32 * 0.35 - y as f32 * 0.9) / (w.max(1) as f32 * 0.35);
                let band = (-(along - 0.1).powi(2) * 7.0).exp();
                let hash =
                    (x as u32).wrapping_mul(0x9e37_79b1) ^ (y as u32).wrapping_mul(0x85eb_ca77);
                let grain = ((hash >> 11) % 100) as f32 / 100.0;
                let level = band * (0.8 + 0.35 * grain) - 0.3;
                if level > 0.0 {
                    canvas.set(x, y, c.haze.at(level * 1.6));
                }
            }
        }
        for &(x, y, bright) in &self.stars {
            canvas.set(x, y, if bright { c.star } else { c.faint_star });
        }
    }

    /// Paints the half of the ring behind the planet (`front` false) or in front of it.
    fn paint_ring(&self, canvas: &mut Canvas, c: &OrbitColours, front: bool) {
        let (cx, cy) = self.centre();
        let r = self.radius();
        let reach = (r * RING_OUTER).ceil() as i32 + 1;
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let (px, py) = (cx + dx as f32 + 0.5, cy + dy as f32 + 0.5);
                let (u, v) = (px - cx, (py - cy) / TILT);
                let along = (u * u + v * v).sqrt() / r;
                if !(RING_INNER..=RING_OUTER).contains(&along) {
                    continue;
                }
                // the near half of the tilted ring is the half below the planet's centre
                if (py >= cy) != front {
                    continue;
                }
                if !front && self.on_planet(px, py) {
                    continue;
                }
                let x = (cx + dx as f32).floor() as i32;
                let y = (cy + dy as f32).floor() as i32;
                // the gap only shows where it is wide enough not to break into loose pixels
                if r * (GAP.1 - GAP.0) * TILT >= 0.6 && (GAP.0..GAP.1).contains(&along) {
                    canvas.set(x, y, c.ring_gap);
                    continue;
                }
                // brightest a little in from the inner edge, dimmer towards the outer edge and
                // on the side away from the sun
                let across = (along - RING_INNER) / (RING_OUTER - RING_INNER);
                let profile = 1.0 - (across - 0.3).abs() * 0.9;
                let side = 0.8 - 0.2 * (u / (r * RING_OUTER)).clamp(-1.0, 1.0);
                let level = profile * side;
                canvas.set(x, y, c.ring_lit.at(level));
            }
        }
    }

    fn paint_planet(&self, canvas: &mut Canvas, c: &OrbitColours) {
        let (cx, cy) = self.centre();
        let r = self.radius();
        let (lx, ly, lz) = light();
        let spot_x = cycle(self.t, TAU / SPOT_PERIOD * self.motion.speed()) / TAU * 2.4 - 1.2;
        let reach = r.ceil() as i32 + 1;
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let (px, py) = (cx + dx as f32 + 0.5, cy + dy as f32 + 0.5);
                let (nx, ny) = ((px - cx) / r, (py - cy) / r);
                let d = nx * nx + ny * ny;
                if d > 1.0 {
                    continue;
                }
                let nz = (1.0 - d).sqrt();
                let lambert = (nx * lx + ny * ly + nz * lz).max(0.0);
                // a little light wraps round the limb, so the dark side keeps its round edge
                let rim = if lambert == 0.0 && d > 0.8 { 0.12 } else { 0.0 };
                let level = (lambert.powf(0.8) + rim).min(1.0);
                // wavy band edges, so the stripes look like weather rather than paint
                let latitude = ny + 0.06 * (nx * 7.0 + ny * 3.0).sin();
                let band = (((latitude + 1.0) * 3.5) as usize) % c.bands.len();
                let spot = ((nx - spot_x) / 0.22).powi(2) + ((ny - 0.32) / 0.11).powi(2) < 1.0;
                let ramp = if spot { &c.spot_lit } else { &c.lit[band] };
                let x = (cx + dx as f32).floor() as i32;
                let y = (cy + dy as f32).floor() as i32;
                canvas.set(x, y, ramp.at(level));
            }
        }
    }

    fn paint_moon(&self, canvas: &mut Canvas, c: &OrbitColours) {
        let (mx, my, front) = self.moon();
        let rm = self.moon_radius();
        let (lx, ly, lz) = light();
        let reach = rm.ceil() as i32 + 1;
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let (px, py) = (mx.floor() + dx as f32 + 0.5, my.floor() + dy as f32 + 0.5);
                let (nx, ny) = ((px - mx) / rm, (py - my) / rm);
                let d = nx * nx + ny * ny;
                if d > 1.0 || (!front && self.on_planet(px, py)) {
                    continue;
                }
                let nz = (1.0 - d).sqrt();
                let level = (nx * lx + ny * ly + nz * lz).max(0.08);
                let x = px.floor() as i32;
                let y = py.floor() as i32;
                // two fixed craters, which turn with the moon's face as it goes round
                let crater = rm >= 2.5
                    && (((nx + 0.3).powi(2) + (ny - 0.2).powi(2) < 0.08)
                        || ((nx - 0.35).powi(2) + (ny + 0.35).powi(2) < 0.05));
                let colour = if crater {
                    mix(c.moon_lit.at(level), c.crater, 0.6)
                } else {
                    c.moon_lit.at(level)
                };
                canvas.set(x, y, colour);
            }
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &OrbitColours) {
        self.paint_background(canvas, c);
        let (_, _, moon_front) = self.moon();
        if !moon_front {
            self.paint_moon(canvas, c);
        }
        self.paint_ring(canvas, c, false);
        self.paint_planet(canvas, c);
        self.paint_ring(canvas, c, true);
        if moon_front {
            self.paint_moon(canvas, c);
        }
    }
}

impl Scene for Orbit {
    fn name(&self) -> &'static str {
        "orbit"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = self
            .motion
            .scaled(count_for(self.width, self.height, 28, 4, 400));
        let tall = self.height * 2;
        self.stars = (0..wanted)
            .map(|_| {
                (
                    rng.below(self.width) as i32,
                    rng.below(tall) as i32,
                    rng.chance(0.2),
                )
            })
            .collect();
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = OrbitColours::new(theme);
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
    use crate::scenes::testing::{check_determinism, check_pixel_scene, check_scene};
    use crate::terminal::pixel_256;
    use crate::theme;

    fn drawn(orbit: &Orbit, width: usize, height: usize, theme: &ThemeData) -> Frame {
        let mut frame = Frame::new(width, height);
        orbit.draw(&mut frame, theme);
        frame
    }

    /// The pixel at a pixel position.
    fn pixel(frame: &Frame, x: i32, y: i32) -> Rgb {
        let (top, bottom) = frame.get(x, y / 2).unwrap().pixel_pair().unwrap();
        if y % 2 == 0 { top } else { bottom }
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("orbit");
        check_determinism("orbit");
        check_pixel_scene("orbit", 6.0);
    }

    #[test]
    fn the_moon_orbits_slowly_and_passes_behind_the_planet() {
        const { assert!(ORBIT_PERIOD >= 20.0) };
        let mut rng = Rng::new(1);
        let mut orbit = Orbit::new(80, 24, Motion::LIVELY, &mut rng);
        let mut in_front = 0;
        let mut behind = 0;
        let mut last = orbit.moon_angle();
        for _ in 0..(60 * 20) {
            orbit.step(0.05, &mut rng);
            let angle = orbit.moon_angle();
            let turned = (angle - last).rem_euclid(TAU);
            // even lively, the moon moves well under a tenth of a turn a second
            assert!(turned * 20.0 < TAU / 10.0, "{turned}");
            last = angle;
            if orbit.moon().2 {
                in_front += 1;
            } else {
                behind += 1;
            }
        }
        assert!(in_front > 100 && behind > 100);
    }

    #[test]
    fn the_planet_is_shaded_from_its_lit_limb_to_its_night_side() {
        for theme in theme::all() {
            let c = OrbitColours::new(theme);
            let mut rng = Rng::new(2);
            for (w, h) in [(20, 8), (80, 24), (200, 60)] {
                let orbit = Orbit::new(w, h, Motion::FULL, &mut rng);
                let frame = drawn(&orbit, w, h, theme);
                let (cx, cy) = orbit.centre();
                let r = orbit.radius();
                // the upper left of the disc is brighter than the lower right
                let lit = pixel(&frame, (cx - r * 0.5) as i32, (cy - r * 0.45) as i32);
                let dark = pixel(&frame, (cx + r * 0.75) as i32, (cy + r * 0.3) as i32);
                assert!(
                    distance(lit, theme.background) > distance(dark, theme.background) - 0.02
                        || lit.luminance() > dark.luminance(),
                    "{} at {w}x{h}: {} and {}",
                    theme.name,
                    lit.hex(),
                    dark.hex()
                );
                // a large planet takes many shades, never one flat slab
                if h >= 24 {
                    let mut used: Vec<Rgb> = frame
                        .cells()
                        .iter()
                        .filter_map(|cell| cell.pixel_pair())
                        .flat_map(|(a, b)| [a, b])
                        .filter(|p| {
                            c.lit.iter().any(|ramp| ramp.steps().contains(p))
                                || c.spot_lit.steps().contains(p)
                        })
                        .collect();
                    used.sort_unstable();
                    used.dedup();
                    assert!(used.len() >= 12, "{} has {}", theme.name, used.len());
                }
                // the ring shows on both sides of the planet
                let ring_y = cy as i32;
                let left = pixel(&frame, (cx - r * 1.7) as i32, ring_y);
                let right = pixel(&frame, (cx + r * 1.7) as i32 - 1, ring_y);
                if h >= 24 {
                    for side in [left, right] {
                        assert!(
                            c.ring_lit.steps().contains(&side) || side == c.ring_gap,
                            "{} at {w}x{h}: {}",
                            theme.name,
                            side.hex()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_colours_stay_distinct_and_in_bounds_in_every_theme() {
        for theme in theme::all() {
            let c = OrbitColours::new(theme);
            let name = theme.name;
            for (what, colour) in [("haze", c.haze.at(1.0)), ("faint star", c.faint_star)] {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            assert!(distance(c.haze.at(1.0), theme.background) >= 0.02, "{name}");
            // at night the planet keeps the text floor; on a light theme it is a bright mid tone
            // that is still solid, under soft imagery stars
            if theme.dark {
                for colour in [c.bands[0], c.bands[2], c.spot, c.moon, c.star] {
                    assert!(theme.contrast_of(colour) >= theme.floor(), "{name}");
                }
            } else {
                for colour in [c.bands[0], c.bands[2], c.spot, c.moon] {
                    assert!(theme.contrast_of(colour) >= IMAGERY_CEILING, "{name}");
                }
                for colour in [c.star, c.ring, c.ring_gap] {
                    assert!(theme.contrast_of(colour) < IMAGERY_CEILING, "{name}");
                }
                assert!(distance(c.ring, c.bands[0]) >= 0.08, "{name}");
            }
            for i in 0..c.bands.len() {
                let j = (i + 1) % c.bands.len();
                assert!(
                    distance(c.bands[i], c.bands[j]) >= 0.03,
                    "bands {i} and {j} collapse in {name}"
                );
            }
            assert!(distance(c.spot, c.bands[0]) >= 0.05, "{name}");
            assert!(distance(c.ring, c.ring_gap) >= 0.05, "{name}");
            assert!(distance(c.ring, theme.background) >= 0.15, "{name}");
            // the near 256 colour entries keep the lit and the night side apart
            let near = |colour: Rgb| xterm_colour(pixel_256(colour, theme.background));
            let ramp = &c.lit[0];
            assert_ne!(
                near(ramp.at(0.0)),
                near(ramp.at(1.0)),
                "the planet flattens in {name}"
            );
            assert_ne!(near(c.ring), near(theme.background), "{name}");
        }
    }
}
