//! Paper boats on a stream, painted in solid pixels: a sky over a far bank of trees, a stream
//! that is pale where it reflects the sky and deeper towards the near bank, ripples carried
//! along by the current, folded paper boats of several colours drifting downstream at three
//! distances and reeds along both banks that sway in the breeze, the near ones in front of the
//! water.
//!
//! The sky, banks, trees, water and far reeds are tints of the background held below the
//! non-text floor, so they read as background. The boats and the near reeds keep the theme's
//! own drawing colours, so they stand out as shapes and keep their outline with colour off.

use std::f32::consts::TAU;

use super::paint::{self, Cloud, Wave, stronger};
use super::{count_for, cycle};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, distance, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "Folded paper boats drifting down a stream past swaying reeds, the current carrying them on";

pub const CAPTIONS: &[&str] = &[
    "Everything floats. Clean build.",
    "Downstream and nothing leaking.",
    "Folded neatly, sailing fine.",
    "The current carries a passing build.",
    "Not a single boat took on water.",
];

/// How fast the current carries the ripples and the nearest boats, in pixels a second at the
/// standard speed.
pub(crate) const FLOW: f32 = 2.4;
/// How fast a boat bobs, in radians a second at the standard pace: once every six seconds or
/// so, a pixel up and down.
pub(crate) const BOB_RATE: f32 = 1.0;
/// How fast the reeds sway, in radians a second at the standard pace.
pub(crate) const REED_RATE: f32 = 0.7;

/// Paper boats seen from the side, facing right. `a` is the lit side of the sail, `b` its fold
/// in shadow, `h` the lit hull and `H` the hull's shaded side.
const BOAT_SMALL: [&str; 4] = ["...a...", "..aab..", "hhhHHHH", ".hhHHH."];
const BOAT_MEDIUM: [&str; 5] = [
    "....a....",
    "...aab...",
    "h.aaabb.H",
    "hhhhHHHHH",
    ".hhhHHHH.",
];
const BOAT_LARGE: [&str; 7] = [
    "......a......",
    ".....aab.....",
    "....aaabb....",
    "h..aaaabbb..H",
    "hhhhhhHHHHHHH",
    ".hhhhhHHHHHH.",
    "..hhhhHHHHH..",
];

#[derive(Clone, Debug, PartialEq)]
struct Boat {
    x: f32,
    /// 0 for the far lane and the smallest boat, 2 for the nearest.
    lane: usize,
    paper: usize,
    phase: f32,
}

impl Boat {
    fn rows(&self) -> &'static [&'static str] {
        match self.lane {
            0 => &BOAT_SMALL,
            1 => &BOAT_MEDIUM,
            _ => &BOAT_LARGE,
        }
    }

    fn width(&self) -> f32 {
        self.rows()[0].len() as f32
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Reed {
    x: f32,
    height: f32,
    head: bool,
    phase: f32,
}

const SKY_STEPS: usize = 6;
const WATER_STEPS: usize = 8;
const LANE_SPEED: [f32; 3] = [0.55, 0.75, 1.0];
const LANE_DEPTH: [f32; 3] = [0.3, 0.58, 0.9];
const PAPERS: usize = 5;

/// The stream's colours in one theme, every one derived from the theme's own.
/// Some are kept only so the tests can check the bands against each other.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct StreamColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub cloud: Rgb,
    pub cloud_shade: Rgb,
    pub far_bank: Rgb,
    pub trees: Rgb,
    pub tree_shade: Rgb,
    pub water_far: Rgb,
    pub water_near: Rgb,
    pub ripple: Rgb,
    pub near_bank: Rgb,
    pub tuft: Rgb,
    pub far_reed: Rgb,
    pub reed: Rgb,
    pub cattail: Rgb,
    pub papers: [Rgb; PAPERS],
    pub folds: [Rgb; PAPERS],
    sky: Ramp,
    water: Ramp,
}

impl StreamColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let green = theme.green();
        let (sky_top, sky_low, far_bank, trees, tree_shade) = if theme.dark {
            (
                shades.tint(theme.info, 0.2, 1.45),
                shades.tint(theme.info, 0.45, 2.05),
                shades.tint(green, 0.35, 1.55),
                shades.tint(green, 0.5, 2.0),
                shades.tint(green, 0.3, 1.3),
            )
        } else {
            (
                shades.tint(theme.info, 0.4, 1.8),
                shades.tint(theme.info, 0.15, 1.2),
                shades.tint(green, 0.3, 1.5),
                shades.tint(green, 0.55, 2.2),
                shades.tint(green, 0.75, 2.75),
            )
        };
        let (water_far, water_near, ripple, near_bank, tuft, far_reed) = if theme.dark {
            (
                shades.tint(theme.info, 0.5, 2.15),
                shades.tint(theme.blue(), 0.45, 1.4),
                shades.tint(theme.bright(), 0.5, 2.75),
                shades.tint(green, 0.6, 2.25),
                shades.tint(green, 0.85, 2.8),
                shades.tint(green, 0.7, 2.55),
            )
        } else {
            (
                shades.tint(theme.info, 0.2, 1.35),
                shades.tint(theme.blue(), 0.5, 2.2),
                theme.background,
                shades.tint(green, 0.45, 1.9),
                shades.tint(green, 0.7, 2.6),
                shades.tint(green, 0.75, 2.65),
            )
        };
        let (cloud, cloud_shade) = paint::cloud_tones(theme, 2.2);
        // white paper on a dark theme; on a light one the theme's text colour would read as
        // ink, not paper, so a fifth colour takes its place
        let papers = [
            if theme.dark {
                theme.foreground
            } else {
                theme.bright_blue()
            },
            theme.yellow(),
            theme.cyan(),
            theme.magenta(),
            theme.red(),
        ];
        let folds = papers.map(|paper| {
            let lit = stronger(theme, paper, 0.4);
            // paper already at the theme's strongest colour folds into its muted grey instead
            if distance(lit, paper) < 0.06 {
                mix(paper, theme.muted, 0.5)
            } else {
                lit
            }
        });
        Self {
            sky_top,
            sky_low,
            cloud,
            cloud_shade,
            far_bank,
            trees,
            tree_shade,
            water_far,
            water_near,
            ripple,
            near_bank,
            tuft,
            far_reed,
            reed: green,
            cattail: mix(theme.yellow(), theme.red(), 0.45),
            papers,
            folds,
            sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
            water: Ramp::new(water_far, water_near, WATER_STEPS),
        }
    }
}

pub struct PaperBoats {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    boats: Vec<Boat>,
    far_reeds: Vec<Reed>,
    near_reeds: Vec<Reed>,
    /// Trees on the far bank as (column, canopy radius).
    trees: Vec<(f32, f32)>,
    clouds: Vec<Cloud>,
    far_edge: Wave,
    near_edge: Wave,
}

impl PaperBoats {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut stream = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            boats: Vec::new(),
            far_reeds: Vec::new(),
            near_reeds: Vec::new(),
            trees: Vec::new(),
            clouds: Vec::new(),
            far_edge: Wave::new(rng, 11.0),
            near_edge: Wave::new(rng, 13.0),
        };
        stream.resize(width, height, rng);
        stream
    }

    fn tall(&self) -> f32 {
        (self.height * 2) as f32
    }

    fn scale(&self) -> f32 {
        (self.tall() / 48.0).clamp(0.5, 3.0)
    }

    /// The pixel row where the sky meets the far bank.
    fn sky_end(&self) -> i32 {
        ((self.tall() * 0.22).round() as i32).max(1)
    }

    /// The far and near edges of the stream in a column.
    fn banks(&self, x: f32) -> (i32, i32) {
        let h = self.tall();
        let far = h * 0.38 + 1.2 * self.far_edge.at(x);
        let near = h * 0.8 + 1.2 * self.near_edge.at(x);
        (far.round() as i32, near.round() as i32)
    }

    /// The pixel row a boat's waterline sits on, the hull's last row.
    fn waterline(&self, boat: &Boat) -> f32 {
        let (far, near) = self.banks(boat.x + boat.width() / 2.0);
        far as f32 + (near - far) as f32 * LANE_DEPTH[boat.lane]
    }

    /// The largest boat that fits the screen, so short screens get only small ones.
    fn biggest_lane(&self) -> usize {
        let (far, near) = self.banks(0.0);
        let water = near - far;
        if water >= 16 {
            2
        } else if water >= 10 {
            1
        } else {
            0
        }
    }

    fn launch(&self, rng: &mut Rng, lane: usize, anywhere: bool) -> Boat {
        let mut boat = Boat {
            x: 0.0,
            lane,
            paper: rng.below(PAPERS),
            phase: rng.range(0.0, TAU),
        };
        boat.x = if anywhere {
            rng.range(-boat.width(), self.width as f32)
        } else {
            -boat.width() - rng.range(2.0, 20.0)
        };
        boat
    }

    fn sway(&self, reed: &Reed, frac: f32) -> f32 {
        let calm = if self.motion.calm { 0.6 } else { 1.0 };
        let wind =
            (cycle(self.t, REED_RATE * self.motion.pace()) + reed.x * 0.07 + reed.phase).sin();
        1.2 * calm * wind * frac.powf(1.5)
    }

    fn paint_reed(&self, canvas: &mut Canvas, reed: &Reed, base: i32, stem: Rgb, head: Rgb) {
        let steps = reed.height.ceil() as i32;
        for i in 0..steps {
            let frac = i as f32 / reed.height;
            let x = (reed.x + self.sway(reed, frac)).round() as i32;
            canvas.set(x, base - i, stem);
        }
        if reed.head && steps >= 6 {
            // a cattail sits a little below the tip of its stem
            let top = steps - 2;
            let x = (reed.x + self.sway(reed, 0.85)).round() as i32;
            let long = (reed.height * 0.3).round().max(2.0) as i32;
            for i in 0..long {
                canvas.set(x, base - top + i, head);
                if reed.height >= 14.0 {
                    canvas.set(x + 1, base - top + i, head);
                }
            }
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &StreamColours) {
        let tall = canvas.height() as i32;
        let wide = canvas.width() as i32;
        let sky_end = self.sky_end();
        paint::gradient(canvas, &c.sky, 0, sky_end);
        for cloud in &self.clouds {
            cloud.paint(canvas, c.cloud, c.cloud_shade);
        }
        for y in sky_end..tall {
            canvas.fill_row(y, c.far_bank);
        }
        for &(x, r) in &self.trees {
            // a round canopy lit from the left, on a short trunk standing on the bank
            let cy = sky_end as f32 - r * 0.5;
            paint::ellipse(canvas, x, cy, r, r, c.tree_shade);
            paint::ellipse(
                canvas,
                x - r * 0.25,
                cy - r * 0.2,
                r * 0.75,
                r * 0.75,
                c.trees,
            );
            for y in (cy + r * 0.6) as i32..=sky_end + 1 {
                canvas.set(x as i32, y, c.tree_shade);
            }
        }

        // the stream, pale where it mirrors the sky and deeper towards the near bank, with
        // ripples the current carries downstream
        let speed = self.motion.speed();
        let drift = cycle(self.t, FLOW * 0.35 * speed);
        let slow = cycle(self.t, FLOW * 0.08 * speed);
        let threshold = if self.motion.calm { 1.6 } else { 1.5 };
        for x in 0..wide {
            let (far, near) = self.banks(x as f32);
            let span = (near - far).max(1) as f32;
            for y in far..near.min(tall) {
                let t = (y - far) as f32 / span;
                let (xf, yf) = (x as f32, y as f32);
                let v = (xf * 0.35 - drift + yf * 1.3).sin()
                    + 0.7 * (xf * 0.08 - slow - yf * 0.6 + 1.0).sin();
                let colour = if v > threshold && y > far {
                    c.ripple
                } else {
                    c.water.at(t)
                };
                canvas.set(x, y, colour);
            }
            for y in near..tall {
                let tuft = paint::hash(x, y, 13) % 7 == 0;
                canvas.set(x, y, if tuft { c.tuft } else { c.near_bank });
            }
        }

        for reed in &self.far_reeds {
            let (far, _) = self.banks(reed.x);
            self.paint_reed(canvas, reed, far, c.far_reed, c.far_reed);
        }

        let pace = self.motion.pace();
        let mut boats: Vec<&Boat> = self.boats.iter().collect();
        boats.sort_by_key(|b| b.lane);
        for boat in boats {
            let rows = boat.rows();
            let bob = ((cycle(self.t, BOB_RATE * pace) + boat.phase).sin() * 0.6).round() as i32;
            let bottom = self.waterline(boat).round() as i32 + bob;
            let x = boat.x.round() as i32;
            let top = bottom - rows.len() as i32 + 1;
            let width = rows[0].len() as i32;
            // a darker shadow under the hull and a short wake trailing upstream
            for dx in 1..width - 1 {
                canvas.set(x + dx, bottom + 1, c.water_near);
            }
            for i in 1..=(width / 3) {
                canvas.set(x - i * 2, bottom + (i % 2), c.ripple);
            }
            let paper = c.papers[boat.paper % PAPERS];
            let fold = c.folds[boat.paper % PAPERS];
            canvas.sprite(
                x,
                top,
                rows,
                &[('a', paper), ('b', fold), ('h', paper), ('H', fold)],
            );
        }

        for reed in &self.near_reeds {
            let (_, near) = self.banks(reed.x);
            self.paint_reed(canvas, reed, near + 2, c.reed, c.cattail);
        }
    }
}

impl Scene for PaperBoats {
    fn name(&self) -> &'static str {
        "paper-boats"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let scale = self.scale();
        let h = self.tall();
        let wide = self.width as f32;

        self.boats.clear();
        let lanes = self.biggest_lane() + 1;
        let count = self
            .motion
            .scaled(count_for(self.width, self.height, 500, 2, 7));
        for i in 0..count {
            let boat = self.launch(rng, i % lanes, true);
            self.boats.push(boat);
        }

        self.trees.clear();
        let mut x = rng.range(0.0, 8.0);
        while x < wide + 6.0 {
            let r = rng.range(2.0, 4.5) * scale;
            self.trees.push((x, r));
            x += r * rng.range(1.6, 3.2);
        }

        self.far_reeds.clear();
        self.near_reeds.clear();
        if h >= 16.0 {
            let mut x = rng.range(0.0, 3.0);
            while x < wide {
                self.far_reeds.push(Reed {
                    x,
                    height: rng.range(2.0, 4.5) * scale,
                    head: false,
                    phase: rng.range(0.0, 0.5),
                });
                x += rng.range(1.5, 4.0);
            }
            // near reeds grow in clumps with gaps between, so the boats show through
            let mut x = rng.range(0.0, 10.0);
            while x < wide {
                let clump = 3 + rng.below(4);
                for _ in 0..clump {
                    self.near_reeds.push(Reed {
                        x: x + rng.range(-3.0, 3.0),
                        height: rng.range(5.0, 13.0) * scale,
                        head: rng.chance(0.45),
                        phase: rng.range(0.0, 0.6),
                    });
                }
                x += rng.range(14.0, 28.0) * scale.max(0.8);
            }
        }

        self.clouds.clear();
        if h >= 24.0 {
            for _ in 0..2 {
                self.clouds.push(Cloud::random(
                    rng,
                    wide,
                    (h * 0.08, h * 0.16),
                    (8.0, 16.0),
                    self.motion.speed(),
                ));
            }
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        let wide = self.width as f32;
        let lanes = self.biggest_lane() + 1;
        for i in 0..self.boats.len() {
            let boat = &mut self.boats[i];
            boat.x += FLOW * LANE_SPEED[boat.lane.min(2)] * speed * dt;
            if boat.x > wide + 2.0 || boat.lane >= lanes {
                let lane = rng.below(lanes);
                self.boats[i] = self.launch(rng, lane, false);
            }
        }
        for cloud in &mut self.clouds {
            cloud.drift(dt, wide);
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = StreamColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{
        assert_apart, assert_imagery, assert_shapes, check_determinism, check_pixel_scene,
        check_scene, pixels,
    };
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("paper-boats");
        check_determinism("paper-boats");
    }

    #[test]
    fn passes_the_pixel_checks_in_every_theme() {
        check_pixel_scene("paper-boats", 6.0);
    }

    #[test]
    fn sprites_are_rectangular() {
        for rows in [&BOAT_SMALL[..], &BOAT_MEDIUM[..], &BOAT_LARGE[..]] {
            assert!(rows.iter().all(|r| r.len() == rows[0].len()), "{rows:?}");
        }
    }

    #[test]
    fn the_colours_hold_in_every_theme() {
        for theme in theme::all() {
            let c = StreamColours::new(theme);
            assert_imagery(
                theme,
                &[
                    ("sky top", c.sky_top),
                    ("sky low", c.sky_low),
                    ("cloud", c.cloud),
                    ("far bank", c.far_bank),
                    ("trees", c.trees),
                    ("tree shade", c.tree_shade),
                    ("far water", c.water_far),
                    ("near water", c.water_near),
                    ("ripple", c.ripple),
                    ("near bank", c.near_bank),
                    ("tuft", c.tuft),
                    ("far reeds", c.far_reed),
                ],
            );
            let mut shapes = vec![("reed", c.reed)];
            for i in 0..PAPERS {
                shapes.push(("paper", c.papers[i]));
                shapes.push(("fold", c.folds[i]));
            }
            assert_shapes(theme, &shapes, &[c.water_far, c.water_near]);
            for i in 0..PAPERS {
                assert!(
                    distance(c.papers[i], c.folds[i]) >= 0.04,
                    "boat {i} has no fold in {}",
                    theme.name
                );
            }
            assert_apart(
                theme,
                &[
                    ("the sky and the trees", c.sky_low, c.trees),
                    ("the far bank and the water", c.far_bank, c.water_far),
                    ("the water and its ripples", c.water_near, c.ripple),
                    ("the near water and the bank", c.water_near, c.near_bank),
                ],
            );
            assert!(
                distance(c.water_far, c.water_near) >= 0.03,
                "{}",
                theme.name
            );
        }
    }

    #[test]
    fn boats_drift_downstream_and_stay_on_the_water() {
        let mut rng = Rng::new(17);
        let mut stream = PaperBoats::new(80, 24, Motion::LIVELY, &mut rng);
        assert!(stream.boats.len() >= 2);
        let mut relaunched = 0;
        for _ in 0..(120 * 20) {
            let before: Vec<f32> = stream.boats.iter().map(|b| b.x).collect();
            stream.step(0.05, &mut rng);
            for (boat, x) in stream.boats.iter().zip(before) {
                if boat.x < x {
                    relaunched += 1;
                    continue;
                }
                // never faster than the current at the lively level
                assert!(boat.x - x <= FLOW * 1.35 * 0.05 + 1e-4);
                let line = stream.waterline(boat);
                let (far, near) = stream.banks(boat.x + boat.width() / 2.0);
                assert!(line > far as f32 && line < near as f32);
            }
        }
        assert!(relaunched >= 2, "boats never came round again");
        let theme = theme::default_theme();
        let c = StreamColours::new(theme);
        let mut frame = Frame::new(80, 24);
        stream.draw(&mut frame, theme);
        let all = pixels(&frame);
        assert!(c.papers.iter().any(|p| all.contains(p)));
        assert!(all.contains(&c.reed) && all.contains(&c.water_far));
    }
}
