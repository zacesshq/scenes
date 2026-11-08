//! Autumn: a tree in full autumn colour losing its leaves one at a time, painted in solid pixels.
//! Its crown is a few rounded clusters of red, orange and gold leaves, lit from the sun and
//! shaded underneath, with a few leaves deeper in its shade. Each leaf turns slowly as it
//! drifts down on the breeze and settles into a pile that builds along the ground, while now and
//! then a skein of geese crosses the low sky.
//!
//! Every colour comes from the active theme. The sky, the far hills and the meadow are tints of
//! the background held below the non-text floor, so they read as background; the tree, its
//! leaves, the sun and the geese keep the theme's own drawing colours, so they stand out as shapes
//! and keep their outline when colour is off.

use std::f32::consts::TAU;

use super::{count_for, cycle};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "A shaded tree in autumn colour dropping leaves that drift down and gather on the ground, in the afternoon in light themes";

pub const CAPTIONS: &[&str] = &[
    "Everything fell into place.",
    "A clean build. Kick through the leaves.",
    "Nothing left to rake.",
    "The season changes. The tests still pass.",
    "Golden all the way down.",
];

/// Seconds a falling leaf shows each of its four turns, so it never changes more than twice a
/// second as it falls.
pub const LEAF_TURN: f32 = 0.6;
/// Pixel rows the pile of leaves may reach at most.
const PILE_ROWS: u32 = 4;
/// Leaves it takes to fill one pixel row of the pile.
const LEAVES_PER_ROW: u32 = 2;

/// A falling leaf's four turns: flat, tipped one way, edge on and tipped the other way.
const TURNS: [&[(i32, i32)]; 4] = [
    &[(0, 0), (1, 0)],
    &[(0, 0), (1, 1)],
    &[(0, 0), (0, 1)],
    &[(1, 0), (0, 1)],
];
/// A goose seen from below with its wings up, then where each goose flies in the skein.
const GOOSE: [(i32, i32); 4] = [(0, 0), (1, 1), (2, 1), (3, 0)];
const SKEIN: [(i32, i32); 7] = [
    (0, 0),
    (5, -2),
    (10, -4),
    (5, 2),
    (10, 4),
    (15, -6),
    (15, 6),
];

/// Steps in the sky ramp.
const SKY_STEPS: usize = 10;

/// A small deterministic hash, for the fixed texture of the meadow and the pile.
fn hash(a: i64, b: i64) -> u64 {
    let mut z = (a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (b as u64).wrapping_mul(0xD1B5_4A32_D192_ED03);
    z = (z ^ (z >> 29)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^ (z >> 32)
}

/// The autumn colours in one theme, every one derived from the theme's own. Some are kept only
/// so the tests can check the bands against each other.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct AutumnColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub hills: Rgb,
    pub meadow: Rgb,
    pub meadow_dark: Rgb,
    pub grass: Rgb,
    pub sun: Rgb,
    /// The glow around the sun: the sun itself on a dark theme, a warm tint around a pale sun
    /// on a light one.
    pub halo: Rgb,
    pub dark: bool,
    pub bark: Rgb,
    pub bark_shade: Rgb,
    pub goose: Rgb,
    /// The leaves: red, orange and gold.
    pub leaves: [Rgb; 3],
    /// Each leaf colour lit by the sun and in the crown's shade.
    pub lit: [Rgb; 3],
    pub shade: [Rgb; 3],
    sky: Ramp,
}

impl AutumnColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        // a warm autumn evening on a dark theme and a clear afternoon on a light one
        let (sky_top, sky_low, hills) = if theme.dark {
            (
                shades.tint(theme.info, 0.18, 1.35),
                shades.tint(theme.warning, 0.45, 2.2),
                shades.tint(theme.blue(), 0.22, 1.3),
            )
        } else {
            (
                shades.tint(theme.info, 0.4, 2.0),
                shades.tint(theme.warning, 0.15, 1.25),
                // strong enough to stay apart from the meadow in the 256 colour table on a warm
                // light theme, where every tint leans to the same beige grey
                shades.tint(theme.blue(), 0.45, 2.0),
            )
        };
        let (meadow, meadow_dark, grass) = if theme.dark {
            (
                shades.tint(mix(theme.green(), theme.yellow(), 0.4), 0.35, 1.9),
                shades.tint(theme.green(), 0.22, 1.45),
                shades.tint(theme.green(), 0.55, 2.6),
            )
        } else {
            (
                shades.tint(mix(theme.green(), theme.yellow(), 0.4), 0.3, 1.6),
                shades.tint(theme.green(), 0.55, 2.3),
                shades.tint(theme.green(), 0.8, 2.9),
            )
        };
        let orange = mix(theme.red(), theme.yellow(), 0.5);
        let leaves = [theme.red(), orange, theme.bright_yellow()];
        let toward_dark = |c: Rgb| shades.darker(c, 0.3);
        let toward_light = |c: Rgb| mix(c, theme.bright_yellow(), 0.3);
        let bark = if theme.dark {
            mix(theme.yellow(), theme.red(), 0.45)
        } else {
            theme.yellow()
        };
        Self {
            sky_top,
            sky_low,
            hills,
            meadow,
            meadow_dark,
            grass,
            // a daylit sky on a light theme has a pale sun in a warm glow, never a dark disc
            sun: if theme.dark {
                theme.bright_yellow()
            } else {
                theme.background
            },
            halo: if theme.dark {
                theme.bright_yellow()
            } else {
                mix(theme.background, shades.tint(theme.warning, 0.55, 2.5), 0.6)
            },
            dark: theme.dark,
            bark,
            bark_shade: shades.darker(bark, 0.3),
            goose: theme.foreground,
            leaves,
            lit: leaves.map(toward_light),
            shade: leaves.map(toward_dark),
            sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
        }
    }
}

/// How a pixel of the crown is lit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Light {
    Lit,
    Plain,
    Shade,
}

#[derive(Clone, Debug)]
struct Tree {
    /// Crown pixels with their leaf colour and light. Gaps are left out, so the branches and
    /// the sky show through.
    leaves: Vec<(i32, i32, u8, Light)>,
    /// The trunk and branches with whether each pixel is on the shaded side.
    wood: Vec<(i32, i32, bool)>,
}

#[derive(Clone, Debug)]
struct Leaf {
    x: f32,
    y: f32,
    fall: f32,
    phase: f32,
    age: f32,
    colour: u8,
}

pub struct Autumn {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    trees: Vec<Tree>,
    falling: Vec<Leaf>,
    /// How many leaves have settled in each column.
    pile: Vec<u32>,
    next_leaf: f32,
    geese: Option<f32>,
    next_geese: f32,
    /// The far hills' top pixel row in each column.
    hills: Vec<i32>,
}

impl Autumn {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut scene = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            trees: Vec::new(),
            falling: Vec::new(),
            pile: Vec::new(),
            next_leaf: 0.0,
            geese: None,
            next_geese: rng.range(4.0, 12.0),
            hills: Vec::new(),
        };
        scene.resize(width, height, rng);
        scene
    }

    /// The pixel row of the ground, where leaves settle.
    fn ground(&self) -> i32 {
        self.height as i32 * 2 - 1
    }

    fn pile_cap(&self) -> u32 {
        let rows = if self.height >= 12 {
            PILE_ROWS
        } else {
            PILE_ROWS / 2
        };
        rows * LEAVES_PER_ROW
    }

    fn turn_period(&self) -> f32 {
        LEAF_TURN / self.motion.pace()
    }

    fn grow_tree(&self, x: i32, scale: f32, rng: &mut Rng) -> Tree {
        let ground = self.ground() as f32;
        let rx = (self.width as f32 * 0.17 * scale).clamp(4.0, 24.0);
        let ry = (rx * 0.8).min(ground * 0.32).max(3.0);
        let cy = ground - ry - (ground * 0.28 * scale).max(4.0);

        // the crown is a few rounded clusters of leaves, each in its own colour, around the ends
        // of the branches: one on top and one or two either side
        let crx = (rx * 0.5).max(2.5);
        let cry = (crx * 0.8).max(2.0);
        let spots: &[(f32, f32)] = if rx >= 8.0 {
            &[
                (-0.62, 0.3),
                (0.62, 0.3),
                (-0.4, -0.3),
                (0.4, -0.3),
                (0.0, -0.55),
                (0.0, 0.15),
            ]
        } else {
            &[(-0.5, 0.2), (0.5, 0.2), (0.0, -0.35)]
        };
        let clusters: Vec<(f32, f32, f32, f32, u8)> = spots
            .iter()
            .map(|&(fx, fy)| {
                (
                    x as f32 + fx * rx,
                    cy + fy * ry,
                    crx * rng.range(0.85, 1.1),
                    cry * rng.range(0.85, 1.15),
                    rng.below(3) as u8,
                )
            })
            .collect();
        let (centre_x, centre_y) = (x as f32, cy);
        let mut leaves = Vec::new();
        let (left, right) = (
            (x as f32 - rx * 1.4) as i32,
            (x as f32 + rx * 1.4) as i32 + 1,
        );
        let (top, bottom) = ((cy - ry * 1.5) as i32, (cy + ry * 1.3) as i32 + 1);
        for py in top..=bottom {
            for px in left..=right {
                let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
                let hit = clusters.iter().rev().find(|&&(cx, cy, a, b, _)| {
                    ((fx - cx) / a).powi(2) + ((fy - cy) / b).powi(2) <= 1.0
                });
                let Some(&(cx, ccy, a, b, colour)) = hit else {
                    continue;
                };
                // a few leaves sit deeper in the crown and fall in its shade; a hole would show
                // as a stray pixel of sky, so the crown stays solid
                let deep = rng.chance(0.05);
                // each cluster and the crown as a whole are lit from the upper left
                let local = (fx - cx) / a + (fy - ccy) / b;
                let whole = (fx - centre_x) / rx + (fy - centre_y) / ry;
                let light = if deep {
                    Light::Shade
                } else if local + 0.5 * whole < -0.7 {
                    Light::Lit
                } else if local + 0.5 * whole > 0.6 {
                    Light::Shade
                } else {
                    Light::Plain
                };
                // a leaf of another colour now and then, so the crown is not in flat patches
                let colour = if rng.chance(0.06) {
                    rng.below(3) as u8
                } else {
                    colour
                };
                leaves.push((px, py, colour, light));
            }
        }

        // the trunk flaring into roots at the ground, then a branch up to each cluster
        let mut wood = Vec::new();
        let thick = if rx >= 8.0 { 3 } else { 2 };
        let trunk_top = (cy + ry * 0.3) as i32;
        let ground = ground as i32;
        for y in trunk_top..ground {
            let flare = i32::from(y >= ground - 2) + i32::from(y == ground - 1);
            for dx in -flare..thick + flare {
                wood.push((x + dx, y, dx >= thick - 1));
            }
        }
        for &(cx, ccy, _, _, _) in &clusters {
            let (mut bx, mut by) = (x as f32 + thick as f32 / 2.0, trunk_top as f32);
            let steps = ((cx - bx).abs().max((ccy - by).abs()) as i32).max(1);
            let dx = (cx - bx) / steps as f32;
            let dy = (ccy - by) / steps as f32;
            for i in 0..steps {
                bx += dx;
                by += dy;
                wood.push((bx.round() as i32, by.round() as i32, false));
                // branches are thicker where they leave the trunk
                if i < steps / 3 {
                    wood.push((bx.round() as i32 + 1, by.round() as i32, true));
                }
            }
        }
        Tree { leaves, wood }
    }

    fn paint(&self, canvas: &mut Canvas, c: &AutumnColours) {
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let ground = self.ground();
        let horizon = self.hills.iter().copied().min().unwrap_or(tall).max(1);
        for y in 0..tall {
            canvas.fill_row(y, c.sky.at(y as f32 / horizon as f32));
        }
        // a still sun low in the sky, which never flashes
        if self.width >= 40 && self.height >= 14 {
            let (sx, sy, r) = (wide as f32 - 12.0, 6.0_f32, 3.0_f32);
            for y in 0..14 {
                for x in (sx as i32 - 6)..(sx as i32 + 7) {
                    let d = ((x as f32 + 0.5 - sx).powi(2) + (y as f32 + 0.5 - sy).powi(2)).sqrt();
                    if d <= r {
                        canvas.set(x, y, c.sun);
                    } else if d <= r + 2.5 {
                        let strength = if c.dark { 0.3 } else { 0.6 };
                        canvas.blend(x, y, c.halo, strength * (1.0 - (d - r) / 2.5));
                    }
                }
            }
        }
        if let Some(x) = self.geese {
            let lead = x.round() as i32;
            let y = (tall / 6).max(7);
            for &(sx, sy) in &SKEIN {
                for &(gx, gy) in &GOOSE {
                    canvas.set(lead + sx + gx, y + sy + gy, c.goose);
                }
            }
        }
        // the far hills and the meadow between them and the ground
        if self.height >= 10 {
            for (x, &top) in self.hills.iter().enumerate() {
                let x = x as i32;
                for y in top..tall {
                    let meadow = y > top + 3;
                    let colour = if !meadow {
                        c.hills
                    } else if hash(i64::from(x / 2), i64::from(y)) % 13 == 0 {
                        c.meadow_dark
                    } else {
                        c.meadow
                    };
                    canvas.set(x, y, colour);
                }
            }
        } else {
            for y in tall / 2..tall {
                canvas.fill_row(y, c.meadow);
            }
        }

        for tree in &self.trees {
            for &(x, y, shaded) in &tree.wood {
                canvas.set(x, y, if shaded { c.bark_shade } else { c.bark });
            }
            for &(x, y, colour, light) in &tree.leaves {
                let i = colour as usize % 3;
                let paint = match light {
                    Light::Lit => c.lit[i],
                    Light::Plain => c.leaves[i],
                    Light::Shade => c.shade[i],
                };
                canvas.set(x, y, paint);
            }
        }

        // grass along the ground and the pile of fallen leaves on it
        for x in 0..wide {
            let count = self.pile.get(x as usize).copied().unwrap_or(0);
            let blade = hash(i64::from(x), 1) % 3 == 0;
            canvas.set(x, ground, c.grass);
            if blade {
                canvas.set(x, ground - 1, c.grass);
            }
            let rows = count.div_ceil(LEAVES_PER_ROW) as i32;
            for r in 0..rows {
                let colour = (hash(i64::from(x), i64::from(r) + 9) % 3) as usize;
                let y = ground - r;
                let paint = if r + 1 == rows && count % LEAVES_PER_ROW != 0 {
                    c.lit[colour]
                } else {
                    c.leaves[colour]
                };
                canvas.set(x, y, paint);
            }
        }

        let period = self.turn_period();
        for leaf in &self.falling {
            let turn = ((leaf.age / period) as usize + leaf.colour as usize) % TURNS.len();
            let shape = if self.motion.calm {
                TURNS[0]
            } else {
                TURNS[turn]
            };
            let paint = c.leaves[leaf.colour as usize % 3];
            let (x, y) = (leaf.x.round() as i32, leaf.y.floor() as i32);
            for &(dx, dy) in shape {
                canvas.set(x + dx, y + dy, paint);
            }
        }
    }
}

impl Scene for Autumn {
    fn name(&self) -> &'static str {
        "autumn"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.trees.clear();
        let main = (self.width as f32 * 0.32) as i32;
        let tree = self.grow_tree(main, 1.0, rng);
        self.trees.push(tree);
        if self.width >= 60 && self.height >= 14 {
            let tree = self.grow_tree((self.width as f32 * 0.78) as i32, 0.6, rng);
            self.trees.push(tree);
        }
        // the ground starts with a thin scattering of leaves already down
        let cap = self.pile_cap();
        self.pile = (0..self.width)
            .map(|_| rng.below(cap as usize / 2 + 1) as u32)
            .collect();
        self.falling.clear();
        let horizon = (self.height as f32 * 2.0 * 0.62) as i32;
        let p = rng.range(0.0, TAU);
        self.hills = (0..self.width)
            .map(|x| horizon + ((x as f32 * 0.06 + p).sin() * 2.4).round() as i32)
            .collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        let ground = self.ground() as f32;
        let breeze = 0.4 + 0.3 * cycle(self.t, 0.05).sin();
        for leaf in &mut self.falling {
            leaf.age += dt;
            leaf.y += leaf.fall * dt;
            let sway = (leaf.age * 1.4 + leaf.phase).sin() * 1.2;
            leaf.x += (breeze + sway) * speed * dt;
        }
        let cap = self.pile_cap();
        let width = self.width;
        let mut landed = Vec::new();
        self.falling.retain(|leaf| {
            if leaf.y >= ground - 1.0 {
                landed.push(leaf.x.round() as i32);
                false
            } else {
                leaf.x > -3.0 && leaf.x < width as f32 + 3.0
            }
        });
        for x in landed {
            if x < 0 || x as usize >= width {
                continue;
            }
            let mut x = x as usize;
            // a leaf landing on a heap slides off to the lower side, so the pile stays rounded
            for _ in 0..3 {
                let here = self.pile[x];
                if x > 0 && self.pile[x - 1] + 1 < here {
                    x -= 1;
                } else if x + 1 < width && self.pile[x + 1] + 1 < here {
                    x += 1;
                } else {
                    break;
                }
            }
            if self.pile[x] < cap {
                self.pile[x] += 1;
            }
        }

        self.next_leaf -= dt;
        if self.next_leaf <= 0.0 {
            self.next_leaf = if self.motion.calm {
                rng.range(1.6, 3.0)
            } else {
                rng.range(0.6, 1.4) / self.motion.busy()
            };
            // few enough falling at once that each one reads as a leaf
            let limit = count_for(self.width, self.height, 220, 2, 12);
            if self.falling.len() < limit && !self.trees.is_empty() {
                let tree = &self.trees[rng.below(self.trees.len())];
                if let Some(&(x, y, colour, _)) = tree.leaves.get(rng.below(tree.leaves.len())) {
                    self.falling.push(Leaf {
                        x: x as f32,
                        y: y as f32,
                        fall: rng.range(1.4, 2.6) * speed,
                        phase: rng.range(0.0, TAU),
                        age: 0.0,
                        colour,
                    });
                }
            }
        }

        if let Some(x) = self.geese.as_mut() {
            *x -= 2.0 * speed * dt;
            if *x < -24.0 {
                self.geese = None;
                self.next_geese = rng.range(15.0, 35.0);
            }
        } else if self.height >= 12 && self.width >= 30 {
            self.next_geese -= dt;
            if self.next_geese <= 0.0 {
                self.geese = Some(self.width as f32 + 2.0);
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = AutumnColours::new(theme);
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
    use crate::scenes::testing::{
        apart, check_determinism, check_pixel_scene, check_scene, pixels, render,
    };
    use crate::terminal::pixel_256;
    use crate::theme;

    fn drawn(scene: &Autumn, width: usize, height: usize, theme: &ThemeData) -> Frame {
        let mut frame = Frame::new(width, height);
        scene.draw(&mut frame, theme);
        frame
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("autumn");
        check_determinism("autumn");
        check_pixel_scene("autumn", 6.0);
    }

    #[test]
    fn leaves_turn_slowly_and_the_pile_stays_low() {
        const { assert!(LEAF_TURN >= 1.0 / 3.0) };
        let mut rng = Rng::new(12);
        let mut scene = Autumn::new(80, 24, Motion::LIVELY, &mut rng);
        assert!(scene.turn_period() >= LEAF_TURN);
        let start: u32 = scene.pile.iter().sum();
        for _ in 0..(600 * 20) {
            scene.step(0.05, &mut rng);
        }
        let end: u32 = scene.pile.iter().sum();
        assert!(end > start, "no leaf settled");
        assert!(scene.pile.iter().all(|&n| n <= scene.pile_cap()));
        // the pile never reaches more than its rows above the ground
        let theme = theme::default_theme();
        let c = AutumnColours::new(theme);
        scene.falling.clear();
        let frame = drawn(&scene, 80, 24, theme);
        let above = 24 - 1 - (PILE_ROWS as i32) / 2;
        for x in 0..80 {
            let (_, bottom) = frame.get(x, above).unwrap().pixel_pair().unwrap();
            let on_tree = scene
                .trees
                .iter()
                .any(|t| t.wood.iter().any(|w| w.0 == x) || t.leaves.iter().any(|l| l.0 == x));
            assert!(
                on_tree || !c.leaves.contains(&bottom),
                "a pile too high at {x}"
            );
        }
        let all = pixels(&frame);
        assert!(all.contains(&c.bark));
    }

    #[test]
    fn a_small_screen_still_has_a_tree() {
        let theme = theme::default_theme();
        let c = AutumnColours::new(theme);
        let mut rng = Rng::new(3);
        let mut scene = Autumn::new(20, 8, Motion::CALM, &mut rng);
        let frame = render(&mut scene, 20, 8, 40);
        let all = pixels(&frame);
        assert!(c.leaves.iter().any(|l| all.contains(l)));
        assert!(all.contains(&c.bark));
    }

    #[test]
    fn the_colours_stay_apart_and_in_their_bands_in_every_theme() {
        for theme in theme::all() {
            let c = AutumnColours::new(theme);
            let name = theme.name;
            apart(
                theme,
                c.sky_top,
                c.sky_low,
                0.03,
                "the sky's top and bottom",
            );
            apart(theme, c.hills, c.sky_low, 0.03, "the hills and the sky");
            apart(theme, c.meadow, c.hills, 0.03, "the meadow and the hills");
            apart(theme, c.grass, c.meadow, 0.03, "the grass and the meadow");
            for (what, colour) in [
                ("sky top", c.sky_top),
                ("sky low", c.sky_low),
                ("hills", c.hills),
                ("meadow", c.meadow),
                ("grass", c.grass),
            ] {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            // a light theme's sun is a pale disc in the sky, so only a dark theme's is a shape
            let sun = theme.dark.then_some(c.sun);
            for colour in c.leaves.iter().copied().chain([c.bark, c.goose]).chain(sun) {
                assert!(
                    theme.contrast_of(colour) >= IMAGERY_CEILING,
                    "{} in {name}",
                    colour.hex()
                );
                apart(theme, colour, c.sky_top, 0.1, "a shape and the sky");
                apart(theme, colour, c.meadow, 0.1, "a shape and the meadow");
            }
            for colour in [c.bark, c.goose, c.leaves[0], c.leaves[2]]
                .into_iter()
                .chain(sun)
            {
                assert!(theme.contrast_of(colour) >= theme.floor(), "{name}");
            }
            apart(
                theme,
                c.leaves[0],
                c.leaves[2],
                0.08,
                "the red and gold leaves",
            );
            apart(
                theme,
                c.bark,
                c.leaves[0],
                0.05,
                "the bark and the red leaves",
            );
        }
    }

    #[test]
    fn the_256_colour_fallback_keeps_the_bands_apart() {
        for theme in theme::all() {
            let c = AutumnColours::new(theme);
            let near = |colour: Rgb| xterm_colour(pixel_256(colour, theme.background));
            let mut rng = Rng::new(8);
            let mut scene = Autumn::new(80, 24, Motion::FULL, &mut rng);
            for _ in 0..60 {
                scene.step(0.05, &mut rng);
            }
            let frame = drawn(&scene, 80, 24, theme);
            let worst = pixels(&frame)
                .into_iter()
                .map(|colour| distance(colour, near(colour)))
                .fold(0.0_f32, f32::max);
            assert!(worst <= 0.15, "{} is {worst:.3} off", theme.name);
            assert_ne!(near(c.hills), near(c.sky_low), "{}", theme.name);
            assert_ne!(near(c.meadow), near(c.hills), "{}", theme.name);
            assert_ne!(near(c.leaves[0]), near(c.leaves[2]), "{}", theme.name);
        }
    }
}
