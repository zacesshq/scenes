//! A row of windmills over open fields, painted in solid pixels: a sky with clouds drifting
//! across it, low hills on the horizon and a patchwork of fields that widens towards the front,
//! furrowed and split by hedges. Tower mills stand in a row from near to far, each with a red
//! cap and four latticed sails turning slowly.
//!
//! The sky, clouds, hills, fields and hedges are tints of the background held below the
//! non-text floor, so they read as background. The towers, caps and sails keep the theme's own
//! drawing colours, so they stand out as shapes and keep their outline with colour off.

use std::f32::consts::{FRAC_PI_2, TAU};

use super::cycle;
use super::paint::{self, Cloud, Wave, stronger};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, distance, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "A row of windmills turning slowly over a patchwork of fields as clouds pass overhead";

pub const CAPTIONS: &[&str] = &[
    "Steady wind, steady build.",
    "Every sail turning. Everything passing.",
    "The mills grind on. Nothing to fix.",
    "Fair weather over the fields.",
    "Round and round, all green.",
];

/// How fast the sails turn, in radians a second at the standard speed: one turn takes about
/// sixteen seconds, so a sail tip never sweeps faster than a few pixels a second.
pub(crate) const SAIL_TURN: f32 = 0.4;
/// The fastest a sail tip may sweep, in pixels a second at the standard speed. A big mill on a
/// large screen turns more slowly to keep to it.
pub(crate) const MAX_TIP_SPEED: f32 = 6.0;

/// The crops a field can hold.
const CROPS: usize = 4;
/// Bands of fields from the horizon to the front.
const BANDS: usize = 6;
const SKY_STEPS: usize = 10;

#[derive(Clone, Debug, PartialEq)]
struct Mill {
    x: f32,
    /// The pixel row the tower stands on.
    base: f32,
    /// The tower's height in pixels.
    tall: f32,
    turn: f32,
    phase: f32,
}

impl Mill {
    /// The centre of the sails, just in front of the cap.
    fn hub(&self) -> (f32, f32) {
        (self.x, self.base - self.tall - self.tall * 0.06)
    }

    fn sail_length(&self) -> f32 {
        self.tall * 0.62
    }
}

/// The countryside's colours in one theme, every one derived from the theme's own.
/// Some are kept only so the tests can check the bands against each other.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct MillColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub cloud: Rgb,
    pub cloud_shade: Rgb,
    pub hills: Rgb,
    pub hedge: Rgb,
    /// Grass, wheat, ploughed earth and flowers, each with its furrow shade.
    pub crops: [(Rgb, Rgb); CROPS],
    pub tower: Rgb,
    pub tower_lit: Rgb,
    pub door: Rgb,
    pub cap: Rgb,
    pub sail: Rgb,
    pub cloth: Rgb,
    sky: Ramp,
}

impl MillColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let green = theme.green();
        let earth = mix(theme.yellow(), theme.red(), 0.5);
        let hazy = mix(green, theme.info, 0.45);
        let crop = |hue: Rgb, strength: f32, ceiling: f64| {
            // a furrow is a step away from the field's own tint, darker on either kind of theme
            let field = shades.tint(hue, strength, ceiling);
            let furrow = if theme.dark {
                shades.tint(hue, strength * 0.6, (ceiling - 0.45).max(1.15))
            } else {
                shades.tint(hue, (strength * 1.5).min(1.0), (ceiling + 0.5).min(2.9))
            };
            (field, furrow)
        };
        let (sky_top, sky_low, hills, hedge, crops) = if theme.dark {
            (
                shades.tint(theme.info, 0.25, 1.5),
                shades.tint(theme.info, 0.55, 2.3),
                shades.tint(hazy, 0.4, 1.7),
                shades.tint(green, 0.35, 1.35),
                [
                    crop(green, 0.55, 2.1),
                    crop(theme.yellow(), 0.6, 2.5),
                    crop(earth, 0.45, 1.75),
                    crop(theme.magenta(), 0.5, 2.0),
                ],
            )
        } else {
            (
                shades.tint(theme.info, 0.45, 1.9),
                shades.tint(theme.info, 0.15, 1.2),
                shades.tint(hazy, 0.3, 1.45),
                shades.tint(green, 0.8, 2.8),
                [
                    crop(green, 0.45, 1.8),
                    crop(theme.yellow(), 0.4, 1.6),
                    crop(earth, 0.5, 2.1),
                    crop(theme.magenta(), 0.35, 1.7),
                ],
            )
        };
        let (cloud, cloud_shade) = paint::cloud_tones(theme, 2.2);
        // where the muted ink sits close to the text colour, as on Warm Dark, the tower steps
        // towards the background, held at the floor, so the sails still stand apart from it
        let tower = if distance(theme.muted, theme.foreground) < 0.1 {
            shades.held(theme.muted, theme.background, 0.5, theme.floor())
        } else {
            theme.muted
        };
        let sail = theme.foreground;
        Self {
            sky_top,
            sky_low,
            cloud,
            cloud_shade,
            hills,
            hedge,
            crops,
            tower,
            tower_lit: stronger(theme, tower, 0.4),
            door: mix(tower, shades.darkest(), 0.6),
            cap: theme.red(),
            sail,
            // the cloth is pale canvas stretched over the lattice, light enough to read as
            // background so the frame carries the outline
            cloth: if theme.dark {
                shades.tint(theme.bright(), 0.55, 2.6)
            } else {
                shades.tint(tower, 0.25, 1.5)
            },
            sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
        }
    }
}

pub struct Windmills {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    mills: Vec<Mill>,
    clouds: Vec<Cloud>,
    /// For each band, the columns where one field ends and the next begins, at the band's top,
    /// and the crop of each field.
    fields: Vec<(Vec<f32>, Vec<usize>)>,
    hills: Wave,
}

impl Windmills {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut mills = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            mills: Vec::new(),
            clouds: Vec::new(),
            fields: Vec::new(),
            hills: Wave::new(rng, 12.0),
        };
        mills.resize(width, height, rng);
        mills
    }

    fn tall(&self) -> f32 {
        (self.height * 2) as f32
    }

    /// The pixel row of the horizon.
    fn horizon(&self) -> i32 {
        let tall = self.height as i32 * 2;
        ((tall as f32 * 0.5).round() as i32).clamp(1, (tall - 1).max(1))
    }

    /// The top row of each band of fields, then the bottom of the screen. Bands widen towards
    /// the front, as fields do in perspective.
    fn band_rows(&self) -> Vec<i32> {
        let horizon = self.horizon() as f32;
        let ground = self.tall() - horizon;
        (0..=BANDS)
            .map(|k| (horizon + ground * (k as f32 / BANDS as f32).powf(1.5)).round() as i32)
            .collect()
    }

    /// How far a mill's sails have turned.
    fn angle(&self, mill: &Mill) -> f32 {
        let rate = (SAIL_TURN * mill.turn).min(MAX_TIP_SPEED / mill.sail_length().max(1.0));
        cycle(self.t, rate * self.motion.speed()) + mill.phase
    }

    fn paint_fields(&self, canvas: &mut Canvas, c: &MillColours) {
        let horizon = self.horizon();
        let wide = canvas.width() as i32;
        let centre = wide as f32 / 2.0;
        let rows = self.band_rows();
        let ground = (self.tall() - horizon as f32).max(1.0);
        for (k, (splits, crops)) in self.fields.iter().enumerate() {
            let (top, bottom) = (rows[k], rows[k + 1]);
            // farther bands fade towards the hills, as distance does
            let haze = 0.5 * (1.0 - k as f32 / BANDS as f32);
            let tones: Vec<(Rgb, Rgb)> = c
                .crops
                .iter()
                .map(|&(a, b)| (mix(a, c.hills, haze), mix(b, c.hills, haze)))
                .collect();
            for y in top..bottom {
                // field edges fan out from a vanishing point in the middle of the horizon
                let spread = 1.0 + 1.6 * (y - horizon) as f32 / ground;
                let depth = (y - horizon + 2) as f32;
                for x in 0..wide {
                    let along = centre + (x as f32 - centre) / spread;
                    let field = splits.iter().take_while(|&&s| s <= along).count();
                    let crop = crops[field.min(crops.len() - 1)];
                    let (base, furrow) = tones[crop];
                    // furrows run towards the vanishing point too
                    let lane = ((x as f32 - centre) / depth * 7.0).floor() as i32;
                    // grass is speckled rather than furrowed
                    let furrowed = if crop == 0 {
                        paint::hash(x, y, 19) % 7 == 0
                    } else {
                        lane.rem_euclid(2) == 0
                    };
                    let colour = if furrowed { furrow } else { base };
                    canvas.set(x, y, colour);
                }
            }
            // a hedge along the top of every band but the farthest, with the odd tree in it
            if k > 0 {
                for x in 0..wide {
                    canvas.set(x, top, c.hedge);
                    if paint::hash(x, top, 29) % 23 == 0 {
                        paint::ellipse(canvas, x as f32, top as f32 - 0.5, 1.6, 1.6, c.hedge);
                    }
                }
            }
        }
    }

    fn paint_mill(&self, canvas: &mut Canvas, c: &MillColours, mill: &Mill) {
        let top = mill.base - mill.tall;
        let bottom_half = (mill.tall * 0.2).max(1.0);
        let top_half = (mill.tall * 0.11).max(0.6);
        // a tapering tower, lit on the left
        for y in top.round() as i32..mill.base.round() as i32 {
            let t = (y as f32 - top) / mill.tall;
            let half = top_half + (bottom_half - top_half) * t;
            let (left, right) = (
                (mill.x - half).round() as i32,
                (mill.x + half).round() as i32,
            );
            for x in left..=right {
                let lit = (x as f32) < mill.x - half * 0.2;
                canvas.set(x, y, if lit { c.tower_lit } else { c.tower });
            }
        }
        if mill.tall >= 10.0 {
            let door_h = (mill.tall * 0.14).round().max(2.0) as i32;
            let base = mill.base.round() as i32;
            canvas.fill_rect(mill.x.round() as i32 - 1, base - door_h, 2, door_h, c.door);
            let window = (top + mill.tall * 0.4).round() as i32;
            canvas.set(mill.x.round() as i32, window, c.door);
        }
        // the cap: a rounded hood over the top of the tower
        let cap_h = (mill.tall * 0.14).max(1.0);
        let cap_w = top_half * 1.5 + 0.5;
        paint::ellipse(canvas, mill.x, top, cap_w, cap_h, c.cap);
        canvas.fill_rect(
            (mill.x - cap_w).round() as i32,
            top.round() as i32,
            (cap_w * 2.0).round() as i32 + 1,
            1,
            c.cap,
        );

        let (hx, hy) = mill.hub();
        let length = mill.sail_length();
        let angle = self.angle(mill);
        for arm in 0..4 {
            let a = angle + arm as f32 * FRAC_PI_2;
            let (ux, uy) = (a.cos(), a.sin());
            // the cloth sits on the trailing side of each spar
            let (vx, vy) = (-uy, ux);
            if length >= 5.0 {
                let wide = (length * 0.22).max(1.0);
                let mut s = length * 0.22;
                while s <= length {
                    let mut q = 0.5;
                    while q <= wide {
                        // a frame round the cloth, with lattice bars only on a sail long enough
                        // to show them: on a shorter one they break up into stray pixels
                        let lattice = length >= 12.0 && (s / 3.0).fract() < 0.17;
                        // a short sail is too small for a frame to read, so it is solid
                        let bar = length < 12.0 || lattice || q > wide - 0.5 || s > length - 0.5;
                        let (x, y) = (hx + s * ux + q * vx, hy + s * uy + q * vy);
                        canvas.set(
                            x.floor() as i32,
                            y.floor() as i32,
                            if bar { c.sail } else { c.cloth },
                        );
                        q += 0.5;
                    }
                    s += 0.5;
                }
            }
            paint::line(
                canvas,
                (hx, hy),
                (hx + length * ux, hy + length * uy),
                1,
                c.sail,
            );
        }
        canvas.set(hx.floor() as i32, hy.floor() as i32, c.cap);
    }

    fn paint(&self, canvas: &mut Canvas, c: &MillColours) {
        let horizon = self.horizon();
        let wide = canvas.width() as i32;
        let h = self.tall();
        paint::gradient(canvas, &c.sky, 0, horizon);
        for cloud in &self.clouds {
            cloud.paint(canvas, c.cloud, c.cloud_shade);
        }
        for x in 0..wide {
            let lift = (1.0 + h * 0.04) * (0.5 + 0.5 * self.hills.at(x as f32));
            for y in (horizon as f32 - lift).round() as i32..horizon {
                canvas.set(x, y, c.hills);
            }
        }
        self.paint_fields(canvas, c);
        // the farthest mill first, so nearer ones stand in front
        for mill in self.mills.iter().rev() {
            self.paint_mill(canvas, c, mill);
        }
    }
}

impl Scene for Windmills {
    fn name(&self) -> &'static str {
        "windmills"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let h = self.tall();
        let wide = self.width as f32;
        let horizon = self.horizon() as f32;

        self.fields.clear();
        for k in 0..BANDS {
            let mut splits = Vec::new();
            let mut crops = vec![rng.below(CROPS)];
            // nearer bands have wider fields
            let mut x = rng.range(-10.0, 4.0);
            while x < wide + 10.0 {
                x += rng.range(8.0, 22.0) * (0.6 + k as f32 * 0.15);
                splits.push(x);
                let last = *crops.last().unwrap_or(&0);
                crops.push((last + 1 + rng.below(CROPS - 1)) % CROPS);
            }
            self.fields.push((splits, crops));
        }

        self.mills.clear();
        if h >= 12.0 && wide >= 16.0 {
            let count = (self.width / 26).clamp(2, 5);
            for i in 0..count {
                let d = i as f32 / (count - 1).max(1) as f32;
                let s = 1.0 - 0.55 * d;
                let base = h * 0.88 - (h * 0.88 - (horizon + 3.0)) * d;
                self.mills.push(Mill {
                    x: wide * (0.16 + 0.7 * d) + rng.range(-2.0, 2.0),
                    base,
                    tall: (h * 0.38 * s).max(5.0),
                    turn: rng.range(0.85, 1.15),
                    phase: rng.range(0.0, TAU),
                });
            }
        }

        self.clouds.clear();
        if h >= 16.0 {
            let count = (self.width / 30).clamp(1, 5);
            for _ in 0..count {
                self.clouds.push(Cloud::random(
                    rng,
                    wide,
                    (h * 0.08, h * 0.3),
                    (8.0, 20.0),
                    self.motion.speed(),
                ));
            }
        }
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
        let wide = self.width as f32;
        for cloud in &mut self.clouds {
            cloud.drift(dt, wide);
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = MillColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixels::distance;
    use crate::scenes::testing::{
        assert_apart, assert_imagery, assert_shapes, check_determinism, check_pixel_scene,
        check_scene, pixels,
    };
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("windmills");
        check_determinism("windmills");
    }

    #[test]
    fn passes_the_pixel_checks_in_every_theme() {
        check_pixel_scene("windmills", 6.0);
    }

    #[test]
    fn the_colours_hold_in_every_theme() {
        for theme in theme::all() {
            let c = MillColours::new(theme);
            let mut imagery = vec![
                ("sky top", c.sky_top),
                ("sky low", c.sky_low),
                ("cloud", c.cloud),
                ("cloud shade", c.cloud_shade),
                ("hills", c.hills),
                ("hedge", c.hedge),
            ];
            for &(field, furrow) in &c.crops {
                imagery.push(("field", field));
                imagery.push(("furrow", furrow));
            }
            assert_imagery(theme, &imagery);
            assert_shapes(
                theme,
                &[
                    ("tower", c.tower),
                    ("lit tower", c.tower_lit),
                    ("cap", c.cap),
                    ("sail", c.sail),
                ],
                &[c.sky_top, c.sky_low, c.crops[0].0, c.crops[1].0],
            );
            assert_imagery(theme, &[("sail cloth", c.cloth)]);
            assert_apart(
                theme,
                &[
                    ("the sky and the hills", c.sky_low, c.hills),
                    ("the cloud and the sky", c.cloud, c.sky_top),
                    ("grass and wheat", c.crops[0].0, c.crops[1].0),
                    ("wheat and earth", c.crops[1].0, c.crops[2].0),
                    ("grass and flowers", c.crops[0].0, c.crops[3].0),
                ],
            );
            for &(field, furrow) in &c.crops {
                assert!(distance(field, furrow) >= 0.03, "{}", theme.name);
            }
            assert!(distance(c.sail, c.tower) >= 0.08, "{}", theme.name);
            assert!(distance(c.tower, c.tower_lit) >= 0.03, "{}", theme.name);
        }
    }

    #[test]
    fn the_sails_turn_slowly_and_the_mills_stand_in_a_row() {
        // a sail tip on the biggest mill a full screen could hold never sweeps fast
        const { assert!(TAU / SAIL_TURN >= 15.0) };
        let mut rng = Rng::new(2);
        let mut mills = Windmills::new(120, 40, Motion::LIVELY, &mut rng);
        assert!(mills.mills.len() >= 3);
        for pair in mills.mills.windows(2) {
            assert!(pair[0].tall > pair[1].tall, "nearer mills are taller");
            assert!(pair[0].base > pair[1].base, "nearer mills stand lower");
            assert!(pair[0].x < pair[1].x);
        }
        let start: Vec<f32> = mills.mills.iter().map(|m| mills.angle(m)).collect();
        for _ in 0..20 {
            mills.step(0.05, &mut rng);
        }
        for (mill, a) in mills.mills.iter().zip(start) {
            let turned = (mills.angle(mill) - a).rem_euclid(TAU);
            let tip = turned * mill.sail_length();
            assert!(turned > 0.0, "a sail stood still");
            assert!(
                tip <= MAX_TIP_SPEED * 1.35 + 0.01,
                "a tip swept {tip} pixels in a second"
            );
        }
        let theme = theme::default_theme();
        let c = MillColours::new(theme);
        let mut frame = Frame::new(120, 40);
        mills.draw(&mut frame, theme);
        let all = pixels(&frame);
        for (what, colour) in [("tower", c.tower), ("cap", c.cap), ("sail", c.sail)] {
            assert!(all.contains(&colour), "no {what}");
        }
    }
}
