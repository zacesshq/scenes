//! A forest stream in solid pixels: a stream winding out of the trees from a far bend on the
//! horizon, widening as it comes nearer between mossy banks, with trunks standing at three
//! depths, a canopy overhead with gaps of sky, slanting shafts of light that drift slowly across
//! the wood, rounded stones on the banks and in the water, leaves riding the current, ferns in
//! the near corners and motes of dust turning in the light.
//!
//! On a dark theme the wood is in shade and the light glows through from the clearing beyond,
//! so the near trunks stand dark against it; on a light theme it is a bright day, the far wood
//! pale with haze and the trunks darkening as they come nearer. The sky, the canopy, the trunks,
//! the light, the floor, the banks, the stones and the water are tints of the background held
//! below the non-text floor; the leaves on the water and the ferns keep the theme's own drawing
//! colours, so they stand out as shapes and keep their outline when colour is off.

use super::paint::{Blender, ellipse, noise, tint, unit};
use super::{cycle, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str =
    "A stream winding through a forest, light slanting through the trees and leaves on the water";
pub const CAPTIONS: &[&str] = &[
    "All clear downstream.",
    "A quiet build in the woods.",
    "Nothing in the way. Let it flow.",
];

/// How long the light shafts take to drift one shaft's width at the standard pace. Each shaft
/// is wide and soft, so a cell under it changes shade only once every few seconds.
pub const LIGHT_PERIOD: f32 = 40.0;
/// How far down the stream a leaf comes into view, clear of the bushes at the far bend.
const LEAF_START: f32 = 0.12;
/// Steps in the water's flow highlights.
const FLOW_STEPS: usize = 3;
/// Steps in the haze from the canopy down to the horizon and in the floor from far to near.
const DEPTH_STEPS: usize = 6;

/// A theme colour of `hue` at about `contrast` against the background, for imagery placed by
/// how far it stands from the background rather than by how much hue it carries.
fn at(theme: &ThemeData, hue: Rgb, contrast: f64) -> Rgb {
    tint(theme, hue, 1.0, contrast)
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct Colours {
    pub sky: Rgb,
    /// From under the canopy down to the glow on the horizon.
    pub haze: Ramp,
    pub treeline: Rgb,
    pub treeline_lit: Rgb,
    pub far_trunk: Rgb,
    pub mid_trunk: Rgb,
    pub mid_shade: Rgb,
    /// The near trunks lit, plain and in shade.
    pub near: [Rgb; 3],
    pub canopy: Rgb,
    pub canopy_lit: Rgb,
    pub light: Rgb,
    /// The floor from the horizon to the bottom edge.
    pub floor: Ramp,
    pub litter: Rgb,
    pub moss: Rgb,
    pub moss_lit: Rgb,
    pub bank_edge: Rgb,
    pub far_water: Rgb,
    pub near_water: Rgb,
    pub ripple: Rgb,
    /// The stones lit, plain and in shade.
    pub stone: [Rgb; 3],
    pub fern: Rgb,
    pub leaves: [Rgb; 3],
    water: Vec<Ramp>,
}

impl Colours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let dark = theme.dark;
        let pick = |d: f64, l: f64| if dark { d } else { l };
        let green = theme.green();
        let leafy = mix(green, theme.cyan(), 0.25);
        // a light theme's brown leans to olive, since a pale red-brown reads as pink
        let brown = if dark {
            mix(mix(theme.yellow(), theme.red(), 0.35), theme.muted, 0.3)
        } else {
            mix(mix(theme.yellow(), theme.red(), 0.2), theme.muted, 0.45)
        };
        let glade = mix(theme.yellow(), green, 0.45);
        let grey = mix(theme.muted, theme.blue(), 0.15);

        // the wood is placed by distance: on a dark theme the clearing behind glows and the
        // nearer things are darker against it, on a light theme the far wood is pale with haze
        // and the nearer things are darker and richer, so depth reads the same way in both
        let haze = Ramp::new(
            at(theme, leafy, pick(1.55, 1.5)),
            at(theme, glade, pick(2.45, 1.12)),
            DEPTH_STEPS,
        );
        let floor = Ramp::new(
            at(theme, mix(brown, glade, 0.4), pick(1.7, 1.3)),
            at(
                theme,
                mix(brown, green, if dark { 0.0 } else { 0.2 }),
                pick(1.4, 1.75),
            ),
            DEPTH_STEPS,
        );
        let far_water = at(theme, mix(theme.cyan(), glade, 0.3), pick(2.35, 1.22));
        let near_water = at(theme, theme.blue(), pick(1.4, 2.15));
        let ripple = if dark {
            at(theme, theme.bright_cyan(), 2.9)
        } else {
            at(theme, theme.info, 1.04)
        };
        let depth = Ramp::new(far_water, near_water, 8);
        let water = depth
            .steps()
            .iter()
            .map(|&base| Ramp::new(base, ripple, FLOW_STEPS))
            .collect();
        Self {
            sky: at(theme, theme.info, pick(2.75, 1.1)),
            haze,
            treeline: at(theme, green, pick(1.8, 1.65)),
            treeline_lit: at(theme, glade, pick(2.15, 1.38)),
            far_trunk: at(theme, mix(theme.muted, green, 0.4), pick(1.95, 1.42)),
            mid_trunk: at(theme, brown, pick(1.3, 1.95)),
            mid_shade: at(theme, brown, pick(1.15, 2.3)),
            near: [
                at(theme, mix(brown, theme.yellow(), 0.3), pick(1.6, 2.15)),
                at(theme, brown, pick(1.15, 2.6)),
                at(theme, brown, pick(1.05, 2.95)),
            ],
            canopy: at(theme, green, pick(1.25, 2.55)),
            canopy_lit: at(theme, mix(green, theme.yellow(), 0.3), pick(1.7, 1.85)),
            light: if dark {
                at(theme, theme.bright_yellow(), 2.85)
            } else {
                theme.background
            },
            floor,
            litter: at(theme, mix(brown, theme.yellow(), 0.3), pick(1.6, 1.9)),
            moss: at(theme, green, pick(1.55, 1.95)),
            moss_lit: at(theme, mix(green, theme.yellow(), 0.35), pick(2.0, 1.5)),
            bank_edge: at(theme, mix(green, brown, 0.5), pick(1.1, 2.6)),
            far_water,
            near_water,
            ripple,
            stone: [
                at(theme, grey, pick(2.6, 1.35)),
                at(theme, grey, pick(1.85, 1.9)),
                at(theme, grey, pick(1.3, 2.6)),
            ],
            fern: green,
            leaves: [theme.warning, theme.red(), theme.bright_green()],
            water,
        }
    }

    fn water(&self, depth: f32, ripple: f32) -> Rgb {
        let last = self.water.len() - 1;
        let i = (depth.clamp(0.0, 1.0) * last as f32).round() as usize;
        self.water[i.min(last)].at(ripple)
    }

    #[cfg(test)]
    pub(crate) fn palette(theme: &ThemeData) -> crate::scenes::testing::Palette {
        let c = Self::new(theme);
        let mut imagery = vec![
            ("sky", c.sky),
            ("treeline", c.treeline),
            ("lit treeline", c.treeline_lit),
            ("far trunk", c.far_trunk),
            ("middle trunk", c.mid_trunk),
            ("middle trunk shade", c.mid_shade),
            ("lit near trunk", c.near[0]),
            ("near trunk", c.near[1]),
            ("near trunk shade", c.near[2]),
            ("canopy", c.canopy),
            ("lit canopy", c.canopy_lit),
            ("light", c.light),
            ("litter", c.litter),
            ("moss", c.moss),
            ("lit moss", c.moss_lit),
            ("bank edge", c.bank_edge),
            ("far water", c.far_water),
            ("near water", c.near_water),
            ("ripple", c.ripple),
            ("lit stone", c.stone[0]),
            ("stone", c.stone[1]),
            ("stone shade", c.stone[2]),
        ];
        imagery.extend(c.haze.steps().iter().map(|&h| ("haze", h)));
        imagery.extend(c.floor.steps().iter().map(|&f| ("floor", f)));
        crate::scenes::testing::Palette {
            imagery,
            shapes: vec![
                ("fern", c.fern),
                ("yellow leaf", c.leaves[0]),
                ("red leaf", c.leaves[1]),
                ("green leaf", c.leaves[2]),
            ],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Leaf {
    /// How far down the stream, from 0 at the far bend to 1 at the bottom edge.
    along: f32,
    /// Across the stream, from -1 to 1.
    across: f32,
    colour: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct Mote {
    x: f32,
    y: f32,
    phase: f32,
}

/// A trunk standing on the floor: its column, its width, how far down its foot stands and its
/// depth from 0 far to 2 near.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Trunk {
    x: i32,
    width: i32,
    foot: i32,
    depth: usize,
}

/// A stone: its centre, its radius and whether it sits out in the water.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Stone {
    x: f32,
    y: f32,
    r: f32,
    wet: bool,
}

pub struct ForestStream {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    trunks: Vec<Trunk>,
    stones: Vec<Stone>,
    ferns: Vec<(i32, bool)>,
    leaves: Vec<Leaf>,
    motes: Vec<Mote>,
}

impl ForestStream {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut forest = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            trunks: Vec::new(),
            stones: Vec::new(),
            ferns: Vec::new(),
            leaves: Vec::new(),
            motes: Vec::new(),
        };
        forest.resize(width, height, rng);
        forest
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The pixel row of the horizon, where the stream comes out of the far wood.
    fn horizon(&self) -> i32 {
        ((self.tall() as f32 * 0.5).round() as i32).clamp(1, self.tall())
    }

    /// The pixel row the canopy hangs down to, before its clumps.
    fn roof(&self) -> f32 {
        (self.tall() as f32 * 0.17).max(1.0)
    }

    /// How far down the floor a row is, from 0 at the horizon to 1 at the bottom.
    fn near(&self, y: f32) -> f32 {
        let span = (self.tall() - self.horizon()).max(1) as f32;
        ((y - self.horizon() as f32) / span).clamp(0.0, 1.0)
    }

    /// The distance along the stream from the viewer, which grows quickly towards the horizon
    /// the way a flat stream does in perspective.
    fn distance(near: f32) -> f32 {
        1.0 / (near + 0.25) - 0.8
    }

    /// The stream's middle and half width at a row: it bends in from the side at the horizon,
    /// swings back across the floor and comes out wide at the bottom.
    fn stream_at(&self, y: f32) -> (f32, f32) {
        let near = self.near(y);
        let w = self.width as f32;
        let swing = (near * 4.6 - 0.9).sin() * near.powf(0.7) * 0.17 - near * 0.05;
        let middle = w * (0.57 + swing);
        let half = 0.9 + w * 0.2 * (0.06 + 0.94 * near.powf(1.25));
        (middle, half)
    }

    /// The width of the moss bank either side of the water at a row.
    fn bank_at(&self, y: f32) -> f32 {
        1.0 + self.near(y) * self.width as f32 * 0.03
    }

    /// The top of the low far wood along the horizon at a column, which hides the stream's
    /// far bend.
    fn treeline_at(&self, x: i32) -> i32 {
        let scale = (self.tall() as f32 / 30.0).max(1.0);
        let lumps = noise(x as f32 * 0.11 / scale, 0.0, 91) * 3.5 + 1.5;
        self.horizon() - (lumps * scale).round() as i32
    }

    /// Whether a point is on the water or its banks, so trunks and ferns never stand in the
    /// stream.
    fn in_stream(&self, x: f32, y: f32, margin: f32) -> bool {
        let (middle, half) = self.stream_at(y);
        (x - middle).abs() <= half + self.bank_at(y) + margin
    }

    fn paint_backdrop(&self, canvas: &mut Canvas, c: &Colours) {
        let (wide, tall) = (canvas.width() as i32, canvas.height() as i32);
        let horizon = self.horizon();
        let span = horizon.max(1) as f32;
        for y in 0..horizon.min(tall) {
            canvas.fill_row(y, c.haze.at(y as f32 / span));
        }
        // the far trunks fade into the haze, so they come before the far wood covers their feet
        for trunk in self.trunks.iter().filter(|t| t.depth == 0) {
            canvas.fill_rect(trunk.x, 0, trunk.width, trunk.foot, c.far_trunk);
        }
        for x in 0..wide {
            let top = self.treeline_at(x);
            for y in top.max(0)..horizon.min(tall) {
                let lit = noise(x as f32 * 0.3, y as f32 * 0.5, 92) > 0.62 && y < horizon - 1;
                canvas.set(x, y, if lit { c.treeline_lit } else { c.treeline });
            }
        }
        // the floor: earth with patches of moss and fallen leaves, its texture finer with
        // distance
        for y in horizon..tall {
            let near = self.near(y as f32);
            let scale = 0.15 + near * 0.85;
            let base = c.floor.at(near);
            for x in 0..wide {
                let fx = (x as f32 - wide as f32 / 2.0) / scale;
                let mossy = noise(fx * 0.12, y as f32 * 0.25 / scale, 84) > 0.6;
                let litter = unit(x / 2, y, 85) < 0.02 + near * 0.03;
                let colour = if mossy {
                    c.moss
                } else if litter {
                    c.litter
                } else {
                    base
                };
                canvas.set(x, y, colour);
            }
        }
        // the stream between its banks, the ripples running down it towards the viewer
        let flow = cycle(self.t, 1.4 * self.motion.speed());
        for y in horizon..tall {
            let fy = y as f32 + 0.5;
            let near = self.near(fy);
            let (middle, half) = self.stream_at(fy);
            let bank = self.bank_at(fy);
            let d = Self::distance(near);
            let left = (middle - half - bank).round() as i32;
            let right = (middle + half + bank).round() as i32;
            for x in left..=right {
                let off = (x as f32 + 0.5 - middle).abs();
                if off > half {
                    // the bank: lit moss on top, a dark wet lip right at the water
                    let colour = if off - half < 1.0 {
                        c.bank_edge
                    } else if noise(x as f32 * 0.4, fy * 0.6, 86) > 0.45 {
                        c.moss_lit
                    } else {
                        c.moss
                    };
                    canvas.set(x, y, colour);
                    continue;
                }
                let across = (x as f32 + 0.5 - middle) / half.max(0.5);
                let wave =
                    (d * 9.0 + flow + across * 0.9).sin() + 0.5 * (x as f32 * 0.31 + d * 4.0).sin();
                let ripple = ((wave - 0.9) / 0.5).clamp(0.0, 1.0) * (1.0 - across.abs() * 0.6);
                canvas.set(x, y, c.water(near.powf(0.7), ripple));
            }
        }
    }

    /// Bushes either side of the far bend, so the stream comes out from between them rather
    /// than ending in a point.
    fn paint_bushes(&self, canvas: &mut Canvas, c: &Colours) {
        let horizon = self.horizon() as f32;
        let floor = (self.tall() as f32 - horizon).max(1.0);
        let (far, half) = self.stream_at(horizon + 1.0);
        let r = (floor * 0.2).max(2.0);
        // the far wood hangs a little over the water where it goes out of sight, so the mouth
        // has a ragged edge rather than a straight one
        let mouth = half.ceil() as i32 + 1;
        for x in far.round() as i32 - mouth..=far.round() as i32 + mouth {
            let down = (noise(x as f32 * 0.7, 5.0, 96) * 2.4).round() as i32;
            for y in horizon as i32..horizon as i32 + down {
                canvas.set(x, y, c.treeline);
            }
        }
        for (side, size) in [(-1.0, 0.75), (1.0, 0.65), (-1.0, 1.0), (1.0, 0.9)] {
            let r = r * size;
            let reach = if size < 0.8 { 0.45 } else { 1.8 };
            let cx = far + side * (half + 1.0 + r * reach);
            let base = horizon + r * 0.3;
            let x0 = (cx - r * 1.3).floor() as i32;
            let x1 = (cx + r * 1.3).ceil() as i32;
            for x in x0..=x1 {
                let u = (x as f32 + 0.5 - cx) / (r * 1.3);
                if u.abs() >= 1.0 {
                    continue;
                }
                let lumps = 0.85 + 0.15 * noise(x as f32 * 0.6, 3.0, 93);
                let top = base - r * (1.0 - u * u).sqrt() * lumps;
                for y in top.round() as i32..base.round() as i32 {
                    // lit from the upper left like everything else in the wood
                    let v = (y as f32 + 0.5 - top) / (base - top).max(1.0);
                    let lit = u * 0.7 + v < 0.45 || noise(x as f32 * 0.5, y as f32 * 0.7, 94) > 0.7;
                    canvas.set(x, y, if lit { c.canopy_lit } else { c.canopy });
                }
            }
        }
    }

    fn paint_stone(&self, canvas: &mut Canvas, c: &Colours, s: &Stone) {
        let (rx, ry) = (s.r * 1.35, s.r);
        if s.wet {
            // the water breaks pale around a stone out in the stream
            ellipse(
                canvas,
                s.x,
                s.y + ry * 0.55,
                rx + 1.0,
                ry * 0.5 + 0.6,
                c.ripple,
            );
        }
        ellipse(canvas, s.x, s.y, rx, ry, c.stone[1]);
        if s.r < 1.0 {
            return;
        }
        let (x0, x1) = ((s.x - rx).floor() as i32, (s.x + rx).ceil() as i32);
        let (y0, y1) = ((s.y - ry).floor() as i32, (s.y + ry).ceil() as i32);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = (x as f32 + 0.5 - s.x) / rx;
                let dy = (y as f32 + 0.5 - s.y) / ry;
                if dx * dx + dy * dy > 1.0 {
                    continue;
                }
                // lit from the upper left like the trees, so each stone reads as a rounded rock
                let side = dx * 0.6 + dy;
                if side < -0.45 {
                    canvas.set(x, y, c.stone[0]);
                } else if side > 0.45 {
                    canvas.set(x, y, c.stone[2]);
                }
            }
        }
    }

    fn paint_trunk(&self, canvas: &mut Canvas, c: &Colours, t: &Trunk) {
        let (lit, base, shade) = match t.depth {
            1 => (c.mid_trunk, c.mid_trunk, c.mid_shade),
            _ => (c.near[0], c.near[1], c.near[2]),
        };
        // the roots flare out over the last few rows, so the trunk stands on the floor
        let flare_rows = (t.width / 2).max(1) + 1;
        for y in 0..t.foot {
            let from_foot = t.foot - y;
            let flare = if from_foot <= flare_rows {
                (flare_rows - from_foot + 1).min(t.width / 2 + 1)
            } else {
                0
            };
            for x in t.x - flare..t.x + t.width + flare {
                let k = x - t.x;
                let colour = if k < (t.width / 4).max(1) && t.width >= 3 {
                    lit
                } else if k >= t.width - (t.width + 1) / 3 {
                    shade
                } else if t.depth == 2 && unit(x, y / 3, 81) < 0.12 {
                    // furrows in the bark of the nearest trunks
                    shade
                } else {
                    base
                };
                canvas.set(x, y, colour);
            }
        }
    }

    fn paint_canopy(&self, canvas: &mut Canvas, c: &Colours) {
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let roof = self.roof();
        let scale = (tall as f32 / 50.0).max(0.6);
        for x in 0..wide {
            let fx = x as f32 / scale;
            // clumps of leaves hang below the roof here and there
            let hang = noise(fx * 0.07, 0.0, 87);
            let edge = roof * (0.65 + hang * 0.9) + noise(fx * 0.4, 1.0, 88) * 2.0 * scale;
            for y in 0..(edge.round() as i32).min(tall) {
                let fy = y as f32 / scale;
                let gap = noise(fx * 0.12, fy * 0.22, 89);
                if y as f32 + 2.0 * scale < edge * 0.75 && gap < 0.2 {
                    canvas.set(x, y, c.sky);
                    continue;
                }
                // leaves catch the light in clusters, with loose leaves at their edges
                let clump = noise(fx * 0.28, fy * 0.45, 90);
                let lit = clump > 0.6 || (clump > 0.5 && unit(x, y, 95) < 0.35);
                canvas.set(x, y, if lit { c.canopy_lit } else { c.canopy });
            }
        }
    }

    fn paint_fern(&self, canvas: &mut Canvas, c: &Colours, x: i32, lean_left: bool) {
        let tall = self.tall();
        let foot = (tall - 1) as f32;
        let size = (tall as f32 / 5.0).max(4.0);
        let lean = if lean_left { -0.25 } else { 0.25 };
        for frond in [-3, -2, -1, 0, 1, 2, 3] {
            let spread = frond as f32;
            let length = size * (1.0 - spread.abs() * 0.15);
            let mut angle = -std::f32::consts::FRAC_PI_2 + spread * 0.38 + lean;
            // each frond arches outwards and its tip droops
            let bend = spread.signum() * 0.06 + lean * 0.2;
            let (mut px, mut py) = (x as f32, foot);
            let steps = length.round() as i32;
            for i in 0..steps {
                px += angle.cos();
                py += angle.sin();
                angle += bend;
                canvas.set(px.round() as i32, py.round() as i32, c.fern);
                // leaflets point forwards along the frond and shrink towards the tip
                let left = 1.0 - i as f32 / steps as f32;
                let leaflet = (left * size * 0.11).round() as i32;
                if i >= 2 && i % 3 == 0 && leaflet >= 1 {
                    for side in [-1.0_f32, 1.0] {
                        let a = angle + side * 1.1;
                        for k in 1..=leaflet {
                            let lx = px + a.cos() * k as f32;
                            let ly = py + a.sin() * k as f32;
                            canvas.set(lx.round() as i32, ly.round() as i32, c.fern);
                        }
                    }
                }
            }
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        let mut blender = Blender::new(3);
        let (wide, tall) = (canvas.width() as i32, canvas.height() as i32);
        let horizon = self.horizon();
        self.paint_backdrop(canvas, c);
        self.paint_bushes(canvas, c);

        // things on the floor and the water, then the trunks from far to near; each trunk only
        // covers the rows above its foot, which are further away than it, so the stones and
        // leaves in front of a trunk stay in front
        for stone in &self.stones {
            self.paint_stone(canvas, c, stone);
        }
        for leaf in &self.leaves {
            let y = horizon as f32 + 1.0 + leaf.along * (tall - horizon - 1) as f32;
            let (middle, half) = self.stream_at(y);
            let x = (middle + leaf.across * half * 0.7).round() as i32;
            let y = y.round() as i32;
            let colour = c.leaves[leaf.colour % c.leaves.len()];
            canvas.set(x, y, colour);
            if leaf.along > 0.3 {
                canvas.set(x + 1, y, colour);
            }
            if leaf.along > 0.65 {
                canvas.set(x, y + 1, colour);
            }
        }
        for depth in 1..3 {
            for trunk in self.trunks.iter().filter(|t| t.depth == depth) {
                self.paint_trunk(canvas, c, trunk);
            }
        }
        self.paint_canopy(canvas, c);

        // shafts of light slanting down through the trees, drifting very slowly; the light stays
        // in place over the wood, so lively never moves it faster than the standard pace
        let drift = self.t as f32 * self.motion.pace() / LIGHT_PERIOD;
        let roof = self.roof() as i32;
        let scale = (tall as f32 / 50.0).max(0.6);
        for y in roof.max(0)..tall {
            let fade = (y - roof) as f32 / (tall - roof).max(1) as f32;
            for x in 0..wide {
                let along = (x as f32 - y as f32 * 0.55) * 0.06 / scale + drift;
                let shaft = noise(along, 0.5, 83);
                if shaft > 0.56 {
                    let amount = ((shaft - 0.56) * 3.5 - fade * 0.5).clamp(0.0, 0.5);
                    blender.blend(canvas, x, y, c.light, amount);
                }
            }
        }

        // ferns in the near corners: fronds arching up and out from one spot, each with leaflets
        // either side that shrink towards its tip
        for &(x, lean_left) in &self.ferns {
            self.paint_fern(canvas, c, x, lean_left);
        }
        for mote in &self.motes {
            canvas.set(mote.x.round() as i32, mote.y.round() as i32, c.light);
        }
    }
}

impl Scene for ForestStream {
    fn name(&self) -> &'static str {
        "forest-stream"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let w = self.width as i32;
        let tall = self.tall();
        let horizon = self.horizon();
        let floor = (tall - horizon).max(1);

        self.trunks.clear();
        // far trunks crowd the haze, thin and close together
        let mut x = rng.below(4) as i32 - 2;
        while x < w {
            self.trunks.push(Trunk {
                x,
                width: 1 + i32::from(w >= 160),
                foot: horizon,
                depth: 0,
            });
            x += 3 + rng.below(4) as i32;
        }
        // middle trunks stand a little way back from the banks
        let mid_width = (w / 50).max(2);
        let mut x = rng.below(8) as i32;
        while x < w {
            let foot = horizon + (floor as f32 * rng.range(0.1, 0.3)).round() as i32;
            if !self.in_stream((x + mid_width / 2) as f32, foot as f32, 1.0) {
                self.trunks.push(Trunk {
                    x,
                    width: mid_width,
                    foot,
                    depth: 1,
                });
            }
            x += mid_width + 6 + rng.below(10) as i32;
        }
        // a few near trunks frame the picture, standing on the floor clear of the water
        let near_width = (w / 14).max(3);
        for (share, depth) in [(0.03, 0.95), (0.88, 0.85), (0.24, 0.62), (0.74, 0.55)] {
            let x = (w as f32 * share).round() as i32 + rng.below(3) as i32 - 1;
            let foot = horizon + (floor as f32 * depth).round() as i32;
            let width = (near_width as f32 * (0.55 + depth * 0.5)).round() as i32;
            if !self.in_stream(
                (x + width / 2) as f32,
                foot as f32,
                width as f32 / 2.0 + 1.0,
            ) {
                self.trunks.push(Trunk {
                    x,
                    width,
                    foot,
                    depth: 2,
                });
            }
        }

        // stones along the banks and a few out in the water, larger nearer the viewer
        self.stones.clear();
        let r = (tall as f32 / 26.0).max(0.6);
        for i in 0..(w / 16).clamp(2, 12) {
            let near = 0.22 + rng.unit() * 0.72;
            let y = horizon as f32 + near * floor as f32;
            let (middle, half) = self.stream_at(y);
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            let wet = i % 3 == 2;
            let out = if wet { rng.range(0.1, 0.55) } else { 1.0 };
            let size = r * (0.25 + near * 1.15);
            let x = middle + side * (half * out + if wet { 0.0 } else { size * 0.4 });
            self.stones.push(Stone { x, y, r: size, wet });
        }
        // the stones are painted far to near, so a nearer one overlaps a further one
        self.stones.sort_by(|a, b| a.y.total_cmp(&b.y));

        self.ferns.clear();
        if w >= 30 && tall >= 16 {
            for (x, lean_left) in [(w / 10, true), (w * 9 / 10, false), (w / 5, false)] {
                if !self.in_stream(x as f32, (tall - 1) as f32, 3.0) {
                    self.ferns.push((x, lean_left));
                }
            }
        }
        self.leaves.clear();
        let count = self.motion.scaled((self.width / 16).clamp(1, 8));
        // the leaves start spread evenly down the stream and keep their order as they go, so two
        // never sit side by side like a pair of eyes
        let gap = (1.0 - LEAF_START) / count as f32;
        for i in 0..count {
            self.leaves.push(Leaf {
                along: LEAF_START + gap * (i as f32 + rng.range(0.2, 0.8)),
                across: rng.range(-0.8, 0.8),
                colour: i,
            });
        }
        self.motes.clear();
        let motes = self
            .motion
            .scaled((self.width * self.height / 220).clamp(2, 18));
        for _ in 0..motes {
            self.motes.push(Mote {
                x: rng.range(0.0, self.width as f32),
                y: rng.range(self.roof() + 2.0, horizon as f32),
                phase: rng.range(0.0, std::f32::consts::TAU),
            });
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let moved = dt * self.motion.speed();
        for leaf in &mut self.leaves {
            // nearer means faster across the screen, as the stream comes towards the viewer
            leaf.along += (0.025 + leaf.along * 0.05) * moved;
            if leaf.along > 1.0 {
                leaf.along = LEAF_START;
                leaf.across = rng.range(-0.8, 0.8);
            }
        }
        let (w, top, bottom) = (self.width as f32, self.roof() + 2.0, self.horizon() as f32);
        let t = self.t;
        for mote in &mut self.motes {
            mote.x = wrap(mote.x + 0.4 * moved, w);
            mote.y += (cycle(t, 0.5) + mote.phase).sin() * 0.3 * moved;
            mote.y = mote.y.clamp(top.min(bottom), bottom);
        }
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

    #[test]
    fn passes_the_shared_checks() {
        check_scene("forest-stream");
        check_determinism("forest-stream");
    }

    #[test]
    fn passes_the_pixel_checks() {
        check_pixel_scene("forest-stream", 6.0);
        check_palette(Colours::palette);
    }

    #[test]
    fn the_wood_and_the_water_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            let haze_low = *c.haze.steps().last().unwrap();
            let floor_near = *c.floor.steps().last().unwrap();
            for (a, b, what) in [
                (haze_low, c.treeline, "the glow and the far wood"),
                (haze_low, c.near[1], "the glow and the near trunks"),
                (c.far_trunk, c.near[1], "the far and near trunks"),
                (c.canopy, c.canopy_lit, "the canopy and its lit leaves"),
                (c.canopy, c.sky, "the canopy and the sky"),
                (floor_near, c.near_water, "the floor and the water"),
                (floor_near, c.near[1], "the floor and the near trunks"),
                (c.moss, c.near_water, "the banks and the water"),
                (c.far_water, c.near_water, "the far and near water"),
                (c.ripple, c.near_water, "the ripples and the water"),
                (c.stone[1], c.near_water, "the stones and the water"),
                (c.stone[0], c.stone[2], "the lit and shaded stone"),
            ] {
                assert!(distance(a, b) >= 0.03, "{what} in {}", theme.name);
            }
        }
    }

    #[test]
    fn leaves_ride_the_stream_and_the_stream_widens_towards_the_viewer() {
        let mut rng = Rng::new(2);
        let mut forest = ForestStream::new(80, 24, Motion::FULL, &mut rng);
        let far = forest.stream_at(forest.horizon() as f32);
        let near = forest.stream_at((forest.tall() - 1) as f32);
        assert!(near.1 > far.1 * 4.0);
        let before: Vec<f32> = forest.leaves.iter().map(|l| l.along).collect();
        for _ in 0..20 {
            forest.step(0.05, &mut rng);
        }
        for (leaf, was) in forest.leaves.iter().zip(before) {
            assert!(leaf.along > was || leaf.along < LEAF_START + 0.05);
        }
    }

    #[test]
    fn the_light_never_drifts_faster_than_the_standard_pace() {
        let draw = |motion: Motion| {
            let mut rng = Rng::new(6);
            let mut forest = ForestStream::new(80, 24, motion, &mut rng);
            forest.leaves.clear();
            forest.motes.clear();
            for _ in 0..200 {
                forest.step(0.05, &mut rng);
            }
            let mut canvas = Canvas::new(80, 24, theme::default_theme().background);
            let c = Colours::new(theme::default_theme());
            // only the light differs once the ripples are held, so compare above the floor
            forest.t = 10.0;
            forest.paint(&mut canvas, &c);
            (0..forest.horizon())
                .flat_map(|y| (0..80).map(move |x| (x, y)))
                .map(|(x, y)| canvas.get(x, y))
                .collect::<Vec<_>>()
        };
        assert!(draw(Motion::LIVELY) == draw(Motion::FULL));
    }

    #[test]
    fn trunks_and_ferns_stand_clear_of_the_water() {
        for (w, h) in [(80, 24), (120, 34), (200, 50)] {
            let mut rng = Rng::new(4);
            let forest = ForestStream::new(w, h, Motion::FULL, &mut rng);
            for trunk in forest.trunks.iter().filter(|t| t.depth > 0) {
                let (middle, half) = forest.stream_at(trunk.foot as f32);
                let centre = (trunk.x + trunk.width / 2) as f32;
                assert!(
                    (centre - middle).abs() > half,
                    "a trunk in the water at {w}x{h}"
                );
            }
            assert!(
                forest.trunks.iter().any(|t| t.depth == 2),
                "no near trunk at {w}x{h}"
            );
        }
    }
}
