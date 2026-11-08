//! Thin ice in solid pixels: a frozen lake in winter, snowy hills and two rows of snow tipped
//! pines on the far shore, a moon over them on a dark theme and a pale low sun on a light one,
//! with a drift of snow and dry reeds on the near bank. From one weak point in the middle of the
//! ice cracks run out in a star, branch as they go and are joined by a ring of fracture round the
//! point. A ragged hole of dark water opens where they started. The cracks reach their full
//! spread in about a minute and then hold, with only the light snow still falling.
//!
//! The sky, the hills, the pines, the ice, the water and the snow are tints of the background
//! held below the non-text floor. The cracks and the reeds keep the theme's own drawing colours,
//! so the fracture reads as a shape and keeps its outline when colour is off.

use std::f32::consts::TAU;

use super::paint::{Blender, disc, noise, tint, unit};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "Cracks spreading in a star across a frozen lake from one weak point and a hole of dark water opening, under a moon or a low winter sun";
pub const CAPTIONS: &[&str] = &[
    "The ice gave way. Step back and read the log.",
    "Cracks are spreading. Fix the first one.",
    "On thin ice. The build is failing.",
];

/// How long the cracks take to reach their full spread at the standard pace.
pub const CRACK_SECONDS: f32 = 60.0;
/// How far the cracks have already run when the scene starts, in seconds, so the weak point
/// and the first cracks show from the start.
const CRACK_START: f32 = 16.0;
/// How long the hole takes to open once the cracks are a fifth of the way out.
const HOLE_SECONDS: f32 = 40.0;
/// How much flatter the ice looks across than along, since it is seen at a low angle.
const FLATTEN: f32 = 0.42;

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct Colours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub moon: Rgb,
    pub hills: Rgb,
    pub far_pine: Rgb,
    pub pine: Rgb,
    pub pine_snow: Rgb,
    pub bank: Rgb,
    pub ice_far: Rgb,
    pub ice_near: Rgb,
    pub drift: Rgb,
    pub sheen: Rgb,
    pub shadow: Rgb,
    pub water: Rgb,
    pub deep: Rgb,
    pub rim: Rgb,
    pub flake: Rgb,
    pub flake_edge: Rgb,
    pub crack: Rgb,
    pub reed: Rgb,
    pub rush_head: Rgb,
    edged_flakes: bool,
    sky: Ramp,
    ice: Ramp,
}

impl Colours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let dark = theme.dark;
        let snow = mix(theme.info, theme.foreground, 0.6);
        // a moonlit night on a dark theme, lighter towards the horizon; a pale winter day on a
        // light one, a clear blue overhead fading to white
        let (sky_top, sky_low) = if dark {
            (
                tint(theme, theme.blue(), 0.14, 1.25),
                tint(theme, mix(theme.info, theme.blue(), 0.5), 0.34, 1.75),
            )
        } else {
            (
                tint(theme, theme.info, 0.42, 1.85),
                tint(theme, theme.info, 0.12, 1.18),
            )
        };
        let (hills, far_pine, pine, pine_snow, bank) = if dark {
            (
                tint(theme, snow, 0.32, 1.85),
                tint(theme, mix(theme.green(), theme.blue(), 0.5), 0.18, 1.35),
                tint(theme, theme.green(), 0.08, 1.1),
                tint(theme, snow, 0.65, 2.7),
                tint(theme, snow, 0.55, 2.5),
            )
        } else {
            (
                tint(theme, theme.info, 0.06, 1.08),
                tint(theme, mix(theme.green(), theme.info, 0.5), 0.42, 1.9),
                tint(theme, theme.green(), 0.8, 2.85),
                tint(theme, theme.info, 0.02, 1.03),
                tint(theme, theme.info, 0.04, 1.05),
            )
        };
        let (ice_far, ice_near, drift, sheen, shadow) = if dark {
            (
                tint(theme, theme.cyan(), 0.3, 1.65),
                tint(theme, theme.cyan(), 0.5, 2.15),
                tint(theme, snow, 0.5, 2.4),
                tint(theme, theme.bright_cyan(), 0.75, 2.85),
                tint(theme, theme.blue(), 0.22, 1.45),
            )
        } else {
            (
                tint(theme, theme.cyan(), 0.18, 1.3),
                tint(theme, theme.cyan(), 0.36, 1.75),
                tint(theme, theme.info, 0.04, 1.05),
                tint(theme, theme.info, 0.01, 1.02),
                tint(theme, theme.blue(), 0.25, 1.4),
            )
        };
        let (water, deep, rim) = if dark {
            (
                tint(theme, theme.blue(), 0.2, 1.3),
                tint(theme, theme.blue(), 0.08, 1.1),
                tint(theme, theme.bright_cyan(), 0.9, 2.9),
            )
        } else {
            (
                tint(theme, theme.blue(), 0.7, 2.6),
                tint(theme, theme.blue(), 0.9, 2.9),
                tint(theme, theme.info, 0.02, 1.03),
            )
        };
        // snow stays white: on a light theme each flake keeps a soft cool grey edge so it shows
        // against the pale sky
        let (flake, flake_edge) = if dark {
            (theme.foreground, tint(theme, theme.foreground, 0.45, 2.0))
        } else {
            (
                tint(theme, theme.info, 0.0, 1.01),
                tint(theme, theme.info, 0.55, 2.1),
            )
        };
        Self {
            sky_top,
            sky_low,
            moon: if dark {
                theme.bright()
            } else {
                tint(theme, theme.yellow(), 0.1, 1.08)
            },
            hills,
            far_pine,
            pine,
            pine_snow,
            bank,
            ice_far,
            ice_near,
            drift,
            sheen,
            shadow,
            water,
            deep,
            rim,
            flake,
            flake_edge,
            crack: if dark { theme.foreground } else { theme.blue() },
            // dry reeds are straw coloured, a quieter yellow that still keeps its outline
            reed: super::paint::solid(theme, theme.yellow(), theme.muted, 0.5),
            rush_head: super::paint::solid(
                theme,
                mix(theme.yellow(), theme.red(), 0.3),
                if dark {
                    theme.background
                } else {
                    theme.foreground
                },
                0.45,
            ),
            edged_flakes: !dark,
            sky: Ramp::new(sky_top, sky_low, 9),
            ice: Ramp::new(ice_far, ice_near, 7),
        }
    }

    #[cfg(test)]
    pub(crate) fn palette(theme: &ThemeData) -> crate::scenes::testing::Palette {
        let c = Self::new(theme);
        let mut imagery = vec![
            ("sky top", c.sky_top),
            ("sky low", c.sky_low),
            ("hills", c.hills),
            ("far pines", c.far_pine),
            ("pines", c.pine),
            ("snow on the pines", c.pine_snow),
            ("bank", c.bank),
            ("far ice", c.ice_far),
            ("near ice", c.ice_near),
            ("drift", c.drift),
            ("sheen", c.sheen),
            ("shadow", c.shadow),
            ("water", c.water),
            ("deep water", c.deep),
            ("rim", c.rim),
            ("snow edge", c.flake_edge),
        ];
        let mut shapes = vec![("crack", c.crack), ("reed", c.reed)];
        if theme.dark {
            shapes.push(("snow", c.flake));
        } else {
            imagery.push(("snow", c.flake));
        }
        if theme.dark {
            shapes.push(("moon", c.moon));
        } else {
            imagery.push(("sun", c.moon));
        }
        crate::scenes::testing::Palette { imagery, shapes }
    }
}

/// One pixel of a crack and how far along the fracture it lies, so it shows once the cracks
/// have spread that far.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Mark {
    x: i32,
    y: i32,
    reach: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct Flake {
    x: f32,
    y: f32,
    speed: f32,
}

pub struct ThinIce {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// The cracks' own clock, run at the motion speed.
    crack_t: f32,
    marks: Vec<Mark>,
    /// The longest reach of any crack, which the spread works towards.
    longest: f32,
    /// The ragged edge of the hole: its radius at each of a ring of angles, from 0.7 to 1.2.
    edge: Vec<f32>,
    flakes: Vec<Flake>,
}

impl ThinIce {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut ice = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            crack_t: CRACK_START,
            marks: Vec::new(),
            longest: 1.0,
            edge: Vec::new(),
            flakes: Vec::new(),
        };
        ice.resize(width, height, rng);
        ice
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The pixel row where the far shore meets the ice.
    fn shore(&self) -> i32 {
        ((self.tall() as f32 * 0.44).round() as i32).clamp(1, self.tall())
    }

    /// The weak point the cracks spread from.
    fn origin(&self) -> (f32, f32) {
        let ice = (self.tall() - self.shore()) as f32;
        (self.width as f32 * 0.5, self.shore() as f32 + ice * 0.52)
    }

    /// How far the cracks have spread, from 0 to 1.
    fn spread(&self) -> f32 {
        (self.crack_t / CRACK_SECONDS).clamp(0.0, 1.0)
    }

    /// How far the hole has opened, from 0 to 1.
    fn hole(&self) -> f32 {
        ((self.crack_t - CRACK_SECONDS / 5.0) / HOLE_SECONDS).clamp(0.0, 1.0)
    }

    /// The hole's largest radius across the ice, in pixels.
    fn hole_size(&self) -> f32 {
        let ice = (self.tall() - self.shore()) as f32;
        (ice * 0.32).min(self.width as f32 * 0.12).max(1.5)
    }

    /// Lays out the fracture: main cracks running out from the weak point in a rough star,
    /// each wandering a little and throwing off branches, then a ring of fracture joining the
    /// mains a little way out. The ice is seen at a low angle, so the cracks run flatter
    /// across it than along it.
    fn grow_cracks(&mut self, rng: &mut Rng) {
        self.marks.clear();
        let (ox, oy) = self.origin();
        let w = self.width as f32;
        let ice = (self.tall() - self.shore()) as f32;
        let mains = 9;
        let reach_out = (w * 0.5).max(4.0);
        let mut paths: Vec<Vec<(f32, f32, f32)>> = Vec::new();
        let mut stack: Vec<(f32, f32, f32, f32, f32, u32)> = Vec::new();
        for i in 0..mains {
            let angle = i as f32 / mains as f32 * TAU + rng.range(-0.25, 0.25);
            let length = rng.range(0.6, 1.0) * reach_out;
            stack.push((ox, oy, angle, 0.0, length, 0));
        }
        while let Some((mut x, mut y, mut angle, start, length, depth)) = stack.pop() {
            let mut reach = start;
            let mut walked = 0.0;
            let mut path = Vec::new();
            while walked < length {
                angle += rng.range(-0.14, 0.14);
                x += angle.cos();
                y += angle.sin() * FLATTEN;
                walked += 1.0;
                reach += 1.0;
                let (px, py) = (x.round() as i32, y.round() as i32);
                if py <= self.shore() + 1 || py >= self.tall() || px < 0 || px >= w as i32 {
                    break;
                }
                path.push((x, y, reach));
                // branches split off further out, where a spreading crack forks
                if depth < 2 && walked > 6.0 && rng.chance(0.035) && ice > 4.0 {
                    let side = if rng.chance(0.5) { 0.65 } else { -0.65 };
                    let rest = (length - walked) * rng.range(0.35, 0.6);
                    stack.push((x, y, angle + side, reach, rest, depth + 1));
                }
            }
            if depth == 0 {
                paths.push(path.clone());
            }
            for (x, y, reach) in path {
                self.marks.push(Mark {
                    x: x.round() as i32,
                    y: y.round() as i32,
                    reach,
                });
            }
        }
        // the ring: each main joined to the next a little way out, bowing outwards
        paths.sort_by(|a, b| {
            let angle = |p: &Vec<(f32, f32, f32)>| {
                p.get(3)
                    .map_or(0.0, |&(x, y, _)| ((y - oy) / FLATTEN).atan2(x - ox))
            };
            angle(a).total_cmp(&angle(b))
        });
        let ring = (reach_out * 0.22).max(3.0);
        let count = paths.len();
        for k in 0..count {
            let (a, b) = (&paths[k], &paths[(k + 1) % count]);
            let at = |p: &Vec<(f32, f32, f32)>| {
                p.iter()
                    .find(|&&(_, _, r)| r >= ring)
                    .map(|&(x, y, _)| (x, y))
            };
            let (Some(pa), Some(pb)) = (at(a), at(b)) else {
                continue;
            };
            let steps = ((pb.0 - pa.0).abs() + (pb.1 - pa.1).abs()).ceil().max(1.0) as i32;
            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let (mx, my) = (pa.0 + (pb.0 - pa.0) * t, pa.1 + (pb.1 - pa.1) * t);
                // bow out away from the weak point
                let bow = 4.0 * t * (1.0 - t) * 0.18;
                let (x, y) = (mx + (mx - ox) * bow, my + (my - oy) * bow);
                if y <= self.shore() as f32 + 1.0 {
                    continue;
                }
                self.marks.push(Mark {
                    x: x.round() as i32,
                    y: y.round() as i32,
                    reach: ring * 1.6 + t * ring * 0.8,
                });
            }
        }
        self.longest = self.marks.iter().map(|m| m.reach).fold(1.0_f32, f32::max);
        self.edge = (0..12).map(|_| rng.range(0.7, 1.2)).collect();
    }

    /// The hole's radius in one direction, from its ragged edge.
    fn hole_radius(&self, angle: f32) -> f32 {
        let n = self.edge.len().max(1);
        let pos = (angle.rem_euclid(TAU) / TAU) * n as f32;
        let (i, f) = (pos.floor() as usize % n, pos.fract());
        let (a, b) = (
            self.edge.get(i).copied().unwrap_or(1.0),
            self.edge.get((i + 1) % n).copied().unwrap_or(1.0),
        );
        a + (b - a) * f
    }

    /// Paints a pine standing on `base`, `size` pixels tall: tiers that widen downwards, each
    /// with snow along its top.
    fn pine(canvas: &mut Canvas, x: i32, base: i32, size: f32, body: Rgb, snow: Option<Rgb>) {
        let top = base as f32 - size;
        let tiers = (size / 5.0).clamp(2.0, 4.0).round();
        let rows_per_tier = size / tiers;
        for y in top.round() as i32..base {
            let p = ((y as f32 - top) / size).clamp(0.0, 0.999) * tiers;
            let (k, f) = (p.floor(), p.fract());
            // each tier flares out from under the one above it
            let bottom = size * 0.34 * (k + 1.0) / tiers;
            let start = if k == 0.0 { 0.0 } else { bottom * 0.4 };
            let half = (start + (bottom - start) * f).round() as i32;
            let capped = f * rows_per_tier < 1.0;
            for dx in -half..=half {
                let colour = match snow {
                    Some(snow) if capped || (f < 0.5 && dx < -half / 2) => snow,
                    _ => body,
                };
                canvas.set(x + dx, y, colour);
            }
        }
        canvas.set(x, base, body);
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        let mut blender = Blender::new(4);
        let (wide, tall) = (canvas.width() as i32, canvas.height() as i32);
        let shore = self.shore();

        for y in 0..shore.min(tall) {
            canvas.fill_row(y, c.sky.at(y as f32 / shore.max(1) as f32));
        }
        // the moon (a pale low sun on a light theme) over the far shore on the right
        let moon = (wide as f32 * 0.78, shore as f32 * 0.3);
        let moon_r = (shore as f32 * 0.11).clamp(1.5, 5.5);
        if shore >= 12 && wide >= 30 {
            disc(canvas, moon.0, moon.1, moon_r, c.moon);
        }
        // snowy hills behind the trees
        for x in 0..wide {
            let lift = noise(x as f32 * 0.03, 0.5, 56) * shore as f32 * 0.32;
            for y in (shore as f32 - 2.0 - lift).round() as i32..shore {
                canvas.set(x, y, c.hills);
            }
        }
        // two rows of pines on the far shore: a dense pale row behind, a darker one in front
        let tree_line = shore - 1;
        let far = (tall as f32 * 0.1).max(3.0);
        let mut x = -2.0;
        let mut i = 0;
        while x < wide as f32 + 3.0 {
            let size = far * (0.7 + 0.4 * unit(i, 0, 51));
            Self::pine(canvas, x as i32, tree_line - 1, size, c.far_pine, None);
            x += size * (0.45 + 0.35 * unit(i, 1, 51));
            i += 1;
        }
        let near = (tall as f32 * 0.19).max(4.0);
        let mut x = unit(0, 2, 52) * near;
        let mut i = 0;
        while x < wide as f32 + 3.0 {
            let size = near * (0.7 + 0.45 * unit(i, 0, 52));
            Self::pine(canvas, x as i32, tree_line, size, c.pine, Some(c.pine_snow));
            x += size * (0.55 + 0.7 * unit(i, 1, 52));
            i += 1;
        }
        // the snow along the shore
        for x in 0..wide {
            let lift = (noise(x as f32 * 0.12, 0.5, 55) * 2.0) as i32;
            for y in (tree_line - lift).max(0)..(shore + 1).min(tall) {
                canvas.set(x, y, c.bank);
            }
        }

        // the ice, nearer ice stronger, with soft drifts of snow lying on it and the shore's
        // shadow along its far edge
        let span = (tall - shore).max(1) as f32;
        for y in shore + 1..tall {
            let near = (y - shore) as f32 / span;
            let ice = c.ice.at(near);
            for x in 0..wide {
                let n = noise(x as f32 * 0.045, y as f32 * 0.16 / near.max(0.2), 57);
                let colour = if y <= shore + 2 && unit(x / 2, y, 58) < 0.7 {
                    c.shadow
                } else if n > 0.77 {
                    c.drift
                } else if n > 0.73 {
                    blender.mix(ice, c.drift, 0.5)
                } else {
                    ice
                };
                canvas.set(x, y, colour);
            }
        }
        // the moon's light lying on the ice below it
        if shore >= 12 && wide >= 30 {
            for y in shore + 2..tall {
                let w = 1.0 + (y - shore) as f32 * 0.12;
                for x in (moon.0 - w) as i32..=(moon.0 + w) as i32 {
                    let fade = 1.0 - (x as f32 - moon.0).abs() / (w + 0.5);
                    if noise(x as f32 * 0.4, y as f32 * 0.7, 59) < 0.6 {
                        blender.blend(canvas, x, y, c.sheen, 0.4 * fade);
                    }
                }
            }
        }

        // the cracks, under the water once the hole opens over them
        let front = self.spread() * self.longest;
        for mark in &self.marks {
            if mark.reach <= front {
                canvas.set(mark.x, mark.y, c.crack);
            }
        }

        // the hole, a ragged ring of open water round the weak point with a lit broken edge
        let (ox, oy) = self.origin();
        let open = self.hole();
        if open > 0.0 {
            let r = open * self.hole_size();
            let reach = (r * 1.3).ceil() as i32 + 2;
            for y in oy as i32 - reach..=oy as i32 + reach {
                for x in ox as i32 - reach..=ox as i32 + reach {
                    let (dx, dy) = (x as f32 + 0.5 - ox, (y as f32 + 0.5 - oy) / FLATTEN);
                    let edge = r * self.hole_radius(dy.atan2(dx));
                    let d = (dx * dx + dy * dy).sqrt();
                    if y <= shore + 1 {
                        continue;
                    }
                    if d <= edge {
                        // deeper water in the middle, the far side of the hole in shadow
                        let colour = if d < edge * 0.55 || dy < -edge * 0.3 {
                            c.deep
                        } else {
                            c.water
                        };
                        canvas.set(x, y, colour);
                    } else if d <= edge + 1.6 && dy > -edge * 0.2 {
                        // the near lip of broken ice catches the light
                        canvas.set(x, y, c.rim);
                    }
                }
            }
        }

        // a drift of snow on the near bank, with dry reeds standing out of it
        if wide >= 30 && tall >= 16 {
            let mound = (tall as f32 * 0.14).max(3.0);
            let reach = wide as f32 * 0.24;
            for x in 0..reach as i32 {
                let t = x as f32 / reach;
                let h = mound * (1.0 - t * t) + noise(x as f32 * 0.2, 0.5, 60) * 1.5;
                for y in (tall as f32 - h).round() as i32..tall {
                    canvas.set(x, y, c.bank);
                }
            }
            // a few bulrushes: a thin stalk, a dark seed head near the top and a leaf
            let count = ((reach / 5.0) as i32).max(3);
            for k in 0..count {
                let x = 2 + k * 5 + (unit(k, 0, 61) * 3.0) as i32;
                let t = x as f32 / reach;
                let foot = tall as f32 - mound * (1.0 - t * t) + 1.0;
                let h = mound * (1.1 + 0.9 * unit(k, 1, 61));
                let lean = (unit(k, 2, 61) - 0.4) * 2.5;
                let top = ((x as f32 + lean).round() as i32, (foot - h).round() as i32);
                canvas.line((x, foot as i32), top, c.reed);
                let head = (h * 0.22).clamp(2.0, 4.0) as i32;
                let hx = x + ((top.0 - x) as f32 * 0.85).round() as i32;
                canvas.fill_rect(hx, top.1 + 2, 1, head, c.rush_head);
                let leaf = (h * 0.45) as i32;
                let side = if k % 2 == 0 { 1 } else { -1 };
                canvas.line(
                    (x, foot as i32 - 1),
                    (x + side * (leaf / 2).max(1), foot as i32 - leaf),
                    c.reed,
                );
            }
        }

        for flake in &self.flakes {
            let (x, y) = (flake.x.round() as i32, flake.y.round() as i32);
            if c.edged_flakes {
                canvas.set(x, y + 1, c.flake_edge);
            }
            canvas.set(x, y, c.flake);
        }
    }
}

impl Scene for ThinIce {
    fn name(&self) -> &'static str {
        "thin-ice"
    }

    fn resize(&mut self, width: usize, height: usize, _rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        // the fracture is laid out from its own seed, so a resize redraws the same pattern at
        // the new size and the spread reached so far carries over
        let mut layout = Rng::new(0x1ce);
        self.grow_cracks(&mut layout);
        self.flakes.clear();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let moved = dt * self.motion.speed();
        self.crack_t += moved;
        let cap = self
            .motion
            .scaled((self.width * self.height / 120).clamp(1, 24));
        if self.flakes.len() < cap && rng.chance(0.3) {
            self.flakes.push(Flake {
                x: rng.range(0.0, self.width as f32),
                y: -1.0,
                speed: rng.range(1.5, 3.0),
            });
        }
        let t = self.t;
        for (i, flake) in self.flakes.iter_mut().enumerate() {
            flake.y += flake.speed * moved;
            flake.x += (super::cycle(t, 0.7) + i as f32).sin() * 0.4 * moved;
        }
        let tall = self.tall() as f32;
        self.flakes.retain(|f| f.y < tall);
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
    use crate::scenes::testing::{check_palette, check_pixel_scene};
    use crate::theme;

    fn cracks_shown(ice: &ThinIce) -> usize {
        let front = ice.spread() * ice.longest;
        ice.marks.iter().filter(|m| m.reach <= front).count()
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("thin-ice");
        check_determinism("thin-ice");
    }

    #[test]
    fn passes_the_pixel_checks() {
        check_pixel_scene("thin-ice", 30.0);
        check_palette(Colours::palette);
    }

    #[test]
    fn the_bands_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            for (a, b, what) in [
                (c.sky_top, c.sky_low, "the sky"),
                (c.pine, c.hills, "the pines and the hills"),
                (c.far_pine, c.pine, "the far and near pines"),
                (c.ice_far, c.ice_near, "the far and near ice"),
                (c.ice_far, c.bank, "the ice and the bank"),
                (c.water, c.ice_near, "the water and the ice"),
                (c.deep, c.ice_far, "the deep water and the ice"),
                (c.flake, c.flake_edge, "a flake and its edge"),
            ] {
                assert!(distance(a, b) >= 0.03, "{what} in {}", theme.name);
            }
        }
    }

    #[test]
    fn cracks_creep_out_slowly_then_hold_and_survive_a_resize() {
        let mut rng = Rng::new(1);
        let mut ice = ThinIce::new(80, 24, Motion::FULL, &mut rng);
        let start = cracks_shown(&ice);
        assert!(start > 0, "the weak point shows from the start");
        let mut last = start;
        for _ in 0..(90 * 20) {
            ice.step(0.05, &mut rng);
            let now = cracks_shown(&ice);
            // only a few pixels of crack appear in any one frame
            assert!(now >= last && now - last <= 24, "{last} then {now}");
            last = now;
        }
        assert_eq!(last, ice.marks.len());
        assert_eq!(ice.hole(), 1.0);
        ice.resize(1, 1, &mut rng);
        ice.resize(80, 24, &mut rng);
        assert_eq!(cracks_shown(&ice), last);
    }

    #[test]
    fn calm_cracks_spread_at_half_the_rate() {
        let mut rng = Rng::new(1);
        let mut full = ThinIce::new(80, 24, Motion::FULL, &mut rng);
        let mut calm = ThinIce::new(80, 24, Motion::CALM, &mut rng);
        for _ in 0..(20 * 20) {
            full.step(0.05, &mut rng);
            calm.step(0.05, &mut rng);
        }
        assert!(cracks_shown(&calm) < cracks_shown(&full));
    }
}
