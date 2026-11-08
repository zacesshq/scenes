//! An overgrown garden in solid pixels: a brick wall with an iron gate between two stone posts
//! and a paved path running up to it through the lawn. Vines climb the posts, the wall and the
//! gate's bars, leafing out as they go, while weeds push up between the paving slabs until the
//! path is lost and the gate is half hidden. The growth takes about a minute and a half and then
//! holds, the vine tips stirring slowly.
//!
//! The sky, the wall, the lawn and the paving are tints of the background held below the
//! non-text floor; the posts, the gate, the vines, the weeds and their flowers keep the theme's
//! own drawing colours, so the overgrowth reads as shapes and keeps its outline when colour is
//! off.

use super::cycle;
use super::paint::{noise, tint, unit};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "Weeds and vines slowly swallowing a garden path and its gate until the way through is lost";
pub const CAPTIONS: &[&str] = &[
    "Left too long. Something needs weeding out.",
    "The path is overgrown. Clear the first error.",
    "Nobody has been through here in a while.",
];

/// How long the growth takes to reach its full spread at the standard pace.
pub const GROW_SECONDS: f32 = 90.0;
/// How far the growth has come when the scene starts, so the neglect shows from the start.
const GROW_START: f32 = 12.0;
/// One stir of the vine tips and the weed heads, a pixel either way.
pub const SWAY_PERIOD: f32 = 9.0;

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct Colours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub brick: Rgb,
    pub mortar: Rgb,
    pub lawn: Rgb,
    pub blade: Rgb,
    pub slab: Rgb,
    pub joint: Rgb,
    pub post: Rgb,
    pub iron: Rgb,
    pub stem: Rgb,
    pub leaf: Rgb,
    pub flower: Rgb,
    sky: Ramp,
}

impl Colours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let dark = theme.dark;
        let (sky_top, sky_low) = if dark {
            (
                tint(theme, theme.info, 0.12, 1.25),
                tint(theme, theme.info, 0.3, 1.6),
            )
        } else {
            (
                tint(theme, theme.info, 0.36, 1.75),
                tint(theme, theme.info, 0.1, 1.15),
            )
        };
        let joint = if dark {
            tint(theme, theme.muted, 0.18, 1.25)
        } else {
            tint(theme, theme.muted, 0.75, 2.7)
        };
        Self {
            sky_top,
            sky_low,
            brick: tint(theme, theme.red(), 0.42, 1.95),
            mortar: tint(theme, theme.muted, 0.22, 1.3),
            lawn: tint(theme, theme.green(), 0.4, 1.9),
            blade: tint(theme, theme.green(), 0.6, 2.5),
            slab: tint(theme, theme.muted, 0.42, 1.9),
            joint,
            post: theme.muted,
            iron: theme.grey(),
            stem: theme.green(),
            leaf: theme.bright_green(),
            flower: theme.yellow(),
            sky: Ramp::new(sky_top, sky_low, 8),
        }
    }

    #[cfg(test)]
    pub(crate) fn palette(theme: &ThemeData) -> crate::scenes::testing::Palette {
        let c = Self::new(theme);
        crate::scenes::testing::Palette {
            imagery: vec![
                ("sky top", c.sky_top),
                ("sky low", c.sky_low),
                ("brick", c.brick),
                ("mortar", c.mortar),
                ("lawn", c.lawn),
                ("grass", c.blade),
                ("slab", c.slab),
                ("joint", c.joint),
            ],
            shapes: vec![
                ("post", c.post),
                ("gate", c.iron),
                ("stem", c.stem),
                ("leaf", c.leaf),
                ("flower", c.flower),
            ],
        }
    }
}

/// One pixel of growth: where it is, when it appears on the growth clock and what it is.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Bud {
    x: i32,
    y: i32,
    at: f32,
    kind: Part,
    /// How far up its stem it is, from 0 at the root to 1 at the tip; the tips stir.
    height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Stem,
    Leaf,
    Flower,
}

pub struct Overgrown {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// The growth's own clock, run at the motion speed.
    grow_t: f32,
    buds: Vec<Bud>,
}

impl Overgrown {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut garden = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            grow_t: GROW_START,
            buds: Vec::new(),
        };
        garden.resize(width, height, rng);
        garden
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    fn wall_top(&self) -> i32 {
        (self.tall() as f32 * 0.3).round() as i32
    }

    /// The pixel row where the wall meets the lawn.
    fn ground(&self) -> i32 {
        ((self.tall() as f32 * 0.6).round() as i32).max(self.wall_top() + 1)
    }

    /// The gate's opening: its left and right columns, inside the posts.
    fn gate(&self) -> (i32, i32) {
        let w = self.width as i32;
        let half = (w / 10).max(3);
        (w / 2 - half, w / 2 + half)
    }

    fn post_width(&self) -> i32 {
        (self.width as i32 / 40).max(2)
    }

    /// The path's left and right edges at a pixel row, widening towards the viewer.
    fn path_at(&self, y: i32) -> (f32, f32) {
        let (left, right) = self.gate();
        let depth = (self.tall() - self.ground()).max(1) as f32;
        let near = ((y - self.ground()) as f32 / depth).clamp(0.0, 1.0);
        let spread = near * self.width as f32 * 0.22;
        (left as f32 - spread, right as f32 + spread)
    }

    fn progress(&self) -> f32 {
        (self.grow_t / GROW_SECONDS).clamp(0.0, 1.0)
    }

    /// A vine from its root, climbing and wandering, with leaves either side and now and then
    /// a flower. `delay` is when it starts on the growth clock and `top` the highest row it may
    /// reach.
    fn vine(&mut self, rng: &mut Rng, root: (i32, i32), top: i32, delay: f32, lean: f32) {
        let (mut x, mut y) = (root.0 as f32, root.1 as f32);
        let length = (root.1 - top).max(2) as f32;
        let rate = GROW_SECONDS * 0.7 / length.max(1.0);
        let mut walked = 0.0;
        while y > top as f32 && walked < length * 1.6 {
            y -= 1.0;
            x += lean + rng.range(-0.5, 0.5);
            walked += 1.0;
            let at = delay + walked * rate;
            let height = walked / length;
            let (px, py) = (x.round() as i32, y.round() as i32);
            self.buds.push(Bud {
                x: px,
                y: py,
                at,
                kind: Part::Stem,
                height,
            });
            if walked as i32 % 2 == 0 {
                let side = if (walked as i32 / 2) % 2 == 0 { 1 } else { -1 };
                for dx in 1..=2 {
                    self.buds.push(Bud {
                        x: px + side * dx,
                        y: py - (dx - 1),
                        at: at + 1.5,
                        kind: Part::Leaf,
                        height,
                    });
                }
            }
            if rng.chance(0.04) {
                self.buds.push(Bud {
                    x: px + 1,
                    y: py - 1,
                    at: at + 6.0,
                    kind: Part::Flower,
                    height,
                });
            }
        }
    }

    /// A weed between two slabs: a few short stems fanning out from one spot, taller nearer the
    /// viewer.
    fn weed(&mut self, rng: &mut Rng, root: (i32, i32), size: f32, delay: f32) {
        let stems = 2 + rng.below(2) as i32;
        for s in 0..stems {
            let lean = (s - stems / 2) as f32 * 0.5;
            let tall = (size * rng.range(0.6, 1.0)).max(1.0);
            let mut x = root.0 as f32;
            for i in 0..tall as i32 {
                x += lean * 0.5;
                let height = (i + 1) as f32 / tall;
                self.buds.push(Bud {
                    x: x.round() as i32,
                    y: root.1 - i,
                    at: delay + i as f32 * 2.5,
                    kind: if i + 1 == tall as i32 && i >= 2 {
                        Part::Leaf
                    } else {
                        Part::Stem
                    },
                    height,
                });
            }
            if tall >= 4.0 && rng.chance(0.4) {
                self.buds.push(Bud {
                    x: x.round() as i32,
                    y: root.1 - tall as i32,
                    at: delay + tall * 2.5 + 4.0,
                    kind: Part::Flower,
                    height: 1.0,
                });
            }
        }
    }

    fn lay_out(&mut self, rng: &mut Rng) {
        self.buds.clear();
        let (w, tall) = (self.width as i32, self.tall());
        if w < 12 || tall < 10 {
            return;
        }
        let (ground, wall_top) = (self.ground(), self.wall_top());
        let (left, right) = self.gate();
        let pw = self.post_width();
        let span = GROW_SECONDS * 0.6;
        // vines up both posts and every bar of the gate, the posts first
        self.vine(rng, (left - pw / 2 - 1, ground), wall_top - 3, 0.0, 0.05);
        self.vine(rng, (right + pw / 2 + 1, ground), wall_top - 3, 4.0, -0.05);
        let mut x = left + 2;
        while x < right - 1 {
            let delay = rng.range(8.0, span);
            let lean = rng.range(-0.15, 0.15);
            self.vine(rng, (x, ground), wall_top + 2, delay, lean);
            x += 3 + rng.below(2) as i32;
        }
        // vines along the wall either side of the gate
        let mut x = 2;
        while x < w - 2 {
            if x < left - pw - 2 || x > right + pw + 2 {
                let delay = rng.range(0.0, span);
                let lean = rng.range(-0.3, 0.3);
                let top = wall_top + rng.below(((ground - wall_top) / 2).max(1) as usize) as i32;
                self.vine(rng, (x, ground), top, delay, lean);
            }
            x += 4 + rng.below(4) as i32;
        }
        // weeds between the slabs, spreading from the gate end of the path to the near end
        let rows = (tall - ground).max(1);
        let count = (rows * w / 30).clamp(4, 80);
        for _ in 0..count {
            let y = ground + 1 + rng.below(rows.max(1) as usize) as i32;
            let (l, r) = self.path_at(y);
            let x = rng.range(l, r.max(l + 1.0)) as i32;
            let near = (y - ground) as f32 / rows as f32;
            let delay = rng.range(0.0, GROW_SECONDS * 0.75);
            self.weed(rng, (x, y), 2.0 + near * 5.0, delay);
        }
        // everything has grown by the end of the growth clock, however long a vine ran
        let last = self.buds.iter().map(|b| b.at).fold(1.0_f32, f32::max);
        if last > GROW_SECONDS {
            for bud in &mut self.buds {
                bud.at *= GROW_SECONDS / last;
            }
        }
        // rows further down are nearer the viewer, so they are drawn last and on top
        self.buds.sort_by_key(|b| b.y);
    }

    /// How far a bud stirs on the sway, a whole pixel at most and only near the tips.
    fn sway(&self, bud: &Bud) -> i32 {
        if bud.height < 0.75 {
            return 0;
        }
        let phase = cycle(
            self.t,
            std::f32::consts::TAU / SWAY_PERIOD * self.motion.pace(),
        );
        let wobble = (phase + bud.x as f32 * 0.4).sin();
        if wobble > 0.6 {
            1
        } else if wobble < -0.6 {
            -1
        } else {
            0
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        let (wide, tall) = (canvas.width() as i32, canvas.height() as i32);
        let (ground, wall_top) = (self.ground(), self.wall_top());
        let (left, right) = self.gate();
        let pw = self.post_width();

        for y in 0..ground.min(tall) {
            canvas.fill_row(y, c.sky.at(y as f32 / ground.max(1) as f32));
        }
        // the brick wall, courses offset by half a brick
        for y in wall_top.max(0)..ground.min(tall) {
            let course = (y - wall_top) / 2;
            for x in 0..wide {
                let offset = if course % 2 == 0 { 0 } else { 3 };
                let mortar = (y - wall_top) % 2 == 1 || (x + offset) % 6 == 0;
                canvas.set(x, y, if mortar { c.mortar } else { c.brick });
            }
        }
        // the gateway shows the garden beyond: sky over a far lawn
        for y in wall_top.max(0)..ground.min(tall) {
            for x in left..=right {
                let far = y > (wall_top + ground) / 2;
                canvas.set(x, y, if far { c.lawn } else { c.sky.at(1.0) });
            }
        }
        // the lawn with fixed tufts of grass
        for y in ground..tall {
            for x in 0..wide {
                let tuft = unit(x, y, 71) < 0.12 || noise(x as f32 * 0.3, y as f32, 72) > 0.85;
                canvas.set(x, y, if tuft { c.blade } else { c.lawn });
            }
        }
        // the paving, slabs getting bigger towards the viewer
        let depth = (tall - ground).max(1) as f32;
        for y in ground..tall {
            let (l, r) = self.path_at(y);
            let near = (y - ground) as f32 / depth;
            let row = (near.sqrt() * 5.0) as i32;
            let next_row = ((((y + 1 - ground) as f32 / depth).sqrt()) * 5.0) as i32;
            for x in l.round() as i32..=r.round() as i32 {
                let across = (x as f32 - l) / (r - l).max(1.0);
                let column = (across * 3.0 + if row % 2 == 0 { 0.0 } else { 0.5 }).fract();
                let joint = next_row != row || column < 0.06 || x == l.round() as i32;
                canvas.set(x, y, if joint { c.joint } else { c.slab });
            }
        }

        // the capped posts and the iron gate between them
        for post in [left - pw, right + 1] {
            canvas.fill_rect(post, wall_top - 2, pw, ground - wall_top + 2, c.post);
            canvas.fill_rect(post - 1, wall_top - 3, pw + 2, 1, c.post);
        }
        if right - left >= 4 {
            for y in [wall_top + 1, (wall_top + ground) / 2, ground - 1] {
                for x in left..=right {
                    canvas.set(x, y, c.iron);
                }
            }
            let mut x = left + 1;
            while x < right {
                for y in wall_top + 1..ground {
                    canvas.set(x, y, c.iron);
                }
                canvas.set(x, wall_top, c.iron);
                x += 2;
            }
        }

        let grown = self.progress() * GROW_SECONDS;
        for bud in self.buds.iter().filter(|b| b.at <= grown) {
            let colour = match bud.kind {
                Part::Stem => c.stem,
                Part::Leaf => c.leaf,
                Part::Flower => c.flower,
            };
            canvas.set(bud.x + self.sway(bud), bud.y, colour);
        }
    }
}

impl Scene for Overgrown {
    fn name(&self) -> &'static str {
        "overgrown"
    }

    fn resize(&mut self, width: usize, height: usize, _rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        // the growth is laid out from its own seed, so a resize keeps the same garden and the
        // growth reached so far
        let mut layout = Rng::new(0x9a7e);
        self.lay_out(&mut layout);
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
        self.grow_t += dt * self.motion.speed();
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

    fn shown(garden: &Overgrown) -> usize {
        let grown = garden.progress() * GROW_SECONDS;
        garden.buds.iter().filter(|b| b.at <= grown).count()
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("overgrown");
        check_determinism("overgrown");
    }

    #[test]
    fn passes_the_pixel_checks() {
        check_pixel_scene("overgrown", 40.0);
        check_palette(Colours::palette);
    }

    #[test]
    fn the_garden_stays_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            for (a, b, what) in [
                (c.sky_top, c.sky_low, "the sky"),
                (c.brick, c.mortar, "the bricks and the mortar"),
                (c.lawn, c.slab, "the lawn and the path"),
                (c.slab, c.joint, "the slabs and their joints"),
                (c.lawn, c.blade, "the lawn and its grass"),
            ] {
                assert!(distance(a, b) >= 0.03, "{what} in {}", theme.name);
            }
        }
    }

    #[test]
    fn the_growth_creeps_on_then_holds_and_survives_a_resize() {
        let mut rng = Rng::new(1);
        let mut garden = Overgrown::new(80, 24, Motion::FULL, &mut rng);
        let start = shown(&garden);
        assert!(start > 0);
        let mut last = start;
        for _ in 0..(120 * 20) {
            garden.step(0.05, &mut rng);
            let now = shown(&garden);
            assert!(now - last <= 20, "{last} then {now}");
            last = now;
        }
        assert_eq!(last, garden.buds.len());
        assert!(last > start * 3, "{start} then {last}");
        garden.resize(1, 1, &mut rng);
        garden.resize(80, 24, &mut rng);
        assert_eq!(shown(&garden), last);
    }

    #[test]
    fn the_tips_stir_slowly() {
        let mut rng = Rng::new(1);
        let garden = Overgrown::new(80, 24, Motion::FULL, &mut rng);
        let tip = garden
            .buds
            .iter()
            .copied()
            .find(|b| b.height >= 0.9)
            .expect("a tip");
        for motion in Motion::ALL {
            let mut probe = Overgrown::new(80, 24, motion, &mut rng);
            let gap = shortest_gap(60.0, |t| {
                probe.t = t;
                (probe.sway(&tip) + 1) as u32
            });
            assert!(gap >= 1.0, "{motion:?}: {gap}");
        }
    }
}
