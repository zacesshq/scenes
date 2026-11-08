//! Drought: a parched plain under a heavy sun in a hot, hazy sky, seen from just above the
//! ground. The earth is cracked into curling plates that are large close by and shrink towards
//! the horizon, each with a lit lip and a dark gap round it. More cracks open one edge at a time
//! over about a minute. A dry riverbed winds away into the distance with its last puddle
//! shrinking to a damp stain. A dead tree stands bare on the right with its shadow across the
//! cracks. Once dry it stays dry, with dust drifting past and a lone bird gliding across.
//!
//! The sky, the earth, the riverbed and the dust are tints of the background held below the
//! non-text floor. The sun on a dark theme, the tree and the bird are solid. On a light theme so
//! are the cracks, so with colour off the bare tree still stands over the cracked ground.

use super::paint::{self, Blender, gradient, noise, solid, unit};
use super::wrap;
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "A parched plain cracked into curling plates under a heavy sun, a dry riverbed with its last puddle and a dead tree";

pub const CAPTIONS: &[&str] = &[
    "Dry spell. The first error is the place to dig.",
    "Nothing grows until it builds. Start at the top.",
    "A long dry patch. One fix brings the rain.",
    "Parched, not lost. Read the first message.",
    "The ground cracked. Look for where it started.",
];

/// Seconds for the cracks to spread across the whole ground at the standard pace.
pub const CRACK_SECONDS: f32 = 60.0;
/// Seconds between the puddle shrinking by one step at the standard pace.
pub const PUDDLE_SHRINK: f32 = 6.0;
/// The steps the puddle shrinks through before it is gone.
const PUDDLE_STEPS: i32 = 8;
/// The share of the cracks already open when the scene starts, so the ground reads as cracked
/// from the first frame.
const FIRST_CRACKS: f32 = 0.82;
/// The size of one plate of dry mud on the ground, in ground units.
const PLATE: f32 = 0.4;
/// How wide a crack is, in ground units, so near cracks are wider than far ones.
const CRACK: f32 = 0.032;
/// How far away plates are still drawn before the ground becomes a smooth far plain.
const FAR_PLATES: f32 = 2.4;
/// The riverbed's half width, in ground units.
const RIVER: f32 = 0.55;
/// How far out the puddle lies, in ground units.
const PUDDLE_AT: f32 = 1.9;

const BIRD: [&str; 2] = ["b...b", ".bbb."];

#[derive(Clone, Copy, Debug)]
struct Mote {
    x: f32,
    y: f32,
    speed: f32,
}

/// What lies under one ground pixel, worked out once for each size: which plate it is on, the
/// next plate over and how far it is from the edge between them, its place on the ground and
/// how far it is from the middle of the riverbed.
#[derive(Clone, Copy, Debug, Default)]
struct Ground {
    plate: u32,
    other: u32,
    edge: f32,
    gx: f32,
    z: f32,
    river: f32,
}

/// One limb of the dead tree, drawn as a line `thick` pixels wide.
#[derive(Clone, Copy, Debug)]
struct Limb {
    from: (f32, f32),
    to: (f32, f32),
    thick: f32,
}

/// The land's colours in one theme.
struct Colours {
    sky: Ramp,
    haze: Rgb,
    hills: Rgb,
    sun: Rgb,
    glow: (Rgb, Rgb),
    earth: Ramp,
    lip: Rgb,
    shade: Rgb,
    crack: Rgb,
    bed: Rgb,
    bank: (Rgb, Rgb),
    pebble: Rgb,
    water: (Rgb, Rgb),
    damp: Rgb,
    shadow: Rgb,
    dust: Rgb,
    bark: Rgb,
    bird: Rgb,
}

impl Colours {
    fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let dark = theme.dark;
        let heat = mix(theme.warning, theme.yellow(), 0.4);
        let soil = mix(theme.yellow(), theme.red(), 0.3);
        let bleached = mix(theme.yellow(), theme.foreground, 0.45);
        let (sky_top, sky_low, haze, hills) = if dark {
            (
                shades.tint(mix(heat, theme.blue(), 0.4), 0.16, 1.3),
                shades.tint(heat, 0.42, 2.1),
                shades.tint(mix(heat, theme.foreground, 0.4), 0.55, 2.55),
                shades.tint(mix(soil, theme.blue(), 0.3), 0.3, 1.7),
            )
        } else {
            (
                shades.tint(mix(heat, theme.info, 0.5), 0.32, 1.6),
                shades.tint(heat, 0.12, 1.16),
                shades.tint(heat, 0.04, 1.05),
                shades.tint(mix(soil, theme.info, 0.4), 0.3, 1.5),
            )
        };
        // the earth is paler far off, where the haze lies over it
        let (near, far_earth, lip, shade, crack, bed) = if dark {
            (
                shades.tint(soil, 0.42, 2.1),
                shades.tint(bleached, 0.42, 2.2),
                shades.tint(bleached, 0.75, 2.85),
                shades.tint(soil, 0.24, 1.55),
                shades.tint(soil, 0.06, 1.08),
                shades.tint(bleached, 0.6, 2.6),
            )
        } else {
            (
                shades.tint(soil, 0.5, 2.25),
                shades.tint(soil, 0.22, 1.4),
                shades.tint(bleached, 0.1, 1.15),
                shades.tint(soil, 0.72, 2.8),
                paint::strong(theme, mix(soil, theme.foreground, 0.5)),
                shades.tint(bleached, 0.22, 1.35),
            )
        };
        let bark = solid(
            theme,
            mix(theme.muted, soil, 0.35),
            if dark {
                shades.lightest()
            } else {
                shades.darkest()
            },
            0.2,
        );
        Self {
            sky: Ramp::new(sky_top, sky_low, 9),
            haze,
            hills,
            // a light theme's bright yellow is a dark ochre, so its sun is a white hot disc
            sun: if dark {
                theme.bright_yellow()
            } else {
                shades.tint(theme.bright_yellow(), 0.12, 1.15)
            },
            glow: if dark {
                (
                    shades.tint(theme.bright_yellow(), 0.55, 2.5),
                    shades.tint(heat, 0.5, 2.25),
                )
            } else {
                (
                    shades.tint(theme.warning, 0.28, 1.5),
                    shades.tint(theme.warning, 0.18, 1.3),
                )
            },
            earth: Ramp::new(near, far_earth, 6),
            lip,
            shade,
            crack,
            bed,
            bank: if dark {
                (
                    shades.tint(soil, 0.16, 1.35),
                    shades.tint(bleached, 0.5, 2.4),
                )
            } else {
                (
                    shades.tint(soil, 0.62, 2.55),
                    shades.tint(bleached, 0.06, 1.08),
                )
            },
            pebble: shades.tint(theme.muted, 0.55, 2.4),
            water: if dark {
                (
                    shades.tint(theme.blue(), 0.3, 1.6),
                    shades.tint(theme.bright_cyan(), 0.65, 2.8),
                )
            } else {
                (
                    shades.tint(theme.blue(), 0.55, 2.4),
                    shades.tint(theme.info, 0.12, 1.2),
                )
            },
            damp: if dark {
                shades.tint(soil, 0.3, 1.7)
            } else {
                shades.tint(soil, 0.6, 2.5)
            },
            shadow: if dark {
                shades.tint(soil, 0.14, 1.28)
            } else {
                shades.tint(soil, 0.66, 2.65)
            },
            dust: if dark {
                shades.tint(bleached, 0.55, 2.5)
            } else {
                shades.tint(soil, 0.3, 1.6)
            },
            bark,
            bird: theme.foreground,
        }
    }
}

pub struct Drought {
    width: usize,
    height: usize,
    motion: Motion,
    /// How far the cracks have spread, from 0 to 1.
    progress: f32,
    /// Seconds the puddle has been drying.
    drying: f32,
    seed: u32,
    ground: Vec<Ground>,
    limbs: Vec<Limb>,
    dust: Vec<Mote>,
    bird: f32,
}

/// A hash of two plates into `[0, 1)`, so a whole edge between them opens at one moment.
fn pair_hash(a: u32, b: u32) -> f32 {
    let (lo, hi) = (a.min(b), a.max(b));
    unit(lo as i32, hi as i32, 0x0d2)
}

impl Drought {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut land = Self {
            width: 1,
            height: 1,
            motion,
            progress: FIRST_CRACKS,
            drying: 0.0,
            seed: (rng.next_u64() >> 40) as u32,
            ground: Vec::new(),
            limbs: Vec::new(),
            dust: Vec::new(),
            bird: 0.0,
        };
        land.resize(width, height, rng);
        land
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The pixel row of the horizon.
    fn horizon(&self) -> i32 {
        ((self.tall() as f32 * 0.36) as i32).max(2)
    }

    /// The puddle's size now, in steps: one fewer every `PUDDLE_SHRINK` seconds.
    fn puddle(&self) -> i32 {
        let lost = (self.drying / PUDDLE_SHRINK) as i32;
        (PUDDLE_STEPS - lost).max(0)
    }

    /// The middle of the riverbed at a distance, in ground units across.
    fn river_centre(&self, z: f32) -> f32 {
        let phase = (self.seed % 100) as f32 * 0.06;
        0.55 * (z * 0.9 + phase).sin() + 0.25 * (z * 2.1 + phase * 2.0).sin() - 0.35
    }

    /// Where the tree stands: its foot in pixels and its height.
    fn tree(&self) -> Option<((f32, f32), f32)> {
        let (w, tall) = (self.width as f32, self.tall() as f32);
        let horizon = self.horizon() as f32;
        if self.width < 24 || tall < 20.0 {
            return None;
        }
        let foot = horizon + (tall - horizon) * 0.3;
        let height = (tall * 0.52).min(foot - 2.0);
        Some(((w * 0.76, foot), height))
    }

    /// Works out what lies under every ground pixel: the plates of mud in the ground's own
    /// space so near plates are big and far ones small, with the riverbed winding through them.
    fn survey(&mut self) {
        let (w, tall) = (self.width as i32, self.tall());
        let horizon = self.horizon();
        let focal = (tall - horizon).max(1) as f32;
        let seed = self.seed;
        let seed_at = |i: i32, j: i32| -> (f32, f32) {
            (
                (i as f32 + 0.15 + 0.7 * unit(i, j, seed)) * PLATE,
                (j as f32 + 0.15 + 0.7 * unit(i, j, seed ^ 0x55)) * PLATE,
            )
        };
        let mut ground = Vec::with_capacity((w * (tall - horizon).max(0)) as usize);
        for y in horizon..tall {
            let z = focal / (y - horizon) as f32 + 0.0;
            let z = z.max(1.0);
            for x in 0..w {
                let gx = (x as f32 - w as f32 / 2.0) * z / focal;
                let (ci, cj) = ((gx / PLATE).floor() as i32, (z / PLATE).floor() as i32);
                let (mut best, mut next) = ((f32::MAX, 0u32), (f32::MAX, 0u32));
                for dj in -1..=1 {
                    for di in -1..=1 {
                        // the horizon row has an infinite depth, so its plate index saturates and the
                        // neighbour step wraps instead of overflowing
                        let (i, j) = (ci.wrapping_add(di), cj.wrapping_add(dj));
                        let (sx, sz) = seed_at(i, j);
                        let d = (gx - sx).powi(2) + (z - sz).powi(2);
                        let id = (i.wrapping_mul(7919) ^ j.wrapping_mul(104_729)) as u32;
                        if d < best.0 {
                            next = best;
                            best = (d, id);
                        } else if d < next.0 {
                            next = (d, id);
                        }
                    }
                }
                let edge = (next.0.sqrt() - best.0.sqrt()) / 2.0;
                let river = (gx - self.river_centre(z)).abs() / RIVER;
                ground.push(Ground {
                    plate: best.1,
                    other: next.1,
                    edge,
                    gx,
                    z,
                    river,
                });
            }
        }
        self.ground = ground;
    }

    /// Grows the dead tree's limbs from a seed of its own, so a resize keeps the same tree.
    fn grow_tree(&mut self) {
        self.limbs.clear();
        let Some((foot, height)) = self.tree() else {
            return;
        };
        let mut rng = Rng::new(0xd7ee ^ u64::from(self.seed));
        let trunk = (height / 10.0).clamp(1.5, 4.0);
        let mut stack = vec![(foot, -0.08_f32, height * 0.4, trunk, 0)];
        while let Some((from, angle, length, thick, depth)) = stack.pop() {
            let to = (from.0 + angle.sin() * length, from.1 - angle.cos() * length);
            self.limbs.push(Limb { from, to, thick });
            if depth >= 4 || length < 2.0 {
                continue;
            }
            let forks = if depth == 0 { 3 } else { 2 };
            for k in 0..forks {
                let side = if forks == 3 {
                    [-0.65, 0.05, 0.6][k]
                } else if k == 0 {
                    -0.5
                } else {
                    0.45
                };
                let turn = side + rng.range(-0.15, 0.15);
                let shrink = rng.range(0.62, 0.78);
                let start = if depth == 0 && k == 1 {
                    // the middle limb carries the trunk on upwards
                    to
                } else {
                    let t = rng.range(0.75, 1.0);
                    (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t)
                };
                stack.push((
                    start,
                    angle + turn,
                    length * shrink,
                    (thick * 0.6).max(1.0),
                    depth + 1,
                ));
            }
        }
    }

    fn paint_sky(&self, canvas: &mut Canvas, c: &Colours) {
        let wide = canvas.width() as i32;
        let horizon = self.horizon();
        gradient(canvas, &c.sky, 0, horizon);
        // the sun, heavy and still, low in the sky with two rings of glare round it
        if self.width >= 16 && horizon >= 8 {
            let r = (horizon as f32 * 0.2).clamp(2.0, 9.0);
            let (sx, sy) = (wide as f32 * 0.3, horizon as f32 * 0.42);
            paint::disc(canvas, sx, sy, r * 1.7, c.glow.1);
            paint::disc(canvas, sx, sy, r * 1.25, c.glow.0);
            paint::disc(canvas, sx, sy, r, c.sun);
        }
        // low hills far off and a band of haze lying along the horizon
        for x in 0..wide {
            let lift = 1.0 + 3.0 * noise(x as f32 * 0.04, 0.5, self.seed);
            let lift = lift * (horizon as f32 / 20.0).min(1.5);
            for y in (horizon as f32 - lift).round() as i32..horizon {
                canvas.set(x, y, c.hills);
            }
        }
        if horizon >= 6 {
            canvas.fill_row(horizon - 1, c.haze);
        }
    }

    fn paint_ground(&self, canvas: &mut Canvas, c: &Colours) {
        let (wide, tall) = (canvas.width() as i32, canvas.height() as i32);
        let horizon = self.horizon();
        let at = |x: i32, y: i32| -> Option<&Ground> {
            if x < 0 || x >= wide || y < horizon || y >= tall {
                return None;
            }
            self.ground.get(((y - horizon) * wide + x) as usize)
        };
        // whether a pixel is in an open crack; a crack is wider close by
        let cracked = |g: &Ground| g.edge < CRACK && pair_hash(g.plate, g.other) < self.progress;
        let puddle = self.puddle() as f32 / PUDDLE_STEPS as f32;
        let puddle_r = RIVER * 0.75;
        let pc = self.river_centre(PUDDLE_AT);
        for y in horizon..tall {
            for x in 0..wide {
                let Some(g) = at(x, y) else {
                    continue;
                };
                let fade = ((g.z - 1.0) / (FAR_PLATES - 1.0)).clamp(0.0, 1.0) * 0.5;
                let colour = if g.river < 1.0 {
                    // the dry bed: pale sand with a few stones round the last of the water
                    let (dx, dz) = ((g.gx - pc) / puddle_r, (g.z - PUDDLE_AT) / puddle_r);
                    let d = (dx * dx + dz * dz).sqrt();
                    if d < puddle && puddle > 0.0 {
                        // the far half of the pool shows the bright sky, the near half is deep
                        if dz > 0.2 * puddle {
                            c.water.1
                        } else {
                            c.water.0
                        }
                    } else if d < 1.0 {
                        c.damp
                    } else if g.z < FAR_PLATES && unit(x, y, self.seed) < 0.025 {
                        c.pebble
                    } else {
                        c.bed
                    }
                } else if g.river < 1.25 {
                    // the cut banks: the left one in shade, the right one lit
                    if g.gx < self.river_centre(g.z) {
                        c.bank.0
                    } else {
                        c.bank.1
                    }
                } else if g.z >= FAR_PLATES {
                    // too far off to make out the plates, it fades into the haze
                    c.earth.at(0.5 + 0.5 * ((g.z - FAR_PLATES) / 4.0).min(1.0))
                } else if cracked(g) {
                    c.crack
                } else {
                    // a plate's lower edge, over a crack, lifts and catches the light; its upper
                    // edge curls away into shade
                    let below = at(x, y + 1).is_some_and(|b| cracked(b) && b.river >= 1.25);
                    let above = at(x, y - 1).is_some_and(|a| cracked(a) && a.river >= 1.25);
                    if above && g.z < 2.2 {
                        c.lip
                    } else if below && g.z < 1.8 {
                        c.shade
                    } else {
                        // each plate its own shade of the earth, paler further off
                        let jitter = (unit(g.plate as i32, 3, 9) - 0.5) * 0.35;
                        c.earth.at(fade + jitter)
                    }
                };
                canvas.set(x, y, colour);
            }
        }
    }

    fn paint_tree(&self, canvas: &mut Canvas, c: &Colours) {
        let Some((foot, height)) = self.tree() else {
            return;
        };
        let mut blender = Blender::new(4);
        // a long shadow thrown away from the sun across the cracks
        let length = height * 0.9;
        let rows = (height * 0.12).max(1.5);
        for y in foot.1 as i32 - 1..=(foot.1 + rows) as i32 {
            for x in foot.0 as i32..(foot.0 + length) as i32 {
                let t = (x as f32 - foot.0) / length;
                let across = (y as f32 - foot.1 - t * rows * 0.6).abs();
                if across < 1.2 * (1.0 - t) + 0.4 {
                    blender.blend(canvas, x, y, c.shadow, 0.7);
                }
            }
        }
        for limb in &self.limbs {
            if limb.thick < 1.4 {
                canvas.line(
                    (limb.from.0.round() as i32, limb.from.1.round() as i32),
                    (limb.to.0.round() as i32, limb.to.1.round() as i32),
                    c.bark,
                );
                continue;
            }
            // thick limbs are a run of discs, so joints are round and the taper smooth
            let steps = ((limb.to.0 - limb.from.0).abs() + (limb.to.1 - limb.from.1).abs())
                .ceil()
                .max(1.0) as i32;
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let r = limb.thick * (1.0 - 0.3 * t) / 2.0;
                let (x, y) = (
                    limb.from.0 + (limb.to.0 - limb.from.0) * t,
                    limb.from.1 + (limb.to.1 - limb.from.1) * t,
                );
                canvas.disc(x, y, r.max(0.5), c.bark);
            }
        }
        // the roots flare into the ground
        let flare = (height / 9.0).clamp(1.0, 3.0);
        for dx in [-flare, flare] {
            paint::line(
                canvas,
                (foot.0, foot.1 - 2.0),
                (foot.0 + dx, foot.1),
                1,
                c.bark,
            );
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        self.paint_sky(canvas, c);
        self.paint_ground(canvas, c);
        self.paint_tree(canvas, c);
        let horizon = self.horizon();
        if horizon >= 10 {
            // the bird keeps to the sky above the sun's glare
            let r = (horizon as f32 * 0.2).clamp(2.0, 9.0);
            let y = ((horizon as f32 * 0.42 - r * 1.7) as i32 - 4).max(1);
            canvas.sprite(self.bird.round() as i32, y, &BIRD, &[('b', c.bird)]);
        }
        let mut blender = Blender::new(4);
        for mote in &self.dust {
            let (x, y) = (mote.x.round() as i32, mote.y.round() as i32);
            blender.blend(canvas, x, y, c.dust, 0.6);
            blender.blend(canvas, x - 1, y, c.dust, 0.3);
        }
    }
}

impl Scene for Drought {
    fn name(&self) -> &'static str {
        "drought"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.survey();
        self.grow_tree();
        let horizon = self.horizon();
        let tall = self.tall();
        let motes = self
            .motion
            .scaled(super::count_for(self.width, self.height, 120, 3, 30));
        self.dust = (0..motes)
            .map(|_| Mote {
                x: rng.range(0.0, self.width as f32),
                y: rng.range((horizon - 4).max(0) as f32, (tall - 1).max(1) as f32),
                speed: rng.range(1.6, 4.0),
            })
            .collect();
        self.bird = rng.range(0.0, self.width as f32);
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        let pace = self.motion.pace();
        let speed = self.motion.speed();
        self.progress = (self.progress + dt * pace / CRACK_SECONDS).min(1.0);
        self.drying += dt * pace;
        let width = self.width as f32;
        for mote in &mut self.dust {
            mote.x = wrap(mote.x + mote.speed * speed * dt, width);
        }
        self.bird = wrap(self.bird + 2.4 * speed * dt + 6.0, width + 12.0) - 6.0;
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
    use crate::pixels::IMAGERY_CEILING;
    use crate::scenes::testing::{apart, check_pixel_scene, pixels};
    use crate::scenes::testing::{check_determinism, check_scene, render};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("drought");
        check_determinism("drought");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("drought", 40.0);
    }

    #[test]
    fn the_cracks_spread_and_the_puddle_dries_then_it_holds() {
        let theme = theme::by_id("deep-contrast-light").unwrap();
        let c = Colours::new(theme);
        let mut rng = Rng::new(8);
        let mut land = Drought::new(100, 30, Motion::FULL, &mut rng);
        let draw = |land: &Drought| {
            let mut frame = Frame::new(100, 30);
            land.draw(&mut frame, theme);
            frame
        };
        let count = |frame: &Frame| pixels(frame).iter().filter(|&&p| p == c.crack).count();
        let early = draw(&land);
        assert!(pixels(&early).contains(&c.water.0));
        for _ in 0..(60 * 60 * 20) {
            land.step(0.05, &mut rng);
        }
        let late = draw(&land);
        assert!(
            count(&late) > count(&early),
            "{} {}",
            count(&late),
            count(&early)
        );
        assert_eq!(land.progress, 1.0);
        assert_eq!(land.puddle(), 0);
        let all = pixels(&late);
        assert!(!all.contains(&c.water.0), "the puddle should be gone");
        assert!(all.contains(&c.damp) && all.contains(&c.bark));
        let dark = render(&mut land, 100, 30, 1);
        let sun = Colours::new(theme::default_theme()).sun;
        assert!(pixels(&dark).contains(&sun));
    }

    #[test]
    fn near_plates_are_bigger_than_far_ones() {
        let mut rng = Rng::new(3);
        let land = Drought::new(120, 34, Motion::FULL, &mut rng);
        let w = 120;
        let plates_in_row = |row: usize| {
            let mut seen: Vec<u32> = land.ground[row * w..(row + 1) * w]
                .iter()
                .map(|g| g.plate)
                .collect();
            seen.dedup();
            seen.len()
        };
        let rows = land.ground.len() / w;
        assert!(
            plates_in_row(rows - 1) * 2 < plates_in_row(rows / 4),
            "{rows}"
        );
        assert!(land.limbs.len() >= 20, "{}", land.limbs.len());
    }

    #[test]
    fn the_puddle_shrinks_one_step_at_a_time_at_most_every_few_seconds() {
        let mut rng = Rng::new(2);
        for motion in Motion::ALL {
            let mut land = Drought::new(80, 24, motion, &mut rng);
            let mut last = land.puddle();
            let mut t = 0.0;
            let mut changed_at = 0.0;
            for _ in 0..(120 * 20) {
                land.step(0.05, &mut rng);
                t += 0.05;
                let now = land.puddle();
                if now != last {
                    assert_eq!(last - now, 1);
                    if changed_at > 0.0 {
                        assert!(t - changed_at >= PUDDLE_SHRINK - 0.06);
                    }
                    changed_at = t;
                    last = now;
                }
            }
        }
    }

    #[test]
    fn the_layers_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            apart(theme, c.sky.at(0.0), c.sky.at(1.0), 0.03, "the hot sky");
            apart(
                theme,
                c.earth.at(1.0),
                c.haze,
                0.03,
                "the far earth and the haze",
            );
            apart(theme, c.hills, c.sky.at(1.0), 0.03, "the hills and the sky");
            apart(
                theme,
                c.crack,
                c.earth.at(0.0),
                0.06,
                "the cracks and the plates",
            );
            apart(
                theme,
                c.lip,
                c.earth.at(0.0),
                0.03,
                "the lips and the plates",
            );
            apart(
                theme,
                c.bed,
                c.earth.at(0.3),
                0.03,
                "the riverbed and the plates",
            );
            apart(theme, c.water.0, c.bed, 0.05, "the puddle and the riverbed");
            apart(theme, c.bark, c.sky.at(0.5), 0.1, "the tree and the sky");
            for colour in c
                .sky
                .steps()
                .iter()
                .chain(c.earth.steps())
                .chain([&c.bed, &c.water.0, &c.dust, &c.glow.0, &c.lip, &c.shadow])
            {
                assert!(
                    theme.contrast_of(*colour) < IMAGERY_CEILING,
                    "{}",
                    theme.name
                );
            }
            for colour in [c.bark, c.bird] {
                assert!(
                    theme.contrast_of(colour) >= paint::SOLID - 0.01,
                    "{}",
                    theme.name
                );
            }
            if theme.dark {
                assert!(
                    theme.contrast_of(c.sun) >= paint::SOLID - 0.01,
                    "{}",
                    theme.name
                );
            } else {
                assert!(
                    theme.contrast_of(c.crack) >= paint::SOLID - 0.01,
                    "{}",
                    theme.name
                );
            }
        }
    }
}
