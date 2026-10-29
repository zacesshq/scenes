//! Autumn: a tree in full autumn colour losing its leaves one at a time. Each leaf turns slowly as
//! it drifts down on the breeze and settles into a pile that builds along the ground, while now
//! and then a skein of geese crosses the low sky.

use std::f32::consts::TAU;

use super::{count_for, cycle};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "A tree in autumn colour dropping leaves that drift down and gather on the ground";

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
/// Rows the pile of leaves may reach at most.
const PILE_ROWS: usize = 2;

const TURNS: [&str; 4] = ["<>", "</", "<>", "\\>"];
const SUN: [&str; 2] = [" .-. ", "(   )"];
const GEESE: [(i32, i32); 7] = [(0, 0), (2, -1), (4, -2), (2, 1), (4, 2), (6, -3), (6, 3)];

#[derive(Clone, Debug)]
struct Tree {
    /// Canopy cells with their character and colour. A space is a gap in the leaves, where the
    /// branches behind show through.
    leaves: Vec<(i32, i32, char, u8)>,
    /// The trunk, the roots and the branches.
    wood: Vec<(i32, i32, char)>,
}

/// The three autumn colours a tree's leaves come in: red, orange and gold.
const LEAF_COLOURS: [u8; 3] = [0, 2, 3];

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

    fn ground(&self) -> i32 {
        self.height as i32 - 1
    }

    fn leaves_per_row() -> u32 {
        3
    }

    fn pile_cap(&self) -> u32 {
        let rows = if self.height >= 12 { PILE_ROWS } else { 1 };
        rows as u32 * Self::leaves_per_row()
    }

    fn turn_period(&self) -> f32 {
        LEAF_TURN / self.motion.pace()
    }

    fn colour(theme: &ThemeData, colour: u8) -> Rgb {
        match colour % 5 {
            0 => theme.red(),
            1 => theme.bright_red(),
            2 => theme.yellow(),
            3 => theme.bright_yellow(),
            _ => theme.bright_magenta(),
        }
    }

    fn grow_tree(&self, x: i32, scale: f32, rng: &mut Rng) -> Tree {
        let ground = self.ground();
        let rx = (self.width as f32 * 0.17 * scale).clamp(3.0, 22.0);
        let ry = (rx * 0.5).min(ground as f32 * 0.32).max(1.5);
        let cy = (ground as f32 - ry - (ground as f32 * 0.25 * scale).max(2.0)) as i32;

        // the canopy is a few rounded clusters of leaves, each in its own colour, around the
        // ends of the branches: one on top and one or two either side
        let mut clusters: Vec<(f32, f32, f32, f32, u8)> = Vec::new();
        let crx = (rx * 0.46).max(2.0);
        let cry = (crx * 0.5).max(1.0);
        let spots: &[(f32, f32)] = if rx >= 8.0 {
            &[
                (-0.62, 0.35),
                (0.62, 0.35),
                (-0.42, -0.25),
                (0.42, -0.25),
                (0.0, -0.5),
                (0.0, 0.2),
            ]
        } else {
            &[(-0.5, 0.2), (0.5, 0.2), (0.0, -0.35)]
        };
        for &(fx, fy) in spots {
            let colour = LEAF_COLOURS[rng.below(LEAF_COLOURS.len())];
            clusters.push((
                x as f32 + fx * rx,
                cy as f32 + fy * ry,
                crx * rng.range(0.85, 1.1),
                cry * rng.range(0.85, 1.15),
                colour,
            ));
        }
        let inside = |px: i32, py: i32| -> Option<u8> {
            clusters
                .iter()
                .rev()
                .find(|&&(cx, cy, a, b, _)| {
                    ((px as f32 - cx) / a).powi(2) + ((py as f32 - cy) / b).powi(2) <= 1.0
                })
                .map(|c| c.4)
        };
        let mut leaves = Vec::new();
        let (left, right) = (
            (x as f32 - rx * 1.3) as i32,
            (x as f32 + rx * 1.3) as i32 + 1,
        );
        let (top, bottom) = (
            (cy as f32 - ry * 1.4) as i32,
            (cy as f32 + ry * 1.2) as i32 + 1,
        );
        for py in top..=bottom {
            for px in left..=right {
                let Some(colour) = inside(px, py) else {
                    continue;
                };
                let out = |dx: i32, dy: i32| inside(px + dx, py + dy).is_none();
                // the edge of the canopy is drawn as an outline, the inside as leaves with a
                // few gaps
                let ch = if out(-1, 0) {
                    '('
                } else if out(1, 0) {
                    ')'
                } else if out(0, -1) {
                    '~'
                } else if out(0, 1) {
                    '"'
                } else if rng.chance(0.08) {
                    ' '
                } else if rng.chance(0.08) {
                    '*'
                } else {
                    '@'
                };
                leaves.push((px, py, ch, colour));
            }
        }

        // the trunk, its roots and a branch up to each cluster
        let mut wood = Vec::new();
        let thick = rx >= 6.0;
        let trunk_top = cy + (ry * 0.4) as i32;
        for y in trunk_top..ground {
            wood.push((x, y, '|'));
            if thick {
                wood.push((x + 1, y, '|'));
            }
        }
        if ground >= 1 {
            wood.push((x - 1, ground - 1, '/'));
            wood.push((x + if thick { 2 } else { 1 }, ground - 1, '\\'));
        }
        for &(cx, cyc, _, _, _) in &clusters {
            let (mut bx, mut by) = (
                x as f32 + if cx > x as f32 { 1.0 } else { 0.0 },
                trunk_top as f32,
            );
            let steps = ((cx - bx).abs().max((cyc - by).abs()) as i32).max(1);
            let dx = (cx - bx) / steps as f32;
            let dy = (cyc - by) / steps as f32;
            for _ in 0..steps {
                bx += dx;
                by += dy;
                let ch = if dx.abs() < 0.35 {
                    '|'
                } else if dx > 0.0 {
                    '/'
                } else {
                    '\\'
                };
                wood.push((bx.round() as i32, by.round() as i32, ch));
            }
        }
        Tree { leaves, wood }
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
            .map(|_| rng.below(cap as usize) as u32)
            .collect();
        self.falling.clear();
        let horizon = (self.height as f32 * 0.62) as i32;
        let p = rng.range(0.0, TAU);
        self.hills = (0..self.width)
            .map(|x| horizon + ((x as f32 * 0.06 + p).sin() * 1.2).round() as i32)
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
            if leaf.y >= ground {
                landed.push(leaf.x.round() as i32);
                false
            } else {
                leaf.x > -2.0 && leaf.x < width as f32 + 2.0
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
                let leaf = tree.leaves.get(rng.below(tree.leaves.len()));
                if let Some(&(x, y, _, colour)) = leaf.filter(|l| l.2 != ' ') {
                    self.falling.push(Leaf {
                        x: x as f32,
                        y: y as f32,
                        fall: rng.range(0.7, 1.3) * speed,
                        phase: rng.range(0.0, TAU),
                        age: 0.0,
                        colour,
                    });
                }
            }
        }

        if let Some(x) = self.geese.as_mut() {
            *x -= 2.0 * speed * dt;
            if *x < -10.0 {
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
        let ground = self.ground();
        let w = self.width as i32;
        if self.width >= 40 && self.height >= 14 {
            frame.put_sprite(w - 12, 2, &SUN, Some(theme.bright_yellow()));
        }
        if let Some(x) = self.geese {
            let lead = x.round() as i32;
            for &(dx, dy) in &GEESE {
                frame.put(lead + dx, 4 + dy, 'v', Some(theme.foreground));
            }
        }
        if self.height >= 10 {
            for (x, &y) in self.hills.iter().enumerate() {
                if y < ground - 1 {
                    frame.put(x as i32, y, '_', Some(theme.blue()));
                }
            }
        }

        for tree in &self.trees {
            // the canopy hides what is behind it, so the far hills never run through the leaves
            for &(x, y, _, _) in &tree.leaves {
                frame.put(x, y, ' ', None);
            }
            for &(x, y, ch) in &tree.wood {
                frame.put(x, y, ch, Some(theme.yellow()));
            }
            for &(x, y, ch, colour) in &tree.leaves {
                if ch != ' ' {
                    frame.put(x, y, ch, Some(Self::colour(theme, colour)));
                }
            }
        }

        // grass and the pile of fallen leaves
        let per_row = Self::leaves_per_row();
        for x in 0..w {
            let count = self.pile[x as usize];
            if count == 0 {
                let ch = if x % 4 == 0 { '"' } else { '_' };
                frame.put(x, ground, ch, Some(theme.green()));
                continue;
            }
            let rows = count.div_ceil(per_row) as i32;
            for r in 0..rows {
                let seed = (x as u64).wrapping_mul(31).wrapping_add(r as u64 * 7);
                let ch = if r + 1 == rows && count % per_row != 0 {
                    ','
                } else {
                    '@'
                };
                let colour = LEAF_COLOURS[(seed % 3) as usize];
                frame.put(x, ground - r, ch, Some(Self::colour(theme, colour)));
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
            frame.put_str(
                leaf.x.round() as i32,
                leaf.y.floor() as i32,
                shape,
                Some(Self::colour(theme, leaf.colour)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("autumn");
        check_determinism("autumn");
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
        let frame = render(&mut scene, 80, 24, 1);
        // the pile never reaches more than its rows above the ground
        let above = frame.row_text(24 - 1 - PILE_ROWS);
        assert!(!above.contains('&') || above.contains("||"), "{above}");
        assert!(frame.to_text().contains("||"));
    }

    #[test]
    fn a_small_screen_still_has_a_tree() {
        let mut rng = Rng::new(3);
        let mut scene = Autumn::new(20, 8, Motion::CALM, &mut rng);
        let frame = render(&mut scene, 20, 8, 40);
        assert!(frame.to_text().contains('@') || frame.to_text().contains('%'));
        assert!(frame.to_text().contains('|'));
    }
}
