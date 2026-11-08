//! Leak: a brick basement lit by one bare bulb, where a metal pipe runs under the ceiling and
//! down the wall to a shut-off valve. The pipe has split open. A bead of water swells under
//! the crack, lets go and falls into a pail on the concrete floor. The pail fills drip by drip,
//! rings spreading on its water, until it brims over, runs down its sides and a puddle spreads
//! across the floor and holds at its widest. A damp stain creeps down the bricks below the crack.
//!
//! The bricks, the ceiling, the floor, the boxes and the bulb's light are tints of the
//! background held below the non-text floor. The pipe, the crack, the valve, the bulb, the pail
//! and the water are solid, so with colour off the split pipe and the brimming pail still read.

use super::paint::{self, Blender, ellipse, unit};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::error_colour;

pub const SUMMARY: &str = "A cracked pipe in a basement dripping into a pail that fills, brims over and spreads into a puddle";

pub const CAPTIONS: &[&str] = &[
    "Something is leaking. Find the first drip.",
    "A slow leak. Patch the first error and the rest dry up.",
    "The pail is filling. Time for a closer look.",
    "One crack at a time. Start at the top of the list.",
    "Not a flood yet. Read the first message.",
];

/// Seconds for a bead of water to swell under the crack and fall, at the standard pace.
pub const DRIP_PERIOD: f32 = 1.2;
/// Drips that fill the pail from empty.
const DRIPS_TO_FILL: f32 = 40.0;
/// How full the pail already is when the scene starts, so the story reads from the first look.
const START_LEVEL: f32 = 0.5;
/// Drips that spill over a full pail before the puddle reaches its widest.
const DRIPS_TO_SPREAD: f32 = 45.0;
/// How long a ring spreads on the pail's water, in seconds.
const RING_LIFE: f32 = 1.4;
/// Seconds for the damp patch on the wall to reach its full length.
const DAMP_SECONDS: f32 = 90.0;

#[derive(Clone, Debug)]
struct Drip {
    y: f32,
    vy: f32,
}

/// The basement's colours in one theme.
struct Colours {
    /// Bricks from the shadowed corners to the wall under the bulb.
    brick: Ramp,
    /// A second brick tone, so the courses are not one flat colour.
    brick_alt: Ramp,
    mortar: Ramp,
    damp: Rgb,
    ceiling: Rgb,
    ceiling_lip: Rgb,
    floor: Ramp,
    floor_shadow: Rgb,
    box_front: Rgb,
    box_side: Rgb,
    box_top: Rgb,
    tape: Rgb,
    cord: Rgb,
    /// The pipe from its shaded underside to its highlight.
    pipe: [Rgb; 4],
    crack: Rgb,
    rust: Rgb,
    valve: (Rgb, Rgb, Rgb),
    bulb: Rgb,
    socket: Rgb,
    /// The pail from its shaded side to its highlight.
    pail: [Rgb; 4],
    inside: Rgb,
    water: Rgb,
    water_light: Rgb,
    /// The floor where the puddle wets it, still imagery.
    wet: Rgb,
}

impl Colours {
    fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let brick_hue = mix(theme.red(), theme.yellow(), 0.35);
        let brick_hue = mix(brick_hue, theme.muted, 0.35);
        let warm = mix(theme.yellow(), theme.foreground, 0.4);
        // a dark theme lights the wall up out of the dark; a light theme's basement is a pale
        // wall that darkens into the corners, so the bulb's pool reads as light in both
        let (brick_dark, brick_lit, mortar_dark, mortar_lit, floor_dark, floor_lit) = if theme.dark
        {
            (
                shades.tint(brick_hue, 0.22, 1.35),
                shades.tint(mix(brick_hue, warm, 0.3), 0.75, 2.5),
                shades.tint(theme.muted, 0.12, 1.2),
                shades.tint(mix(theme.muted, warm, 0.4), 0.42, 1.85),
                shades.tint(theme.muted, 0.18, 1.3),
                shades.tint(mix(theme.muted, warm, 0.3), 0.55, 2.2),
            )
        } else {
            (
                shades.tint(brick_hue, 0.62, 2.5),
                shades.tint(brick_hue, 0.3, 1.5),
                shades.tint(theme.muted, 0.4, 1.9),
                shades.tint(theme.muted, 0.1, 1.12),
                shades.tint(theme.muted, 0.5, 2.2),
                shades.tint(theme.muted, 0.16, 1.25),
            )
        };
        let brick_alt_dark = mix(brick_dark, mortar_dark, 0.3);
        let brick_alt_lit = mix(brick_lit, mortar_lit, 0.3);
        let metal = mix(theme.muted, theme.info, 0.3);
        let pipe = |lit: f32| tone(theme, metal, lit);
        let pail_hue = mix(theme.muted, theme.foreground, 0.2);
        let pail = |lit: f32| tone(theme, pail_hue, lit);
        let water_hue = mix(theme.info, theme.blue(), 0.4);
        let water = tone(theme, water_hue, 0.7);
        let box_hue = mix(theme.yellow(), theme.muted, 0.45);
        let (box_front, box_side, box_top) = if theme.dark {
            (
                shades.tint(box_hue, 0.55, 2.2),
                shades.tint(box_hue, 0.38, 1.75),
                shades.tint(mix(box_hue, warm, 0.4), 0.7, 2.6),
            )
        } else {
            (
                shades.tint(box_hue, 0.45, 2.1),
                shades.tint(box_hue, 0.62, 2.6),
                shades.tint(box_hue, 0.25, 1.5),
            )
        };
        let ceiling = if theme.dark {
            shades.tint(theme.muted, 0.08, 1.12)
        } else {
            shades.tint(theme.muted, 0.6, 2.5)
        };
        Self {
            brick: Ramp::new(brick_dark, brick_lit, 7),
            brick_alt: Ramp::new(brick_alt_dark, brick_alt_lit, 7),
            mortar: Ramp::new(mortar_dark, mortar_lit, 7),
            damp: if theme.dark {
                mix(brick_dark, shades.tint(theme.blue(), 0.2, 1.3), 0.4)
            } else {
                shades.tint(mix(brick_hue, theme.blue(), 0.3), 0.8, 2.7)
            },
            ceiling,
            ceiling_lip: if theme.dark {
                shades.tint(theme.muted, 0.3, 1.6)
            } else {
                shades.tint(theme.muted, 0.35, 1.7)
            },
            floor: Ramp::new(floor_dark, floor_lit, 6),
            floor_shadow: if theme.dark {
                shades.tint(theme.muted, 0.04, 1.06)
            } else {
                shades.tint(theme.muted, 0.7, 2.8)
            },
            box_front,
            box_side,
            box_top,
            tape: mix(box_front, box_side, 0.5),
            cord: if theme.dark {
                shades.tint(theme.muted, 0.5, 2.0)
            } else {
                shades.tint(theme.muted, 0.75, 2.8)
            },
            pipe: [pipe(0.1), pipe(0.4), pipe(0.75), pipe(1.0)],
            crack: if theme.dark {
                theme.background
            } else {
                shades.darkest()
            },
            rust: error_colour(theme),
            valve: (
                tone(theme, theme.red(), 0.1),
                tone(theme, theme.red(), 0.45),
                tone(theme, theme.red(), 0.85),
            ),
            bulb: tone(theme, theme.yellow(), 1.0),
            socket: pipe(0.1),
            pail: [pail(0.15), pail(0.45), pail(0.75), pail(1.0)],
            inside: if theme.dark {
                shades.tint(theme.muted, 0.15, 1.25)
            } else {
                shades.tint(theme.muted, 0.72, 2.85)
            },
            water,
            water_light: tone(theme, water_hue, 1.0),
            wet: if theme.dark {
                shades.tint(mix(theme.blue(), theme.info, 0.5), 0.4, 1.9)
            } else {
                shades.tint(mix(theme.blue(), theme.info, 0.5), 0.45, 2.2)
            },
        }
    }
}

/// A solid tone of `hue` for a lit shape, from 0 for its shaded side to 1 for its highlight.
/// The lit side moves towards the theme's light end on a dark theme and towards the background
/// on a light one, never falling below the solid contrast, so the shape keeps its outline.
fn tone(theme: &ThemeData, hue: Rgb, lit: f32) -> Rgb {
    let lit = lit.clamp(0.0, 1.0);
    let target = f64::from(if theme.dark {
        3.5 + lit * 5.0
    } else {
        3.4 + (1.0 - lit) * 3.2
    });
    let background = theme.background;
    let far = if theme.dark {
        Shades::new(theme).lightest()
    } else {
        Shades::new(theme).darkest()
    };
    // the path runs from the background through the hue to the theme's far end, so a tone
    // keeps as much of its hue as its contrast allows
    let (from, to) = if theme.contrast_of(hue) >= target {
        (background, hue)
    } else {
        (hue, far)
    };
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for _ in 0..22 {
        let mid = (low + high) / 2.0;
        if mix(from, to, mid).contrast(background) >= target {
            high = mid;
        } else {
            low = mid;
        }
    }
    mix(from, to, high)
}

pub struct Leak {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// How far the bead under the crack has swollen, from 0 to 1, when it lets go.
    bead: f32,
    drips: Vec<Drip>,
    /// How full the pail is, from 0 to 1.
    level: f32,
    /// How far the puddle has spread, from 0 to 1.
    spread: f32,
    /// The ages of the rings spreading on the pail's water.
    rings: Vec<f32>,
}

impl Leak {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut leak = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            bead: 0.0,
            drips: Vec::new(),
            level: START_LEVEL,
            spread: 0.0,
            rings: Vec::new(),
        };
        leak.resize(width, height, rng);
        leak
    }

    fn wide(&self) -> i32 {
        self.width as i32
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The bottom pixel row of the ceiling slab.
    fn ceiling(&self) -> i32 {
        (self.tall() / 16).max(1)
    }

    /// The first pixel row of the floor.
    fn floor(&self) -> i32 {
        self.tall() - (self.tall() as f32 * 0.2).round().max(3.0) as i32
    }

    /// The pipe's top row and how many rows thick it is.
    fn pipe(&self) -> (i32, i32) {
        let thick = (self.tall() / 12).clamp(3, 9);
        (self.ceiling() + (self.tall() / 22).max(1), thick)
    }

    fn pipe_bottom(&self) -> i32 {
        let (top, thick) = self.pipe();
        top + thick - 1
    }

    fn leak_x(&self) -> i32 {
        (self.wide() as f32 * 0.46).round() as i32
    }

    /// The column the pipe turns down the wall at.
    fn bend(&self) -> i32 {
        let (_, thick) = self.pipe();
        let (bw, _, _) = self.pail_size();
        (self.wide() as f32 * 0.8)
            .round()
            .max((self.leak_x() + bw / 2 + 3) as f32) as i32
            - thick / 2
    }

    /// The pail's width at the rim, its height and the half height of its open top.
    fn pail_size(&self) -> (i32, i32, i32) {
        let bw = (self.wide() / 7).clamp(9, 31) | 1;
        let bh = ((self.tall() as f32 * 0.27).round() as i32).clamp(6, 28);
        let ry = (bw / 5).clamp(2, 6);
        (bw, bh, ry)
    }

    /// The pail's centre column, the row of the middle of its rim and the row of its base.
    fn pail_at(&self) -> (i32, i32, i32) {
        let (_, bh, _) = self.pail_size();
        let depth = self.tall() - self.floor();
        let base = self.floor() + (depth as f32 * 0.45).round() as i32;
        (self.leak_x(), base - bh + 1, base)
    }

    /// How far below the rim the water's surface shows, in pixels, from the open top's full
    /// height when empty to nothing when full.
    fn sink(&self) -> f32 {
        let (_, _, ry) = self.pail_size();
        (1.0 - self.level) * (2 * ry) as f32
    }

    /// The row a drip disappears at: the water where it shows, otherwise the near rim.
    fn landing(&self) -> f32 {
        let (_, rim, _) = self.pail_at();
        let (_, _, ry) = self.pail_size();
        (rim as f32 + self.sink() - ry as f32 * 0.4).min((rim + ry - 1) as f32)
    }

    fn drip_period(&self) -> f32 {
        DRIP_PERIOD / self.motion.pace()
    }

    /// The bulb's centre and the reach of its light, if there is room for it.
    fn bulb(&self) -> Option<(f32, f32, f32)> {
        let x = (self.wide() as f32 * 0.2).round();
        let y = (self.pipe_bottom() + (self.tall() / 7).max(3)) as f32;
        let fits = self.width >= 30 && self.height >= 10 && y + 4.0 < self.floor() as f32;
        fits.then_some((x, y, self.tall().max(self.wide() / 3) as f32 * 0.75))
    }

    /// How brightly the bulb lights a point, from 0 in the corners to 1 beside it.
    fn light(&self, x: i32, y: i32) -> f32 {
        let Some((bx, by, reach)) = self.bulb() else {
            // with no bulb the room is lit evenly from above
            return 0.55 - 0.3 * (y as f32 / self.tall().max(1) as f32);
        };
        let (dx, dy) = (x as f32 - bx, (y as f32 - by) * 1.15);
        let d = (dx * dx + dy * dy).sqrt() / reach;
        (1.0 - d).clamp(0.0, 1.0).powf(1.4) * 0.92 + 0.08
    }

    fn paint_wall(&self, canvas: &mut Canvas, c: &Colours) {
        let wide = self.wide();
        let floor = self.floor();
        let course = (self.tall() / 17).clamp(2, 5);
        let length = course * 2 + course / 2 + 2;
        for y in self.ceiling()..floor {
            let row = (y - self.ceiling()) / course;
            let in_row = (y - self.ceiling()) % course;
            let shift = if row % 2 == 0 { 0 } else { length / 2 };
            for x in 0..wide {
                let joint = (x + shift) % length == 0;
                let colour = if in_row == course - 1 || joint {
                    c.mortar.at(self.light(x, y))
                } else {
                    // each brick takes the light at its middle, so the pool of light falls
                    // brick by brick instead of in stairs across them; a few bricks are a
                    // shade apart, so the wall is built of bricks and not paint
                    let brick = (x + shift).div_euclid(length);
                    let mid = brick * length - shift + length / 2;
                    let lit = self.light(mid, y - in_row + course / 2);
                    if unit(brick, row, 31) < 0.3 {
                        c.brick_alt.at(lit)
                    } else {
                        c.brick.at(lit)
                    }
                };
                canvas.set(x, y, colour);
            }
        }
        // the ceiling slab with its lit front edge
        canvas.fill_rect(0, 0, wide, self.ceiling(), c.ceiling);
        canvas.fill_rect(0, self.ceiling() - 1, wide, 1, c.ceiling_lip);
    }

    fn paint_damp(&self, canvas: &mut Canvas, mixer: &mut Blender, c: &Colours) {
        let top = self.pipe_bottom() + 1;
        let full = (self.floor() - top) as f32 * 0.8;
        let grown = (0.35 + 0.65 * (self.t as f32 / DAMP_SECONDS)).min(1.0);
        let lx = self.leak_x() as f32;
        let half = (self.wide() as f32 * 0.035).max(2.5);
        for x in (lx - half * 1.6).floor() as i32..=(lx + half * 1.6).ceil() as i32 {
            let across = ((x as f32 - lx) / (half * 1.6)).abs();
            if across >= 1.0 {
                continue;
            }
            // each column of the stain runs its own way down the bricks, longest under the
            // crack, so it reads as water that has trickled rather than a painted patch
            let run = full
                * grown
                * (1.0 - across * across)
                * (0.55 + 0.45 * paint::noise(x as f32 * 0.45, 0.0, 77));
            for y in top..top + run as i32 {
                let v = (y - top) as f32 / run.max(1.0);
                let amount = (1.0 - across) * (1.0 - v * v) * 0.7;
                if amount > 0.12 {
                    mixer.blend(canvas, x, y, c.damp, amount);
                }
            }
        }
    }

    fn paint_floor(&self, canvas: &mut Canvas, c: &Colours) {
        let floor = self.floor();
        let tall = self.tall();
        let span = (tall - floor).max(1) as f32;
        for y in floor..tall {
            let v = (y - floor) as f32 / span;
            for x in 0..self.wide() {
                // the floor catches the bulb's light, a little more towards the front
                let lit = self.light(x, floor - 2) * (0.8 + 0.2 * v);
                // soft patches in the concrete, fixed to the floor
                let lit = lit * (0.82 + 0.3 * paint::noise(x as f32 / 9.0, y as f32 / 2.5, 5));
                canvas.set(x, y, c.floor.at(lit));
            }
        }
        // the shadow along the foot of the wall
        canvas.fill_rect(0, floor, self.wide(), 1, c.floor_shadow);
    }

    fn paint_boxes(&self, canvas: &mut Canvas, c: &Colours) {
        if self.width < 56 || self.height < 14 {
            return;
        }
        let floor = self.floor();
        let unit_w = (self.wide() / 9).clamp(9, 30);
        let unit_h = (self.tall() / 6).clamp(6, 20);
        let foot = floor + 2;
        let x0 = (self.wide() / 28).max(2);
        // two boxes side by side on the floor and a smaller one stacked on them
        let boxes = [
            (x0, foot - unit_h, unit_w, unit_h),
            (
                x0 + unit_w - 1,
                foot - unit_h + 2,
                unit_w * 4 / 5,
                unit_h - 2,
            ),
            (
                x0 + unit_w / 3,
                foot - unit_h - unit_h * 3 / 4 + 1,
                unit_w * 3 / 4,
                unit_h * 3 / 4,
            ),
        ];
        for &(x, y, w, h) in &boxes {
            let side = (w / 5).max(2);
            let lid = (h / 6).max(1);
            canvas.fill_rect(x, y, w - side, h, c.box_front);
            canvas.fill_rect(x + w - side, y, side, h, c.box_side);
            canvas.fill_rect(x, y, w, lid, c.box_top);
            // the tape along the lid and down the front
            let mid = x + (w - side) / 2;
            canvas.fill_rect(mid, y, 2, h / 3 + lid, c.tape);
        }
        // a soft shadow where the boxes meet the floor
        let right = x0 + unit_w + unit_w * 4 / 5;
        canvas.fill_rect(x0 - 1, foot, right - x0 + 2, 1, c.floor_shadow);
    }

    fn paint_bulb(&self, canvas: &mut Canvas, c: &Colours) {
        let Some((bx, by, _)) = self.bulb() else {
            return;
        };
        let (x, y) = (bx as i32, by as i32);
        let size = (self.tall() / 34).clamp(1, 3);
        // the flex hangs behind the pipe, which is painted over it
        canvas.fill_rect(x, self.ceiling(), 1, y - self.ceiling(), c.cord);
        canvas.fill_rect(
            x - size / 2 - 1,
            y - size,
            size + 2 + size % 2,
            size + 1,
            c.socket,
        );
        paint::disc(
            canvas,
            x as f32,
            (y + size + 1) as f32,
            size as f32 + 0.6,
            c.bulb,
        );
        canvas.set(x, y + 2 * size + 2, c.bulb);
    }

    fn paint_pipe(&self, canvas: &mut Canvas, c: &Colours) {
        let (top, thick) = self.pipe();
        let bend = self.bend();
        let floor = self.floor();
        let shade = |f: f32| -> Rgb {
            if f < 0.18 {
                c.pipe[2]
            } else if f < 0.4 {
                c.pipe[3]
            } else if f < 0.72 {
                c.pipe[1]
            } else {
                c.pipe[0]
            }
        };
        let across = |i: i32| (i as f32 + 0.5) / thick as f32;
        // the run under the ceiling, lit from the bulb below and in front
        for i in 0..thick {
            canvas.fill_rect(0, top + i, bend + thick, 1, shade(across(i)));
        }
        // the drop down the wall, lit from the left
        for i in 0..thick {
            canvas.fill_rect(bend + i, top, 1, floor - top + 1, shade(across(i)));
        }
        // brackets holding the run to the ceiling
        let step = (self.wide() / 5).max(14);
        let mut x = step / 2;
        while x < bend - 3 {
            if (x - self.leak_x()).abs() > 4 {
                canvas.fill_rect(x, self.ceiling(), 1, top - self.ceiling(), c.pipe[0]);
                canvas.fill_rect(x - 1, top - 1, 3, 1, c.pipe[0]);
            }
            x += step;
        }
        // joints: a collar a pixel proud of the pipe, with a lit edge
        let mut x = step;
        while x < bend - 4 {
            if (x - self.leak_x()).abs() > 5 {
                canvas.fill_rect(x, top - 1, 3, thick + 2, c.pipe[1]);
                canvas.fill_rect(x, top - 1, 1, thick + 2, c.pipe[2]);
                canvas.fill_rect(x + 2, top - 1, 1, thick + 2, c.pipe[0]);
            }
            x += step;
        }
        // the elbow, a heavier fitting where the pipe turns down
        canvas.fill_rect(bend - 1, top - 1, thick + 2, thick + 2, c.pipe[1]);
        canvas.fill_rect(bend - 1, top - 1, thick + 2, 1, c.pipe[2]);
        canvas.fill_rect(bend - 1, top - 1, 1, thick + 2, c.pipe[2]);
        canvas.fill_rect(bend - 1, top + thick, thick + 2, 1, c.pipe[0]);
        // the foot of the drop into the floor
        canvas.fill_rect(bend - 1, floor - 1, thick + 2, 2, c.pipe[1]);
        canvas.fill_rect(bend - 1, floor - 1, thick + 2, 1, c.pipe[2]);
    }

    fn paint_valve(&self, canvas: &mut Canvas, c: &Colours) {
        let (_, thick) = self.pipe();
        let bend = self.bend();
        let top = self.pipe_bottom() + 2;
        let room = self.floor() - top;
        if room < 8 {
            return;
        }
        let cx = (bend + thick / 2) as f32;
        let cy = (top + room * 2 / 5) as f32;
        let r = (thick as f32 * 0.85).max(2.5);
        let (dark, body, light) = c.valve;
        // the valve body round the pipe, then the hand wheel on its front
        canvas.fill_rect(bend - 1, cy as i32 - 1, thick + 2, 3, c.pipe[0]);
        for y in (cy - r - 1.0) as i32..=(cy + r + 1.0) as i32 {
            for x in (cx - r - 1.0) as i32..=(cx + r + 1.0) as i32 {
                let (dx, dy) = (x as f32 - cx, y as f32 - cy);
                let d = (dx * dx + dy * dy).sqrt();
                let ring = d <= r + 0.4 && d >= r - 0.9;
                let spoke = d < r && (dx == 0.0 || dy == 0.0);
                if ring || spoke {
                    // lit from the upper left like the rest of the room
                    let colour = if dx + dy < -r * 0.6 {
                        light
                    } else if dx + dy > r * 0.6 {
                        dark
                    } else {
                        body
                    };
                    canvas.set(x, y, colour);
                }
            }
        }
        canvas.set(cx.round() as i32, cy.round() as i32, light);
    }

    fn paint_crack(&self, canvas: &mut Canvas, c: &Colours) {
        let (top, thick) = self.pipe();
        let lx = self.leak_x();
        // rust bleeding out round the split, then the split itself, jagged and dark
        let jag = [0, 1, 1, 0, -1, -1, 0, 1, 0];
        for i in 0..thick {
            let x = lx + jag[i as usize % jag.len()];
            canvas.set(x - 1, top + i, c.rust);
            canvas.set(x + 1, top + i, c.rust);
        }
        canvas.set(lx + 2, top + thick / 2, c.rust);
        canvas.set(lx - 2, top + thick - 1, c.rust);
        for i in 1..thick {
            canvas.set(lx + jag[i as usize % jag.len()], top + i, c.crack);
        }
        canvas.set(lx, top + thick, c.rust);
    }

    fn paint_drips(&self, canvas: &mut Canvas, c: &Colours) {
        let lx = self.leak_x();
        let under = self.pipe_bottom() + 1;
        // the bead swelling under the crack, pear shaped just before it lets go
        if self.bead > 0.25 {
            canvas.set(lx, under, c.water);
        }
        if self.bead > 0.65 {
            canvas.set(lx, under + 1, c.water_light);
            if self.tall() >= 40 {
                canvas.set(lx - 1, under + 1, c.water);
                canvas.set(lx + 1, under + 1, c.water);
                canvas.set(lx, under + 2, c.water);
            }
        }
        for drip in &self.drips {
            let y = drip.y.floor() as i32;
            canvas.set(lx, y, c.water);
            canvas.set(lx, y - 1, c.water_light);
            if drip.vy > 24.0 {
                canvas.set(lx, y - 2, c.water_light);
            }
        }
    }

    fn paint_pail(&self, canvas: &mut Canvas, c: &Colours) {
        let (bw, _, ry) = self.pail_size();
        let (cx, rim, base) = self.pail_at();
        let half_top = bw as f32 / 2.0;
        let half_base = half_top - (bw as f32 / 9.0).max(1.0);
        let cxf = cx as f32 + 0.5;
        // a soft shadow on the floor under it
        ellipse(
            canvas,
            cxf,
            base as f32 + 0.5,
            half_top + 1.0,
            (ry as f32 * 0.6).max(1.2),
            c.floor_shadow,
        );
        // the body, a cylinder lit from the left and tapering to its base
        for y in rim..=base {
            let v = (y - rim) as f32 / (base - rim).max(1) as f32;
            let half = half_top + (half_base - half_top) * v;
            // the base is rounded by the same ellipse as the rim, seen from a little above
            let bottom_curve = if y > base - ry {
                let k = (y - (base - ry)) as f32 / ry as f32;
                half * (1.0 - k * k * 0.55).max(0.0).sqrt()
            } else {
                half
            };
            for x in (cxf - bottom_curve).floor() as i32..(cxf + bottom_curve).ceil() as i32 {
                let u = (x as f32 + 0.5 - (cxf - half)) / (2.0 * half);
                let colour = if u < 0.12 {
                    c.pail[2]
                } else if u < 0.3 {
                    c.pail[3]
                } else if u < 0.62 {
                    c.pail[1]
                } else {
                    c.pail[0]
                };
                canvas.set(x, y, colour);
            }
        }
        // two pressed bands round the body
        for band in [rim + (base - rim) / 3 + ry / 2, rim + (base - rim) * 2 / 3] {
            let v = (band - rim) as f32 / (base - rim).max(1) as f32;
            let half = half_top + (half_base - half_top) * v;
            for x in (cxf - half).floor() as i32..(cxf + half).ceil() as i32 {
                canvas.set(x, band, c.pail[0]);
                if x < cx {
                    canvas.set(x, band + 1, c.pail[3]);
                }
            }
        }

        // the open top: the dark inside, the water showing more of itself as it rises, then
        // the rim round it
        let rimf = rim as f32 + 0.5;
        let open = |x: i32, y: i32, rx: f32, ry: f32, cy: f32| {
            let dx = (x as f32 + 0.5 - cxf) / rx;
            let dy = (y as f32 + 0.5 - cy) / ry;
            dx * dx + dy * dy <= 1.0
        };
        let (rx_in, ry_in) = (half_top - 1.0, ry as f32 - 0.6);
        let surface = rimf + self.sink();
        for y in rim - ry - 1..=rim + ry + 1 {
            for x in cx - bw / 2 - 1..=cx + bw / 2 + 1 {
                if !open(x, y, rx_in, ry_in, rimf) {
                    continue;
                }
                let colour = if open(x, y, rx_in, ry_in, surface) {
                    // the water: a lit band at the back, deeper in front
                    let back = (y as f32 + 0.5 - (surface - ry_in)) / (2.0 * ry_in);
                    if back < 0.3 { c.water_light } else { c.water }
                } else {
                    c.inside
                };
                canvas.set(x, y, colour);
            }
        }
        // rings spreading from where each drip landed
        for &age in &self.rings {
            let k = age / RING_LIFE;
            let (rx, ryr) = (1.0 + k * rx_in * 0.8, 0.5 + k * ry_in * 0.7);
            for step in 0..24 {
                let a = step as f32 / 24.0 * std::f32::consts::TAU;
                let x = (cxf + a.cos() * rx).floor() as i32;
                let y = (surface + a.sin() * ryr).floor() as i32;
                if open(x, y, rx_in, ry_in, rimf) && open(x, y, rx_in, ry_in, surface) {
                    canvas.set(x, y, c.water_light);
                }
            }
        }
        for step in 0..(bw * 4) {
            let a = step as f32 / (bw * 4) as f32 * std::f32::consts::TAU;
            let x = (cxf + a.cos() * (half_top - 0.3)).floor() as i32;
            let y = (rimf + a.sin() * (ry as f32 - 0.2)).floor() as i32;
            // the near lip catches the light, the far lip is in shadow
            let colour = if a.sin() > 0.2 { c.pail[3] } else { c.pail[1] };
            canvas.set(x, y, colour);
        }
        // the wire handle resting back over the far rim, with its two ears
        let lift = (bw as f32 * 0.32).max(2.0);
        let steps = bw * 3;
        for k in 0..=steps {
            let s = k as f32 / steps as f32;
            let a = s * std::f32::consts::PI;
            let x = cxf - half_top + 0.5 + s * (2.0 * half_top - 1.0);
            let y = rimf - 1.0 - a.sin() * lift;
            canvas.set(x.floor() as i32, y.floor() as i32, c.pail[0]);
        }
        canvas.fill_rect(cx - bw / 2, rim, 1, 2, c.pail[0]);
        canvas.fill_rect(cx + bw / 2, rim, 1, 2, c.pail[0]);

        // a brimming pail runs over its lip and down its sides
        if self.level >= 1.0 {
            let run = (self.t as f32 * 3.0 * self.motion.speed()) as i32;
            // water sheets over the near lip, then runs down in two thin streams
            for x in cx - bw / 3..=cx + bw / 3 {
                canvas.set(x, rim + ry - 1, c.water);
            }
            for (x, phase) in [(cx - bw / 4, 0), (cx + bw / 5, 3)] {
                for y in rim + ry..=base {
                    let glint = (y - run + phase).rem_euclid(5) == 0;
                    canvas.set(x, y, if glint { c.water_light } else { c.water });
                }
            }
        }
    }

    fn paint_puddle(&self, canvas: &mut Canvas, c: &Colours) {
        if self.spread <= 0.0 {
            return;
        }
        let (bw, _, _) = self.pail_size();
        let (cx, _, base) = self.pail_at();
        let floor = self.floor();
        let widest = self.wide() as f32 * 0.3;
        let rx = bw as f32 / 2.0 + 1.0 + (widest - bw as f32 / 2.0) * paint::ease(self.spread);
        // the floor is seen from a little above, so the puddle is a flat oval, kept off the wall
        let cy = base as f32 + 0.5;
        let room = (cy - floor as f32 - 0.5)
            .min(self.tall() as f32 - cy)
            .max(1.0);
        let ryp = (rx / 6.0).clamp(1.0, room);
        let cxf = cx as f32 + 0.5 - rx * 0.08;
        for y in (cy - ryp).floor() as i32..=(cy + ryp).ceil() as i32 {
            if y <= floor {
                continue;
            }
            for x in (cxf - rx).floor() as i32..=(cxf + rx).ceil() as i32 {
                let dx = (x as f32 + 0.5 - cxf) / rx;
                let dy = (y as f32 + 0.5 - cy) / ryp;
                // a lobed edge, so it reads as spilt water rather than a drawn oval
                let lobe = 1.0 + 0.14 * (dx * 8.0 + 1.3).sin() * dx.abs();
                let d = dx * dx + dy * dy;
                if d > lobe {
                    continue;
                }
                // a wet sheen with a solid rim plus the bulb caught as a streak on the far side
                let rim = d > lobe * 0.8;
                let streak = dy < -0.1 && dy > -0.75 && dx > -0.7 && dx < -0.15;
                let colour = if rim {
                    c.water
                } else if streak {
                    c.water_light
                } else {
                    c.wet
                };
                canvas.set(x, y, colour);
            }
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        let mut mixer = Blender::new(6);
        self.paint_wall(canvas, c);
        self.paint_damp(canvas, &mut mixer, c);
        self.paint_floor(canvas, c);
        self.paint_boxes(canvas, c);
        self.paint_bulb(canvas, c);
        self.paint_pipe(canvas, c);
        self.paint_valve(canvas, c);
        self.paint_crack(canvas, c);
        self.paint_puddle(canvas, c);
        self.paint_pail(canvas, c);
        self.paint_drips(canvas, c);
    }
}

impl Scene for Leak {
    fn name(&self) -> &'static str {
        "leak"
    }

    fn resize(&mut self, width: usize, height: usize, _rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        // drips in flight are dropped, the pail and the puddle are kept
        self.drips.clear();
        self.rings.clear();
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
        self.bead += dt / self.drip_period();
        if self.bead >= 1.0 {
            self.bead = 0.0;
            if self.drips.len() < 4 {
                self.drips.push(Drip {
                    y: (self.pipe_bottom() + 3) as f32,
                    vy: 2.0,
                });
            }
        }
        let pace = self.motion.pace();
        for age in &mut self.rings {
            *age += dt * pace;
        }
        self.rings.retain(|&age| age < RING_LIFE);
        let landing = self.landing();
        let gravity = 50.0 * self.motion.speed();
        let mut landed = 0;
        for drip in &mut self.drips {
            drip.vy += gravity * dt;
            drip.y += drip.vy * dt;
            if drip.y >= landing {
                landed += 1;
            }
        }
        self.drips.retain(|d| d.y < landing);
        for _ in 0..landed {
            if self.rings.len() < 3 {
                self.rings.push(0.0);
            }
            if self.level < 1.0 {
                self.level = (self.level + 1.0 / DRIPS_TO_FILL).min(1.0);
            } else {
                self.spread = (self.spread + 1.0 / DRIPS_TO_SPREAD).min(1.0);
            }
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
    use crate::pixels::IMAGERY_CEILING;
    use crate::scenes::testing::{apart, check_pixel_scene, pixels};
    use crate::scenes::testing::{check_determinism, check_scene, render};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("leak");
        check_determinism("leak");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("leak", 40.0);
    }

    #[test]
    fn drips_are_slow_and_never_quicker_on_lively() {
        let mut rng = Rng::new(1);
        for motion in Motion::ALL {
            let leak = Leak::new(80, 24, motion, &mut rng);
            assert!(leak.drip_period() >= DRIP_PERIOD, "{motion:?}");
        }
    }

    #[test]
    fn the_pail_fills_then_the_puddle_spreads_and_holds() {
        let theme = theme::default_theme();
        let c = Colours::new(theme);
        let mut rng = Rng::new(4);
        let mut leak = Leak::new(80, 24, Motion::FULL, &mut rng);
        for _ in 0..(10 * 20) {
            leak.step(0.05, &mut rng);
        }
        assert!(
            leak.level > START_LEVEL && leak.level < 1.0,
            "{}",
            leak.level
        );
        assert_eq!(leak.spread, 0.0);
        for _ in 0..(10 * 60 * 20) {
            leak.step(0.05, &mut rng);
        }
        assert_eq!(leak.level, 1.0);
        assert_eq!(leak.spread, 1.0);
        let frame = render(&mut leak, 80, 24, 1);
        let all = pixels(&frame);
        for (what, colour) in [
            ("crack", c.crack),
            ("rust", c.rust),
            ("water", c.water),
            ("puddle", c.wet),
            ("valve", c.valve.1),
            ("bulb", c.bulb),
        ] {
            assert!(all.contains(&colour), "no {what}");
        }
    }

    #[test]
    fn a_resize_keeps_the_water() {
        let mut rng = Rng::new(4);
        let mut leak = Leak::new(80, 24, Motion::FULL, &mut rng);
        for _ in 0..(20 * 20) {
            leak.step(0.05, &mut rng);
        }
        let level = leak.level;
        leak.resize(20, 8, &mut rng);
        assert_eq!(leak.level, level);
    }

    #[test]
    fn the_layers_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            apart(
                theme,
                c.brick.at(0.5),
                c.mortar.at(0.5),
                0.03,
                "bricks and mortar",
            );
            apart(
                theme,
                c.floor_shadow,
                c.floor.at(0.5),
                0.03,
                "the foot of the wall and the floor",
            );
            apart(
                theme,
                c.pipe[1],
                c.brick.at(1.0),
                0.08,
                "the pipe and the wall",
            );
            apart(theme, c.rust, c.pipe[1], 0.08, "the rust and the pipe");
            apart(theme, c.water, c.inside, 0.08, "the water and the pail");
            apart(
                theme,
                c.water,
                c.floor.at(0.5),
                0.08,
                "the puddle and the floor",
            );
            for colour in c
                .brick
                .steps()
                .iter()
                .chain(c.mortar.steps())
                .chain(c.floor.steps())
                .chain([&c.box_front, &c.box_top, &c.inside, &c.damp, &c.wet])
            {
                assert!(
                    theme.contrast_of(*colour) < IMAGERY_CEILING,
                    "{}",
                    theme.name
                );
            }
            for colour in [c.pipe[1], c.pail[1], c.rust, c.water, c.bulb, c.valve.1] {
                assert!(
                    theme.contrast_of(colour) >= paint::SOLID - 0.01,
                    "{}",
                    theme.name
                );
            }
        }
    }
}
