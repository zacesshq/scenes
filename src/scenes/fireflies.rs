//! Fireflies: a meadow at dusk, painted in solid pixels. A moon hangs in a sky that warms towards
//! the hills, a line of trees and one great tree stand dark against it and long grass sways in
//! the foreground while fireflies drift and glow, each light softened by a halo. Far lights are
//! small and faint, near ones larger.
//!
//! A light theme cannot hold a night sky, so there the same meadow is a bright summer afternoon:
//! a sun and slow clouds over the hills, wild flowers in the grass, butterflies drifting about
//! and the fireflies' paths taken by motes of pollen floating in the light.
//!
//! The sky, the hills, the trees, the grass and every halo are tints of the background held below
//! the non-text floor, so they read as background. The moon, the lights, the flowers and the
//! butterflies keep the theme's own drawing colours, so they stand out as shapes and keep their
//! outline when colour is off. A light grows its halo as it brightens, so its brightness is a
//! change of shape as well as of colour.

use std::f32::consts::TAU;

use super::paint::{Cloud, Wave, cloud_tones, disc, noise, strong, unit};
use super::{count_for, cycle};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, IMAGERY_CEILING, Lab, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// The quickest pulse, in seconds per cycle, well under one flash a second.
pub const MIN_PULSE_PERIOD: f32 = 2.5;
/// Seconds a butterfly's wings take to open and close.
const FLUTTER_PERIOD: f32 = 1.3;
const SKY_STEPS: usize = 10;

/// A butterfly with its wings open, half open and folded: `w` wing, `W` wing edge, `b` body.
const WINGS_OPEN: [&str; 4] = ["WWW.WWW", "WwwbwwW", ".WwbwW.", ".ww.ww."];
const WINGS_HALF: [&str; 4] = ["..W.W..", ".WwbwW.", "..wbw..", "..w.w.."];
const WINGS_SHUT: [&str; 4] = ["...W...", "..Wb...", "..wb...", "...w..."];

#[derive(Clone, Debug)]
struct Fly {
    /// Where it is, in pixels.
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    phase: f32,
    period: f32,
}

#[derive(Clone, Debug)]
struct Butterfly {
    centre: f32,
    reach: f32,
    rate: f32,
    phase: f32,
    row: f32,
}

/// A tree on the far side of the meadow: a trunk and a crown of overlapping rounds.
#[derive(Clone, Debug)]
struct Tree {
    x: f32,
    /// The crown's radius in pixels.
    r: f32,
    /// How far the crown sits above the meadow's edge.
    lift: f32,
    seed: u32,
}

/// A blade of grass rising from `base`, swaying at its tip.
#[derive(Clone, Debug)]
struct Blade {
    x: i32,
    base: i32,
    height: i32,
    phase: f32,
    /// Whether it carries a seed head.
    head: bool,
}

/// The colours of the meadow in one theme, every one derived from the theme's own.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct MeadowColours {
    pub dark: bool,
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub star: Rgb,
    /// The moon by night, the sun by day.
    pub orb: Rgb,
    pub halo: Rgb,
    pub halo_far: Rgb,
    pub cloud: Rgb,
    pub cloud_shade: Rgb,
    pub hills_far: Rgb,
    pub hills_near: Rgb,
    pub trees: Rgb,
    pub trees_lit: Rgb,
    pub big_tree: Rgb,
    pub big_tree_lit: Rgb,
    pub grass_far: Rgb,
    pub grass_near: Rgb,
    pub blade: Rgb,
    pub blade_lit: Rgb,
    pub seed: Rgb,
    pub dim: Rgb,
    pub light: Rgb,
    pub bright: Rgb,
    pub glow: Rgb,
    pub faint_glow: Rgb,
    pub pollen: Rgb,
    pub flowers: [Rgb; 3],
    pub wings: [Rgb; 3],
    pub body: Rgb,
    sky: Ramp,
    grass: Ramp,
}

/// A colour's own hue lifted to a bright, clear gold: sunlight in a pale sky.
fn sunlit(colour: Rgb) -> Rgb {
    let lab = Lab::from_rgb(colour);
    let chroma = (lab.a * lab.a + lab.b * lab.b).sqrt().max(0.001);
    let lift = 0.13_f32.max(chroma) / chroma;
    Lab {
        l: 0.88,
        a: lab.a * lift,
        b: lab.b * lift,
    }
    .to_rgb()
}

impl MeadowColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let s = Shades::new(theme);
        let green = theme.green();
        let light = theme.yellow();
        let (cloud, cloud_shade) = cloud_tones(theme, IMAGERY_CEILING - 0.3);
        if theme.dark {
            // dusk: deep blue overhead warming to a glow over the hills, with the trees dark
            // against it and the moon lighting their edges
            let sky_top = s.tint(theme.blue(), 0.2, 1.3);
            let sky_low = s.tint(mix(theme.magenta(), theme.warning, 0.5), 0.3, 1.75);
            let moon = strong(theme, mix(theme.foreground, theme.yellow(), 0.25));
            Self {
                dark: true,
                sky_top,
                sky_low,
                star: s.tint(theme.foreground, 0.5, 2.0),
                orb: moon,
                halo: s.tint(theme.foreground, 0.25, 1.75),
                halo_far: s.tint(theme.foreground, 0.16, 1.5),
                cloud,
                cloud_shade,
                hills_far: s.tint(mix(theme.blue(), theme.magenta(), 0.3), 0.3, 1.5),
                hills_near: s.tint(mix(theme.blue(), green, 0.4), 0.17, 1.27),
                trees: s.tint(green, 0.08, 1.1),
                trees_lit: s.tint(green, 0.26, 1.45),
                big_tree: s.tint(green, 0.03, 1.04),
                big_tree_lit: s.tint(green, 0.2, 1.32),
                grass_far: s.tint(green, 0.12, 1.16),
                grass_near: s.tint(green, 0.24, 1.4),
                blade: s.tint(green, 0.34, 1.65),
                blade_lit: s.tint(green, 0.48, 2.0),
                seed: s.tint(theme.yellow(), 0.45, 2.1),
                dim: s.held(light, sky_top, 0.4, IMAGERY_CEILING + 0.2),
                light,
                bright: theme.bright_yellow(),
                glow: s.tint(theme.yellow(), 0.6, IMAGERY_CEILING - 0.2),
                faint_glow: s.tint(theme.yellow(), 0.35, 2.0),
                pollen: s.tint(theme.yellow(), 0.5, 2.4),
                flowers: [theme.magenta(), theme.blue(), theme.yellow()],
                wings: [theme.magenta(), theme.cyan(), theme.yellow()],
                body: theme.bright(),
                sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
                grass: Ramp::new(s.tint(green, 0.12, 1.16), s.tint(green, 0.24, 1.4), 6),
            }
        } else {
            // a summer afternoon: a pale sky over soft hills, trees a little darker than the
            // grass and a gold sun
            let sky_top = s.tint(theme.info, 0.45, 1.7);
            let sky_low = s.tint(theme.info, 0.08, 1.08);
            let sun = sunlit(theme.yellow());
            let gold = strong(theme, sun);
            Self {
                dark: false,
                sky_top,
                sky_low,
                star: sky_top,
                orb: sun,
                halo: mix(theme.background, sun, 0.3),
                halo_far: mix(theme.background, sun, 0.12),
                cloud,
                cloud_shade,
                hills_far: s.tint(mix(theme.info, green, 0.45), 0.3, 1.3),
                hills_near: s.tint(green, 0.32, 1.5),
                trees: s.tint(green, 0.6, 2.2),
                trees_lit: s.tint(green, 0.42, 1.75),
                big_tree: s.tint(green, 0.75, 2.6),
                big_tree_lit: s.tint(green, 0.55, 2.05),
                grass_far: s.tint(green, 0.32, 1.45),
                grass_near: s.tint(green, 0.45, 1.8),
                blade: s.tint(green, 0.62, 2.3),
                blade_lit: s.tint(green, 0.4, 1.65),
                seed: s.tint(gold, 0.8, 2.2),
                dim: gold,
                light: gold,
                bright: gold,
                glow: s.tint(gold, 0.5, 1.5),
                faint_glow: s.tint(gold, 0.3, 1.25),
                pollen: s.tint(gold, 1.0, 2.3),
                flowers: [theme.magenta(), theme.blue(), gold],
                wings: [theme.magenta(), theme.cyan(), gold],
                body: theme.bright(),
                sky: Ramp::new(sky_top, sky_low, SKY_STEPS),
                grass: Ramp::new(s.tint(green, 0.32, 1.45), s.tint(green, 0.45, 1.8), 6),
            }
        }
    }
}

pub struct Fireflies {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    flies: Vec<Fly>,
    butterflies: Vec<Butterfly>,
    clouds: Vec<Cloud>,
    hills: [Wave; 2],
    trees: Vec<Tree>,
    /// Which side the great tree stands on; the moon or the sun is on the other.
    big_left: bool,
    blades: Vec<Blade>,
    stars: Vec<(i32, i32)>,
    /// Wild flowers in the grass, as (column, row, colour index).
    flowers: Vec<(i32, i32, usize)>,
}

impl Fireflies {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut meadow = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            flies: Vec::new(),
            butterflies: Vec::new(),
            clouds: Vec::new(),
            hills: [Wave::new(rng, 18.0), Wave::new(rng, 11.0)],
            trees: Vec::new(),
            big_left: rng.chance(0.5),
            blades: Vec::new(),
            stars: Vec::new(),
            flowers: Vec::new(),
        };
        meadow.resize(width, height, rng);
        meadow
    }

    fn tall(&self) -> f32 {
        self.height as f32 * 2.0
    }

    /// The pixel row where the meadow meets the foot of the trees.
    fn meadow(&self) -> i32 {
        ((self.tall() * 0.66).round() as i32).max(1)
    }

    /// The lowest pixel row a light may reach: just above the bottom.
    fn bottom(&self) -> f32 {
        (self.tall() - 2.0).max(0.0)
    }

    fn top_speed(&self) -> f32 {
        if self.motion.calm { 1.4 } else { 3.0 }
    }

    /// The moon's or the sun's middle and radius.
    fn orb(&self) -> (f32, f32, f32) {
        let tall = self.tall();
        let r = (tall * 0.07).clamp(1.5, 9.0);
        let w = self.width as f32;
        let x = if self.big_left { w * 0.78 } else { w * 0.22 };
        (x, (tall * 0.2).max(r + 1.0), r)
    }

    /// The great tree's foot and crown radius.
    fn big_tree(&self) -> (f32, f32) {
        let w = self.width as f32;
        let x = if self.big_left { w * 0.1 } else { w * 0.9 };
        (x, (self.tall() * 0.2).clamp(3.0, 40.0))
    }

    /// A crown of overlapping rounds, lit on the side towards the sky's light: the lit rounds
    /// are painted first and the shaded body over them a little away from the light, leaving a
    /// lit edge.
    #[allow(clippy::too_many_arguments)]
    fn crown(
        canvas: &mut Canvas,
        cx: f32,
        cy: f32,
        r: f32,
        seed: u32,
        toward: f32,
        body: Rgb,
        lit: Rgb,
    ) {
        let puffs: [(f32, f32, f32); 6] = [
            (0.0, 0.0, 1.0),
            (-0.7, 0.25, 0.72),
            (0.7, 0.3, 0.7),
            (-0.35, -0.5, 0.66),
            (0.4, -0.42, 0.62),
            (0.05, 0.5, 0.72),
        ];
        let shift = (r * 0.14).max(1.0);
        for pass in 0..2 {
            for (i, &(dx, dy, k)) in puffs.iter().enumerate() {
                let wobble = 0.85 + 0.3 * unit(i as i32, 0, seed);
                let (x, y) = (cx + dx * r, cy + dy * r);
                let rr = r * k * wobble;
                if pass == 0 {
                    disc(canvas, x, y, rr, lit);
                } else {
                    disc(canvas, x - toward * shift, y + shift * 0.7, rr, body);
                }
            }
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &MeadowColours) {
        let tall = canvas.height() as i32;
        let wide = canvas.width() as i32;
        let meadow = self.meadow();
        let tall_f = self.tall();
        // the side the light comes from: +1 for the right
        let toward = if self.big_left { 1.0 } else { -1.0 };

        for y in 0..tall {
            canvas.fill_row(y, c.sky.at(y as f32 / meadow.max(1) as f32));
        }
        let (ox, oy, r) = self.orb();
        if c.dark {
            for &(x, y) in &self.stars {
                canvas.set(x, y, c.star);
            }
        }
        if self.height >= 6 {
            let reach = if c.dark { (2.6, 1.7) } else { (2.0, 1.45) };
            disc(canvas, ox, oy, r * reach.0, c.halo_far);
            disc(canvas, ox, oy, r * reach.1, c.halo);
            disc(canvas, ox, oy, r, c.orb);
        }
        if !c.dark {
            for cloud in &self.clouds {
                cloud.paint(canvas, c.cloud, c.cloud_shade);
            }
        }

        // two bands of hills, the far one paler and higher
        let horizon = meadow as f32;
        for x in 0..wide {
            let far = horizon - tall_f * 0.11 + self.hills[0].at(x as f32) * tall_f * 0.05;
            let near = horizon - tall_f * 0.04 + self.hills[1].at(x as f32) * tall_f * 0.03;
            let far = far.round() as i32;
            let near = near.round() as i32;
            canvas.fill_rect(x, far, 1, meadow - far, c.hills_far);
            canvas.fill_rect(x, near, 1, meadow - near, c.hills_near);
        }

        // the tree line along the far edge of the meadow
        for tree in &self.trees {
            let cy = horizon - tree.lift - tree.r;
            canvas.fill_rect(
                tree.x.round() as i32,
                cy as i32,
                1,
                meadow - cy as i32,
                c.trees,
            );
            Self::crown(
                canvas,
                tree.x,
                cy,
                tree.r,
                tree.seed,
                toward,
                c.trees,
                c.trees_lit,
            );
        }

        // the meadow, deeper green towards the front
        let depth = (tall - meadow).max(1) as f32;
        for y in meadow..tall {
            canvas.fill_row(y, c.grass.at((y - meadow) as f32 / depth));
        }

        // the great tree standing in the meadow to one side
        if self.height >= 8 {
            let (tx, tr) = self.big_tree();
            let foot = tall - (depth * 0.25) as i32;
            let crown_y = tall_f * 0.3;
            let trunk = (tr * 0.22).round().max(1.0) as i32;
            for y in crown_y as i32..foot {
                // the trunk widens towards its roots
                let flare = ((y - foot + 4).max(0)) / 2;
                let x0 = tx.round() as i32 - trunk / 2 - flare;
                canvas.fill_rect(x0, y, trunk + 2 * flare, 1, c.big_tree);
                canvas.set(
                    if toward > 0.0 {
                        x0 + trunk + 2 * flare - 1
                    } else {
                        x0
                    },
                    y,
                    c.big_tree_lit,
                );
            }
            Self::crown(
                canvas,
                tx,
                crown_y,
                tr,
                7,
                toward,
                c.big_tree,
                c.big_tree_lit,
            );
        }

        // long grass, swaying gently at the tips, lit along the side towards the light
        let sway_t = self.t as f32 * self.motion.speed();
        for blade in &self.blades {
            let sway =
                (sway_t * 0.7 + blade.phase).sin() + 0.4 * noise(sway_t * 0.2, blade.phase, 3);
            // a far blade is a soft step above the grass and a near one the full blade colour,
            // with its tip catching the light
            let near = (blade.base - meadow) as f32 / depth;
            let body = if near > 0.55 { c.blade } else { c.grass_near };
            for k in 0..blade.height {
                let bend = sway * (k * k) as f32 / (blade.height * blade.height).max(1) as f32;
                let x = blade.x + (bend * 1.4).round() as i32;
                let tip = near > 0.55 && k + 2 >= blade.height && blade.height > 3;
                canvas.set(x, blade.base - k, if tip { c.blade_lit } else { body });
            }
            if blade.head {
                let x = blade.x + (sway * 1.4).round() as i32;
                let y = blade.base - blade.height;
                canvas.set(x, y, c.seed);
                canvas.set(x, y - 1, c.seed);
            }
        }
        if !c.dark {
            for &(x, y, i) in &self.flowers {
                let colour = c.flowers[i % c.flowers.len()];
                canvas.set(x, y, colour);
                canvas.set(x - 1, y, colour);
                canvas.set(x + 1, y, colour);
                canvas.set(x, y - 1, colour);
                canvas.set(x, y + 1, c.seed);
            }
        }

        if c.dark {
            self.paint_lights(canvas, c, meadow);
        } else {
            self.paint_pollen(canvas, c);
            self.paint_butterflies(canvas, c);
        }
    }

    fn level(&self, fly: &Fly) -> f32 {
        let rate = TAU / fly.period * self.motion.pace();
        ((cycle(self.t, rate) + fly.phase).sin() + 1.0) / 2.0
    }

    fn paint_lights(&self, canvas: &mut Canvas, c: &MeadowColours, meadow: i32) {
        for fly in &self.flies {
            let level = self.level(fly);
            if level < 0.25 {
                continue;
            }
            let (x, y) = (fly.x.round() as i32, fly.y.round() as i32);
            // a light over the meadow is near, so larger; one over the trees is far and small
            let near = y > meadow + 3;
            if level >= 0.5 {
                let r = if near { 3.2 } else { 1.8 };
                disc(canvas, fly.x, fly.y, r, c.faint_glow);
                if level >= 0.8 || near {
                    disc(canvas, fly.x, fly.y, r * 0.5, c.glow);
                }
            }
            let core = if level >= 0.8 {
                c.bright
            } else if level >= 0.5 {
                c.light
            } else {
                c.dim
            };
            if near && level >= 0.5 {
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    canvas.set(x + dx, y + dy, c.light);
                }
            }
            canvas.set(x, y, core);
        }
    }

    fn paint_pollen(&self, canvas: &mut Canvas, c: &MeadowColours) {
        for fly in &self.flies {
            let level = self.level(fly);
            let (x, y) = (fly.x.round() as i32, fly.y.round() as i32);
            if level > 0.6 {
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    canvas.set(x + dx, y + dy, c.faint_glow);
                }
            }
            canvas.set(x, y, c.pollen);
        }
    }

    fn paint_butterflies(&self, canvas: &mut Canvas, c: &MeadowColours) {
        let period = f64::from(FLUTTER_PERIOD / self.motion.pace());
        for (i, fly) in self.butterflies.iter().enumerate() {
            let a = cycle(self.t, fly.rate * TAU * self.motion.speed()) + fly.phase;
            let x = fly.centre + fly.reach * a.sin();
            let y = fly.row + 3.0 * (a * 2.3).sin();
            let beat = ((self.t / period) + i as f64 * 0.37).rem_euclid(1.0);
            let rows: &[&str] = match (beat * 4.0) as u32 {
                0 => &WINGS_OPEN,
                2 => &WINGS_SHUT,
                _ => &WINGS_HALF,
            };
            let wing = c.wings[i % c.wings.len()];
            canvas.sprite(
                x.round() as i32 - 3,
                y.round() as i32 - 2,
                rows,
                &[('w', wing), ('W', mix(wing, c.body, 0.3)), ('b', c.body)],
            );
        }
    }
}

impl Scene for Fireflies {
    fn name(&self) -> &'static str {
        "fireflies"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let (w, tall) = (self.width as f32, self.tall());
        let meadow = self.meadow();
        let wanted = self
            .motion
            .scaled(count_for(self.width, self.height, 70, 3, 36));
        let (low, high) = if self.motion.calm {
            (MIN_PULSE_PERIOD * 2.0, 9.0)
        } else {
            (MIN_PULSE_PERIOD, 5.0)
        };
        let top_speed = self.top_speed();
        let sky = tall * 0.35;
        self.flies = (0..wanted)
            .map(|_| Fly {
                x: rng.range(0.0, w),
                y: rng.range(sky, self.bottom().max(sky)),
                vx: rng.range(-top_speed, top_speed),
                vy: rng.range(-top_speed, top_speed) * 0.5,
                phase: rng.range(0.0, TAU),
                period: rng.range(low, high),
            })
            .collect();
        self.butterflies = if self.height >= 10 {
            (0..self.motion.scaled((self.width / 45).max(1)).min(4))
                .map(|_| Butterfly {
                    centre: rng.range(0.2, 0.8) * w,
                    reach: rng.range(0.1, 0.25) * w,
                    rate: rng.range(0.03, 0.06),
                    phase: rng.range(0.0, TAU),
                    row: rng.range(tall * 0.45, meadow as f32 + 4.0),
                })
                .collect()
        } else {
            Vec::new()
        };
        self.clouds = if self.height >= 10 {
            (0..(self.width / 50).clamp(1, 4))
                .map(|_| {
                    Cloud::random(
                        rng,
                        w,
                        (tall * 0.12, tall * 0.3),
                        (w * 0.08, (w * 0.15).max(8.0)),
                        self.motion.speed(),
                    )
                })
                .collect()
        } else {
            Vec::new()
        };
        // a tree line of rounded crowns with gaps of open sky between clumps
        self.trees.clear();
        let base_r = (tall * 0.06).clamp(1.5, 12.0);
        let mut x = rng.range(-base_r, base_r);
        while x < w + base_r {
            let r = base_r * rng.range(0.7, 1.25);
            self.trees.push(Tree {
                x,
                r,
                lift: r * rng.range(0.2, 0.7),
                seed: rng.next_u64() as u32,
            });
            x += r * rng.range(1.1, 1.8);
            if rng.chance(0.18) {
                x += base_r * rng.range(2.0, 5.0);
            }
        }
        // grass: short blades far back, long ones at the front, a few carrying seed heads
        self.blades.clear();
        let depth = (tall as i32 - meadow).max(1);
        let count = (self.width as f32 * 2.2) as usize;
        for _ in 0..count {
            // most blades grow towards the front, where they are long enough to read
            let near = 1.0 - rng.unit() * rng.unit();
            let base = meadow + (near * depth as f32).round() as i32;
            let reach = 1.0 + near * near * tall * 0.16;
            self.blades.push(Blade {
                x: rng.below(self.width) as i32,
                base,
                height: (reach * rng.range(0.5, 1.0)).round().max(1.0) as i32,
                phase: rng.range(0.0, TAU),
                head: near > 0.5 && rng.chance(0.08),
            });
        }
        // nearer blades are painted last, over the ones behind them
        self.blades.sort_by_key(|b| b.base);
        self.stars = (0..(self.width / 6).max(1))
            .map(|_| {
                (
                    rng.below(self.width) as i32,
                    rng.below(((tall * 0.4) as usize).max(1)) as i32,
                )
            })
            .collect();
        self.flowers = (0..count_for(self.width, self.height, 140, 2, 40))
            .map(|i| {
                let y = meadow + depth / 3 + rng.below(((depth * 2 / 3) as usize).max(1)) as i32;
                (rng.below(self.width) as i32, y, i)
            })
            .collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let top_speed = self.top_speed();
        let width = self.width as f32;
        let (top, bottom) = (self.tall() * 0.35, self.bottom());
        for fly in &mut self.flies {
            fly.vx = (fly.vx + rng.range(-3.0, 3.0) * dt).clamp(-top_speed, top_speed);
            fly.vy = (fly.vy + rng.range(-1.5, 1.5) * dt).clamp(-top_speed * 0.5, top_speed * 0.5);
            fly.x += fly.vx * dt;
            fly.y += fly.vy * dt;
            if fly.x < 0.0 {
                fly.x = 0.0;
                fly.vx = fly.vx.abs();
            } else if fly.x > width - 1.0 {
                fly.x = (width - 1.0).max(0.0);
                fly.vx = -fly.vx.abs();
            }
            if fly.y < top.min(bottom) {
                fly.y = top.min(bottom);
                fly.vy = fly.vy.abs();
            } else if fly.y > bottom {
                fly.y = bottom;
                fly.vy = -fly.vy.abs();
            }
        }
        for cloud in &mut self.clouds {
            cloud.drift(dt, width);
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = MeadowColours::new(theme);
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
        check_determinism, check_pixel_scene, check_scene, pixels, render,
    };
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("fireflies");
        check_determinism("fireflies");
        check_pixel_scene("fireflies", 6.0);
    }

    #[test]
    fn the_meadow_and_its_lights_stay_distinct_in_every_theme() {
        for theme in theme::all() {
            let c = MeadowColours::new(theme);
            let name = theme.name;
            for (a, b, what) in [
                (c.sky_top, c.sky_low, "the sky's top and bottom"),
                (c.hills_far, c.sky_low, "the far hills and the sky"),
                (c.hills_near, c.hills_far, "the near hills and the far ones"),
                (c.trees, c.hills_far, "the trees and the hills"),
                (c.trees, c.trees_lit, "the trees and their lit edges"),
                (c.blade, c.grass_near, "the blades and the grass"),
                (c.halo, c.sky_top, "the halo and the sky"),
            ] {
                assert!(distance(a, b) >= 0.03, "{what} collapse in {name}");
            }
            for colour in [
                c.sky_top,
                c.sky_low,
                c.hills_far,
                c.hills_near,
                c.trees,
                c.trees_lit,
                c.big_tree,
                c.big_tree_lit,
                c.grass_far,
                c.grass_near,
                c.blade,
                c.blade_lit,
                c.halo,
                c.halo_far,
                c.glow,
                c.faint_glow,
                c.pollen,
            ] {
                assert!(theme.contrast_of(colour) < IMAGERY_CEILING, "{name}");
            }
            for colour in c.flowers.iter().chain(&c.wings) {
                assert!(theme.contrast_of(*colour) >= IMAGERY_CEILING, "{name}");
            }
            if theme.dark {
                for colour in [c.light, c.bright, c.orb] {
                    assert!(theme.contrast_of(colour) >= theme.floor(), "{name}");
                    assert!(distance(colour, c.sky_top) >= 0.1, "{name}");
                }
                assert!(distance(c.light, c.glow) >= 0.05, "{name}");
                assert!(theme.contrast_of(c.dim) >= IMAGERY_CEILING, "{name}");
            } else {
                // by day the sun is a pale gold disc in the sky, never a dark one
                assert!(theme.contrast_of(c.orb) < IMAGERY_CEILING, "{name}");
                assert!(distance(c.orb, c.sky_top) >= 0.05, "{name}");
            }
        }
    }

    #[test]
    fn lights_pulse_slowly_and_glow_by_night() {
        let c = MeadowColours::new(theme::default_theme());
        let mut rng = Rng::new(6);
        let mut meadow = Fireflies::new(60, 20, Motion::FULL, &mut rng);
        assert!(meadow.flies.iter().all(|f| f.period >= MIN_PULSE_PERIOD));
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..60 {
            let frame = render(&mut meadow, 60, 20, 1);
            for p in pixels(&frame) {
                if [c.dim, c.light, c.bright, c.glow, c.orb].contains(&p) {
                    seen.insert(p);
                }
            }
        }
        assert_eq!(seen.len(), 5, "{seen:?}");
    }

    #[test]
    fn by_day_the_meadow_has_butterflies_and_flowers_instead() {
        let theme = theme::by_id("deep-contrast-light").unwrap();
        let c = MeadowColours::new(theme);
        let mut rng = Rng::new(6);
        let meadow = Fireflies::new(80, 24, Motion::FULL, &mut rng);
        let mut frame = Frame::new(80, 24);
        meadow.draw(&mut frame, theme);
        let all = pixels(&frame);
        assert!(all.contains(&c.body), "no butterfly");
        assert!(c.flowers.iter().any(|f| all.contains(f)), "no flower");
        assert!(all.contains(&c.pollen), "no pollen");
    }

    #[test]
    fn flies_stay_inside_the_screen() {
        let mut rng = Rng::new(14);
        let mut meadow = Fireflies::new(20, 8, Motion::CALM, &mut rng);
        for _ in 0..(60 * 20) {
            meadow.step(0.05, &mut rng);
            for fly in &meadow.flies {
                assert!((0.0..20.0).contains(&fly.x), "{}", fly.x);
                assert!((0.0..=14.0).contains(&fly.y), "{}", fly.y);
                assert!(fly.vx.abs() <= 1.4);
            }
        }
    }
}
