//! Volcano: a tall cone at dusk, its flanks cut by ridges and gullies and lit from the left,
//! with lower ranges fading behind it. The crater glows and a plume of ash billows up from it,
//! warmed underneath by the fire and drifting away on the wind at its top. Lava breaks over the
//! rim in three tongues that creep down the gullies, cooling to a crust at their edges, and
//! spread into glowing pools on the plain at the foot, where they hold.
//!
//! The sky, the far ranges, the cone, the ash and the plain are tints of the background held
//! below the non-text floor. The crater's fire and the lava are solid, so with colour off the
//! lava still shows creeping down the mountain.

use std::f32::consts::TAU;

use super::paint::{self, Blender, Wave, billow, noise};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "A volcano at dusk with a glowing crater and a billowing plume of ash, three lava flows creeping down its gullies to pool at the foot";
pub const CAPTIONS: &[&str] = &[
    "Something erupted. Start at the crater.",
    "The build blew its top.",
    "Hot errors creeping down. Read the first one.",
];

/// How long one lava flow takes to creep from the crater to the foot at the standard pace.
pub const LAVA_SECONDS: f32 = 70.0;
/// When each flow breaks out, in seconds of the eruption's clock, so they follow one another.
const FLOW_DELAYS: [f32; 3] = [0.0, 18.0, 34.0];
/// Where each flow runs, as a share of the half width of the cone, left negative.
const FLOW_PLACES: [f32; 3] = [-0.42, 0.18, 0.58];
/// How long a pool at the foot takes to spread to its full width once a flow arrives.
const POOL_SECONDS: f32 = 30.0;
/// Seconds of the eruption already passed when the scene starts, so it reads as wrong at once.
const HEAD_START: f32 = 28.0;
/// Pixels a second the ash rises at the standard pace.
pub const ASH_RISE: f32 = 1.6;
/// Pixels a second the glints of molten rock run down a flow at the standard pace.
pub const RUN_SPEED: f32 = 1.4;

/// A solid tone of `hue` for a lit shape, from 0 for its shaded side to 1 for its highlight.
/// The lit side moves towards the theme's light end on a dark theme and towards the background
/// on a light one, never falling below the solid contrast, so the shape keeps its outline.
fn tone(theme: &ThemeData, hue: Rgb, lit: f32) -> Rgb {
    let lit = lit.clamp(0.0, 1.0);
    let target = f64::from(if theme.dark {
        3.5 + lit * 5.0
    } else {
        3.4 + (1.0 - lit) * 3.6
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

/// `colour` pushed away from the background, if it needs it, until it keeps the theme's text
/// floor, for the parts of a shape that carry the scene's meaning.
fn at_floor(theme: &ThemeData, colour: Rgb) -> Rgb {
    let far = if theme.dark {
        Shades::new(theme).lightest()
    } else {
        Shades::new(theme).darkest()
    };
    (0..=40)
        .map(|i| mix(colour, far, i as f32 / 40.0))
        .find(|&c| theme.contrast_of(c) >= theme.floor())
        .unwrap_or(far)
}

/// The volcano's colours in one theme, every one derived from the theme's own.
pub(crate) struct Colours {
    sky: Ramp,
    far: Rgb,
    near: Rgb,
    /// The cone from its lit faces to its shaded ones.
    rock: Ramp,
    plain: Ramp,
    /// The ash from its thin edge to its thick lit body.
    ash: Ramp,
    /// Warm light thrown up by the fire, mixed into the ash and the ground near it.
    warm: Rgb,
    crust: Rgb,
    lava: Rgb,
    hot: Rgb,
    fire: Rgb,
}

impl Colours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let ember = mix(theme.red(), theme.yellow(), 0.35);
        let rock = mix(theme.muted, theme.red(), 0.12);
        // a dark theme's dusk is a deep sky reddened low down by the fire, with the cone lit
        // out of it; a light theme's is a pale hazy evening with the cone and the ash darker
        // than the sky, so both read as the same place
        let (sky_top, sky_low, far, near, lit, shaded, plain_far, plain_near, ash_thin, ash_thick) =
            if theme.dark {
                (
                    shades.tint(theme.blue(), 0.12, 1.18),
                    shades.tint(ember, 0.6, 1.95),
                    shades.tint(mix(theme.muted, theme.blue(), 0.4), 0.3, 1.4),
                    shades.tint(mix(theme.muted, theme.blue(), 0.2), 0.16, 1.2),
                    shades.tint(mix(rock, theme.foreground, 0.2), 0.75, 2.75),
                    shades.tint(rock, 0.3, 1.5),
                    shades.tint(rock, 0.24, 1.38),
                    shades.tint(rock, 0.12, 1.15),
                    shades.tint(theme.muted, 0.25, 1.4),
                    shades.tint(mix(theme.muted, theme.foreground, 0.3), 0.75, 2.75),
                )
            } else {
                (
                    shades.tint(theme.blue(), 0.2, 1.3),
                    shades.tint(ember, 0.22, 1.25),
                    shades.tint(mix(theme.muted, theme.blue(), 0.3), 0.4, 1.5),
                    shades.tint(theme.muted, 0.5, 1.85),
                    shades.tint(rock, 0.38, 1.7),
                    shades.tint(rock, 0.82, 2.85),
                    shades.tint(rock, 0.4, 1.75),
                    shades.tint(rock, 0.62, 2.4),
                    shades.tint(theme.muted, 0.3, 1.45),
                    shades.tint(theme.muted, 0.72, 2.6),
                )
            };
        Self {
            sky: Ramp::new(sky_top, sky_low, 9),
            far,
            near,
            rock: Ramp::new(lit, shaded, 7),
            plain: Ramp::new(plain_far, plain_near, 5),
            ash: Ramp::new(ash_thin, ash_thick, 7),
            warm: if theme.dark {
                shades.tint(ember, 0.8, 2.6)
            } else {
                shades.tint(ember, 0.5, 1.9)
            },
            // on a light theme the molten rock already sits at the text floor, so the crust is
            // a darker brown beyond it rather than a lighter one
            crust: if theme.dark {
                tone(theme, mix(theme.red(), theme.muted, 0.15), 0.0)
            } else {
                mix(
                    at_floor(
                        theme,
                        tone(theme, mix(theme.red(), theme.yellow(), 0.2), 0.55),
                    ),
                    shades.darkest(),
                    0.5,
                )
            },
            // on a dark theme the molten rock takes the theme's own red and orange, which already
            // keep the floor, leaning to orange, rather than a tone mixed towards the text colour,
            // which washed it out to pink
            lava: if theme.dark {
                at_floor(theme, mix(theme.red(), theme.yellow(), 0.4))
            } else {
                at_floor(theme, tone(theme, mix(theme.red(), ember, 0.4), 0.55))
            },
            hot: if theme.dark {
                at_floor(theme, mix(theme.yellow(), theme.red(), 0.12))
            } else {
                at_floor(
                    theme,
                    tone(theme, mix(theme.red(), theme.yellow(), 0.55), 0.9),
                )
            },
            fire: tone(theme, mix(theme.yellow(), theme.red(), 0.3), 1.0),
        }
    }

    #[cfg(test)]
    pub(crate) fn palette(theme: &ThemeData) -> crate::scenes::testing::Palette {
        let c = Self::new(theme);
        crate::scenes::testing::Palette {
            imagery: vec![
                ("sky top", c.sky.at(0.0)),
                ("sky low", c.sky.at(1.0)),
                ("far range", c.far),
                ("lit rock", c.rock.at(0.0)),
                ("shaded rock", c.rock.at(1.0)),
                ("plain", c.plain.at(0.5)),
                ("thick ash", c.ash.at(1.0)),
                ("glow", c.warm),
            ],
            shapes: vec![("lava", c.lava), ("hot lava", c.hot)],
        }
    }
}

pub struct Volcano {
    width: usize,
    height: usize,
    motion: Motion,
    /// The eruption's own clock, run at the motion speed, so reduced motion creeps at half rate.
    lava_t: f32,
    far: Wave,
    near: Wave,
    phases: [f32; 3],
    seed: u32,
}

impl Volcano {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut volcano = Self {
            width: 1,
            height: 1,
            motion,
            lava_t: HEAD_START,
            far: Wave::new(rng, 30.0),
            near: Wave::new(rng, 18.0),
            phases: [0.0; 3],
            seed: rng.next_u64() as u32,
        };
        volcano.resize(width, height, rng);
        volcano
    }

    fn wide(&self) -> i32 {
        self.width as i32
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The pixel row of the foot of the volcano and the far edge of the plain.
    fn base(&self) -> i32 {
        ((self.tall() as f32 * 0.8).round() as i32).clamp(1, self.tall())
    }

    fn peak(&self) -> i32 {
        ((self.tall() as f32 * 0.28).round() as i32).clamp(0, self.base() - 1)
    }

    fn centre(&self) -> f32 {
        self.width as f32 * 0.46
    }

    fn crater_half(&self) -> f32 {
        (self.width as f32 / 13.0).max(2.0)
    }

    fn foot_half(&self) -> f32 {
        (self.width as f32 * 0.47).min((self.base() - self.peak()) as f32 * 3.4)
    }

    /// Half the width of the cone at a pixel row. The flanks are steep under the crater and
    /// spread out towards the foot, the way a real cone settles.
    fn half_at(&self, y: i32) -> f32 {
        let span = (self.base() - self.peak()).max(1) as f32;
        let along = ((y - self.peak()) as f32 / span).clamp(0.0, 1.0);
        let ch = self.crater_half();
        ch + (self.foot_half() - ch) * along.powf(1.75)
    }

    /// The column a flow runs down at a pixel row, following its gully with a small wander.
    fn flow_x(&self, i: usize, y: i32) -> f32 {
        let place = FLOW_PLACES[i];
        let ch = self.crater_half();
        let half = self.half_at(y);
        let run = ch * place * 0.6 + place * (half - ch);
        self.centre() + run + 0.9 * (y as f32 * 0.3 + self.phases[i]).sin()
    }

    /// How far down its flank a flow has crept, from 0 at the crater to 1 at the foot.
    fn progress(&self, i: usize) -> f32 {
        ((self.lava_t - FLOW_DELAYS[i]) / LAVA_SECONDS).clamp(0.0, 1.0)
    }

    /// How far a flow's pool has spread at the foot, from 0 to 1.
    fn pool(&self, i: usize) -> f32 {
        let arrived = FLOW_DELAYS[i] + LAVA_SECONDS;
        ((self.lava_t - arrived) / POOL_SECONDS).clamp(0.0, 1.0)
    }

    fn paint_sky(&self, canvas: &mut Canvas, c: &Colours) {
        let base = self.base();
        for y in 0..base {
            canvas.fill_row(y, c.sky.at(y as f32 / base.max(1) as f32));
        }
        // two lower ranges behind the cone, the further one paler
        let (w, peak) = (self.wide(), self.peak());
        let span = (base - peak) as f32;
        for x in 0..w {
            let far = base as f32 - span * (0.42 + 0.14 * self.far.at(x as f32));
            let near = base as f32 - span * (0.2 + 0.1 * self.near.at(x as f32 + 40.0));
            for y in far.round() as i32..base {
                canvas.set(x, y, if (y as f32) < near { c.far } else { c.near });
            }
        }
    }

    fn paint_ash(&self, canvas: &mut Canvas, blender: &mut Blender, c: &Colours) {
        let peak = self.peak();
        if peak < 2 {
            return;
        }
        let cx = self.centre();
        let ch = self.crater_half();
        let drift = self.wide() as f32 * 0.18;
        let scroll = self.lava_t * ASH_RISE;
        let scale = (self.tall() as f32 / 10.0).max(3.0);
        for y in 0..peak + 1 {
            let rise = (peak - y) as f32;
            let up = rise / peak as f32;
            // the column widens as it rises and leans away on the wind higher up
            let centre = cx + drift * up.powf(1.8);
            let half = ch * 1.2 + rise * 0.75 + up * up * drift * 0.5;
            for x in (centre - half - 2.0).floor() as i32..=(centre + half + 2.0).ceil() as i32 {
                let across = ((x as f32 - centre) / half).abs();
                if across >= 1.0 {
                    continue;
                }
                let n = billow(
                    (x as f32 - centre) / scale,
                    (y as f32 + scroll) / (scale * 0.8),
                    self.seed,
                );
                // the billows thin out towards the column's edges, so it has no hard outline
                let body = (1.0 - across).powf(0.8) * (1.0 - up * 0.45);
                let density = ((n - 0.62 + body * 0.9) * 2.2).clamp(0.0, 1.0);
                if density < 0.12 {
                    continue;
                }
                // billows lit from the upper left and warmed from below by the fire
                let lit =
                    (density * 0.8 + (0.5 - (x as f32 - centre) / half) * 0.25).clamp(0.0, 1.0);
                blender.blend(canvas, x, y, c.ash.at(lit), density.min(0.95));
                let near_fire = (1.0 - rise / (peak as f32 * 0.5)).clamp(0.0, 1.0)
                    * (1.0 - across).clamp(0.0, 1.0);
                if near_fire > 0.05 {
                    blender.blend(canvas, x, y, c.warm, near_fire * density * 0.8);
                }
            }
        }
    }

    fn paint_cone(&self, canvas: &mut Canvas, c: &Colours) {
        let (peak, base) = (self.peak(), self.base());
        let cx = self.centre();
        let ch = self.crater_half();
        let span = (base - peak).max(1) as f32;
        for y in peak..base {
            let half = self.half_at(y) + 1.1 * (noise(y as f32 * 0.35, 1.0, self.seed) - 0.5);
            let along = (y - peak) as f32 / span;
            for x in (cx - half).floor() as i32..=(cx + half).ceil() as i32 {
                let u = (x as f32 + 0.5 - cx) / half;
                if u.abs() > 1.0 {
                    continue;
                }
                // ridges and gullies fan out from the crater, so they are spaced by the share of
                // the width rather than by pixels; each ridge has a lit left face
                let wander = noise(u * 3.0, along * 2.0, self.seed ^ 5) * 1.6;
                let ridge = (u * 11.0 + wander).sin();
                let face = (u * 11.0 + wander).cos();
                let shade = 0.42 + 0.34 * u + 0.3 * face - 0.08 * ridge + 0.12 * along;
                canvas.set(x, y, c.rock.at(shade));
            }
        }
        // the crater: a notch in the summit with the fire showing over its far lip
        let (top, lip) = (peak, peak + (self.tall() / 30).max(1));
        for y in top..=lip {
            let inner = ch * (0.75 - 0.25 * (y - top) as f32 / (lip - top + 1) as f32);
            for x in (cx - inner).round() as i32..=(cx + inner).round() as i32 {
                let colour = if y == top { c.fire } else { c.hot };
                canvas.set(x, y, colour);
            }
        }
        canvas.fill_rect(
            (cx - ch * 0.5).round() as i32,
            top - 1,
            (ch).round() as i32 + 1,
            1,
            c.fire,
        );
    }

    fn paint_plain(&self, canvas: &mut Canvas, c: &Colours) {
        let (base, tall) = (self.base(), self.tall());
        let span = (tall - base).max(1) as f32;
        for y in base..tall {
            let v = (y - base) as f32 / span;
            for x in 0..self.wide() {
                let patch = noise(x as f32 / 14.0, y as f32 / 3.0, self.seed ^ 9);
                canvas.set(x, y, c.plain.at(v * 0.85 + (patch - 0.5) * 0.2));
            }
        }
    }

    fn paint_lava(&self, canvas: &mut Canvas, blender: &mut Blender, c: &Colours) {
        let (peak, base) = (self.peak(), self.base());
        let span = (base - peak) as f32;
        let run = self.lava_t * RUN_SPEED;
        for i in 0..FLOW_PLACES.len() {
            let progress = self.progress(i);
            if progress <= 0.0 {
                continue;
            }
            let front = peak as f32 + 1.0 + progress * (span - 1.0);
            for y in peak + 1..=front.floor() as i32 {
                let along = (y - peak) as f32 / span;
                let x = self.flow_x(i, y);
                let width = 1.5 + along * (self.wide() as f32 / 40.0).clamp(1.0, 5.0);
                let left = (x - width / 2.0).round() as i32;
                let right = (x + width / 2.0).round() as i32;
                let tip = front - y as f32;
                for px in left..=right {
                    let edge = px == left || px == right;
                    let middle = (px as f32 - x).abs() <= width / 2.0 - 0.5 || right == left;
                    let near_front = (tip <= 1.0 && middle) || (tip <= 2.0 && !edge && middle);
                    // molten rock in the middle with glints running down it, a cooling crust
                    // at the edges while the front itself glowing hottest
                    let glint = ((y as f32 - run) * 0.45 + px as f32 * 0.3).sin() > 0.55;
                    let colour = if near_front {
                        c.fire
                    } else if edge && right - left >= 2 {
                        c.crust
                    } else if glint {
                        c.hot
                    } else {
                        c.lava
                    };
                    canvas.set(px, y, colour);
                }
                // warm light thrown on the rock either side
                for px in [left - 1, right + 1] {
                    blender.blend(canvas, px, y, c.warm, 0.45);
                }
            }
            // the pool spreading out on the plain once the flow reaches the foot
            let pool = self.pool(i);
            if progress >= 1.0 {
                let x = self.flow_x(i, base - 1);
                let room = (self.tall() - base) as f32;
                let rx = 1.5 + (self.wide() as f32 * 0.07).max(3.0) * paint::ease(pool);
                let ry = (rx * 0.28).clamp(1.0, (room * 0.45).max(1.0));
                let cy = base as f32 + ry * 0.7;
                self.paint_pool(canvas, blender, c, (x, cy), (rx, ry), run);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_pool(
        &self,
        canvas: &mut Canvas,
        blender: &mut Blender,
        c: &Colours,
        (cx, cy): (f32, f32),
        (rx, ry): (f32, f32),
        run: f32,
    ) {
        // a glow on the ground round it first, then the pool with a crusted rim
        for y in (cy - ry * 2.0).floor() as i32..=(cy + ry * 2.0).ceil() as i32 {
            for x in (cx - rx * 1.6).floor() as i32..=(cx + rx * 1.6).ceil() as i32 {
                let dx = (x as f32 + 0.5 - cx) / rx;
                let dy = (y as f32 + 0.5 - cy) / ry;
                let d = dx * dx + dy * dy;
                if d <= 1.0 {
                    let swirl = ((x as f32 * 0.5 + run * 0.3).sin()
                        + (y as f32 * 0.9 - run * 0.2).cos())
                        * 0.5;
                    let colour = if d > 0.7 {
                        c.crust
                    } else if swirl > 0.45 {
                        c.hot
                    } else {
                        c.lava
                    };
                    canvas.set(x, y, colour);
                } else if d <= 2.6 && y >= self.base() {
                    blender.blend(canvas, x, y, c.warm, 0.5 * (2.6 - d) / 1.6);
                }
            }
        }
    }

    /// Where the glowing embers lifted in the plume are, one above another up its leaning
    /// middle. They all rise at one pace, evenly spaced, so two never sit side by side like a
    /// pair of eyes.
    fn embers(&self) -> Vec<(i32, i32)> {
        let peak = self.peak();
        if peak < 8 || self.width < 40 {
            return Vec::new();
        }
        let life = peak as f32 * 0.8;
        // at least five rows between embers, so they read as sparks rising one after another
        let count = ((life / 5.0) as i32).clamp(1, 4);
        let cx = self.centre();
        let drift = self.wide() as f32 * 0.18;
        let gap = life / count as f32;
        (0..count)
            .map(|k| {
                let rise = (self.lava_t * 1.5 + k as f32 * gap).rem_euclid(life);
                let up = rise / peak as f32;
                // each follows the plume as it leans on the wind, wandering a little either way
                let x = cx + drift * up.powf(1.8) + 1.5 * (rise * 0.35 + k as f32 * 2.1).sin();
                let y = peak as f32 - 1.0 - rise;
                (x.round() as i32, y.round() as i32)
            })
            .collect()
    }

    fn paint_embers(&self, canvas: &mut Canvas, c: &Colours) {
        for (x, y) in self.embers() {
            canvas.set(x, y, c.hot);
        }
    }

    /// The fire's light on the sky over the crater, which the ash then rises through.
    fn paint_glow(&self, canvas: &mut Canvas, blender: &mut Blender, c: &Colours) {
        let (cx, peak) = (self.centre(), self.peak() as f32);
        let reach = self.crater_half() * 2.0;
        // the light rises with the heat, so the glow is taller than it is wide
        for y in (peak - reach * 1.6).floor() as i32..=peak as i32 {
            for x in (cx - reach).floor() as i32..=(cx + reach).ceil() as i32 {
                let (dx, dy) = (x as f32 + 0.5 - cx, (y as f32 + 0.5 - peak) * 0.62);
                let d = (dx * dx + dy * dy).sqrt() / reach;
                if d < 1.0 {
                    let fade = (1.0 - d) * (1.0 - d);
                    blender.blend(canvas, x, y, c.warm, fade * 0.8);
                }
            }
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        let mut blender = Blender::new(5);
        self.paint_sky(canvas, c);
        self.paint_plain(canvas, c);
        self.paint_glow(canvas, &mut blender, c);
        self.paint_cone(canvas, c);
        self.paint_ash(canvas, &mut blender, c);
        self.paint_lava(canvas, &mut blender, c);
        self.paint_embers(canvas, c);
    }
}

impl Scene for Volcano {
    fn name(&self) -> &'static str {
        "volcano"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.phases = [
            rng.range(0.0, TAU),
            rng.range(0.0, TAU),
            rng.range(0.0, TAU),
        ];
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.lava_t += dt * self.motion.speed();
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

    fn count(volcano: &Volcano, colours: &[Rgb]) -> usize {
        let mut frame = Frame::new(volcano.width, volcano.height);
        volcano.draw(&mut frame, theme::default_theme());
        frame
            .cells()
            .iter()
            .filter_map(|c| c.pixel_pair())
            .map(|(a, b)| usize::from(colours.contains(&a)) + usize::from(colours.contains(&b)))
            .sum()
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("volcano");
        check_determinism("volcano");
    }

    #[test]
    fn passes_the_pixel_checks() {
        check_pixel_scene("volcano", 20.0);
        check_palette(Colours::palette);
    }

    #[test]
    fn the_bands_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            for (a, b, what) in [
                (c.sky.at(0.0), c.sky.at(1.0), "the sky"),
                (c.rock.at(0.0), c.rock.at(1.0), "the two flanks"),
                (c.ash.at(1.0), c.sky.at(0.0), "the ash and the sky"),
                (c.far, c.sky.at(0.7), "the far range and the sky"),
                (c.rock.at(0.0), c.far, "the cone and the far range"),
                (c.lava, c.crust, "the lava and its crust"),
            ] {
                assert!(distance(a, b) >= 0.03, "{what} in {}", theme.name);
            }
        }
    }

    #[test]
    fn lava_creeps_down_slowly_then_pools_and_holds() {
        let theme = theme::default_theme();
        let c = Colours::new(theme);
        let molten = [c.lava, c.hot, c.crust, c.fire];
        let mut rng = Rng::new(4);
        let mut volcano = Volcano::new(80, 24, Motion::FULL, &mut rng);
        let start = count(&volcano, &molten);
        let mut fronts = Vec::new();
        for _ in 0..(150 * 20) {
            volcano.step(0.05, &mut rng);
            fronts.push(volcano.progress(0));
        }
        // the front creeps: never more than a pixel row's worth of the flank in a second
        let span = (volcano.base() - volcano.peak()) as f32;
        for w in fronts.windows(20) {
            assert!((w[19] - w[0]) * span <= 1.0);
        }
        let later = count(&volcano, &molten);
        assert!(later > start * 2, "{start} then {later}");
        for i in 0..FLOW_DELAYS.len() {
            assert_eq!(volcano.progress(i), 1.0);
            assert_eq!(volcano.pool(i), 1.0);
        }
    }

    #[test]
    fn embers_never_sit_side_by_side() {
        for (w, h) in [(80, 24), (120, 34), (200, 50), (300, 90)] {
            let mut rng = Rng::new(8);
            let mut volcano = Volcano::new(w, h, Motion::FULL, &mut rng);
            for _ in 0..(60 * 20) {
                volcano.step(0.05, &mut rng);
                let embers = volcano.embers();
                for (i, a) in embers.iter().enumerate() {
                    for b in &embers[i + 1..] {
                        assert!(
                            (a.1 - b.1).abs() >= 3,
                            "embers at {a:?} and {b:?} at {w}x{h}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_ash_rises_slowly() {
        for motion in Motion::ALL {
            assert!(ASH_RISE * motion.speed() < 2.5, "{motion:?}");
            assert!(RUN_SPEED * motion.speed() < 2.5, "{motion:?}");
        }
    }
}
