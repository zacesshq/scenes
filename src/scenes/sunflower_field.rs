//! A sunflower field under a warm sky, painted in solid pixels: a sky that warms towards the
//! horizon, a low sun, drifting clouds, two lines of hills, rows of sunflowers at three depths
//! that sway as the wind passes along the field and bees drifting between the heads.
//!
//! The sky, hills, field and the farthest row are tints of the background held below the
//! non-text floor, so they read as background. The nearer petals, stems and the bees keep the
//! theme's own drawing colours, so they stand out as shapes and keep their outline with colour
//! off.

use std::f32::consts::TAU;

use super::paint::{self, Cloud, Wave, stronger};
use super::{count_for, cycle, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Lab, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::terminal::pixel_256;
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "Rows of sunflowers swaying under a warm sky, with bees drifting between the heads";

pub const CAPTIONS: &[&str] = &[
    "Every head turned towards a clean build.",
    "A whole field in bloom. Nothing failed.",
    "The bees found nothing to fix.",
    "Warm sky, tall flowers, no errors.",
    "Green stems from end to end.",
];

/// How fast the wind's sway moves, in radians a second at the standard pace. One sway takes
/// about ten seconds, slow enough that a head shifts by a pixel at a time.
pub(crate) const SWAY_RATE: f32 = 0.6;

/// A bee facing right: wings over a round striped body.
const BEE: [&str; 3] = [".ww..", "ykyky", ".kyk."];

#[derive(Clone, Debug, PartialEq)]
struct Flower {
    x: f32,
    /// The pixel row the stem rises from.
    base: f32,
    /// The stem's length in pixels.
    stem: f32,
    /// The radius of the head in pixels.
    head: f32,
    /// 0 for the farthest row, 2 for the nearest.
    row: usize,
    phase: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct Bee {
    home_x: f32,
    home_y: f32,
    reach_x: f32,
    reach_y: f32,
    rate_x: f32,
    rate_y: f32,
    phase: f32,
    /// Pixels per second the bee's home drifts along the field.
    drift: f32,
}

const SKY_STEPS: usize = 10;
const FIELD_STEPS: usize = 8;

/// The field's colours in one theme, every one derived from the theme's own.
/// Some are kept only so the tests can check the bands against each other.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct FieldColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub halo: Rgb,
    pub sun: Rgb,
    pub cloud: Rgb,
    pub cloud_shade: Rgb,
    pub far_hill: Rgb,
    pub near_hill: Rgb,
    pub field_far: Rgb,
    pub field_near: Rgb,
    pub speck: Rgb,
    pub back_stem: Rgb,
    pub petal: Rgb,
    pub petal_lit: Rgb,
    pub centre: Rgb,
    pub seed: Rgb,
    pub stem: Rgb,
    pub leaf: Rgb,
    pub bee: Rgb,
    pub stripe: Rgb,
    pub wing: Rgb,
    sky: Ramp,
    field: Ramp,
}

impl FieldColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let green = theme.green();
        let yellow = theme.yellow();
        // a warm sky: cool overhead and warming towards the sun on the horizon, darker at the
        // top on a dark theme and paler at the bottom on a light one
        let (sky_top, sky_low, halo) = if theme.dark {
            (
                shades.tint(theme.info, 0.18, 1.45),
                shades.tint(theme.warning, 0.5, 2.3),
                shades.tint(theme.warning, 0.8, 2.85),
            )
        } else {
            (
                shades.tint(theme.info, 0.4, 1.9),
                shades.tint(theme.warning, 0.2, 1.3),
                shades.tint(theme.warning, 0.1, 1.12),
            )
        };
        // a sunlit field leans towards yellow; the near ground is the strongest band
        let sunlit = mix(green, yellow, 0.35);
        // the far hill takes a little of the sky's blue, as distant land does
        let hazy = mix(green, theme.info, 0.4);
        let (far_hill, near_hill, field_far, field_near) = if theme.dark {
            (
                shades.tint(hazy, 0.35, 1.95),
                shades.tint(green, 0.4, 1.3),
                shades.tint(sunlit, 0.45, 1.95),
                shades.tint(sunlit, 0.9, 2.9),
            )
        } else {
            (
                shades.tint(hazy, 0.3, 1.5),
                shades.tint(green, 0.5, 1.8),
                shades.tint(sunlit, 0.6, 2.45),
                shades.tint(sunlit, 0.8, 2.9),
            )
        };
        let (cloud, cloud_shade) = paint::cloud_tones(theme, 2.2);
        let centre = mix(mix(yellow, theme.red(), 0.45), shades.darkest(), 0.45);
        let bee = theme.bright_yellow();
        Self {
            sky_top,
            sky_low,
            halo,
            // on a light theme the sun is the palest thing in the sky; on a dark one it glows
            sun: if theme.dark {
                stronger(theme, yellow, 0.45)
            } else {
                theme.background
            },
            cloud,
            cloud_shade,
            far_hill,
            near_hill,
            field_far,
            field_near,
            speck: shades.tint(yellow, 0.9, 2.9),
            back_stem: shades.tint(green, 0.9, 2.9),
            petal: yellow,
            // the rays catch the light: lighter on every theme, so on a light one they lose a
            // little contrast while the darker tips keep the outline at the floor
            petal_lit: {
                // on a light theme the theme's yellow is dark enough to read as brown, so the lit
                // rays are a clear gold of the same hue instead and the darker rays keep the
                // outline at the floor
                let lit = if theme.dark {
                    shades.lighter(yellow, 0.3)
                } else {
                    gold(theme.bright_yellow())
                };
                let near = |c: Rgb| pixel_256(c, theme.background);
                // where the lit rays would share a 256 colour entry with the near field, they
                // lean towards the bright yellow instead, so they never melt into it
                if near(lit) == near(field_near) {
                    mix(yellow, theme.bright_yellow(), 0.6)
                } else {
                    lit
                }
            },
            centre,
            seed: mix(centre, shades.darkest(), 0.45),
            stem: green,
            leaf: stronger(theme, green, 0.2),
            bee,
            stripe: mix(bee, shades.darkest(), 0.7),
            wing: if theme.dark {
                shades.tint(theme.bright(), 0.6, 2.7)
            } else {
                theme.background
            },
            sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
            field: Ramp::new(field_far, field_near, FIELD_STEPS),
        }
    }
}

/// A bright, saturated gold near the hue of `yellow`, for sunlit petals on a light theme. Some
/// light themes have an amber yellow, so the hue is held between gold and lemon, never orange.
fn gold(yellow: Rgb) -> Rgb {
    let lab = Lab::from_rgb(yellow);
    let hue = lab.b.atan2(lab.a).clamp(1.36, 1.66);
    Lab {
        l: 0.8,
        a: 0.16 * hue.cos(),
        b: 0.16 * hue.sin(),
    }
    .to_rgb()
}

pub struct SunflowerField {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    flowers: Vec<Flower>,
    bees: Vec<Bee>,
    clouds: Vec<Cloud>,
    far_line: Wave,
    near_line: Wave,
}

impl SunflowerField {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut field = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            flowers: Vec::new(),
            bees: Vec::new(),
            clouds: Vec::new(),
            far_line: Wave::new(rng, 14.0),
            near_line: Wave::new(rng, 9.0),
        };
        field.resize(width, height, rng);
        field
    }

    fn tall(&self) -> f32 {
        (self.height * 2) as f32
    }

    /// The pixel row where the sky meets the hills.
    fn horizon(&self) -> i32 {
        let tall = self.height as i32 * 2;
        ((tall as f32 * 0.45).round() as i32).clamp(1, (tall - 1).max(1))
    }

    /// How far a part of a flower at `frac` of the way up its stem leans in the wind.
    fn sway(&self, flower: &Flower, frac: f32) -> f32 {
        let amp = [0.4, 0.9, 1.8][flower.row.min(2)] * if self.motion.calm { 0.6 } else { 1.0 };
        let wind =
            (cycle(self.t, SWAY_RATE * self.motion.pace()) - flower.x * 0.05 + flower.phase).sin();
        amp * wind * frac.clamp(0.0, 1.0).powf(1.5)
    }

    /// The centre of a flower's head right now.
    fn head_at(&self, flower: &Flower) -> (f32, f32) {
        (flower.x + self.sway(flower, 1.0), flower.base - flower.stem)
    }

    fn bee_at(&self, bee: &Bee) -> (f32, f32, bool) {
        let speed = self.motion.speed();
        let ax = cycle(self.t, bee.rate_x * speed) + bee.phase;
        let ay = cycle(self.t, bee.rate_y * speed) + bee.phase * 1.7;
        let x = bee.home_x + bee.reach_x * ax.sin();
        let y = bee.home_y + bee.reach_y * ay.sin();
        let heading = bee.reach_x * bee.rate_x * ax.cos() + bee.drift;
        (x, y, heading >= 0.0)
    }

    fn plant_row(&mut self, rng: &mut Rng, row: usize, base: f32, head: f32, stem: f32) {
        let spacing = head * [3.0, 3.4, 3.7][row];
        let mut x = rng.range(0.0, spacing);
        while x < self.width as f32 + spacing {
            self.flowers.push(Flower {
                x: x + rng.range(-0.3, 0.3) * spacing,
                base,
                stem: (stem * rng.range(0.85, 1.15)).max(1.0),
                head,
                row,
                phase: rng.range(0.0, 0.6),
            });
            x += spacing * rng.range(0.85, 1.15);
        }
    }

    fn paint_head(&self, canvas: &mut Canvas, c: &FieldColours, flower: &Flower) {
        let (hx, hy) = self.head_at(flower);
        let r = flower.head;
        if flower.row == 0 {
            paint::ellipse(canvas, hx, hy, r, r, c.speck);
            return;
        }
        let petals = if flower.row == 2 { 11.0 } else { 8.0 };
        let reach = r + 1.3;
        let (x0, x1) = ((hx - reach).floor() as i32, (hx + reach).ceil() as i32);
        let (y0, y1) = ((hy - reach).floor() as i32, (hy + reach).ceil() as i32);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = x as f32 + 0.5 - hx;
                let dy = y as f32 + 0.5 - hy;
                let d = (dx * dx + dy * dy).sqrt();
                let colour = if d <= r * 0.58 {
                    if paint::hash(x, y, 7) & 3 == 0 {
                        c.seed
                    } else {
                        c.centre
                    }
                } else {
                    let ray = (petals * dy.atan2(dx)).cos();
                    if d <= r * 0.85 {
                        if ray > 0.0 { c.petal_lit } else { c.petal }
                    } else if d <= r || (d <= reach && ray > 0.2) {
                        c.petal
                    } else {
                        continue;
                    }
                };
                canvas.set(x, y, colour);
            }
        }
    }

    fn paint_flower(&self, canvas: &mut Canvas, c: &FieldColours, flower: &Flower) {
        let steps = flower.stem.ceil() as i32;
        let (stem, wide) = match flower.row {
            0 => (c.back_stem, false),
            1 => (c.stem, false),
            _ => (c.stem, flower.head >= 3.0),
        };
        for i in 0..steps {
            let frac = i as f32 / flower.stem;
            let x = (flower.x + self.sway(flower, frac)).round() as i32;
            let y = (flower.base - i as f32).round() as i32;
            canvas.set(x, y, stem);
            if wide {
                canvas.set(x + 1, y, c.leaf);
            }
        }
        if flower.row > 0 && flower.stem >= 5.0 {
            for (k, frac) in [0.3_f32, 0.58].into_iter().enumerate() {
                let side = if (k + flower.x as usize) % 2 == 0 {
                    1
                } else {
                    -1
                };
                let x = (flower.x + self.sway(flower, frac)).round() as i32;
                let y = (flower.base - flower.stem * frac).round() as i32;
                let len = (flower.head * 1.1).round().max(2.0) as i32;
                // a blade that rises from the stem, arches and droops at the tip
                for i in 0..len {
                    let arch = (std::f32::consts::PI * (i as f32 + 0.5) / len as f32).sin();
                    let lift = (arch * len as f32 * 0.3).round() as i32;
                    let px = x + side * (i + 1);
                    canvas.set(px, y - lift, c.leaf);
                    if i > 0 && i + 1 < len {
                        canvas.set(px, y - lift + 1, c.leaf);
                    }
                }
            }
        }
        self.paint_head(canvas, c, flower);
    }

    fn paint(&self, canvas: &mut Canvas, c: &FieldColours) {
        let horizon = self.horizon();
        let tall = canvas.height() as i32;
        let wide = canvas.width() as i32;
        let h = tall as f32;

        paint::gradient(canvas, &c.sky, 0, horizon);
        // a still sun low over the hills: it never changes, so it can never flash
        if horizon >= 8 && wide >= 20 {
            let (sx, sy) = (wide as f32 * 0.76, horizon as f32 - h * 0.1);
            let r = (h * 0.07).clamp(2.0, 6.0);
            paint::ellipse(canvas, sx, sy, r + 2.5, r + 2.5, c.halo);
            paint::ellipse(canvas, sx, sy, r, r, c.sun);
        }
        for cloud in &self.clouds {
            cloud.paint(canvas, c.cloud, c.cloud_shade);
        }

        // two lines of hills, the far one paler and higher
        for x in 0..wide {
            let xf = x as f32;
            let far = horizon as f32 - (1.5 + h * 0.07) * (0.5 + 0.5 * self.far_line.at(xf));
            let near = horizon as f32 - (1.0 + h * 0.035) * (0.5 + 0.5 * self.near_line.at(xf));
            for y in far.round() as i32..horizon {
                canvas.set(x, y, c.far_hill);
            }
            for y in near.round() as i32..horizon {
                canvas.set(x, y, c.near_hill);
            }
        }

        // the field deepens in colour towards the front, with a scatter of distant heads in
        // rows that fade into the haze
        let span = (tall - horizon).max(1) as f32;
        for y in horizon..tall {
            let t = (y - horizon) as f32 / span;
            canvas.fill_row(y, c.field.at(t));
            // tufts of a step lighter or darker give the near ground some grain
            if t > 0.3 {
                for x in 0..wide {
                    let n = paint::hash(x, y, 11) % 9;
                    if n == 0 {
                        canvas.set(x, y, c.field.at(t - 0.2));
                    } else if n == 1 {
                        canvas.set(x, y, c.field.at(t + 0.2));
                    }
                }
            }
            if (y - horizon) % 2 == 1 && t < 0.4 {
                for x in 0..wide {
                    if paint::unit(x, y, 3) < 0.25 + t {
                        canvas.set(x, y, c.speck);
                    }
                }
            }
        }

        for flower in self.flowers.iter().filter(|f| f.row < 2) {
            self.paint_flower(canvas, c, flower);
        }
        for bee in &self.bees {
            let (x, y, right) = self.bee_at(bee);
            paint::sprite_facing(
                canvas,
                x.round() as i32,
                y.round() as i32,
                &BEE,
                right,
                &[('y', c.bee), ('k', c.stripe), ('w', c.wing)],
            );
        }
        for flower in self.flowers.iter().filter(|f| f.row == 2) {
            self.paint_flower(canvas, c, flower);
        }
    }
}

impl Scene for SunflowerField {
    fn name(&self) -> &'static str {
        "sunflower-field"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let h = self.tall();
        let horizon = self.horizon() as f32;
        let ground = h - horizon;
        let scale = (h / 48.0).clamp(0.5, 3.0);
        self.flowers.clear();
        if h >= 16.0 {
            self.plant_row(
                rng,
                0,
                horizon + ground * 0.1,
                (1.0 * scale).max(0.8),
                h * 0.04,
            );
            self.plant_row(
                rng,
                1,
                horizon + ground * 0.3,
                (2.0 * scale).max(1.3),
                h * 0.1,
            );
        }
        self.plant_row(rng, 2, h + 1.0, (4.2 * scale).max(1.6), h * 0.34);

        self.bees.clear();
        if h >= 16.0 {
            let count = self
                .motion
                .scaled(count_for(self.width, self.height, 700, 1, 5));
            for _ in 0..count {
                self.bees.push(Bee {
                    home_x: rng.range(0.0, self.width as f32),
                    home_y: rng.range(horizon - h * 0.16, horizon + ground * 0.25),
                    reach_x: rng.range(3.0, 8.0),
                    reach_y: rng.range(1.0, 2.5),
                    rate_x: rng.range(0.4, 0.8),
                    rate_y: rng.range(0.7, 1.2),
                    phase: rng.range(0.0, TAU),
                    drift: rng.range(-1.0, 1.0),
                });
            }
        }

        self.clouds.clear();
        if h >= 20.0 {
            for _ in 0..2 {
                self.clouds.push(Cloud::random(
                    rng,
                    self.width as f32,
                    (h * 0.12, h * 0.28),
                    (10.0 * scale.max(0.8), 20.0 * scale.max(0.8)),
                    self.motion.speed(),
                ));
            }
        }
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
        let width = self.width as f32;
        let speed = self.motion.speed();
        for bee in &mut self.bees {
            bee.home_x = wrap(bee.home_x + bee.drift * speed * dt + 10.0, width + 20.0) - 10.0;
        }
        for cloud in &mut self.clouds {
            cloud.drift(dt, width);
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = FieldColours::new(theme);
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
        check_scene("sunflower-field");
        check_determinism("sunflower-field");
    }

    #[test]
    fn passes_the_pixel_checks_in_every_theme() {
        check_pixel_scene("sunflower-field", 6.0);
    }

    #[test]
    fn the_colours_hold_in_every_theme() {
        for theme in theme::all() {
            let c = FieldColours::new(theme);
            assert_imagery(
                theme,
                &[
                    ("sky top", c.sky_top),
                    ("sky low", c.sky_low),
                    ("halo", c.halo),
                    ("cloud", c.cloud),
                    ("cloud shade", c.cloud_shade),
                    ("far hill", c.far_hill),
                    ("near hill", c.near_hill),
                    ("far field", c.field_far),
                    ("near field", c.field_near),
                    ("distant heads", c.speck),
                    ("distant stems", c.back_stem),
                    ("wing", c.wing),
                ],
            );
            assert_shapes(
                theme,
                &[("petal", c.petal), ("stem", c.stem), ("bee", c.bee)],
                &[c.field_near, c.field_far, c.sky_low, c.sky_top],
            );
            assert_apart(
                theme,
                &[
                    ("the sky's top and bottom", c.sky_top, c.sky_low),
                    ("the sky and the far hill", c.sky_low, c.far_hill),
                    ("the two hills", c.far_hill, c.near_hill),
                    ("the near hill and the field", c.near_hill, c.field_far),
                    ("distant heads and the field", c.speck, c.field_far),
                    ("the cloud and the sky", c.cloud, c.sky_top),
                    ("the petals and the centre", c.petal, c.centre),
                ],
            );
            // the field is one gradient, so its ends need only look different in full colour
            assert!(
                distance(c.field_far, c.field_near) >= 0.03,
                "{}",
                theme.name
            );
        }
    }

    #[test]
    fn the_sway_is_slow_and_the_roots_stay_put() {
        const { assert!(TAU / SWAY_RATE >= 8.0) };
        let mut rng = Rng::new(5);
        let mut field = SunflowerField::new(80, 24, Motion::LIVELY, &mut rng);
        let roots: Vec<(f32, f32)> = field.flowers.iter().map(|f| (f.x, f.base)).collect();
        let mut leaned = 0.0_f32;
        for _ in 0..400 {
            field.step(0.05, &mut rng);
            for flower in &field.flowers {
                let (hx, _) = field.head_at(flower);
                leaned = leaned.max((hx - flower.x).abs());
                assert!((hx - flower.x).abs() <= 1.81);
                assert_eq!(field.sway(flower, 0.0), 0.0);
            }
        }
        assert!(leaned > 1.0, "the front row never leans");
        let after: Vec<(f32, f32)> = field.flowers.iter().map(|f| (f.x, f.base)).collect();
        assert_eq!(roots, after);
        assert!(field.flowers.iter().any(|f| f.row == 0));
        assert!(field.flowers.iter().any(|f| f.row == 2));
    }

    #[test]
    fn bees_stay_over_the_field_and_flowers_show_in_every_theme() {
        let mut rng = Rng::new(8);
        let mut field = SunflowerField::new(80, 24, Motion::FULL, &mut rng);
        assert!(!field.bees.is_empty());
        let h = field.tall();
        for _ in 0..1200 {
            field.step(0.05, &mut rng);
            for bee in &field.bees {
                let (x, y, _) = field.bee_at(bee);
                assert!(y > h * 0.2 && y < h * 0.8, "{y}");
                assert!(x > -20.0 && x < 100.0, "{x}");
            }
        }
        for theme in theme::all() {
            let c = FieldColours::new(theme);
            let mut frame = Frame::new(80, 24);
            field.draw(&mut frame, theme);
            let all = pixels(&frame);
            for (what, colour) in [
                ("petal", c.petal),
                ("centre", c.centre),
                ("stem", c.stem),
                ("sky", c.sky_top),
            ] {
                assert!(all.contains(&colour), "no {what} in {}", theme.name);
            }
        }
    }
}
