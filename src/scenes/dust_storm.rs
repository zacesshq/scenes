//! A dust storm in solid pixels: open desert under a low sun, flat topped mesas far off, dunes in
//! two ranges and a line of telegraph poles running away to the right, with a wire fence along
//! the near ground. A towering wall of dust rolls in from the right, its billowing front lit by
//! the sun. It swallows the far poles, the mesas, the dunes and finally the sun one by one while
//! sand streams past. Once the wall has passed the dust holds, thick and drifting, with the near
//! pole and the fence still standing clear in front of it.
//!
//! The sky, the land, the far poles and the dust are tints of the background held below the
//! non-text floor. The sun, the near pole and the fence keep the theme's own drawing colours, so
//! the near things stay readable however thick the dust and keep their outline when colour is
//! off.

use super::paint::{Blender, billow, disc, noise, tint, unit};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "A towering wall of dust rolling over the desert, swallowing the far poles, the mesas, the dunes and the sun, by day in light themes";
pub const CAPTIONS: &[&str] = &[
    "Visibility dropping. Find the first error.",
    "Something blew in. The build is failing.",
    "Hard to see from here. Read the log.",
];

/// How long the wall of dust takes to cross the screen at the standard pace.
pub const STORM_SECONDS: f32 = 60.0;
/// Where the wall starts on the storm's clock, so its front is already in sight at the start.
const STORM_START: f32 = 9.0;
/// How thick the dust settles once the wall has passed, from 0 to 1.
const SETTLED: f32 = 0.78;
/// How fast the wall's billows churn, in noise units a second at the standard pace. The churn
/// is slow, so the face of the wall boils gently rather than flickering.
const CHURN: f32 = 0.05;
/// Steps of dust over the scene; small steps keep a region from changing brightness at once.
const DUST_LEVELS: u8 = 8;

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct Colours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub glow: Rgb,
    pub sun: Rgb,
    pub mesa: (Rgb, Rgb),
    pub dunes: [(Rgb, Rgb); 2],
    pub ground: Rgb,
    pub far_pole: Rgb,
    pub wall_lit: Rgb,
    pub wall_deep: Rgb,
    pub grain: Rgb,
    pub post: Rgb,
    pub wire: Rgb,
    sky: Ramp,
    wall: Ramp,
}

impl Colours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let dark = theme.dark;
        let sand = mix(theme.yellow(), theme.warning, 0.3);
        let rust = mix(theme.warning, theme.red(), 0.35);
        // a dark theme's sky is a deep dusk warming to the horizon; a light one's is a pale
        // hot blue fading to cream
        let (sky_top, sky_low) = if dark {
            (
                tint(theme, theme.blue(), 0.15, 1.28),
                tint(theme, theme.warning, 0.3, 1.75),
            )
        } else {
            (
                tint(theme, theme.info, 0.32, 1.65),
                tint(theme, theme.warning, 0.1, 1.15),
            )
        };
        // the far land is paler and nearer the sky, the near land stronger
        let (mesa, dunes, ground) = if dark {
            (
                (tint(theme, rust, 0.22, 1.45), tint(theme, rust, 0.14, 1.25)),
                [
                    (tint(theme, sand, 0.34, 1.85), tint(theme, sand, 0.2, 1.4)),
                    (tint(theme, sand, 0.5, 2.3), tint(theme, sand, 0.3, 1.7)),
                ],
                tint(theme, mix(sand, theme.foreground, 0.3), 0.24, 1.55),
            )
        } else {
            (
                (tint(theme, rust, 0.24, 1.4), tint(theme, rust, 0.36, 1.7)),
                [
                    (tint(theme, sand, 0.3, 1.5), tint(theme, sand, 0.48, 1.95)),
                    (tint(theme, sand, 0.42, 1.85), tint(theme, sand, 0.66, 2.45)),
                ],
                tint(theme, sand, 0.16, 1.22),
            )
        };
        let far_pole = if dark {
            tint(theme, theme.foreground, 0.5, 2.3)
        } else {
            tint(theme, theme.muted, 0.7, 2.5)
        };
        // the wall is lit on its face by the low sun: lighter than the sky on a dark theme and
        // a deep ochre with paler rims on a light one
        let dust = mix(theme.warning, theme.yellow(), 0.4);
        let (wall_lit, wall_deep) = if dark {
            (
                tint(theme, mix(dust, theme.foreground, 0.3), 0.7, 2.85),
                tint(theme, mix(dust, theme.red(), 0.2), 0.3, 1.65),
            )
        } else {
            (
                tint(theme, dust, 0.3, 1.45),
                tint(theme, mix(dust, theme.red(), 0.25), 0.8, 2.8),
            )
        };
        Self {
            sky_top,
            sky_low,
            glow: if dark {
                tint(theme, theme.bright_yellow(), 0.4, 2.2)
            } else {
                tint(theme, theme.yellow(), 0.08, 1.1)
            },
            // a light theme's bright yellow is a dark ochre, so its sun is a pale disc instead
            sun: if dark {
                theme.bright_yellow()
            } else {
                tint(theme, theme.bright_yellow(), 0.35, 1.6)
            },
            mesa,
            dunes,
            ground,
            far_pole,
            wall_lit,
            wall_deep,
            grain: if dark {
                tint(theme, theme.bright_yellow(), 0.75, 2.9)
            } else {
                tint(theme, theme.warning, 0.9, 2.9)
            },
            post: theme.muted,
            wire: theme.muted,
            sky: Ramp::new(sky_top, sky_low, 9),
            wall: Ramp::new(wall_deep, wall_lit, 6),
        }
    }

    #[cfg(test)]
    pub(crate) fn palette(theme: &ThemeData) -> crate::scenes::testing::Palette {
        let c = Self::new(theme);
        let mut imagery = vec![
            ("sky top", c.sky_top),
            ("sky low", c.sky_low),
            ("glow", c.glow),
            ("far mesa", c.mesa.0),
            ("mesa shade", c.mesa.1),
            ("ground", c.ground),
            ("far pole", c.far_pole),
            ("lit dust", c.wall_lit),
            ("deep dust", c.wall_deep),
            ("grain", c.grain),
        ];
        for (i, &(lit, shade)) in c.dunes.iter().enumerate() {
            imagery.push((["far dune", "near dune"][i], lit));
            imagery.push((["far shade", "near shade"][i], shade));
        }
        let mut shapes = vec![("fence post", c.post)];
        if theme.dark {
            shapes.push(("sun", c.sun));
        } else {
            imagery.push(("sun", c.sun));
        }
        crate::scenes::testing::Palette { imagery, shapes }
    }
}

/// One round billow of the wall of dust, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Billow {
    x: f32,
    y: f32,
    r: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct Grain {
    x: f32,
    y: f32,
    speed: f32,
}

pub struct DustStorm {
    width: usize,
    height: usize,
    motion: Motion,
    /// The storm's own clock, run at the motion speed.
    storm_t: f32,
    grains: Vec<Grain>,
}

impl DustStorm {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut storm = Self {
            width: 1,
            height: 1,
            motion,
            storm_t: STORM_START,
            grains: Vec::new(),
        };
        storm.resize(width, height, rng);
        storm
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    fn horizon(&self) -> i32 {
        (self.tall() as f32 * 0.56) as i32
    }

    /// How far the wall has come, from 0 at the right edge to 1 once it has passed.
    fn progress(&self) -> f32 {
        (self.storm_t / STORM_SECONDS).clamp(0.0, 1.0)
    }

    /// The column of the foot of the wall's leading face.
    fn front(&self) -> f32 {
        let w = self.width as f32;
        // it travels until the last billow along its top has left on the left
        let travel = w + 8.0 + self.extent() + self.wall_height() * 0.3;
        w + 4.0 - travel * self.progress()
    }

    fn wall_height(&self) -> f32 {
        self.tall() as f32 * 0.92
    }

    /// How far back from the face the billows along the top run.
    fn extent(&self) -> f32 {
        (self.width as f32 * 0.5).max(self.wall_height() * 0.9)
    }

    /// How far the wall's face leans back at a share of its height, in pixels.
    fn lean(&self, up: f32) -> f32 {
        self.wall_height() * 0.3 * up.clamp(0.0, 1.0).powf(1.6)
    }

    /// The billows of the wall: round puffs stacked up its leading face and along its top, each
    /// breathing slowly. They are listed back to front, so the last one holding a pixel is the
    /// one seen there.
    fn billows(&self) -> Vec<Billow> {
        let tall = self.tall() as f32;
        let height = self.wall_height();
        let front = self.front();
        let breath = self.storm_t * 0.3;
        let size = |i: i32, salt: u32| 1.0 + 0.07 * (breath + unit(i, 0, salt) * 6.0).sin();
        let mut out = Vec::new();
        // along the top, behind the face, out past the right edge
        let spacing = (height * 0.2).max(3.0);
        let mut rel = self.lean(1.0) + spacing * 0.4;
        let mut i = 0;
        let extent = self.extent();
        while front + rel - spacing < self.width as f32 + spacing && rel < extent {
            let r = spacing * (0.7 + 0.45 * unit(i, 1, 36)) * size(i, 37);
            out.push(Billow {
                x: front + rel,
                y: tall - height * (0.8 + 0.1 * unit(i, 2, 36)) + r * 0.3,
                r,
            });
            rel += spacing * (0.8 + 0.4 * unit(i, 3, 36));
            i += 1;
        }
        // up the face, the lowest in front
        let count = 6;
        for k in (0..count).rev() {
            let up = (k as f32 + 0.5) / count as f32;
            let r = height / count as f32 * (0.85 + 0.35 * unit(k, 4, 36)) * size(k + 40, 38);
            out.push(Billow {
                x: front + self.lean(up) + r * 0.25 * (unit(k, 5, 36) - 0.3),
                y: tall - up * height * 0.95,
                r,
            });
        }
        out
    }

    /// The wall at a pixel: how opaque it is there (from 0 for clear air to 1) and how lit,
    /// from 0 for its deep base to 1 for its sunlit rim.
    #[cfg(test)]
    fn wall(&self, x: i32, y: i32) -> (f32, f32) {
        self.wall_at(&self.billows(), x, y)
    }

    fn wall_at(&self, billows: &[Billow], x: i32, y: i32) -> (f32, f32) {
        let (xf, yf) = (x as f32 + 0.5, y as f32 + 0.5);
        let tall = self.tall() as f32;
        let height = self.wall_height();
        let front = self.front();
        let settled = SETTLED * self.progress().powi(3);
        let low = yf / tall;
        // the sun is low on the left, so each billow is lit on its upper left
        let (lx, ly) = (-0.75_f32, -0.66_f32);
        for b in billows.iter().rev() {
            let (dx, dy) = ((xf - b.x) / b.r, (yf - b.y) / b.r);
            let d2 = dx * dx + dy * dy;
            if d2 <= 1.0 {
                let facing = dx * lx + dy * ly;
                let lit = (0.45 + 0.55 * facing - 0.35 * low + 0.15 * d2).clamp(0.0, 1.0);
                return (1.0, lit);
            }
        }
        let up = (tall - yf) / height;
        // well behind the billows the dust towers up out of sight
        let rise = ((xf - front - self.extent() * 0.85) / (self.extent() * 0.4)).clamp(0.0, 1.0);
        let top = (tall - height * 0.8) * (1.0 - rise) - 4.0 * rise;
        let inside = xf >= front + self.lean(up) && yf >= top;
        if inside {
            // the body behind the billows is in their shade, thinning a little far back
            let rel = xf - front;
            let behind = (rel - 30.0).max(0.0) / 60.0;
            let opacity = (1.0 - behind).max(SETTLED);
            let texture = billow(rel * 0.07, yf * 0.1 + self.storm_t * CHURN, 35);
            let lit = (0.22 + 0.12 * texture - 0.18 * low).clamp(0.0, 1.0);
            return (opacity.max(settled), lit);
        }
        // a low haze of kicked up sand runs ahead of the foot of the wall
        let ahead = (front - xf).max(0.0);
        let ground = ((low - 0.55) / 0.45).clamp(0.0, 1.0);
        let haze = 0.4 * ground * (-ahead / 14.0).exp();
        (haze.max(settled), 0.55)
    }

    /// The top pixel row of a dune range at a column: a long gentle rise to each crest and a
    /// short steep fall on the lee side, as wind shaped dunes have.
    fn dune_top(&self, range: usize, x: i32) -> f32 {
        let tall = self.tall() as f32;
        let horizon = self.horizon() as f32;
        let ground = tall - horizon;
        let (base, height, length, salt) = [(0.24, 0.2, 30.0, 41), (0.62, 0.26, 44.0, 42)][range];
        let xf = x as f32 + range as f32 * 19.0;
        let u = (xf / length).rem_euclid(1.0);
        let shape = if u < 0.72 {
            super::paint::ease(u / 0.72)
        } else {
            1.0 - super::paint::ease((u - 0.72) / 0.28)
        };
        let swell = 0.55 + 0.45 * noise(xf / length * 0.7, 0.5, salt);
        horizon + ground * base - ground * height * shape * swell
    }

    /// The top of the mesas on the horizon, the horizon itself between them.
    fn mesa_top(&self, x: i32) -> f32 {
        let horizon = self.horizon() as f32;
        let w = self.width as f32;
        let height = (self.tall() as f32 * 0.16).max(2.0);
        let mut top = horizon;
        for (centre, half, scale) in [(0.34, 0.1, 1.0), (0.72, 0.05, 0.75), (0.08, 0.04, 0.55)] {
            let d = (x as f32 - w * centre).abs() - w * half;
            // steep cliffs that spread into a talus slope at the foot
            let fall = d.max(0.0) / (height * scale);
            let rise = if fall < 0.25 {
                1.0 - fall * 0.8
            } else {
                (0.8 - (fall - 0.25) * 1.1).max(0.0)
            };
            top = top.min(horizon - height * scale * rise);
        }
        top
    }

    /// The telegraph poles from the nearest to the furthest: foot column, foot row and height.
    fn poles(&self) -> Vec<(f32, f32, f32)> {
        let w = self.width as f32;
        let tall = self.tall() as f32;
        let horizon = self.horizon() as f32;
        let (near_x, far_x) = (w * 0.14, w * 0.97);
        let (near_foot, far_foot) = (tall - 5.0, horizon + 1.0);
        let near_h = (tall * 0.6).min(near_foot - 2.0);
        let mut out = Vec::new();
        let mut s = 0.0_f32;
        let mut step = 0.38_f32;
        while s < 0.98 && out.len() < 9 {
            let h = near_h * (1.0 - s).powf(1.15);
            if h < 3.0 {
                break;
            }
            out.push((
                near_x + (far_x - near_x) * s,
                near_foot + (far_foot - near_foot) * s,
                h,
            ));
            s += step * (1.0 - s);
            step *= 0.95;
        }
        out
    }

    fn paint_land(&self, canvas: &mut Canvas, c: &Colours, depth: &mut [f32]) {
        let (wide, tall) = (canvas.width() as i32, canvas.height() as i32);
        let horizon = self.horizon();
        let mut mark = |canvas: &mut Canvas, x: i32, y: i32, colour: Rgb, d: f32| {
            if x >= 0 && x < wide && y >= 0 && y < tall {
                canvas.set(x, y, colour);
                depth[(y * wide + x) as usize] = d;
            }
        };
        for y in 0..tall {
            canvas.fill_row(y, c.sky.at(y as f32 / horizon.max(1) as f32));
        }
        // the sun low on the left, in a soft glow
        if wide >= 24 && tall >= 16 {
            let (sx, sy) = (wide as f32 * 0.2, horizon as f32 * 0.36);
            let r = (tall as f32 / 13.0).clamp(1.5, 6.0);
            disc(canvas, sx, sy, r + 1.5, c.glow);
            disc(canvas, sx, sy, r, c.sun);
        }
        // the mesas, their sunlit left faces lighter than their shaded right ones
        for x in 0..wide {
            let top = self.mesa_top(x).round() as i32;
            let lit = self.mesa_top(x - 1) >= self.mesa_top(x) - 0.01;
            for y in top.max(0)..horizon + 1 {
                mark(canvas, x, y, if lit { c.mesa.0 } else { c.mesa.1 }, 0.97);
            }
        }
        // a flat plain runs out to the horizon behind the dunes
        let ground = (tall - horizon).max(1) as f32;
        for y in horizon.max(0)..tall {
            for x in 0..wide {
                mark(canvas, x, y, c.ground, 0.92);
            }
        }
        // two ranges of dunes, each running to the bottom so the near one covers the far
        for range in 0..2 {
            let (lit, shade) = c.dunes[range];
            let d = [0.85, 0.55][range];
            let band = ground * [0.2, 0.28][range];
            for x in 0..wide {
                let top = self.dune_top(range, x);
                // the face towards the sun on the left is lit and the lee side is in shade, down
                // to where the dune flattens out
                // the steeper the lee face the deeper its shade, so the shade tapers smoothly
                let fall = (top - self.dune_top(range, x - 1)).max(0.0);
                let depth_of_shade = band * ((fall - 0.3) * 2.0).clamp(0.0, 1.0);
                for y in (top.round() as i32).max(0)..tall {
                    let colour = if (y as f32) < top + depth_of_shade {
                        shade
                    } else {
                        lit
                    };
                    mark(canvas, x, y, colour, d - 0.25 * (y as f32 - top) / ground);
                }
            }
            if range == 0 {
                self.paint_far_poles(&mut mark, canvas, c);
            }
        }
    }

    fn paint_far_poles(
        &self,
        mark: &mut impl FnMut(&mut Canvas, i32, i32, Rgb, f32),
        canvas: &mut Canvas,
        c: &Colours,
    ) {
        // the far poles, their wires sagging between the cross arms
        let poles = self.poles();
        for (i, &(px, foot, h)) in poles.iter().enumerate().skip(1) {
            let d = 0.6 + 0.35 * i as f32 / poles.len() as f32;
            let (x, foot) = (px.round() as i32, foot.round() as i32);
            let top = foot - h.round() as i32;
            for y in top..=foot {
                mark(canvas, x, y, c.far_pole, d);
            }
            let arm = (h / 6.0).round().max(1.0) as i32;
            for dx in -arm..=arm {
                mark(canvas, x + dx, top + 1, c.far_pole, d);
            }
        }
        for pair in poles.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let (ax, ay) = (a.0, a.1 - a.2 + 1.0);
            let (bx, by) = (b.0, b.1 - b.2 + 1.0);
            let span = (bx - ax).max(1.0);
            for wx in ax.round() as i32 + 1..bx.round() as i32 {
                let t = (wx as f32 - ax) / span;
                let sag = 4.0 * t * (1.0 - t) * (a.2 / 10.0).max(0.5);
                let wy = (ay + (by - ay) * t + sag).round() as i32;
                mark(canvas, wx, wy, c.far_pole, 0.7);
            }
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        let mut blender = Blender::new(DUST_LEVELS);
        let (wide, tall) = (canvas.width() as i32, canvas.height() as i32);
        // how far each pixel is, from 1 for the sky to under half for the near sand, so the dust
        // hides far things first
        let mut depth = vec![1.0_f32; (wide * tall).max(0) as usize];
        self.paint_land(canvas, c, &mut depth);

        let billows = self.billows();
        for y in 0..tall {
            for x in 0..wide {
                let (opacity, lit) = self.wall_at(&billows, x, y);
                if opacity <= 0.02 {
                    continue;
                }
                let d = depth[(y * wide + x) as usize];
                let hidden = 1.0 - (1.0 - opacity).powf(d * 1.8);
                let dust = c.wall.at(lit);
                if hidden >= 0.94 {
                    canvas.set(x, y, dust);
                } else {
                    blender.blend(canvas, x, y, dust, hidden);
                }
            }
        }
        for grain in &self.grains {
            let (x, y) = (grain.x.round() as i32, grain.y.round() as i32);
            // sand streams only inside the dust, never through the clear air ahead of it
            if self.wall_at(&billows, x, y).0 < 0.5 {
                continue;
            }
            blender.blend(canvas, x, y, c.grain, 0.6);
            blender.blend(canvas, x + 1, y, c.grain, 0.35);
        }

        // the near pole and the fence stand clear in front of the dust
        if let Some(&(px, foot, h)) = self.poles().first() {
            if tall >= 16 && wide >= 30 {
                let (x, foot) = (px.round() as i32, foot.round() as i32);
                let top = foot - h.round() as i32;
                canvas.fill_rect(x, top, 2, foot - top + 1, c.post);
                let arm = (h / 5.0).round().max(2.0) as i32;
                canvas.fill_rect(x - arm, top + 2, 2 * arm + 2, 1, c.post);
                for dx in [-arm, arm + 1] {
                    canvas.set(x + dx, top + 1, c.post);
                }
            }
        }
        if tall >= 10 {
            let ground = tall - 2;
            let spacing = if wide >= 80 { 9 } else { 7 };
            let post_h = if tall >= 30 { 5 } else { 3 };
            for x in 0..wide {
                if x % spacing == 4 {
                    for y in ground - post_h..=ground {
                        canvas.set(x, y, c.post);
                    }
                } else {
                    canvas.set(x, ground - post_h + 1, c.wire);
                    if post_h >= 5 {
                        canvas.set(x, ground - 2, c.wire);
                    }
                }
            }
        }
    }
}

impl Scene for DustStorm {
    fn name(&self) -> &'static str {
        "dust-storm"
    }

    fn resize(&mut self, width: usize, height: usize, _rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.grains.clear();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        let moved = dt * self.motion.speed();
        self.storm_t += moved;
        // sand streams past inside the dust, more of it as the storm builds
        let cap = self
            .motion
            .scaled((self.width * self.height / 80).clamp(2, 40));
        let wanted = (cap as f32 * (0.25 + 0.75 * self.progress())) as usize;
        if self.grains.len() < wanted && rng.chance(0.4) {
            let tall = self.tall() as f32;
            let start = self.front().clamp(0.0, self.width as f32) + rng.range(4.0, 30.0);
            self.grains.push(Grain {
                x: start.min(self.width as f32 + 2.0),
                y: rng.range(tall * 0.4, tall - 4.0).max(0.0),
                speed: rng.range(9.0, 15.0),
            });
        }
        for grain in &mut self.grains {
            grain.x -= grain.speed * moved;
        }
        self.grains.retain(|g| g.x > -4.0);
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
        check_scene("dust-storm");
        check_determinism("dust-storm");
    }

    #[test]
    fn passes_the_pixel_checks() {
        check_pixel_scene("dust-storm", 30.0);
        check_palette(Colours::palette);
    }

    #[test]
    fn the_bands_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            assert!(distance(c.sky_top, c.sky_low) >= 0.03, "{}", theme.name);
            for (lit, shade) in c.dunes {
                assert!(
                    distance(lit, shade) >= 0.03,
                    "a dune's sides in {}: {} and {}",
                    theme.name,
                    lit.hex(),
                    shade.hex()
                );
            }
            assert!(distance(c.wall_lit, c.wall_deep) >= 0.05, "{}", theme.name);
            assert!(distance(c.wall_lit, c.sky_low) >= 0.03, "{}", theme.name);
            assert!(distance(c.mesa.0, c.sky_low) >= 0.03, "{}", theme.name);
        }
    }

    #[test]
    fn the_wall_swallows_the_far_poles_first_and_then_holds() {
        let mut rng = Rng::new(2);
        let mut storm = DustStorm::new(100, 30, Motion::FULL, &mut rng);
        let mean = |s: &DustStorm| {
            let mut sum = 0.0;
            for y in 0..60 {
                for x in 0..100 {
                    sum += s.wall(x, y).0;
                }
            }
            sum / (100.0 * 60.0)
        };
        let start = mean(&storm);
        // the wall's front is in sight on the right from the start
        assert!(
            storm.front() < 100.0 && storm.front() > 50.0,
            "{}",
            storm.front()
        );
        let mut last = start;
        let mut means = Vec::new();
        for i in 0..(90 * 20) {
            storm.step(0.05, &mut rng);
            if i % 20 == 0 {
                let now = mean(&storm);
                // a second never thickens the whole screen by more than a little
                assert!((now - last).abs() < 0.05, "{last} then {now}");
                last = now;
                means.push(now);
            }
        }
        assert!(last > start + 0.3, "{start} then {last}");
        assert_eq!(storm.progress(), 1.0);
        let settled = &means[80..];
        let (low, high) = settled
            .iter()
            .fold((f32::MAX, f32::MIN), |(l, h), &m| (l.min(m), h.max(m)));
        assert!(high - low < 0.06, "{low} to {high}");
        assert!((low - SETTLED).abs() < 0.15, "{low}");
        // the poles run away to the right, each smaller than the last
        let poles = storm.poles();
        assert!(poles.len() >= 4);
        assert!(poles.windows(2).all(|p| p[1].0 > p[0].0 && p[1].2 < p[0].2));
    }

    #[test]
    fn calm_storms_roll_in_at_half_the_rate() {
        let mut rng = Rng::new(2);
        let mut full = DustStorm::new(80, 24, Motion::FULL, &mut rng);
        let mut calm = DustStorm::new(80, 24, Motion::CALM, &mut rng);
        for _ in 0..(20 * 20) {
            full.step(0.05, &mut rng);
            calm.step(0.05, &mut rng);
        }
        let gained = |s: &DustStorm| s.storm_t - STORM_START;
        assert!((gained(&calm) * 2.0 - gained(&full)).abs() < 0.01);
    }
}
