//! Blackout: a city skyline at night whose lit windows go dark one at a time over about a minute
//! and a half, beside a wooden pole whose power line has snapped and hangs to the street. A few
//! windows keep a candle and a handful stay lit on their own power, so the city is quiet but
//! never empty. Once dark it stays dark, with cloud drifting past a crescent moon.
//!
//! A light theme shows the same street at dusk: a fading sky with a pale moon and the lit windows
//! a warm gold, bright but still solid, so the lights going out read the same way.
//!
//! The sky, cloud, stars and buildings are tints of the background held below the non-text
//! floor. The lit windows, the candles, the moon, the pole and the break in the line are solid,
//! so with colour off the lights going out are seen as shapes going away.

use super::paint::{self, Blender, clouds, gradient, line, shaded, solid, unit};
use super::wrap;
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, distance, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::{error_colour, warning_colour};

pub const SUMMARY: &str = "A night skyline whose windows go dark one by one beside a snapped power line, leaving candles, at dusk in light themes";

pub const CAPTIONS: &[&str] = &[
    "The lights went out. Start with the first error.",
    "Power is down. One fix brings it back.",
    "Dark for now. The cause is in the list below.",
    "A candle and the compiler output. Good enough.",
    "Lights out. Retrace the last change.",
];

/// Seconds for the whole city to go dark at the standard pace.
pub const DARKEN_SECONDS: f32 = 90.0;
/// The shortest time between two windows going dark. Each window changes once, so this keeps
/// the change to a trickle even on a very wide screen.
pub const MIN_WINDOW_GAP: f32 = 0.25;
/// Seconds after a window goes dark before its candle is lit.
const CANDLE_DELAY: f32 = 5.0;

#[derive(Clone, Copy, Debug)]
struct Window {
    x: i32,
    y: i32,
    /// Whether it was lit when the scene began.
    lit: bool,
    /// Its place in the order the lights go out.
    rank: usize,
    /// Whether someone lights a candle there once it is dark.
    candle: bool,
    /// Whether it stays lit on its own power.
    generator: bool,
    /// The second shade of yellow, so the windows are not all one colour.
    warm: bool,
}

#[derive(Clone, Copy, Debug)]
struct Building {
    x: i32,
    width: i32,
    top: i32,
    tone: usize,
    /// A water tank, an aerial or a plain flat roof.
    roof: u8,
}

/// What a window shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Light {
    Unlit,
    Lit,
    Dark,
    Candle,
}

/// The city's colours in one theme.
struct Colours {
    sky: Ramp,
    cloud: Ramp,
    star: Rgb,
    starry: bool,
    moon: Rgb,
    buildings: [(Rgb, Rgb); 3],
    glass: Rgb,
    lit: (Rgb, Rgb),
    candle: Rgb,
    glow: Rgb,
    street: Rgb,
    pole: (Rgb, Rgb, Rgb),
    wire: Rgb,
    spark: Rgb,
}

impl Colours {
    fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let night = mix(theme.info, theme.magenta(), 0.2);
        let (sky_top, sky_low, cloud_dark, cloud_light, block, street) = if theme.dark {
            (
                shades.tint(night, 0.06, 1.1),
                shades.tint(night, 0.36, 1.8),
                shades.tint(theme.muted, 0.14, 1.3),
                shades.tint(theme.muted, 0.3, 1.7),
                shades.tint(theme.muted, 0.12, 1.2),
                shades.tint(theme.muted, 0.3, 1.7),
            )
        } else {
            (
                shades.tint(night, 0.5, 2.1),
                shades.tint(night, 0.25, 1.5),
                shades.tint(theme.muted, 0.55, 2.3),
                shades.tint(theme.muted, 0.3, 1.6),
                shades.tint(theme.muted, 0.72, 2.75),
                shades.tint(theme.muted, 0.6, 2.4),
            )
        };
        let tone = |t: f32| {
            let face = mix(block, sky_top, t);
            (face, mix(face, shades.lightest(), 0.12))
        };
        let wood = solid(
            theme,
            mix(theme.muted, theme.yellow(), 0.35),
            shades.darkest(),
            0.3,
        );
        Self {
            sky: Ramp::new(sky_top, sky_low, 8),
            cloud: Ramp::new(cloud_dark, cloud_light, 5),
            star: shades.tint(theme.foreground, 0.6, 2.8),
            starry: theme.dark,
            // a solid moon at night; at dusk a pale one in the fading sky
            moon: if theme.dark {
                theme.foreground
            } else {
                mix(Rgb::new(255, 255, 255), sky_top, 0.15)
            },
            buildings: [tone(0.0), tone(0.25), tone(0.45)],
            // a dark window is the sky seen through the glass. Where the sky and the walls are
            // too close to tell apart, as on Warm Dark, it is a step darker than the wall
            glass: if distance(mix(block, sky_top, 0.7), block) < 0.025 {
                mix(block, theme.background, 0.8)
            } else {
                mix(block, sky_top, 0.7)
            },
            // at night the windows shine in the theme's yellows; at dusk they are a warm gold,
            // as bright as a lit window can be while it stays solid
            lit: if theme.dark {
                (theme.yellow(), theme.bright_yellow())
            } else {
                (
                    paint::strong(theme, mix(theme.warning, Rgb::new(232, 140, 30), 0.6)),
                    paint::strong(theme, mix(theme.yellow(), Rgb::new(236, 178, 40), 0.6)),
                )
            },
            candle: warning_colour(theme),
            glow: shades.tint(warning_colour(theme), 0.45, 2.2),
            street,
            pole: shaded(theme, wood),
            wire: solid(theme, theme.muted, shades.darkest(), 0.3),
            spark: error_colour(theme),
        }
    }
}

pub struct Blackout {
    width: usize,
    height: usize,
    motion: Motion,
    seed: u32,
    buildings: Vec<Building>,
    windows: Vec<Window>,
    stars: Vec<(i32, i32)>,
    cloud_drift: f32,
    /// How many windows have gone dark, in rank order.
    darkened: usize,
    /// When each window went dark, by rank, in seconds of scene time.
    dark_since: Vec<f32>,
    since_last: f32,
    t: f32,
}

impl Blackout {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut city = Self {
            width: 1,
            height: 1,
            motion,
            seed: (rng.next_u64() >> 32) as u32,
            buildings: Vec::new(),
            windows: Vec::new(),
            stars: Vec::new(),
            cloud_drift: 0.0,
            darkened: 0,
            dark_since: Vec::new(),
            since_last: 0.0,
            t: 0.0,
        };
        city.resize(width, height, rng);
        city
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The first pixel row of the street.
    fn ground(&self) -> i32 {
        (self.tall() - 2).max(1)
    }

    /// How many windows were lit at the start, which sets the gap between two going dark.
    fn lit_count(&self) -> usize {
        self.windows.iter().filter(|w| w.lit).count()
    }

    fn gap(&self) -> f32 {
        let lit = self.lit_count().max(1) as f32;
        (DARKEN_SECONDS / lit).max(MIN_WINDOW_GAP) / self.motion.pace()
    }

    /// The left edge of the skyline, leaving room for the pole on a wide enough screen.
    fn skyline_start(&self) -> i32 {
        if self.width >= 40 { 10 } else { 0 }
    }

    fn is_dark(&self, window: &Window) -> bool {
        window.lit && !window.generator && window.rank < self.darkened
    }

    fn light(&self, window: &Window) -> Light {
        if !window.lit {
            Light::Unlit
        } else if self.is_dark(window) {
            let since = self.dark_since[window.rank];
            if window.candle && self.t - since >= CANDLE_DELAY {
                Light::Candle
            } else {
                Light::Dark
            }
        } else {
            Light::Lit
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        let mut mixer = Blender::new(8);
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let ground = self.ground();
        gradient(canvas, &c.sky, 0, ground);
        // stars only show against a dark sky; on a light theme the sky is a pale dusk
        if c.starry {
            for &(x, y) in &self.stars {
                canvas.set(x, y, c.star);
            }
        }
        // a crescent moon, behind the cloud
        if self.width >= 30 && self.height >= 12 {
            let (mx, my) = (wide as f32 * 0.68, (tall as f32 * 0.14).max(4.0));
            paint::disc(canvas, mx, my, 4.2, c.moon);
            let shadow = c.sky.at(my / ground.max(1) as f32);
            paint::disc(canvas, mx + 2.2, my - 1.2, 3.8, shadow);
        }
        clouds(
            canvas,
            &mut mixer,
            &c.cloud,
            0,
            (tall as f32 * 0.32) as i32,
            self.cloud_drift,
            self.seed,
            0.45,
        );

        for b in &self.buildings {
            let (face, edge) = c.buildings[b.tone % c.buildings.len()];
            canvas.fill_rect(b.x, b.top, b.width, ground - b.top, face);
            canvas.fill_rect(b.x, b.top, 1, ground - b.top, edge);
            canvas.fill_rect(b.x, b.top, b.width, 1, edge);
            match b.roof {
                0 if b.width >= 8 => {
                    // a water tank on legs
                    let x = b.x + b.width - 6;
                    canvas.fill_rect(x, b.top - 4, 4, 3, face);
                    canvas.set(x, b.top - 1, face);
                    canvas.set(x + 3, b.top - 1, face);
                }
                1 => {
                    let x = b.x + b.width / 2;
                    canvas.fill_rect(x, b.top - 5, 1, 5, edge);
                }
                _ => {}
            }
        }
        for window in &self.windows {
            let (x, y) = (window.x, window.y);
            match self.light(window) {
                Light::Unlit | Light::Dark => canvas.fill_rect(x, y, 2, 2, c.glass),
                Light::Lit => {
                    let colour = if window.warm { c.lit.0 } else { c.lit.1 };
                    canvas.fill_rect(x, y, 2, 2, colour);
                }
                Light::Candle => {
                    canvas.fill_rect(x, y, 2, 2, c.glow);
                    canvas.set(x, y + 1, c.candle);
                }
            }
        }
        for y in ground..tall {
            canvas.fill_row(y, c.street);
        }

        // the pole with its snapped line hanging down towards the street
        if self.width >= 40 && self.height >= 10 {
            let (dark, wood, light) = c.pole;
            let top = (ground - self.tall() / 2).max(2);
            canvas.fill_rect(3, top, 2, ground - top, wood);
            canvas.fill_rect(3, top, 1, ground - top, light);
            canvas.fill_rect(0, top + 1, 8, 1, dark);
            for x in [0, 7] {
                canvas.set(x, top, c.wire);
            }
            // the line sags from the crossbar towards the buildings, then hangs where it broke
            let end = (self.skyline_start() as f32 - 1.0, (ground - 3) as f32);
            let mut last = (7.0, top as f32);
            for k in 1..=10 {
                let s = k as f32 / 10.0;
                let point = (
                    7.0 + (end.0 - 7.0) * s,
                    top as f32 + (end.1 - top as f32) * s * s,
                );
                line(canvas, last, point, 1, c.wire);
                last = point;
            }
            // the break, a cross of sparks that never moves
            let (bx, by) = (end.0.round() as i32, end.1.round() as i32);
            for d in -1..=1 {
                canvas.set(bx + d, by + d, c.spark);
                canvas.set(bx + d, by - d, c.spark);
            }
        }
    }
}

impl Scene for Blackout {
    fn name(&self) -> &'static str {
        "blackout"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let ground = self.ground();
        let sky = (ground as f32).max(1.0);
        self.buildings.clear();
        let mut x = self.skyline_start();
        while x < self.width as i32 {
            let w = 7 + rng.below(8) as i32;
            let tall = rng.range(0.35, 0.78);
            let top = (ground - (sky * tall) as i32).max(6.min(ground));
            self.buildings.push(Building {
                x,
                width: w,
                top,
                tone: rng.below(3),
                roof: rng.below(3) as u8,
            });
            x += w + rng.below(2) as i32;
        }
        let mut windows = Vec::new();
        for b in &self.buildings {
            let mut y = b.top + 3;
            while y + 2 < ground - 1 {
                let mut wx = b.x + 2;
                while wx + 2 < b.x + b.width {
                    windows.push(Window {
                        x: wx,
                        y,
                        lit: rng.chance(0.65),
                        rank: 0,
                        candle: rng.chance(0.08),
                        generator: rng.chance(0.03),
                        warm: rng.chance(0.4),
                    });
                    wx += 3;
                }
                y += 4;
            }
        }
        // a new order for the lights, with the windows already dark kept dark
        let mut order: Vec<usize> = (0..windows.len()).collect();
        for i in (1..order.len()).rev() {
            order.swap(i, rng.below(i + 1));
        }
        let mut rank = 0;
        for &i in &order {
            if windows[i].lit && !windows[i].generator {
                windows[i].rank = rank;
                rank += 1;
            } else {
                windows[i].rank = usize::MAX;
            }
        }
        let fraction = if self.dark_since.is_empty() {
            0.0
        } else {
            self.darkened as f32 / self.dark_since.len() as f32
        };
        self.darkened = (fraction * rank as f32).round() as usize;
        // candles in windows already dark appear at once rather than all together later
        self.dark_since = vec![self.t - CANDLE_DELAY; self.darkened];
        self.dark_since.resize(rank, f32::MAX);
        self.windows = windows;
        let stars = super::count_for(self.width, self.height, 40, 2, 90);
        self.stars = (0..stars)
            .map(|i| {
                (
                    rng.below(self.width) as i32,
                    (unit(i as i32, 3, self.seed) * (ground as f32 * 0.45)) as i32,
                )
            })
            .collect();
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += dt;
        let speed = self.motion.speed();
        self.cloud_drift = wrap(self.cloud_drift + 0.8 * speed * dt, 100_000.0);
        self.since_last += dt;
        let gap = self.gap();
        if self.since_last >= gap && self.darkened < self.dark_since.len() {
            self.since_last = 0.0;
            self.dark_since[self.darkened] = self.t;
            self.darkened += 1;
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
        check_scene("blackout");
        check_determinism("blackout");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("blackout", 40.0);
    }

    #[test]
    fn windows_go_dark_one_at_a_time_and_stay_dark() {
        let mut rng = Rng::new(6);
        let mut city = Blackout::new(100, 30, Motion::LIVELY, &mut rng);
        let lit = |c: &Blackout| c.windows.iter().filter(|w| w.lit && !c.is_dark(w)).count();
        let start = lit(&city);
        assert!(start > 20, "{start}");
        let mut last = city.darkened;
        for _ in 0..(10 * 60 * 20) {
            city.step(0.05, &mut rng);
            assert!(city.darkened - last <= 1);
            last = city.darkened;
        }
        assert_eq!(city.darkened, city.dark_since.len());
        let generators = city.windows.iter().filter(|w| w.lit && w.generator).count();
        assert_eq!(lit(&city), generators);
        assert!(city.gap() >= MIN_WINDOW_GAP);
        let theme = theme::default_theme();
        let all = pixels(&render(&mut city, 100, 30, 1));
        let c = Colours::new(theme);
        assert!(all.contains(&c.spark) && all.contains(&c.moon));
    }

    #[test]
    fn no_window_changes_more_than_once_a_second_while_the_lights_go_out() {
        let mut rng = Rng::new(2);
        let mut city = Blackout::new(80, 24, Motion::LIVELY, &mut rng);
        let mut changes: Vec<Vec<f32>> = vec![Vec::new(); city.windows.len()];
        let mut last: Vec<Light> = city.windows.iter().map(|w| city.light(w)).collect();
        for i in 0..(60 * 20) {
            city.step(0.05, &mut rng);
            for (k, window) in city.windows.iter().enumerate() {
                let now = city.light(window);
                if now != last[k] {
                    changes[k].push(i as f32 * 0.05);
                    last[k] = now;
                }
            }
        }
        for times in &changes {
            // dark once, then at most a candle, seconds later
            assert!(times.len() <= 2, "{times:?}");
            if let [a, b] = times[..] {
                assert!(b - a >= 1.0, "{times:?}");
            }
        }
        assert!(changes.iter().any(|t| !t.is_empty()));
    }

    #[test]
    fn a_resize_keeps_how_dark_the_city_is() {
        let mut rng = Rng::new(9);
        let mut city = Blackout::new(80, 24, Motion::FULL, &mut rng);
        for _ in 0..(45 * 20) {
            city.step(0.05, &mut rng);
        }
        let before = city.darkened as f32 / city.dark_since.len() as f32;
        city.resize(120, 40, &mut rng);
        let after = city.darkened as f32 / city.dark_since.len() as f32;
        assert!((before - after).abs() < 0.05, "{before} {after}");
    }

    #[test]
    fn the_layers_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            apart(theme, c.sky.at(0.0), c.sky.at(1.0), 0.03, "the night sky");
            apart(
                theme,
                c.buildings[0].0,
                c.sky.at(1.0),
                0.03,
                "the buildings and the sky",
            );
            apart(
                theme,
                c.glass,
                c.buildings[0].0,
                0.02,
                "dark windows and the walls",
            );
            apart(
                theme,
                c.lit.0,
                c.buildings[0].0,
                0.1,
                "lit windows and the walls",
            );
            apart(theme, c.candle, c.glow, 0.05, "a candle and its glow");
            for colour in c
                .sky
                .steps()
                .iter()
                .chain(c.cloud.steps())
                .chain([&c.glass, &c.glow])
            {
                assert!(
                    theme.contrast_of(*colour) < IMAGERY_CEILING,
                    "{}",
                    theme.name
                );
            }
            let mut solid = vec![c.lit.0, c.lit.1, c.candle, c.spark, c.pole.1];
            if theme.dark {
                solid.push(c.moon);
            } else {
                assert!(
                    theme.contrast_of(c.moon) < IMAGERY_CEILING,
                    "{}",
                    theme.name
                );
                apart(theme, c.moon, c.sky.at(0.0), 0.08, "the moon and the sky");
            }
            for colour in solid {
                assert!(
                    theme.contrast_of(colour) >= paint::SOLID - 0.01,
                    "{}",
                    theme.name
                );
            }
        }
    }
}
