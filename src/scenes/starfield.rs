//! A night sky painted in solid pixels: a faint band of the galaxy, stars that brighten and fade
//! slowly, a crescent moon and, now and then, a slow shooting star.
//!
//! A light theme shows the same sky at dawn: deep blue overhead warming to gold at the horizon,
//! the galaxy gone, a pale morning moon and the last few stars, soft and white, fading out
//! towards the light. A low line of hills closes the bottom of the sky in every theme.
//!
//! The sky and the galaxy's band are tints of the background held below the non-text floor, so
//! they read as background. The stars, the moon and the shooting star keep the theme's own
//! drawing colours, so they stand out as shapes and keep their outline when colour is off. A
//! bright star grows from a point to a small cross as it brightens, so its brightness is a
//! change of shape as well as of colour.

use std::f32::consts::TAU;

use super::{count_for, cycle, paint};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// The quickest a star may complete one bright to dim cycle, in seconds, so nothing twinkles
/// faster than once every two and a half seconds.
pub const MIN_TWINKLE_PERIOD: f32 = 2.5;
const SKY_STEPS: usize = 8;
const TRAIL: usize = 8;

#[derive(Clone, Debug)]
struct Star {
    x: i32,
    /// The pixel row.
    y: i32,
    phase: f32,
    period: f32,
    bright: bool,
    tint: usize,
}

#[derive(Clone, Debug)]
struct Shooting {
    x: f32,
    /// The pixel row of the head.
    y: f32,
    vx: f32,
    /// Pixels a second downwards.
    vy: f32,
    age: f32,
}

/// The colours of the night sky in one theme, every one derived from the theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct SkyColours {
    pub sky_top: Rgb,
    pub sky_mid: Rgb,
    pub sky_low: Rgb,
    pub galaxy: Rgb,
    pub moon: Rgb,
    pub halo: Rgb,
    pub star: Rgb,
    pub warm_star: Rgb,
    pub cool_star: Rgb,
    pub faint_star: Rgb,
    pub head: Rgb,
    pub hill: Rgb,
    pub hill_far: Rgb,
    /// Whether this is the dawn sky of a light theme, where the stars are pale imagery.
    pub dawn: bool,
    upper: Ramp,
    lower: Ramp,
    /// The galaxy's band from nothing to its brightest, over the sky's middle.
    band: Ramp,
    /// The shooting star's trail from its head back to the sky.
    trail: Ramp,
}

/// Pure white, the last stars and the morning moon at dawn: a touch above an off-white
/// background, so they stay imagery while showing against the blue.
const WHITE: Rgb = Rgb::new(255, 255, 255);

impl SkyColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        if !theme.dark {
            return Self::dawn(theme);
        }
        let sky_top = shades.tint(theme.info, 0.08, 1.15);
        let sky_low = shades.tint(theme.info, 0.32, 1.6);
        let sky_mid = mix(sky_top, sky_low, 0.5);
        let galaxy = shades.tint(mix(theme.magenta(), theme.foreground, 0.5), 0.45, 1.9);
        let star = theme.foreground;
        let head = theme.bright();
        // the hills are the night itself, darker than the sky behind them
        let hill = mix(theme.background, theme.green(), 0.04);
        let hill_far = shades.tint(mix(theme.info, theme.green(), 0.5), 0.12, 1.25);
        Self {
            sky_top,
            sky_mid,
            sky_low,
            galaxy,
            moon: theme.bright_yellow(),
            halo: shades.tint(theme.bright_yellow(), 0.18, IMAGERY_CEILING - 0.6),
            star,
            warm_star: theme.bright_yellow(),
            cool_star: theme.bright_cyan(),
            faint_star: theme.muted,
            head,
            hill,
            hill_far,
            dawn: false,
            upper: Ramp::new(sky_top, sky_mid, SKY_STEPS),
            lower: Ramp::new(sky_mid, sky_low, SKY_STEPS),
            band: Ramp::new(sky_mid, galaxy, 4),
            trail: Ramp::new(head, sky_mid, TRAIL),
        }
    }

    /// Dawn: blue overhead, a soft lilac and then gold low down, a pale moon and a few last
    /// white stars, with hills dark enough to keep the outline.
    fn dawn(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let sky_top = shades.tint(theme.info, 0.8, 2.5);
        let sky_mid = shades.tint(mix(theme.info, theme.magenta(), 0.45), 0.4, 1.7);
        let sky_low = shades.tint(mix(theme.warning, theme.yellow(), 0.3), 0.35, 1.3);
        let star = WHITE;
        let soft = mix(WHITE, sky_top, 0.35);
        // the near hills are only just solid, a soft blue green in the early light
        let hill = paint::strong(
            theme,
            shades.tint(mix(theme.info, theme.green(), 0.5), 1.0, 2.9),
        );
        let hill_far = shades.tint(mix(theme.info, theme.magenta(), 0.3), 0.55, 2.1);
        Self {
            sky_top,
            sky_mid,
            sky_low,
            galaxy: sky_mid,
            moon: WHITE,
            halo: mix(WHITE, sky_top, 0.6),
            star,
            warm_star: mix(WHITE, theme.yellow(), 0.12),
            cool_star: mix(WHITE, theme.info, 0.12),
            faint_star: soft,
            head: WHITE,
            hill,
            hill_far,
            dawn: true,
            upper: Ramp::new(sky_top, sky_mid, SKY_STEPS),
            lower: Ramp::new(sky_mid, sky_low, SKY_STEPS),
            band: Ramp::new(sky_mid, sky_mid, 2),
            trail: Ramp::new(WHITE, sky_top, TRAIL),
        }
    }

    /// The sky's colour at a height from 0 at the top to 1 at the horizon.
    fn sky_at(&self, t: f32) -> Rgb {
        if t < 0.55 {
            self.upper.at(t / 0.55)
        } else {
            self.lower.at((t - 0.55) / 0.45)
        }
    }

    /// A star's colour dimmed `t` of the way towards the sky, never so far that it loses its
    /// outline at night; at dawn a star is imagery and simply softens.
    fn dimmed(&self, theme: &ThemeData, colour: Rgb, t: f32) -> Rgb {
        if self.dawn {
            return mix(colour, self.sky_top, t * 0.8);
        }
        Shades::new(theme).held(colour, self.sky_top, t, IMAGERY_CEILING + 0.2)
    }
}

const MOON_RADIUS: f32 = 3.5;

pub struct Starfield {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    stars: Vec<Star>,
    shooting: Option<Shooting>,
    next_shooting: f32,
    shooting_seen: u32,
    /// The galaxy band's slope and offset, fixed for each size.
    band: (f32, f32),
    /// The phases of the two hill lines.
    hill_phase: (f32, f32),
}

impl Starfield {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut sky = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            stars: Vec::new(),
            shooting: None,
            next_shooting: if motion.calm {
                rng.range(15.0, 30.0)
            } else {
                rng.range(6.0, 16.0)
            },
            shooting_seen: 0,
            band: (0.0, 0.0),
            hill_phase: (0.0, 0.0),
        };
        sky.resize(width, height, rng);
        sky
    }

    /// Seconds until the next shooting star once one has gone.
    fn shooting_wait(&self, rng: &mut Rng) -> f32 {
        if self.motion.calm {
            rng.range(20.0, 40.0)
        } else {
            rng.range(8.0, 20.0)
        }
    }

    fn moon_fits(&self) -> bool {
        self.width >= 24 && self.height >= 8
    }

    /// The moon's centre in pixels.
    fn moon(&self) -> (f32, f32) {
        (self.width as f32 - 7.0, MOON_RADIUS + 2.5)
    }

    #[cfg(test)]
    pub(crate) fn shooting_seen(&self) -> u32 {
        self.shooting_seen
    }

    #[cfg(test)]
    pub(crate) fn shooting_speed(&self) -> Option<f32> {
        // the speed in cells, a pixel row being half a cell
        self.shooting.as_ref().map(|s| s.vx.hypot(s.vy / 2.0))
    }

    /// The pixel row where the sky meets the far hills.
    fn horizon(&self) -> i32 {
        let tall = self.height as i32 * 2;
        if self.height >= 6 {
            (tall as f32 * 0.86) as i32
        } else {
            tall
        }
    }

    /// Two low lines of hills along the bottom: the far one paler, the near one with a few
    /// small trees on its crest.
    fn paint_hills(&self, canvas: &mut Canvas, c: &SkyColours) {
        if self.height < 6 {
            return;
        }
        let tall = canvas.height() as i32;
        let horizon = self.horizon() as f32;
        let (p, q) = self.hill_phase;
        for x in 0..canvas.width() as i32 {
            let xf = x as f32;
            let far = horizon - 2.0 + 2.5 * (xf / 13.0 + p).sin() + 1.2 * (xf / 5.0 + q).sin();
            canvas.fill_rect(x, far.round() as i32, 1, tall, c.hill_far);
            let near = horizon + 2.0 + 1.8 * (xf / 9.0 + q).sin() + 0.8 * (xf / 4.0 + p).cos();
            let mut top = near.round() as i32;
            // a small pine every so often along the near crest: a point over a wider base
            let pine = |at: i32| paint::hash(at, 0, 41) % 9 == 0;
            if pine(x) {
                top -= 4;
            } else if pine(x - 1) || pine(x + 1) {
                top -= 2;
            }
            canvas.fill_rect(x, top, 1, tall, c.hill);
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &SkyColours, theme: &ThemeData) {
        let tall = canvas.height() as i32;
        let wide = canvas.width() as i32;
        let horizon = self.horizon();
        for y in 0..tall {
            canvas.fill_row(y, c.sky_at(y as f32 / horizon.max(1) as f32));
        }

        // the galaxy: a soft diagonal band of fixed speckle, so it never moves or flickers
        let (slope, offset) = self.band;
        let width = (tall as f32 * 0.22).max(2.0);
        let band_rows = if c.dawn { 0 } else { tall };
        for y in 0..band_rows {
            for x in 0..wide {
                let d = (y as f32 - (offset + slope * x as f32)).abs() / width;
                if d >= 1.0 {
                    continue;
                }
                let h = (x as u32).wrapping_mul(0x9e37_79b1) ^ (y as u32).wrapping_mul(0x85eb_ca77);
                let speckle = ((h >> 9) % 100) as f32 / 100.0;
                let level = (1.0 - d) * (0.55 + 0.45 * speckle);
                if level > 0.3 {
                    canvas.set(x, y, c.band.at((level - 0.3) / 0.6));
                }
            }
        }

        for star in &self.stars {
            // at dawn only the stars high up in the deepest blue are still out
            if c.dawn && (!star.bright || (star.y as f32) > horizon as f32 * 0.45) {
                continue;
            }
            let level = ((cycle(self.t, TAU / star.period) + star.phase).sin() + 1.0) / 2.0;
            if star.bright {
                let colour = match star.tint {
                    0 => c.warm_star,
                    3 => c.cool_star,
                    _ => c.star,
                };
                // a point, a brighter point, then a small cross at its brightest
                if level < 0.33 {
                    canvas.set(star.x, star.y, c.dimmed(theme, colour, 0.45));
                } else {
                    canvas.set(star.x, star.y, colour);
                }
                if level >= 0.66 {
                    let arm = c.dimmed(theme, colour, 0.5);
                    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        canvas.set(star.x + dx, star.y + dy, arm);
                    }
                }
            } else {
                let colour = if level < 0.5 {
                    c.dimmed(theme, c.faint_star, 0.4)
                } else {
                    c.faint_star
                };
                canvas.set(star.x, star.y, colour);
            }
        }

        if self.moon_fits() {
            // a crescent: the moon's disc with a second, offset disc of sky taken out of it,
            // inside a faint halo
            let (cx, cy) = self.moon();
            let r = MOON_RADIUS;
            let reach = r as i32 + 2;
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    let (x, y) = (cx.round() as i32 + dx, cy.round() as i32 + dy);
                    let lit = (x as f32 - cx).powi(2) + (y as f32 - cy).powi(2);
                    let cut = (x as f32 - cx - 1.6).powi(2) + (y as f32 - cy + 1.0).powi(2);
                    if lit <= r * r && cut > (r * 0.85).powi(2) {
                        canvas.set(x, y, c.moon);
                    } else if lit <= (r + 1.0).powi(2) && cut > (r * 0.85 - 1.0).powi(2) {
                        canvas.set(x, y, c.halo);
                    }
                }
            }
        }

        self.paint_hills(canvas, c);

        if let Some(star) = &self.shooting {
            for k in (1..TRAIL).rev() {
                let back = k as f32 * 0.04;
                let x = (star.x - star.vx * back).round() as i32;
                let y = (star.y - star.vy * back).round() as i32;
                canvas.set(x, y, c.trail.at(k as f32 / TRAIL as f32));
            }
            canvas.set(star.x.round() as i32, star.y.round() as i32, c.head);
        }
    }
}

impl Scene for Starfield {
    fn name(&self) -> &'static str {
        "starfield"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = count_for(self.width, self.height, 12, 6, 600);
        let (low, high) = if self.motion.calm {
            (MIN_TWINKLE_PERIOD * 2.0, 10.0)
        } else {
            (MIN_TWINKLE_PERIOD, 6.0)
        };
        let tall = self.height * 2;
        self.stars = (0..wanted)
            .map(|i| Star {
                x: rng.below(self.width) as i32,
                y: rng.below(tall) as i32,
                phase: rng.range(0.0, TAU),
                period: rng.range(low, high),
                bright: rng.chance(0.4),
                tint: i % 7,
            })
            .collect();
        let slope = -(tall as f32) / (self.width as f32).max(1.0) * rng.range(0.5, 0.9);
        self.band = (slope, tall as f32 * rng.range(0.7, 1.0));
        self.hill_phase = (rng.range(0.0, TAU), rng.range(0.0, TAU));
        // a shooting star cut short by a resize counts as gone, so the next waits its full time
        if self.shooting.take().is_some() {
            self.next_shooting = self.shooting_wait(rng);
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        if let Some(star) = &mut self.shooting {
            star.x += star.vx * dt;
            star.y += star.vy * dt;
            star.age += dt;
            let gone = star.age > 2.5
                || star.x < -6.0
                || star.x > self.width as f32 + 6.0
                || star.y > self.height as f32 * 2.0 + 2.0;
            if gone {
                self.shooting = None;
                self.next_shooting = self.shooting_wait(rng);
            }
        } else if self.width >= 10 {
            self.next_shooting -= dt;
            if self.next_shooting <= 0.0 {
                let speed = self.motion.speed();
                let leftwards = rng.chance(0.5);
                let vx = if leftwards { -14.0 } else { 14.0 } * speed;
                self.shooting = Some(Shooting {
                    x: rng.range(self.width as f32 * 0.25, self.width as f32 * 0.85),
                    y: rng.range(0.0, (self.height as f32 * 0.6).max(1.0)),
                    vx,
                    vy: 10.0 * speed,
                    age: 0.0,
                });
                self.shooting_seen += 1;
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = SkyColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours, theme);
        canvas.blit(frame);
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

    #[test]
    fn passes_the_shared_checks() {
        check_scene("starfield");
        check_determinism("starfield");
        check_pixel_scene("starfield", 6.0);
    }

    #[test]
    fn the_sky_and_its_lights_stay_distinct_in_every_theme() {
        for theme in theme::all() {
            let c = SkyColours::new(theme);
            let name = theme.name;
            assert!(distance(c.sky_top, c.sky_low) >= 0.03, "{name}");
            assert!(distance(c.hill, c.hill_far) >= 0.03, "{name}");
            assert!(distance(c.hill_far, c.sky_low) >= 0.03, "{name}");
            for (what, colour) in [
                ("sky top", c.sky_top),
                ("sky mid", c.sky_mid),
                ("sky low", c.sky_low),
                ("galaxy", c.galaxy),
                ("halo", c.halo),
                ("far hill", c.hill_far),
            ] {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            let lights = [
                c.moon,
                c.star,
                c.warm_star,
                c.cool_star,
                c.faint_star,
                c.head,
            ];
            if theme.dark {
                assert!(
                    distance(c.galaxy, mix(c.sky_top, c.sky_low, 0.5)) >= 0.03,
                    "{name}"
                );
                assert!(theme.contrast_of(c.hill) < IMAGERY_CEILING, "{name}");
                for colour in lights {
                    assert!(theme.contrast_of(colour) >= theme.floor(), "{name}");
                    assert!(distance(colour, c.sky_top) >= 0.1, "{name}");
                    assert!(distance(colour, c.galaxy) >= 0.1, "{name}");
                    // a star at its dimmest still keeps an outline
                    let dim = c.dimmed(theme, colour, 0.45);
                    assert!(theme.contrast_of(dim) >= IMAGERY_CEILING, "{name}");
                }
            } else {
                // at dawn the stars and the moon are pale imagery against the blue overhead,
                // and the near hills keep the outline
                assert!(theme.contrast_of(c.hill) >= IMAGERY_CEILING, "{name}");
                for colour in lights {
                    assert!(theme.contrast_of(colour) < IMAGERY_CEILING, "{name}");
                    assert!(distance(colour, c.sky_top) >= 0.1, "{name}");
                }
            }
        }
    }

    #[test]
    fn stars_twinkle_slowly_and_the_moon_hangs_top_right() {
        let c = SkyColours::new(theme::default_theme());
        let mut rng = Rng::new(9);
        let mut sky = Starfield::new(60, 20, Motion::FULL, &mut rng);
        assert!(sky.stars.iter().all(|s| s.period >= MIN_TWINKLE_PERIOD));
        let frame = render(&mut sky, 60, 20, 10);
        let all = pixels(&frame);
        assert!(all.contains(&c.star) || all.contains(&c.warm_star));
        // the moon sits in the top right cells
        let mut moon = Vec::new();
        for y in 0..5 {
            for x in 45..60 {
                let (a, b) = frame.get(x, y).unwrap().pixel_pair().unwrap();
                moon.extend([a, b]);
            }
        }
        assert!(moon.iter().filter(|&&p| p == c.moon).count() >= 6);
        let mut small = Starfield::new(20, 8, Motion::FULL, &mut rng);
        assert!(!small.moon_fits());
        let small = render(&mut small, 20, 8, 10);
        let lights = pixels(&small)
            .iter()
            .filter(|&&p| theme::default_theme().contrast_of(p) >= IMAGERY_CEILING)
            .count();
        assert!(lights >= 6, "{lights}");
    }

    #[test]
    fn a_bright_star_grows_into_a_cross_and_shrinks_back() {
        let theme = theme::default_theme();
        let c = SkyColours::new(theme);
        let mut rng = Rng::new(9);
        let mut sky = Starfield::new(60, 20, Motion::FULL, &mut rng);
        sky.stars = vec![Star {
            x: 10,
            y: 10,
            phase: 0.0,
            period: MIN_TWINKLE_PERIOD,
            bright: true,
            tint: 1,
        }];
        let mut sizes = std::collections::BTreeSet::new();
        for _ in 0..60 {
            sky.step(0.05, &mut rng);
            let mut canvas = Canvas::new(60, 20, theme.background);
            sky.paint(&mut canvas, &c, theme);
            let lit = [(10, 10), (9, 10), (11, 10), (10, 9), (10, 11)]
                .iter()
                .filter(|&&(x, y)| theme.contrast_of(canvas.get(x, y).unwrap()) >= IMAGERY_CEILING)
                .count();
            sizes.insert(lit);
        }
        assert!(sizes.contains(&1) && sizes.contains(&5), "{sizes:?}");
    }

    #[test]
    fn a_shooting_star_crosses_the_sky_and_is_slower_when_calm() {
        let mut rng = Rng::new(31);
        let mut sky = Starfield::new(80, 24, Motion::FULL, &mut rng);
        let mut fastest = 0.0_f32;
        for _ in 0..(40 * 20) {
            sky.step(0.05, &mut rng);
            if let Some(speed) = sky.shooting_speed() {
                fastest = fastest.max(speed);
            }
        }
        assert!(sky.shooting_seen() >= 1);
        assert!(fastest > 10.0, "{fastest}");

        let mut calm = Starfield::new(80, 24, Motion::CALM, &mut rng);
        let mut calm_fastest = 0.0_f32;
        for _ in 0..(90 * 20) {
            calm.step(0.05, &mut rng);
            if let Some(speed) = calm.shooting_speed() {
                calm_fastest = calm_fastest.max(speed);
            }
        }
        assert!(calm.shooting_seen() >= 1);
        assert!(calm_fastest < fastest, "{calm_fastest} vs {fastest}");
    }
}
