//! Erosion: a cliff above the sea crumbling slowly. Every few seconds a piece of the face breaks
//! away and tumbles into the water, the edge creeping back until a fence on top is left hanging
//! over the drop. After a few minutes the cliff reaches harder rock and holds, with the odd
//! pebble still falling and the waves still breaking below.

use super::wrap;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::{error_colour, warning_colour};

pub const SUMMARY: &str =
    "A cliff edge crumbling into the sea, stones tumbling down and a fence left hanging";

pub const CAPTIONS: &[&str] = &[
    "Something gave way. Start with the first error.",
    "The edge moved. Check the last change.",
    "A small slip, not a landslide. Read the list.",
    "Shore it up one error at a time.",
    "Firm ground is one fix away.",
];

/// Seconds between two pieces breaking away at the standard pace.
pub const CRUMBLE_PERIOD: f32 = 2.2;
/// Seconds between two pebbles once the cliff has stopped eroding.
const PEBBLE_PERIOD: f32 = 7.0;
/// How long a splash shows. Its cell changes twice in this time.
pub const SPLASH_LENGTH: f32 = 0.7;
/// The share of the screen's width the edge can creep back before it reaches hard rock.
const MOST_LOSS: f32 = 0.14;

#[derive(Clone, Debug)]
struct Stone {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

pub struct Erosion {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// How far each row of the face has worn back, as a share of the most it can.
    loss: Vec<f32>,
    stones: Vec<Stone>,
    splashes: Vec<(i32, f32)>,
    since_crumble: f32,
    clouds: Vec<(f32, i32, usize)>,
    roughness: Vec<i32>,
}

impl Erosion {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut cliff = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            loss: Vec::new(),
            stones: Vec::new(),
            splashes: Vec::new(),
            since_crumble: 0.0,
            clouds: Vec::new(),
            roughness: Vec::new(),
        };
        cliff.resize(width, height, rng);
        cliff
    }

    /// The row of the clifftop.
    fn top(&self) -> i32 {
        ((self.height as f32 * 0.38) as i32).max(2)
    }

    /// The first row of sea.
    fn sea(&self) -> i32 {
        let h = self.height as i32;
        (h - (h / 4).max(2)).max(self.top() + 2)
    }

    /// The most cells the face can wear back.
    fn most_loss(&self) -> f32 {
        (self.width as f32 * MOST_LOSS).max(2.0)
    }

    /// The column of the face on a row, from the start shape less what has worn away.
    fn edge(&self, y: i32) -> i32 {
        let base = (self.width as f32 * 0.5) as i32;
        let rough = self.roughness.get(y as usize).copied().unwrap_or(0);
        // the foot of the cliff runs out a little under the water
        let foot = (y - self.sea() + 1).max(0);
        // the shore below the water wears back with the bottom of the face
        let worn_row = y.min(self.sea() - 1).max(0) as usize;
        let worn = self.loss.get(worn_row).copied().unwrap_or(0.0) * self.most_loss();
        base + rough + foot - worn.round() as i32
    }

    fn eroding(&self) -> bool {
        (self.top()..self.sea()).any(|y| self.loss.get(y as usize).is_some_and(|&w| w < 1.0))
    }

    fn period(&self) -> f32 {
        let base = if self.eroding() {
            CRUMBLE_PERIOD
        } else {
            PEBBLE_PERIOD
        };
        base / self.motion.pace()
    }

    /// The fence posts, from back to front, ending at the original edge.
    fn fence_posts(&self) -> Vec<i32> {
        let start = (self.width as f32 * 0.5) as i32 - 1;
        let count = (self.width as i32 / 12).clamp(2, 7);
        (0..count).map(|i| start - (count - 1 - i) * 3).collect()
    }

    /// Breaks one piece from the face, top and bottom rows wearing faster than the middle.
    fn crumble(&mut self, rng: &mut Rng) {
        let top = self.top();
        let sea = self.sea();
        let rows: Vec<i32> = (top..sea)
            .filter(|&y| self.loss[y as usize] < 1.0)
            .collect();
        let from = if rows.is_empty() {
            top + rng.below((sea - top).max(1) as usize) as i32
        } else {
            let weights: Vec<f32> = rows
                .iter()
                .map(|&y| if y == top || y >= sea - 2 { 2.0 } else { 1.0 })
                .collect();
            let total: f32 = weights.iter().sum();
            let mut pick = rng.range(0.0, total);
            let mut chosen = rows[0];
            for (&y, &w) in rows.iter().zip(&weights) {
                if pick < w {
                    chosen = y;
                    break;
                }
                pick -= w;
            }
            let step = 1.0 / self.most_loss();
            let row = &mut self.loss[chosen as usize];
            *row = (*row + step).min(1.0);
            // the face stays rough but continuous: no row may stand more than two cells out
            // from its neighbours, so a cut can bring the rows around it down with it
            for _ in 0..(sea - top) {
                let mut settled = true;
                for y in top..sea - 1 {
                    for (high, low) in [(y, y + 1), (y + 1, y)] {
                        let worn = self.loss[high as usize];
                        if self.edge(high) > self.edge(low) + 2 && worn < 1.0 {
                            self.loss[high as usize] = (worn + step).min(1.0);
                            settled = false;
                        }
                    }
                }
                if settled {
                    break;
                }
            }
            chosen
        };
        self.stones.push(Stone {
            x: self.edge(from) as f32 + 1.0,
            y: from as f32,
            vx: rng.range(0.6, 1.6),
            vy: 0.0,
        });
    }
}

impl Scene for Erosion {
    fn name(&self) -> &'static str {
        "erosion"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        // what has worn away is kept, row for row by its height on the screen
        let old = std::mem::take(&mut self.loss);
        let old_height = self.height.max(1);
        self.width = width.max(1);
        self.height = height.max(1);
        self.loss = (0..self.height)
            .map(|y| {
                let from = y * old_height / self.height;
                old.get(from).copied().unwrap_or(0.0)
            })
            .collect();
        let mut wobble = 0_i32;
        self.roughness = (0..self.height)
            .map(|_| {
                wobble = (wobble + rng.below(3) as i32 - 1).clamp(-1, 1);
                wobble
            })
            .collect();
        self.stones.clear();
        self.splashes.clear();
        let sky = (self.top() - 2).max(1) as usize;
        let count = self.motion.scaled((self.width / 35).max(1));
        self.clouds = (0..count)
            .map(|_| {
                (
                    rng.range(0.0, self.width as f32),
                    rng.below(sky) as i32,
                    5 + rng.below(9),
                )
            })
            .collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        let width = self.width as f32;
        for cloud in &mut self.clouds {
            cloud.0 = wrap(cloud.0 + 0.4 * speed * dt + 16.0, width + 32.0) - 16.0;
        }
        self.since_crumble += dt;
        // a screen too short for a cliff and a sea has nothing to wear away
        if self.since_crumble >= self.period() && (self.sea() as usize) < self.height {
            self.since_crumble = 0.0;
            self.crumble(rng);
        }
        let sea = self.sea() as f32;
        let self_edges: Vec<i32> = (0..self.height as i32 + 1).map(|y| self.edge(y)).collect();
        let mut landed = Vec::new();
        for stone in &mut self.stones {
            stone.vy += 9.0 * speed * dt;
            stone.x += stone.vx * speed * dt;
            stone.y += stone.vy * dt;
            stone.x = stone
                .x
                .max(self_edges[(stone.y as usize).min(self_edges.len() - 1)] as f32 + 1.0);
            if stone.y >= sea {
                landed.push(stone.x.round() as i32);
            }
        }
        self.stones.retain(|s| s.y < sea);
        let foot = self.edge(self.sea()) + 2;
        for x in landed {
            self.splashes.push((x.max(foot), 0.0));
        }
        let pace = self.motion.pace();
        for splash in &mut self.splashes {
            splash.1 += dt * pace;
        }
        self.splashes.retain(|s| s.1 < SPLASH_LENGTH);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let width = self.width as i32;
        let height = self.height as i32;
        let top = self.top();
        let sea = self.sea();
        let speed = self.motion.speed();

        for &(x, y, len) in &self.clouds {
            for i in 0..len as i32 {
                frame.put(x.round() as i32 + i, y, '~', Some(theme.grey()));
            }
        }

        // the sea, its waves rolling in towards the cliff
        for y in sea..height {
            let depth = (y - sea) as f32;
            let drift = super::cycle(self.t, (1.0 + 0.3 * depth) * speed / 6.0);
            let long = 6.0 + (y % 3) as f32 * 1.5;
            for x in 0..width {
                let xf = x as f32;
                let v = 0.6 * (xf / long + drift + depth * 2.39).sin()
                    + 0.4 * (xf / 2.7 + drift * 1.6 + depth * 4.1).sin();
                if v > 0.6 {
                    frame.put(x, y, '~', Some(theme.cyan()));
                } else if v > 0.15 {
                    frame.put(x, y, '-', Some(theme.blue()));
                }
            }
        }

        // the cliff: rock behind a rough face, with the grass along the top
        for y in top..height {
            let edge = self.edge(y).min(width);
            for x in 0..edge {
                frame.put(x, y, theme.faint_shade(), Some(theme.grey()));
            }
            if y == top {
                for x in 0..edge {
                    let ch = if x % 3 == 0 { '"' } else { ',' };
                    frame.put(x, y, ch, Some(theme.green()));
                }
            }
            let above = if y == top { edge } else { self.edge(y - 1) };
            let ch = match edge.cmp(&above) {
                std::cmp::Ordering::Greater => '\\',
                std::cmp::Ordering::Less => '/',
                std::cmp::Ordering::Equal => '|',
            };
            frame.put(edge, y, ch, Some(theme.yellow()));
        }
        // surf breaking at the foot of the cliff
        frame.put(self.edge(sea) + 1, sea, '*', Some(theme.foreground));

        // the fence: posts on solid ground stand up, the ones past the edge hang down the face
        if top >= 1 {
            let edge = self.edge(top);
            let posts = self.fence_posts();
            let mut hanging = 0;
            for (i, &x) in posts.iter().enumerate() {
                if x < edge {
                    frame.put(x, top - 1, '+', Some(theme.foreground));
                    if let Some(&next) = posts.get(i + 1).filter(|&&n| n < edge) {
                        for rail in x + 1..next {
                            frame.put(rail, top - 1, '-', Some(theme.foreground));
                        }
                    }
                } else {
                    hanging += 1;
                }
            }
            // the hanging length follows the face down; only the last few posts still hang
            for i in 0..(hanging.min(3) * 2) {
                let y = top + i;
                if y >= sea {
                    break;
                }
                let ch = if i % 2 == 0 { '\\' } else { '+' };
                frame.put(self.edge(y) + 1, y, ch, Some(warning_colour(theme)));
            }
            // a warning sign well back from the edge
            let sign = posts[0] - 6;
            if sign >= 6 && top >= 2 {
                frame.put_str(sign, top - 2, "/!\\", Some(warning_colour(theme)));
                frame.put(sign + 1, top - 1, '|', Some(theme.foreground));
                if sign >= 12 && top >= 3 {
                    frame.put_str(sign - 3, top - 3, "KEEP BACK", Some(error_colour(theme)));
                }
            }
            // a cottage far back from the edge, still safe
            if self.width >= 50 && top >= 4 {
                frame.put_sprite(
                    2,
                    top - 3,
                    &[" /\\ ", "/__\\", "|[]|"],
                    Some(theme.foreground),
                );
            }
        }

        for stone in &self.stones {
            frame.put(
                stone.x.round() as i32,
                stone.y.floor() as i32,
                'o',
                Some(theme.muted),
            );
        }
        for &(x, age) in &self.splashes {
            if age < SPLASH_LENGTH / 2.0 {
                frame.put(x, sea, 'o', Some(theme.bright_cyan()));
            } else {
                frame.put(x - 1, sea, '(', Some(theme.cyan()));
                frame.put(x + 1, sea, ')', Some(theme.cyan()));
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
        check_scene("erosion");
        check_determinism("erosion");
    }

    #[test]
    fn the_edge_creeps_back_then_holds_with_the_fence_hanging() {
        let mut rng = Rng::new(12);
        let mut cliff = Erosion::new(100, 30, Motion::FULL, &mut rng);
        let start: Vec<i32> = (0..30).map(|y| cliff.edge(y)).collect();
        for _ in 0..(60 * 20) {
            cliff.step(0.05, &mut rng);
        }
        let worn = (0..30)
            .filter(|&y| cliff.edge(y) < start[y as usize])
            .count();
        assert!(worn >= 3, "{worn}");
        for _ in 0..(60 * 60 * 20) {
            cliff.step(0.05, &mut rng);
        }
        assert!(!cliff.eroding());
        let most = cliff.most_loss().round() as i32;
        for y in cliff.top()..cliff.sea() {
            assert!(start[y as usize] - cliff.edge(y) <= most + 1);
            assert!(cliff.edge(y) > 0);
        }
        let frame = render(&mut cliff, 100, 30, 1);
        let face: String = (cliff.top()..cliff.sea())
            .map(|y| frame.get(cliff.edge(cliff.top()) + 1, y).unwrap().ch)
            .collect();
        assert!(face.contains('\\'), "the fence should hang: {face}");
        assert!(frame.to_text().contains("KEEP BACK"));
    }

    #[test]
    fn pieces_break_away_slowly_on_every_level() {
        let mut rng = Rng::new(1);
        for motion in Motion::ALL {
            let cliff = Erosion::new(80, 24, motion, &mut rng);
            assert!(cliff.period() >= CRUMBLE_PERIOD);
        }
    }

    #[test]
    fn a_resize_keeps_the_worn_edge() {
        let mut rng = Rng::new(3);
        let mut cliff = Erosion::new(80, 24, Motion::FULL, &mut rng);
        for _ in 0..(120 * 20) {
            cliff.step(0.05, &mut rng);
        }
        let worn: f32 = cliff.loss.iter().sum();
        cliff.resize(80, 24, &mut rng);
        let kept: f32 = cliff.loss.iter().sum();
        assert!((worn - kept).abs() < 0.01);
    }
}
