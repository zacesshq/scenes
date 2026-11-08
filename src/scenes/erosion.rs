//! Erosion: a layered chalk cliff above the sea, crumbling slowly. Every few seconds a piece of
//! the face breaks away and tumbles into the water, the edge creeping back until a fence on top
//! is left hanging down the drop. After a few minutes the cliff reaches harder rock and holds,
//! with the odd pebble still falling and the waves still breaking white at its foot.
//!
//! The sky, the sea and the rock are tints of the background held below the non-text floor. The
//! fence, the KEEP BACK board, the cottage and the falling stones are solid, so with colour off the fence
//! is seen hanging over the edge.

use super::paint::{self, Blender, Overcast, clouds, gradient, line, shaded, solid, unit};
use super::{cycle, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::{error_colour, warning_colour};

pub const SUMMARY: &str = "A layered cliff crumbling into the sea, stones tumbling down and a fence left hanging over the edge";

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
/// How long a splash shows. Its pixels change twice in this time.
pub const SPLASH_LENGTH: f32 = 0.7;
/// The share of the screen's width the edge can creep back before it reaches hard rock.
const MOST_LOSS: f32 = 0.14;

/// The words on the board well back from the edge.
const BOARD_TEXT: &str = "KEEP BACK";

const COTTAGE: [&str; 7] = [
    "....rr....",
    "..rrrrrr..",
    "rrrrrrrrrr",
    ".wwwwwwww.",
    ".wyywwyyw.",
    ".wyywdwyw.",
    ".wwwwdwww.",
];

#[derive(Clone, Debug)]
struct Stone {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

/// The cliff's colours in one theme.
struct Colours {
    overcast: Overcast,
    sea: Ramp,
    crest: Rgb,
    strata: [Rgb; 3],
    face: Rgb,
    shadow: Rgb,
    grass: (Rgb, Rgb),
    post: Rgb,
    hanging: Rgb,
    stone: Rgb,
    cottage: [(char, Rgb); 4],
    board: Rgb,
    plain: Rgb,
}

impl Colours {
    fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let chalk = mix(theme.muted, theme.yellow(), 0.3);
        let (sea_top, sea_deep, strata, face, shadow, grass) = if theme.dark {
            (
                shades.tint(theme.cyan(), 0.45, 2.2),
                shades.tint(theme.blue(), 0.25, 1.4),
                [
                    shades.tint(chalk, 0.4, 2.1),
                    shades.tint(chalk, 0.3, 1.8),
                    shades.tint(chalk, 0.5, 2.4),
                ],
                shades.tint(chalk, 0.65, 2.85),
                shades.tint(chalk, 0.18, 1.4),
                (
                    shades.tint(theme.green(), 0.55, 2.5),
                    shades.tint(theme.green(), 0.35, 1.9),
                ),
            )
        } else {
            (
                shades.tint(theme.cyan(), 0.35, 1.8),
                shades.tint(theme.blue(), 0.7, 2.75),
                [
                    shades.tint(chalk, 0.4, 1.9),
                    shades.tint(chalk, 0.55, 2.3),
                    shades.tint(chalk, 0.3, 1.6),
                ],
                shades.tint(chalk, 0.15, 1.25),
                shades.tint(chalk, 0.75, 2.85),
                (
                    shades.tint(theme.green(), 0.55, 2.3),
                    shades.tint(theme.green(), 0.75, 2.8),
                ),
            )
        };
        let crest = if theme.dark {
            shades.tint(theme.foreground, 0.6, 2.9)
        } else {
            mix(sea_top, theme.background, 0.7)
        };
        let (_, roof, _) = shaded(theme, theme.red());
        Self {
            overcast: Overcast::new(theme),
            sea: Ramp::new(sea_top, sea_deep, 8),
            crest,
            strata,
            face,
            shadow,
            grass,
            post: solid(
                theme,
                mix(theme.muted, theme.yellow(), 0.3),
                shades.darkest(),
                0.2,
            ),
            hanging: warning_colour(theme),
            stone: solid(theme, theme.muted, shades.darkest(), 0.2),
            board: error_colour(theme),
            plain: theme.background,
            cottage: [
                ('r', roof),
                ('w', theme.foreground),
                ('y', theme.bright_blue()),
                ('d', solid(theme, theme.muted, shades.darkest(), 0.5)),
            ],
        }
    }
}

pub struct Erosion {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    seed: u32,
    /// How far each pixel row of the face has worn back, as a share of the most it can.
    loss: Vec<f32>,
    stones: Vec<Stone>,
    splashes: Vec<(i32, f32)>,
    since_crumble: f32,
    cloud_drift: f32,
    roughness: Vec<i32>,
}

impl Erosion {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut cliff = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            seed: (rng.next_u64() >> 32) as u32,
            loss: Vec::new(),
            stones: Vec::new(),
            splashes: Vec::new(),
            since_crumble: 0.0,
            cloud_drift: 0.0,
            roughness: Vec::new(),
        };
        cliff.resize(width, height, rng);
        cliff
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The pixel row of the clifftop.
    fn top(&self) -> i32 {
        ((self.tall() as f32 * 0.38) as i32).max(4)
    }

    /// The first pixel row of sea.
    fn sea(&self) -> i32 {
        let tall = self.tall();
        (tall - (tall / 4).max(4)).max(self.top() + 4)
    }

    /// The most pixels the face can wear back.
    fn most_loss(&self) -> f32 {
        (self.width as f32 * MOST_LOSS).max(2.0)
    }

    /// The column of the face on a pixel row, from the start shape less what has worn away.
    fn edge(&self, y: i32) -> i32 {
        let base = (self.width as f32 * 0.5) as i32;
        let rough = self.roughness.get(y.max(0) as usize).copied().unwrap_or(0);
        // the foot of the cliff runs out a little under the water
        let foot = (y - self.sea() + 1).max(0) / 2;
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
        let count = (self.width as i32 / 12).clamp(2, 6);
        (0..count).map(|i| start - (count - 1 - i) * 4).collect()
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
                .map(|&y| {
                    if y <= top + 1 || y >= sea - 3 {
                        2.0
                    } else {
                        1.0
                    }
                })
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
            // a piece takes a few rows together, so the face breaks in chunks, not slivers
            for y in chosen..(chosen + 2).min(sea) {
                let row = &mut self.loss[y as usize];
                *row = (*row + step).min(1.0);
            }
            // the face stays rough but continuous: no row may stand more than two pixels out
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
            vx: rng.range(1.2, 3.2),
            vy: 0.0,
        });
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        let mut mixer = Blender::new(8);
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let top = self.top();
        let sea = self.sea();
        gradient(canvas, &c.overcast.sky, 0, sea);
        clouds(
            canvas,
            &mut mixer,
            &c.overcast.cloud,
            -2,
            (top as f32 * 0.9) as i32,
            self.cloud_drift,
            self.seed,
            0.5,
        );
        // the sea, its waves rolling in towards the cliff
        let speed = self.motion.speed();
        let span = (tall - sea).max(1) as f32;
        for y in sea..tall {
            let depth = (y - sea) as f32 / span;
            let drift = cycle(self.t, (1.0 + 0.4 * depth) * speed / 3.0);
            for x in 0..wide {
                let xf = x as f32;
                let v = 0.6 * (xf / 4.5 + drift + depth * 6.0).sin()
                    + 0.4 * (xf / 2.1 + drift * 1.6 + depth * 9.0).sin();
                let lift = ((v - 0.55) / 0.4).clamp(0.0, 1.0) * (1.0 - depth * 0.5);
                canvas.set(x, y, mixer.mix(c.sea.at(depth), c.crest, lift));
            }
        }
        // the cliff: layered rock lit on its face, grass along the top
        for y in top..tall {
            let edge = self.edge(y).min(wide);
            for x in 0..edge {
                let wobble = (unit(x / 5, y / 3, self.seed) * 2.0) as i32;
                let band = ((y + wobble) / 3).rem_euclid(3) as usize;
                let colour = if x >= edge - 1 {
                    c.face
                } else if x >= edge - 3 && y > top + 1 {
                    mixer.mix(c.strata[band], c.shadow, 0.5)
                } else {
                    c.strata[band]
                };
                canvas.set(x, y, colour);
            }
            if y < top + 2 {
                for x in 0..edge {
                    canvas.set(x, y, if y == top { c.grass.0 } else { c.grass.1 });
                }
            }
        }
        // surf breaking white at the foot of the cliff
        for k in 0..3 {
            let x = self.edge(sea + k) + 1 + k;
            canvas.set(x, sea + k, c.crest);
            canvas.set(x + 1, sea + k, c.crest);
        }
        // the cottage, far back from the edge and still safe
        if self.width >= 50 && top >= 9 {
            canvas.sprite(2, top - COTTAGE.len() as i32, &COTTAGE, &c.cottage);
        }
        // the fence: posts on solid ground stand up, the ones past the edge hang down the face
        let edge = self.edge(top);
        let posts = self.fence_posts();
        let mut hanging = 0;
        for (i, &x) in posts.iter().enumerate() {
            if x < edge {
                canvas.fill_rect(x, top - 4, 1, 4, c.post);
                if let Some(&next) = posts.get(i + 1).filter(|&&n| n < edge) {
                    for rail in x + 1..next {
                        canvas.set(rail, top - 3, c.post);
                        canvas.set(rail, top - 1, c.post);
                    }
                }
            } else {
                hanging += 1;
            }
        }
        // the hanging length follows the face down, its posts dangling crosswise
        let mut last = (edge as f32, top as f32 - 2.0);
        for i in 0..(hanging.min(3) * 3 + 2) {
            let y = top + i;
            if y >= sea {
                break;
            }
            let here = ((self.edge(y) + 1 + i % 2) as f32, y as f32);
            line(canvas, last, here, 1, c.hanging);
            if i % 4 == 3 {
                canvas.fill_rect(here.0 as i32, y, 3, 1, c.hanging);
            }
            last = here;
        }
        // a KEEP BACK board on a post well back from the edge, framed in the error colour with
        // its middle left plain for the words written over it after the canvas
        if let (Some(sx), Some(row)) = (self.sign_x(), self.board_row()) {
            let len = BOARD_TEXT.len() as i32;
            let left = sx - len / 2 - 2;
            let board_top = 2 * row - 1;
            canvas.fill_rect(left, board_top, len + 4, 4, c.board);
            canvas.fill_rect(left + 1, board_top + 1, len + 2, 2, c.plain);
            canvas.fill_rect(sx, board_top + 4, 1, top - board_top - 4, c.post);
        }

        for stone in &self.stones {
            canvas.fill_rect(
                stone.x.round() as i32,
                stone.y.floor() as i32,
                2,
                2,
                c.stone,
            );
        }
        for &(x, age) in &self.splashes {
            if age < SPLASH_LENGTH / 2.0 {
                canvas.fill_rect(x, sea - 1, 2, 2, c.crest);
            } else {
                canvas.set(x - 2, sea - 1, c.crest);
                canvas.set(x + 3, sea - 1, c.crest);
            }
        }
    }

    fn sign_x(&self) -> Option<i32> {
        let sign = self.fence_posts()[0] - 4;
        (sign >= 15 && self.top() >= 14).then_some(sign)
    }

    /// The text row of the KEEP BACK board, with its frame clear of the top edge and its foot
    /// a post's height above the clifftop.
    fn board_row(&self) -> Option<i32> {
        let row = (self.top() - 10) / 2;
        (row >= 1).then_some(row)
    }
}

impl Scene for Erosion {
    fn name(&self) -> &'static str {
        "erosion"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        // what has worn away is kept, row for row by its height on the screen
        let old = std::mem::take(&mut self.loss);
        let old_rows = (self.height * 2).max(1);
        self.width = width.max(1);
        self.height = height.max(1);
        let rows = self.height * 2;
        self.loss = (0..rows)
            .map(|y| {
                let from = y * old_rows / rows;
                old.get(from).copied().unwrap_or(0.0)
            })
            .collect();
        let mut wobble = 0_i32;
        self.roughness = (0..rows)
            .map(|_| {
                wobble = (wobble + rng.below(3) as i32 - 1).clamp(-1, 1);
                wobble
            })
            .collect();
        self.stones.clear();
        self.splashes.clear();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        self.cloud_drift = wrap(self.cloud_drift + 0.8 * speed * dt, 100_000.0);
        self.since_crumble += dt;
        // a screen too short for a cliff and a sea has nothing to wear away
        if self.since_crumble >= self.period() && self.sea() < self.tall() {
            self.since_crumble = 0.0;
            self.crumble(rng);
        }
        let sea = self.sea() as f32;
        let edges: Vec<i32> = (0..=self.tall()).map(|y| self.edge(y)).collect();
        let mut landed = Vec::new();
        for stone in &mut self.stones {
            stone.vy += 18.0 * speed * dt;
            stone.x += stone.vx * speed * dt;
            stone.y += stone.vy * dt;
            let row = (stone.y.max(0.0) as usize).min(edges.len() - 1);
            stone.x = stone.x.max(edges[row] as f32 + 1.0);
            if stone.y >= sea {
                landed.push(stone.x.round() as i32);
            }
        }
        self.stones.retain(|s| s.y < sea);
        let foot = self.edge(self.sea()) + 3;
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
        let colours = Colours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
        if let (Some(sx), Some(row)) = (self.sign_x(), self.board_row()) {
            let x = sx - BOARD_TEXT.len() as i32 / 2;
            paint::label(frame, x, row, BOARD_TEXT, error_colour(theme));
        }
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
        check_scene("erosion");
        check_determinism("erosion");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("erosion", 40.0);
    }

    #[test]
    fn the_edge_creeps_back_then_holds_with_the_fence_hanging() {
        let mut rng = Rng::new(12);
        let mut cliff = Erosion::new(100, 30, Motion::FULL, &mut rng);
        let rows = cliff.tall();
        let start: Vec<i32> = (0..rows).map(|y| cliff.edge(y)).collect();
        for _ in 0..(60 * 20) {
            cliff.step(0.05, &mut rng);
        }
        let worn = (0..rows)
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
        let theme = theme::default_theme();
        let c = Colours::new(theme);
        let frame = render(&mut cliff, 100, 30, 1);
        // the fence hangs down the face below the clifftop
        let below = (cliff.top() + 2) / 2..cliff.sea() / 2;
        let hangs = below.clone().any(|row| {
            (0..100).any(|x| {
                frame
                    .get(x, row)
                    .and_then(|cell| cell.pixel_pair())
                    .is_some_and(|(a, b)| a == c.hanging || b == c.hanging)
            })
        });
        assert!(hangs, "the fence should hang");
        assert!(frame.to_text().contains("KEEP BACK"));
        assert!(pixels(&frame).contains(&warning_colour(theme)));
    }

    #[test]
    fn pieces_break_away_slowly_on_every_level() {
        let mut rng = Rng::new(1);
        for motion in Motion::ALL {
            let cliff = Erosion::new(80, 24, motion, &mut rng);
            assert!(cliff.period() >= CRUMBLE_PERIOD);
        }
        const { assert!(SPLASH_LENGTH / 2.0 > 1.0 / 3.0) };
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

    #[test]
    fn the_layers_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            apart(
                theme,
                c.sea.at(0.0),
                c.sea.at(1.0),
                0.03,
                "shallow and deep sea",
            );
            apart(theme, c.crest, c.sea.at(0.2), 0.03, "the surf and the sea");
            apart(theme, c.strata[0], c.strata[1], 0.015, "the layers of rock");
            apart(theme, c.face, c.strata[0], 0.03, "the face and the rock");
            apart(
                theme,
                c.strata[0],
                c.overcast.sky.at(1.0),
                0.03,
                "the rock and the sky",
            );
            apart(
                theme,
                c.hanging,
                c.face,
                0.1,
                "the hanging fence and the face",
            );
            for colour in c
                .sea
                .steps()
                .iter()
                .chain(c.strata.iter())
                .chain([&c.face, &c.crest])
            {
                assert!(
                    theme.contrast_of(*colour) < IMAGERY_CEILING,
                    "{}",
                    theme.name
                );
            }
            for colour in [c.post, c.hanging, c.stone] {
                assert!(
                    theme.contrast_of(colour) >= paint::SOLID - 0.01,
                    "{}",
                    theme.name
                );
            }
        }
    }
}
