//! A desert night in solid pixels: a deep sky full of stars that brighten and dim slowly, a
//! moon rising from behind the dunes over a few minutes, dunes in three ranges lit on the side
//! facing it, a caravan of camels walking the far ridge, a cactus in the foreground and, now and
//! then, a slow shooting star.
//!
//! A light theme shows the same desert at sunrise instead: a dusty blue sky warming to apricot
//! at the horizon, a pale sun climbing where the moon would, golden dunes and the camels and the
//! cactus dark against the morning light.
//!
//! The sky, the dunes and the moon's halo are tints of the background held below the non-text
//! floor; the moon, the bright stars, the camels, the cactus and the shooting star keep the
//! theme's own drawing colours, so they stand out as shapes and keep their outline when colour is
//! off.

use super::paint::{Blender, disc, tint};
use super::{cycle, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "Desert dunes under a starry sky, a moon rising slowly and camels walking the far ridge, at sunrise in light themes";
pub const CAPTIONS: &[&str] = &[
    "A clear night and a clean build.",
    "Not a cloud. Not an error.",
    "The caravan moves on. So can you.",
];

/// How long the moon takes to rise to its height at the standard pace.
pub const MOON_SECONDS: f32 = 150.0;
/// One slow brighten and dim of a star. Each star changes between two shades twice a cycle.
pub const STAR_PERIOD: f32 = 7.0;
/// One step of a camel's legs.
pub const STRIDE_PERIOD: f32 = 1.2;
/// The shortest time between shooting stars.
const SHOOTING_GAP: f32 = 25.0;

/// A camel walking right: a tail, a single tall hump, a long neck curving up to a head held
/// forward and long legs.
const CAMEL: [&str; 9] = [
    "..........##.",
    "....###...###",
    "...#####..#..",
    "..#######.#..",
    ".##########..",
    "#.#########..",
    "..##....##...",
    "..#.#...#.#..",
    "..#.#...#.#..",
];
/// The same camel mid stride.
const CAMEL_STEP: [&str; 9] = [
    "..........##.",
    "....###...###",
    "...#####..#..",
    "..#######.#..",
    ".##########..",
    "#.#########..",
    "..##....##...",
    "..#..#..#..#.",
    ".#...#.#...#.",
];
const CAMEL_WIDTH: f32 = 13.0;

/// A saguaro: a trunk with an arm on each side turned up, its right side in shade.
const CACTUS: [&str; 12] = [
    "....#d....",
    "...##dd...",
    "...###d...",
    ".#.###d...",
    "#d.###d...",
    "#d.###d.#.",
    "#d.###d.#d",
    "######d.#d",
    ".#d#######",
    "...###d...",
    "...###d...",
    "...###d...",
];

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct Colours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub halo: Rgb,
    pub dim_star: Rgb,
    /// Lit and shaded sides of the far, middle and near dunes.
    pub dunes: [(Rgb, Rgb); 3],
    pub moon: Rgb,
    pub star: Rgb,
    pub camel: Rgb,
    pub cactus: Rgb,
    pub cactus_shade: Rgb,
    /// Whether this is the night of a dark theme, with stars, rather than a light theme's
    /// sunrise.
    pub night: bool,
    sky: Ramp,
}

/// A warm dark silhouette colour for the camels against a sunrise.
fn shades_far(theme: &ThemeData) -> Rgb {
    mix(theme.foreground, theme.warning, 0.25)
}

impl Colours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let dark = theme.dark;
        let (sky_top, sky_low) = if dark {
            (
                tint(theme, theme.blue(), 0.1, 1.15),
                tint(theme, theme.magenta(), 0.3, 1.6),
            )
        } else {
            (
                tint(theme, mix(theme.blue(), theme.info, 0.5), 0.55, 2.0),
                tint(theme, mix(theme.warning, theme.magenta(), 0.2), 0.5, 1.5),
            )
        };
        let sand = if dark {
            theme.yellow()
        } else {
            mix(theme.yellow(), theme.warning, 0.35)
        };
        // the moonlit face of a dune is lighter than its shade, which leans towards the night
        // blue; on a light theme the shade is the stronger, warmer tint
        let dune = |strength: f32, ceiling: f64| {
            let lit = tint(theme, sand, strength, ceiling);
            let shade = if dark {
                mix(
                    lit,
                    tint(
                        theme,
                        theme.blue(),
                        strength * 0.5,
                        1.0 + (ceiling - 1.0) * 0.45,
                    ),
                    0.7,
                )
            } else {
                tint(theme, theme.warning, strength * 1.5, ceiling + 0.45)
            };
            (lit, shade)
        };
        let (moon, halo, star, dim_star) = if dark {
            (
                theme.bright_yellow(),
                tint(theme, theme.bright_yellow(), 0.5, 2.2),
                theme.foreground,
                tint(theme, theme.foreground, 0.5, 2.3),
            )
        } else {
            // a pale morning sun in a soft glow with no stars left in the sky
            let sun = mix(Rgb::new(255, 252, 238), theme.yellow(), 0.08);
            (sun, mix(sun, sky_low, 0.4), sky_top, sky_top)
        };
        Self {
            sky_top,
            sky_low,
            halo,
            dim_star,
            dunes: if dark {
                [dune(0.28, 1.5), dune(0.45, 1.95), dune(0.65, 2.45)]
            } else {
                [dune(0.5, 1.9), dune(0.6, 2.15), dune(0.75, 2.5)]
            },
            moon,
            star,
            camel: if dark { theme.muted } else { shades_far(theme) },
            cactus: theme.green(),
            cactus_shade: crate::pixels::Shades::new(theme).held(
                theme.green(),
                crate::pixels::Shades::new(theme).darkest(),
                0.35,
                theme.floor(),
            ),
            night: dark,
            sky: Ramp::new(sky_top, sky_low, 10),
        }
    }

    #[cfg(test)]
    pub(crate) fn palette(theme: &ThemeData) -> crate::scenes::testing::Palette {
        let c = Self::new(theme);
        let mut imagery = vec![
            ("sky top", c.sky_top),
            ("sky low", c.sky_low),
            ("halo", c.halo),
            ("dim star", c.dim_star),
        ];
        let mut shapes = vec![
            ("camel", c.camel),
            ("cactus", c.cactus),
            ("cactus shade", c.cactus_shade),
        ];
        if c.night {
            shapes.extend([("moon", c.moon), ("star", c.star)]);
        } else {
            imagery.push(("sun", c.moon));
        }
        for (i, &(lit, shade)) in c.dunes.iter().enumerate() {
            imagery.push((["far dune", "middle dune", "near dune"][i], lit));
            imagery.push((["far shade", "middle shade", "near shade"][i], shade));
        }
        crate::scenes::testing::Palette { imagery, shapes }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Shooting {
    x: f32,
    y: f32,
    age: f32,
}

pub struct DesertNight {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// The moon's own clock, run at the motion speed.
    moon_t: f32,
    stars: Vec<(i32, i32, f32)>,
    caravan_x: f32,
    shooting: Option<Shooting>,
    next_shooting: f32,
}

impl DesertNight {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut night = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            // the moon has just cleared the dunes when the scene opens
            moon_t: MOON_SECONDS * 0.15,
            stars: Vec::new(),
            caravan_x: 0.0,
            shooting: None,
            next_shooting: rng.range(SHOOTING_GAP, SHOOTING_GAP * 1.6),
        };
        night.resize(width, height, rng);
        night
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The top pixel row of a dune range at a column: long, smooth swells, each range lower and
    /// nearer than the last.
    fn dune_top(&self, range: usize, x: i32) -> f32 {
        let tall = self.tall() as f32;
        let (base, height, length) =
            [(0.62, 0.09, 52.0), (0.77, 0.11, 40.0), (0.93, 0.13, 31.0)][range.min(2)];
        let xf = x as f32 + range as f32 * 11.0;
        // each dune rises gently to a sharp crest and drops away more steeply beyond it
        let wave = (xf / length).fract();
        let shape = if wave < 0.65 {
            let rise = wave / 0.65;
            rise * rise * (3.0 - 2.0 * rise)
        } else {
            let fall = (wave - 0.65) / 0.35;
            1.0 - fall.powf(0.75)
        };
        let swell = 0.65 + 0.35 * (xf / length * 0.37 + 1.3).sin();
        tall * (base - height * shape * swell)
    }

    fn dune_length(&self, range: usize) -> f32 {
        [52.0, 40.0, 31.0][range.min(2)]
    }

    /// The column of the crest of the dune a column belongs to, at or left of it.
    fn crest(&self, range: usize, x: i32) -> i32 {
        let length = self.dune_length(range);
        let shift = range as f32 * 11.0;
        let xf = x as f32 + shift;
        let mut at = ((xf / length).floor() + 0.65) * length;
        if at > xf {
            at -= length;
        }
        (at - shift).round() as i32
    }

    /// The moon's centre: it rises from behind the dunes on a slow arc and then stays.
    fn moon(&self) -> (f32, f32) {
        let rise = (self.moon_t / MOON_SECONDS).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - rise).powi(2);
        let (w, tall) = (self.width as f32, self.tall() as f32);
        (w * (0.72 - 0.1 * eased), tall * (0.62 - 0.42 * eased))
    }

    /// Whether a star is at its brighter shade at time `t`: each star brightens and dims on its
    /// own phase of the same slow cycle.
    fn star_bright(&self, phase: f32, t: f64) -> bool {
        let angle = cycle(t, std::f32::consts::TAU / STAR_PERIOD * self.motion.pace());
        (angle + phase).sin() > 0.0
    }

    fn stride(&self) -> bool {
        (self.t as f32 * self.motion.pace() / STRIDE_PERIOD).fract() < 0.5
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        let mut blender = Blender::new(4);
        let (wide, tall) = (canvas.width() as i32, canvas.height() as i32);
        let horizon = (tall as f32 * 0.6) as i32;
        for y in 0..tall {
            canvas.fill_row(y, c.sky.at(y as f32 / horizon.max(1) as f32));
        }
        let stars: &[(i32, i32, f32)] = if c.night { &self.stars } else { &[] };
        for &(x, y, phase) in stars {
            let colour = if self.star_bright(phase, self.t) {
                c.star
            } else {
                c.dim_star
            };
            canvas.set(x, y, colour);
        }
        if let Some(shot) = self.shooting.as_ref().filter(|_| c.night) {
            for i in 0..4 {
                let (x, y) = (shot.x - i as f32 * 1.5, shot.y - i as f32 * 0.5);
                canvas.set(x.round() as i32, y.round() as i32, c.star);
            }
        }

        let (mx, my) = self.moon();
        let r = (tall as f32 / 12.0).max(1.0);
        if wide >= 16 && tall >= 10 {
            let reach = (r * 2.2).ceil() as i32;
            for y in my as i32 - reach..=my as i32 + reach {
                for x in mx as i32 - reach..=mx as i32 + reach {
                    let d = ((x as f32 - mx).powi(2) + (y as f32 - my).powi(2)).sqrt();
                    if d <= r * 2.0 {
                        blender.blend(canvas, x, y, c.halo, (1.0 - d / (r * 2.0)) * 0.9);
                    }
                }
            }
            disc(canvas, mx, my, r, c.moon);
        }

        for range in 0..3 {
            let (lit, shade) = c.dunes[range];
            for x in 0..wide {
                let top = self.dune_top(range, x);
                // each dune's steep slip face lies in shadow in a wedge under its crest, so the
                // dune is shaded on its slope rather than split top to bottom
                let crest_x = self.crest(range, x);
                let crest = self.dune_top(range, crest_x);
                let slip = x > crest_x && x - crest_x < self.dune_length(range) as i32 / 2;
                let wedge = if slip { (top - crest) * 0.9 } else { 0.0 };
                let from = (top.round() as i32).max(0);
                for y in from..tall {
                    let colour = if (y as f32) < top + wedge { shade } else { lit };
                    canvas.set(x, y, colour);
                }
            }
            if range == 0 && wide >= 30 && tall >= 16 {
                // the caravan walks the far ridge, three camels nose to tail
                let step = self.stride();
                for k in 0..3 {
                    let x = self.caravan_x - k as f32 * (CAMEL_WIDTH + 2.0);
                    let px = x.round() as i32;
                    let ground = self.dune_top(0, px + 6).round() as i32 + 1;
                    let rows: Vec<String> = if (k % 2 == 0) == step {
                        CAMEL.iter().map(|r| r.to_string()).collect()
                    } else {
                        CAMEL_STEP.iter().map(|r| r.to_string()).collect()
                    };
                    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
                    canvas.sprite(
                        px,
                        ground - CAMEL.len() as i32 + 1,
                        &rows,
                        &[('#', c.camel)],
                    );
                }
            }
        }
        if wide >= 40 && tall >= 24 {
            let x = wide / 7;
            let ground = self.dune_top(2, x + 4).round() as i32 + 2;
            canvas.sprite(
                x,
                ground - CACTUS.len() as i32,
                &CACTUS,
                &[('#', c.cactus), ('d', c.cactus_shade)],
            );
        }
    }
}

impl Scene for DesertNight {
    fn name(&self) -> &'static str {
        "desert-night"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let horizon = (self.tall() as f32 * 0.55) as usize;
        let count = (self.width * horizon / 45).clamp(3, 300);
        self.stars = (0..count)
            .map(|_| {
                (
                    rng.below(self.width) as i32,
                    rng.below(horizon.max(1)) as i32,
                    rng.range(0.0, std::f32::consts::TAU),
                )
            })
            .collect();
        self.caravan_x = rng.range(0.0, self.width as f32);
        self.shooting = None;
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let moved = dt * self.motion.speed();
        self.moon_t += moved;
        // the caravan walks slowly off one side and comes back in at the other
        let span = self.width as f32 + 3.0 * (CAMEL_WIDTH + 2.0);
        self.caravan_x = wrap(self.caravan_x + 0.7 * moved, span);

        if let Some(shot) = &mut self.shooting {
            shot.age += moved;
            shot.x += 9.0 * moved;
            shot.y += 3.0 * moved;
            if shot.age > 1.6 {
                self.shooting = None;
            }
        } else if !self.motion.calm {
            self.next_shooting -= dt;
            if self.next_shooting <= 0.0 {
                self.next_shooting = rng.range(SHOOTING_GAP, SHOOTING_GAP * 1.8);
                let tall = self.tall() as f32;
                self.shooting = Some(Shooting {
                    x: rng.range(0.0, self.width as f32 * 0.6),
                    y: rng.range(0.0, tall * 0.25),
                    age: 0.0,
                });
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
    use crate::pixels::distance;
    use crate::scenes::testing::{check_determinism, check_scene};
    use crate::scenes::testing::{check_palette, check_pixel_scene, shortest_gap};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("desert-night");
        check_determinism("desert-night");
    }

    #[test]
    fn passes_the_pixel_checks() {
        check_pixel_scene("desert-night", 6.0);
        check_palette(Colours::palette);
    }

    #[test]
    fn the_night_stays_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            assert!(distance(c.sky_top, c.sky_low) >= 0.03, "{}", theme.name);
            assert!(distance(c.halo, c.sky_low) >= 0.03, "{}", theme.name);
            for (lit, shade) in c.dunes {
                assert!(distance(lit, shade) >= 0.03, "{}", theme.name);
            }
            assert!(
                distance(c.dunes[0].0, c.sky_low) >= 0.03,
                "the far dunes and the sky in {}",
                theme.name
            );
        }
        assert!(CAMEL.iter().chain(&CAMEL_STEP).all(|r| r.len() == 13));
        assert!(CACTUS.iter().all(|r| r.len() == 10));
    }

    #[test]
    fn the_moon_rises_then_holds_and_stars_change_slowly() {
        let mut rng = Rng::new(3);
        let mut night = DesertNight::new(80, 24, Motion::FULL, &mut rng);
        let (_, start) = night.moon();
        for _ in 0..(200 * 20) {
            night.step(0.05, &mut rng);
        }
        let (_, risen) = night.moon();
        assert!(risen < start - 10.0, "{start} then {risen}");
        night.step(0.05, &mut rng);
        assert_eq!(night.moon().1, risen);
        for motion in Motion::ALL {
            let probe = DesertNight::new(80, 24, motion, &mut rng);
            let gap = shortest_gap(60.0, |t| u32::from(probe.star_bright(0.3, t)));
            assert!(gap >= STAR_PERIOD / 2.0 - 0.1, "{motion:?}: {gap}");
        }
    }
}
