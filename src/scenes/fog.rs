//! Fog: a country road with signs and lamp posts along its verge. A bank of fog rolls in from the
//! left over the first minute, fading the signs to grey and then hiding them. It keeps drifting
//! across with only the odd gap where a sign shows through.

use super::count_for;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::warning_colour;

/// Seconds the fog bank takes to cover the screen at the standard pace.
pub const ROLL_SECONDS: f32 = 50.0;
/// Columns a second the fog drifts by. Its texture is many columns wide, so a cell takes
/// seconds to go from clear to thick and back.
pub const DRIFT: f32 = 1.2;
/// Columns across one patch of the fog's texture.
const PATCH: f32 = 9.0;
/// The densities from which the fog greys out what is behind it and then hides it.
const FADES: f32 = 0.3;
const HIDES: f32 = 0.55;

pub const SUMMARY: &str =
    "A road where a fog bank rolls in and the signs along the verge fade from view";

pub const CAPTIONS: &[&str] = &[
    "Low visibility. Slow down and read the first error.",
    "The fog lifts one fix at a time.",
    "Cannot see the road? Follow the error message.",
    "Dip your lights and take it steady.",
    "Somewhere in the fog is a clean build.",
];

const TREE: [&str; 4] = ["  ^  ", " /^\\ ", "/^^^\\", "  |  "];

const SIGNS: [&str; 5] = ["[SLOW]", "[ 30 ]", "/!\\", "[<<<]", "[BEND]"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Post {
    Sign(usize),
    Lamp,
}

pub struct Fog {
    width: usize,
    height: usize,
    motion: Motion,
    /// Seconds of rolling in so far, kept across a resize so the fog never clears.
    rolled: f32,
    /// How far the fog's texture has drifted, in columns.
    drift: f32,
    seed: u32,
    posts: Vec<(i32, Post)>,
    trees: Vec<(i32, i32)>,
    ridge_phase: f32,
    grass: Vec<char>,
}

fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// A repeatable value from 0 to 1 for a lattice point.
fn lattice(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x27d4_eb2d)
        ^ (y as u32).wrapping_mul(0x1656_67b1)
        ^ seed.wrapping_mul(0x9e37_79b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    (h & 0xffff) as f32 / 65535.0
}

/// Smooth value noise from 0 to 1.
fn noise(x: f32, y: f32, seed: u32) -> f32 {
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - ix as f32, y - iy as f32);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let top = lattice(ix, iy, seed) + (lattice(ix + 1, iy, seed) - lattice(ix, iy, seed)) * sx;
    let bottom = lattice(ix, iy + 1, seed)
        + (lattice(ix + 1, iy + 1, seed) - lattice(ix, iy + 1, seed)) * sx;
    top + (bottom - top) * sy
}

impl Fog {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut fog = Self {
            width: 1,
            height: 1,
            motion,
            rolled: 0.0,
            drift: 0.0,
            seed: (rng.next_u64() >> 32) as u32,
            posts: Vec::new(),
            trees: Vec::new(),
            ridge_phase: rng.range(0.0, 6.0),
            grass: Vec::new(),
        };
        fog.resize(width, height, rng);
        fog
    }

    /// The verge row the posts stand on.
    fn verge(&self) -> i32 {
        (self.height as i32 - 4).max(0)
    }

    /// The row of the far ridge at a column.
    fn ridge(&self, x: i32) -> i32 {
        let base = self.height as f32 * 0.4;
        let x = x as f32;
        let lift = 1.6 * (x * 0.07 + self.ridge_phase).sin() + 0.8 * (x * 0.19).sin();
        (base - lift).round() as i32
    }

    /// How thick the fog is at a cell, from 0 to 1.
    fn density(&self, x: i32, y: i32) -> f32 {
        let w = self.width as f32;
        let front = -0.3 * w + 1.7 * w * ease(self.rolled / ROLL_SECONDS);
        let soft = (0.25 * w).max(4.0);
        let cover = ease((front - x as f32) / soft);
        if cover <= 0.0 {
            return 0.0;
        }
        let fx = (x as f32 - self.drift) / PATCH;
        let fy = y as f32 / 3.0;
        let texture =
            0.65 * noise(fx, fy, self.seed) + 0.35 * noise(fx * 2.3, fy * 2.0, !self.seed);
        // fog lies thickest near the ground
        let low = 0.75 + 0.25 * (y as f32 / self.height.max(1) as f32);
        (cover * (0.15 + 0.85 * texture) * low).min(1.0)
    }

    /// The share of the screen the fog hides.
    #[cfg(test)]
    fn hidden(&self) -> f32 {
        let mut count = 0;
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                if self.density(x, y) >= HIDES {
                    count += 1;
                }
            }
        }
        count as f32 / (self.width * self.height) as f32
    }
}

impl Scene for Fog {
    fn name(&self) -> &'static str {
        "fog"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = count_for(self.width, self.height, 450, 1, 7);
        let gap = (self.width as f32 / wanted as f32).max(10.0);
        self.posts = (0..wanted)
            .filter_map(|i| {
                let x = (gap * (i as f32 + 0.5)) as i32;
                (x + 3 < self.width as i32).then(|| {
                    let post = if i % 3 == 2 {
                        Post::Lamp
                    } else {
                        Post::Sign(rng.below(SIGNS.len()))
                    };
                    (x, post)
                })
            })
            .collect();
        // pines between the posts, when there is height for them
        self.trees.clear();
        if self.height >= 14 {
            let verge = self.verge();
            let mut x = rng.below(6) as i32;
            while x + 5 < self.width as i32 {
                let clear = self.posts.iter().all(|&(px, _)| (px - (x + 2)).abs() > 4);
                if clear && rng.chance(0.6) {
                    self.trees.push((x, verge - TREE.len() as i32));
                }
                x += 6 + rng.below(6) as i32;
            }
        }
        self.grass = (0..self.width)
            .map(|_| *rng.pick(&[',', '.', ' ', '\'', ' ', ',']))
            .collect();
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        let speed = self.motion.speed();
        self.rolled = (self.rolled + dt * speed).min(ROLL_SECONDS * 2.0);
        // the drift wraps far beyond any texture repeat, so it never jumps on a long run
        self.drift = (self.drift + DRIFT * speed * dt) % 100_000.0;
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let w = self.width as i32;
        let h = self.height as i32;
        let verge = self.verge();

        // the road: its near edge, the dashed centre line and its far edge
        if h >= 4 {
            for (x, ch) in self.grass.iter().enumerate() {
                if *ch != ' ' {
                    frame.put(x as i32, verge, *ch, Some(theme.green()));
                }
            }
            frame.fill_row(verge + 1, '_', Some(theme.muted));
            for x in 0..w {
                if x % 6 < 3 {
                    frame.put(x, verge + 2, '-', Some(theme.yellow()));
                }
            }
            frame.fill_row(h - 1, '_', Some(theme.muted));
        } else {
            frame.fill_row(h - 1, '_', Some(theme.muted));
        }

        // a far ridge and pines, which the fog takes first
        if h >= 10 {
            for x in 0..w {
                let (here, next) = (self.ridge(x), self.ridge(x + 1));
                let ch = match next.cmp(&here) {
                    std::cmp::Ordering::Less => '/',
                    std::cmp::Ordering::Greater => '\\',
                    std::cmp::Ordering::Equal => '_',
                };
                let y = if ch == '\\' { here + 1 } else { here };
                frame.put(x, y.min(verge - 1), ch, Some(theme.grey()));
            }
        }
        for &(x, y) in &self.trees {
            frame.put_sprite(x, y, &TREE, Some(theme.green()));
        }

        // signs and lamps on posts along the verge
        let tall = (verge / 2).clamp(1, 6);
        for &(x, post) in &self.posts {
            let top = verge - tall;
            for y in top.max(0)..verge {
                frame.put(x, y, '|', Some(theme.grey()));
            }
            match post {
                Post::Sign(i) => {
                    let text = SIGNS[i];
                    let colour = if i == 2 {
                        warning_colour(theme)
                    } else {
                        theme.foreground
                    };
                    let left = x - text.chars().count() as i32 / 2;
                    frame.put_str(left, top - 1, text, Some(colour));
                }
                Post::Lamp => {
                    frame.put_str(x, top - 1, "_", Some(theme.grey()));
                    frame.put(x + 1, top - 1, 'o', Some(theme.yellow()));
                }
            }
        }

        // the fog greys out what it thins and hides what it thickens over
        let shade = theme.faint_shade();
        for y in 0..h {
            for x in 0..w {
                let d = self.density(x, y);
                if d < FADES {
                    continue;
                }
                let behind = frame.get(x, y).filter(|c| c.ch != ' ');
                // a fixed scatter per cell, so the haze has texture but no cell flickers as
                // the fog moves over it
                let scatter = lattice(x, y, self.seed ^ 0x5bd1);
                if d >= HIDES + 0.25 {
                    frame.put(x, y, shade, Some(theme.grey()));
                } else if d >= HIDES {
                    let ch = if scatter < 0.3 {
                        '.'
                    } else if scatter < 0.45 {
                        ':'
                    } else {
                        ' '
                    };
                    frame.put(x, y, ch, Some(theme.grey()));
                } else if let Some(cell) = behind {
                    frame.put(x, y, cell.ch, Some(theme.grey()));
                } else if scatter < (d - FADES) / (HIDES - FADES) * 0.35 {
                    frame.put(x, y, '.', Some(theme.grey()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("fog");
        check_determinism("fog");
    }

    #[test]
    fn the_fog_rolls_in_slowly_and_hides_the_signs() {
        let mut rng = Rng::new(14);
        let mut fog = Fog::new(80, 24, Motion::FULL, &mut rng);
        let clear = render(&mut fog, 80, 24, 1);
        assert!(fog.hidden() < 0.01);
        let signs = SIGNS
            .iter()
            .filter(|s| clear.to_text().contains(*s))
            .count();
        assert!(signs >= 1, "{}", clear.to_text());
        let mut last = fog.hidden();
        for _ in 0..(3600 * 20) {
            fog.step(0.05, &mut rng);
            if fog.rolled < ROLL_SECONDS * 1.5 {
                let now = fog.hidden();
                // the bank never covers more than a tenth of the screen in a second
                assert!((now - last).abs() < 0.1 / 20.0 * 2.0, "{last} to {now}");
                last = now;
            }
        }
        // thick enough to hide about a third of the road, with gaps a sign can show through
        assert!(fog.hidden() > 0.25, "{}", fog.hidden());
        let thick = render(&mut fog, 80, 24, 1);
        let shown = SIGNS
            .iter()
            .filter(|s| thick.to_text().contains(*s))
            .count();
        assert!(shown <= signs, "{}", thick.to_text());
    }

    #[test]
    fn the_noise_is_smooth_and_in_range() {
        for i in 0..2000 {
            let x = i as f32 * 0.07;
            let v = noise(x, x * 0.3, 7);
            assert!((0.0..=1.0).contains(&v));
            let next = noise(x + 0.05, x * 0.3, 7);
            assert!((next - v).abs() < 0.2);
        }
        const { assert!(DRIFT / PATCH < 1.0 / 3.0) };
    }
}
