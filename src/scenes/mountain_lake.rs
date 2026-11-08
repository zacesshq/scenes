//! Mountain lake: first light over a still lake. The sky warms from a cool blue overhead to a
//! dawn glow on the horizon, where the sun's rim shows through a notch in the peaks and thin
//! bands of cloud catch its light. Three ranges stand behind one another, the furthest palest
//! and snow capped, the nearest a dark line of pine forest along the far shore. The lake mirrors
//! all of it, a slow ripple passing through the reflection now and then, with mist lying low on
//! the water. A grey heron stands in the shallows, now and then lowering its neck to fish, and
//! rings spread where its beak meets the water.
//!
//! The sky, the sun's light on the water, the mountains, the forest, the mist and the whole
//! reflection are tints of the background held below the non-text floor. The heron and the
//! sun's rim keep solid colours, so they stand out as shapes and keep their outline with colour
//! off.

use std::f32::consts::TAU;

use super::paint::{self, Blender, Wave, held_imagery, noise};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "Mountains and pines mirrored in a still lake at sunrise, with mist on the water, a slow ripple and a heron fishing";

pub const CAPTIONS: &[&str] = &[
    "Still water, clear reflection, clean build.",
    "Not a ripple out of place.",
    "The heron waits. Nothing to catch.",
    "Clear all the way to the peaks.",
    "A quiet lake and a passing build.",
];

/// Seconds between two ripples passing through the reflection at the standard pace.
pub(crate) const RIPPLE_PERIOD: f32 = 14.0;
/// How fast a ripple travels out across the lake, in pixels a second at the standard pace.
pub(crate) const RIPPLE_SPEED: f32 = 2.2;
/// The shortest time the heron holds a pose, in seconds, so it never bobs.
pub(crate) const HERON_HOLD: f32 = 3.0;
/// How fast a ring spreads where the heron fishes, in pixels a second at the standard pace.
pub(crate) const RING_GROWTH: f32 = 1.6;
/// How long a ring lasts, in seconds.
const RING_LIFE: f32 = 5.0;
/// How fast the bands of cloud and the mist drift, in pixels a second at the standard pace.
const DRIFT: f32 = 0.35;

/// The heron standing upright, facing left: dagger beak, head and black crest, an S-curved
/// neck, the body with its darker flight feathers at the back and two long legs.
const HERON_UP: [&str; 20] = [
    ".....hh........",
    "bbbbhhhcc......",
    ".....hh..c.....",
    "......hh.......",
    "......hh.......",
    ".......hh......",
    ".......hh......",
    "......hh.......",
    "......hwww.....",
    ".....hwwwwww...",
    ".....wwwwwwwww.",
    "......wwwwwwwdd",
    ".......wwwwwddd",
    "........wwwdd.d",
    ".........l.l...",
    ".........l.l...",
    ".........l.l...",
    ".........l.l...",
    ".........l.l...",
    "........ll.l...",
];

/// The same heron with its neck stretched forward and down, its beak at the water.
const HERON_FISHING: [&str; 20] = [
    "...............",
    "...............",
    "...............",
    "...............",
    "...............",
    "...............",
    "...............",
    "...............",
    "........www....",
    "......hwwwwww..",
    ".....hhwwwwwwww",
    "....hh.wwwwwwdd",
    "...hh...wwwwddd",
    "..hh.....wwdd.d",
    ".hhcc....l.l...",
    ".hh......l.l...",
    ".b.......l.l...",
    "b........l.l...",
    ".........l.l...",
    "........ll.l...",
];

/// A small heron for short screens, standing and fishing.
const HERON_SMALL_UP: [&str; 10] = [
    "..hc....", "bbh.....", "..h.....", "..hh....", "...hwww.", "...wwwwd", "....wwdd", ".....l.l",
    ".....l.l", "....ll.l",
];
const HERON_SMALL_FISHING: [&str; 10] = [
    "........", "........", "........", "....www.", "...hwwwd", "..h..wdd", ".hc..l.l", "b....l.l",
    ".....l.l", "....ll.l",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pose {
    Upright,
    Fishing,
}

#[derive(Clone, Debug, PartialEq)]
struct Ring {
    x: f32,
    y: f32,
    age: f32,
}

/// A thin band of cloud lit from below by the sunrise.
#[derive(Clone, Debug, PartialEq)]
struct Band {
    x: f32,
    y: f32,
    length: f32,
}

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

/// The lake's colours in one theme, every one derived from the theme's own.
pub(crate) struct LakeColours {
    sky: Ramp,
    dawn: Rgb,
    band: Rgb,
    band_shade: Rgb,
    /// Each range's lit face, shaded face and snow, furthest first.
    far: (Rgb, Rgb, Rgb),
    mid: (Rgb, Rgb, Rgb),
    forest: Rgb,
    forest_lit: Rgb,
    water: Rgb,
    mist: Rgb,
    glint: Rgb,
    sun: Rgb,
    heron: Rgb,
    neck: Rgb,
    crest: Rgb,
    beak: Rgb,
    legs: Rgb,
    background: Rgb,
}

impl LakeColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let dawn = mix(theme.magenta(), theme.yellow(), 0.55);
        let stone = mix(theme.blue(), theme.muted, 0.5);
        let haze = mix(theme.info, theme.magenta(), 0.25);
        let pine = mix(theme.green(), theme.blue(), 0.4);
        // on a dark theme the sky lightens to a warm glow on the horizon and the far ranges are
        // lit out of the dusk, the nearest darkest; on a light theme it is a pale dawn and the
        // ranges darken as they come nearer, so both read as layers of distance
        let (top, low, far_lit, far_shade, mid_lit, mid_shade, forest, water) = if theme.dark {
            (
                shades.tint(theme.blue(), 0.16, 1.22),
                shades.tint(dawn, 0.62, 2.55),
                shades.tint(haze, 0.6, 2.25),
                shades.tint(haze, 0.42, 1.8),
                shades.tint(stone, 0.42, 1.65),
                shades.tint(stone, 0.26, 1.35),
                shades.tint(pine, 0.2, 1.15),
                shades.tint(theme.blue(), 0.28, 1.35),
            )
        } else {
            (
                shades.tint(theme.blue(), 0.32, 1.55),
                shades.tint(dawn, 0.2, 1.22),
                shades.tint(haze, 0.25, 1.35),
                shades.tint(haze, 0.42, 1.65),
                shades.tint(stone, 0.45, 1.95),
                shades.tint(stone, 0.62, 2.35),
                shades.tint(pine, 0.85, 2.85),
                shades.tint(theme.blue(), 0.3, 1.5),
            )
        };
        let (snow, snow_shade) = if theme.dark {
            (
                shades.tint(theme.bright(), 0.85, 2.9),
                shades.tint(mix(theme.bright(), theme.info, 0.5), 0.6, 2.45),
            )
        } else {
            (theme.background, shades.tint(theme.info, 0.14, 1.2))
        };
        let heron_grey = mix(theme.muted, theme.info, 0.15);
        Self {
            sky: Ramp::new(top, low, 10),
            dawn: if theme.dark {
                shades.tint(mix(dawn, theme.yellow(), 0.4), 0.9, 2.9)
            } else {
                shades.tint(mix(dawn, theme.yellow(), 0.4), 0.3, 1.25)
            },
            band: if theme.dark {
                shades.tint(dawn, 0.85, 2.75)
            } else {
                shades.tint(dawn, 0.42, 1.55)
            },
            band_shade: if theme.dark {
                shades.tint(mix(dawn, theme.blue(), 0.5), 0.45, 1.75)
            } else {
                shades.tint(mix(dawn, theme.blue(), 0.5), 0.5, 1.8)
            },
            far: (far_lit, far_shade, snow),
            mid: (
                mid_lit,
                mid_shade,
                if theme.dark { snow_shade } else { snow },
            ),
            forest,
            forest_lit: if theme.dark {
                shades.tint(pine, 0.32, 1.3)
            } else {
                shades.tint(pine, 0.7, 2.5)
            },
            water,
            mist: if theme.dark {
                shades.tint(mix(haze, theme.foreground, 0.4), 0.55, 2.2)
            } else {
                theme.background
            },
            glint: if theme.dark {
                shades.tint(mix(dawn, theme.yellow(), 0.5), 0.9, 2.9)
            } else {
                theme.background
            },
            // on a light theme the low sun is the palest thing in the sky, so it stays imagery
            // rather than a dark disc; the heron alone carries the outline there
            sun: if theme.dark {
                tone(theme, mix(theme.yellow(), theme.red(), 0.2), 1.0)
            } else {
                theme.background
            },
            heron: tone(theme, heron_grey, 0.35),
            neck: tone(theme, heron_grey, 1.0),
            crest: tone(theme, heron_grey, 0.0),
            beak: tone(theme, theme.yellow(), 0.8),
            legs: tone(theme, mix(heron_grey, theme.yellow(), 0.3), 0.15),
            background: theme.background,
        }
    }
}

/// A mountain range: its height above the shore in each column, from steep peaks with a small
/// jag along the ridge.
fn range(rng: &mut Rng, width: usize, tallest: f32, slopes: (f32, f32), every: usize) -> Vec<f32> {
    let peaks: Vec<(f32, f32, f32, f32)> = (0..width / every + 2)
        .map(|_| {
            (
                rng.range(-10.0, width as f32 + 10.0),
                rng.range(0.55, 1.0) * tallest,
                rng.range(slopes.0, slopes.1),
                rng.range(slopes.0, slopes.1),
            )
        })
        .collect();
    let jag = rng.next_u64() as u32;
    (0..width)
        .map(|x| {
            let xf = x as f32;
            let base = peaks
                .iter()
                .map(|&(px, h, left, right)| {
                    let slope = if xf < px { left } else { right };
                    h - slope * (xf - px).abs()
                })
                .fold(tallest * 0.2, f32::max);
            base + (noise(xf / 3.0, 0.5, jag) - 0.5) * 1.6 * (base / tallest.max(1.0))
        })
        .collect()
}

pub struct MountainLake {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    far: Vec<f32>,
    mid: Vec<f32>,
    hills: Vec<f32>,
    /// Pines along the far shore as (column, height in pixels).
    pines: Vec<(i32, i32)>,
    bands: Vec<Band>,
    snow_line: Wave,
    sun_x: f32,
    heron_x: f32,
    pose: Pose,
    pose_left: f32,
    rings: Vec<Ring>,
    seed: u32,
}

impl MountainLake {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut lake = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            far: Vec::new(),
            mid: Vec::new(),
            hills: Vec::new(),
            pines: Vec::new(),
            bands: Vec::new(),
            snow_line: Wave::new(rng, 4.0),
            sun_x: 0.0,
            heron_x: 0.0,
            pose: Pose::Upright,
            pose_left: rng.range(5.0, 8.0),
            rings: Vec::new(),
            seed: rng.next_u64() as u32,
        };
        lake.resize(width, height, rng);
        lake
    }

    fn wide(&self) -> i32 {
        self.width as i32
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The pixel row of the far shore, where the land meets its reflection.
    fn shore(&self) -> i32 {
        let tall = self.tall();
        ((tall as f32 * 0.54).round() as i32).clamp(1, (tall - 1).max(1))
    }

    fn heron_rows(&self) -> Option<&'static [&'static str]> {
        let lake = self.tall() - self.shore();
        if self.width < 24 || lake < 8 {
            None
        } else if self.tall() >= 56 && lake >= 22 {
            Some(match self.pose {
                Pose::Upright => &HERON_UP,
                Pose::Fishing => &HERON_FISHING,
            })
        } else {
            Some(match self.pose {
                Pose::Upright => &HERON_SMALL_UP,
                Pose::Fishing => &HERON_SMALL_FISHING,
            })
        }
    }

    /// How many canvas pixels each pixel of the heron covers, so it keeps its size on a very
    /// large screen.
    fn heron_scale(&self) -> i32 {
        if self.tall() >= 150 { 2 } else { 1 }
    }

    /// The row the heron's feet stand on in the shallows.
    fn heron_feet(&self) -> i32 {
        let lake = self.tall() - self.shore();
        self.shore() + (lake as f32 * 0.8).round() as i32
    }

    /// The sun's centre and radius, sitting in a notch of the far range with its rim showing.
    fn sun(&self) -> Option<(f32, f32, f32)> {
        let x = self.sun_x.round() as usize;
        let ridge = self.far.get(x)?.max(*self.mid.get(x)?);
        let r = (self.tall() as f32 / 18.0).clamp(1.5, 7.0);
        // the rim stands a little more than half clear of the ridge in front of it
        Some((self.sun_x, self.shore() as f32 - ridge - r * 0.2, r))
    }

    fn paint_sky(&self, canvas: &mut Canvas, mixer: &mut Blender, c: &LakeColours) {
        let shore = self.shore();
        for y in 0..shore {
            canvas.fill_row(y, c.sky.at(y as f32 / shore.max(1) as f32));
        }
        // the glow of the coming sun, wide and low over the horizon
        let (sx, sy, r) = self.sun().unwrap_or((self.sun_x, shore as f32, 2.0));
        let reach = (self.wide() as f32 * 0.32).max(r * 6.0);
        for y in 0..shore {
            for x in 0..self.wide() {
                let (dx, dy) = ((x as f32 - sx) / reach, (y as f32 - sy) / (reach * 0.42));
                let d = (dx * dx + dy * dy).sqrt();
                if d < 1.0 {
                    mixer.blend(canvas, x, y, c.dawn, (1.0 - d) * (1.0 - d) * 0.9);
                }
            }
        }
        // thin bands of cloud, lit underneath by the sun and shaded on top
        for band in &self.bands {
            let len = band.length;
            let x0 = wrap_x(band.x, self.wide() as f32 + len) - len;
            for i in 0..len as i32 {
                let s = i as f32 / len;
                // tapered ends, thickest a little left of centre
                let thick = ((s * std::f32::consts::PI).sin() * 2.2).round() as i32;
                let x = (x0 + i as f32).round() as i32;
                let y = band.y.round() as i32;
                if thick >= 1 {
                    canvas.set(x, y, c.band);
                }
                if thick >= 2 {
                    canvas.set(x, y - 1, c.band_shade);
                }
            }
        }
    }

    fn paint_sun(&self, canvas: &mut Canvas, c: &LakeColours) {
        if let Some((sx, sy, r)) = self.sun() {
            paint::disc(canvas, sx, sy, r, c.sun);
        }
    }

    /// One range of mountains: faces turned towards the sun are lit, the others shaded, with
    /// snow above a ragged line when the range carries it.
    fn paint_range(
        &self,
        canvas: &mut Canvas,
        heights: &[f32],
        colours: (Rgb, Rgb, Rgb),
        snow_from: Option<f32>,
    ) {
        let shore = self.shore();
        let (lit, shade, snow) = colours;
        for (x, &h) in heights.iter().enumerate() {
            let left = heights.get(x.wrapping_sub(1)).copied().unwrap_or(h);
            let right = heights.get(x + 1).copied().unwrap_or(h);
            let slope = right - left;
            let towards = (x as f32 - self.sun_x).signum();
            let sunny = slope * towards > 0.0;
            let top = (shore as f32 - h).round() as i32;
            for y in top.max(0)..shore {
                let above = (shore - y) as f32;
                let colour = match snow_from {
                    Some(line) if above > line + 1.5 * self.snow_line.at(x as f32) => {
                        if sunny {
                            snow
                        } else {
                            mix(snow, shade, 0.45)
                        }
                    }
                    _ if sunny => lit,
                    _ => shade,
                };
                canvas.set(x as i32, y, colour);
            }
        }
    }

    fn paint_forest(&self, canvas: &mut Canvas, c: &LakeColours) {
        let shore = self.shore();
        for (x, &h) in self.hills.iter().enumerate() {
            let top = (shore as f32 - h).round() as i32;
            for y in top.max(0)..shore {
                canvas.set(x as i32, y, c.forest);
            }
        }
        // pines along the shore, each a narrow spire on the line of the hills
        for &(px, height) in &self.pines {
            let ground =
                (shore as f32 - self.hills.get(px as usize).copied().unwrap_or(0.0)).round() as i32;
            for k in 0..height {
                let y = ground - height + k;
                let half = (k + 1) / 2 - i32::from(k == height - 1 && height > 3);
                for dx in -half..=half {
                    let colour = if dx < 0 && k > 1 {
                        c.forest_lit
                    } else {
                        c.forest
                    };
                    canvas.set(px + dx, y, colour);
                }
            }
        }
    }

    /// The lake: the land above mirrored in the water, drawn a little darker and bluer, bent by
    /// a slow ripple travelling out from the shore.
    fn paint_lake(&self, canvas: &mut Canvas, mixer: &mut Blender, c: &LakeColours) {
        let shore = self.shore();
        let (w, tall) = (self.wide(), self.tall());
        let lake = (tall - shore).max(1) as f32;
        let speed = self.motion.speed();
        let travelled =
            (self.t as f32 * RIPPLE_SPEED * speed).rem_euclid(RIPPLE_PERIOD * RIPPLE_SPEED);
        let swell = self.t as f32 * 0.5 * speed;
        let source: Vec<Rgb> = (0..shore)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| canvas.get(x, y).unwrap_or(c.water))
            .collect();
        for y in shore..tall {
            let depth = (y - shore) as f32;
            // the passing ripple: a band of stronger bending moving out across the water
            let band = (-((depth - travelled) / 3.0).powi(2)).exp();
            let amp = 0.25 + depth / lake * 0.6 + band * 1.6;
            let bend = (depth * 0.8 - swell).sin() * amp;
            let src_y = (shore - 1 - depth as i32).max(0);
            // in a few steps, so the reflection keeps to a small number of colours
            let darken = 0.28 + 0.1 * (3.0 * depth / lake).floor();
            for x in 0..w {
                let sx = (x as f32 + bend).round() as i32;
                let sx = sx.clamp(0, w - 1);
                let from = source[(src_y * w + sx) as usize];
                let colour = reflected(c.background, from, c.water, darken);
                canvas.set(x, y, colour);
            }
        }
        // a pale line of light along the shore where the water meets the land
        for x in 0..w {
            mixer.blend(canvas, x, shore, c.mist, 0.5);
        }
        // mist lying low over the water, drifting slowly
        let drift = self.t as f32 * DRIFT * speed;
        for y in shore - 2..shore + (lake * 0.18) as i32 {
            let v = (y - shore + 2) as f32 / (lake * 0.18 + 2.0);
            for x in 0..w {
                let n = noise((x as f32 + drift) / 12.0, y as f32 / 3.0, self.seed);
                let amount = ((n - 0.45) * 2.0).clamp(0.0, 1.0) * (1.0 - v) * 0.55;
                if amount > 0.05 {
                    mixer.blend(canvas, x, y, c.mist, amount);
                }
            }
        }
        // the sun's light on the water, a soft column broken by the ripple
        if let Some((sx, _, r)) = self.sun() {
            for y in shore + 1..tall {
                let depth = (y - shore) as f32;
                let half = r * 0.6 + depth * 0.25;
                let wobble = (depth * 0.8 - swell).sin() * (0.4 + depth / lake * 0.6);
                for x in (sx - half).floor() as i32..=(sx + half).ceil() as i32 {
                    let across = ((x as f32 - sx - wobble) / half).abs();
                    let broken = noise(x as f32 / 2.0, depth / 1.2 + swell * 0.3, 3);
                    if across < 1.0 && broken > 0.45 {
                        let amount = (1.0 - across) * (1.0 - depth / lake * 0.6) * 0.8;
                        mixer.blend(canvas, x, y, c.glint, amount);
                    }
                }
            }
        }
        for ring in &self.rings {
            let rx = 1.0 + ring.age * RING_GROWTH * speed.max(0.35);
            let ry = (rx * 0.3).max(0.6);
            let fade = 1.0 - ring.age / RING_LIFE;
            let steps = (rx * 8.0) as i32 + 8;
            for k in 0..steps {
                let a = k as f32 / steps as f32 * TAU;
                let x = (ring.x + a.cos() * rx).round() as i32;
                let y = (ring.y + a.sin() * ry).round() as i32;
                if y > shore {
                    mixer.blend(canvas, x, y, c.glint, 0.75 * fade);
                }
            }
        }
    }

    fn paint_heron(&self, canvas: &mut Canvas, mixer: &mut Blender, c: &LakeColours) {
        let Some(rows) = self.heron_rows() else {
            return;
        };
        let s = self.heron_scale();
        let feet = self.heron_feet();
        let height = rows.len() as i32 * s;
        let x0 = self.heron_x.round() as i32;
        let y0 = feet - height + 1;
        let colour = |ch: char| match ch {
            'h' => Some(c.neck),
            'w' => Some(c.heron),
            'd' | 'c' => Some(c.crest),
            'b' => Some(c.beak),
            'l' => Some(c.legs),
            _ => None,
        };
        // its reflection first, upside down below its feet and held as imagery, so it reads as
        // part of the water
        for (ry, row) in rows.iter().enumerate() {
            for (rx, ch) in row.chars().enumerate() {
                if let Some(solid) = colour(ch) {
                    for k in 0..s * s {
                        let (dx, dy) = (k % s, k / s);
                        let x = x0 + rx as i32 * s + dx;
                        let y = feet + (feet - (y0 + ry as i32 * s + dy)) + 1;
                        let swell = (self.t as f32 * 0.5 * self.motion.speed()
                            + (y - feet) as f32 * 0.8)
                            .sin();
                        let x = x + (swell * 0.6).round() as i32;
                        if let Some(under) = canvas.get(x, y) {
                            let ghost = held_imagery(c.background, under, solid, 0.55);
                            mixer.blend(canvas, x, y, ghost, 1.0);
                        }
                    }
                }
            }
        }
        for (ry, row) in rows.iter().enumerate() {
            for (rx, ch) in row.chars().enumerate() {
                if let Some(solid) = colour(ch) {
                    canvas.fill_rect(x0 + rx as i32 * s, y0 + ry as i32 * s, s, s, solid);
                }
            }
        }
        // the water round its legs
        let legs = rows[rows.len() - 1].find('l').unwrap_or(0) as i32 * s;
        for dx in -2..=4 * s {
            mixer.blend(canvas, x0 + legs + dx, feet + 1, c.glint, 0.6);
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &LakeColours) {
        let mut mixer = Blender::new(5);
        self.paint_sky(canvas, &mut mixer, c);
        self.paint_sun(canvas, c);
        let shore = self.shore() as f32;
        self.paint_range(canvas, &self.far, c.far, Some(shore * 0.38));
        self.paint_range(canvas, &self.mid, c.mid, Some(shore * 0.5));
        self.paint_forest(canvas, c);
        self.paint_lake(canvas, &mut mixer, c);
        self.paint_heron(canvas, &mut mixer, c);
    }

    fn beak_tip(&self) -> (f32, f32) {
        let s = self.heron_scale() as f32;
        (self.heron_x - s, self.heron_feet() as f32 - s)
    }
}

/// A colour seen in the water: `from` moved `t` of the way towards the water's own colour and,
/// should it still stand out as a shape, such as the sun's rim, faded towards the background
/// until it is imagery, so nothing in the reflection competes with the real thing.
fn reflected(background: Rgb, from: Rgb, water: Rgb, t: f32) -> Rgb {
    let wanted = mix(from, water, t);
    if wanted.contrast(background) < crate::pixels::IMAGERY_CEILING {
        return wanted;
    }
    held_imagery(background, background, wanted, 1.0)
}

/// Wraps a position into `[0, size)`.
fn wrap_x(value: f32, size: f32) -> f32 {
    super::wrap(value, size)
}

impl Scene for MountainLake {
    fn name(&self) -> &'static str {
        "mountain-lake"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let shore = self.shore() as f32;
        let every = (self.width / 3).clamp(18, 60);
        self.far = range(rng, self.width, shore * 0.78, (0.55, 0.95), every);
        self.mid = range(rng, self.width, shore * 0.5, (0.35, 0.7), every * 2 / 3);
        self.hills = range(rng, self.width, shore * 0.16, (0.08, 0.2), every);
        self.pines.clear();
        let tallest = (self.tall() as f32 * 0.09).round().max(3.0) as i32;
        let mut x = rng.below(3) as i32;
        while x < self.width as i32 {
            let height = 3 + rng.below((tallest - 2).max(1) as usize) as i32;
            self.pines.push((x, height));
            x += 2 + height / 2 + rng.below(3) as i32;
        }
        // the sun rises in the lowest notch of the far range across the middle of the view
        let (from, to) = (
            self.width * 2 / 5,
            (self.width * 4 / 5).max(self.width * 2 / 5 + 1),
        );
        self.sun_x = (from..to.min(self.width))
            .min_by(|&a, &b| {
                let ridge = |x: usize| self.far[x].max(self.mid[x]);
                ridge(a).total_cmp(&ridge(b))
            })
            .unwrap_or(self.width / 2) as f32;
        self.bands.clear();
        if self.tall() >= 20 {
            let count = (self.width / 40).clamp(2, 6);
            for i in 0..count {
                self.bands.push(Band {
                    x: rng.range(0.0, self.width as f32),
                    y: shore * (0.14 + 0.32 * i as f32 / count as f32) + rng.range(0.0, 2.0),
                    length: rng.range(10.0, 26.0) * (self.width as f32 / 120.0).max(0.6),
                });
            }
        }
        self.heron_x = self.width as f32 * 0.66;
        self.rings.clear();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        for band in &mut self.bands {
            band.x += DRIFT * speed * dt;
        }
        let slow = if self.motion.calm { 2.0 } else { 1.0 };
        self.pose_left -= dt;
        if self.pose_left <= 0.0 {
            self.pose = match self.pose {
                Pose::Upright => Pose::Fishing,
                Pose::Fishing => Pose::Upright,
            };
            self.pose_left = slow
                * match self.pose {
                    Pose::Upright => rng.range(8.0, 14.0),
                    Pose::Fishing => rng.range(HERON_HOLD + 1.0, HERON_HOLD + 4.0),
                };
            if self.pose == Pose::Fishing && self.heron_rows().is_some() {
                let (x, y) = self.beak_tip();
                self.rings.push(Ring { x, y, age: 0.0 });
            }
        }
        for ring in &mut self.rings {
            ring.age += dt;
        }
        self.rings.retain(|r| r.age < RING_LIFE);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = LakeColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        if self.far.len() == self.width {
            self.paint(&mut canvas, &colours);
        }
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixels::IMAGERY_CEILING;
    use crate::scenes::testing::{
        apart, check_determinism, check_pixel_scene, check_scene, pixels,
    };
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("mountain-lake");
        check_determinism("mountain-lake");
    }

    #[test]
    fn passes_the_pixel_checks_in_every_theme() {
        check_pixel_scene("mountain-lake", 6.0);
    }

    #[test]
    fn sprites_are_rectangular() {
        for rows in [
            &HERON_UP[..],
            &HERON_FISHING[..],
            &HERON_SMALL_UP[..],
            &HERON_SMALL_FISHING[..],
        ] {
            assert!(rows.iter().all(|r| r.len() == rows[0].len()), "{rows:?}");
        }
        assert_eq!(HERON_UP.len(), HERON_FISHING.len());
        assert_eq!(HERON_SMALL_UP.len(), HERON_SMALL_FISHING.len());
    }

    #[test]
    fn the_colours_hold_in_every_theme() {
        for theme in theme::all() {
            let c = LakeColours::new(theme);
            for colour in c.sky.steps().iter().chain([
                &c.dawn, &c.band, &c.far.0, &c.far.1, &c.far.2, &c.mid.0, &c.mid.1, &c.forest,
                &c.water, &c.mist, &c.glint,
            ]) {
                assert!(
                    theme.contrast_of(*colour) < IMAGERY_CEILING,
                    "{} in {}",
                    colour.hex(),
                    theme.name
                );
            }
            for colour in [c.heron, c.neck, c.crest, c.beak, c.legs] {
                assert!(
                    theme.contrast_of(colour) >= paint::SOLID - 0.01,
                    "{} in {}",
                    colour.hex(),
                    theme.name
                );
            }
            apart(theme, c.far.0, c.mid.0, 0.03, "the far and middle ranges");
            apart(
                theme,
                c.mid.1,
                c.forest,
                0.03,
                "the middle range and the forest",
            );
            apart(theme, c.neck, c.crest, 0.08, "the heron's neck and crest");
        }
    }

    #[test]
    fn the_lake_mirrors_the_land_and_the_heron_holds_each_pose() {
        let mut rng = Rng::new(5);
        let mut lake = MountainLake::new(80, 24, Motion::FULL, &mut rng);
        let theme = theme::default_theme();
        let c = LakeColours::new(theme);
        let mut frame = Frame::new(80, 24);
        lake.draw(&mut frame, theme);
        assert!(pixels(&frame).contains(&c.heron));
        let mut last = lake.pose;
        let mut since = 0.0;
        for _ in 0..(120 * 20) {
            lake.step(0.05, &mut rng);
            since += 0.05;
            if lake.pose != last {
                assert!(since >= HERON_HOLD, "the heron changed pose after {since}");
                since = 0.0;
                last = lake.pose;
            }
        }
    }

    #[test]
    fn short_screens_drop_the_heron_and_keep_the_mirror() {
        let mut rng = Rng::new(5);
        let lake = MountainLake::new(20, 6, Motion::FULL, &mut rng);
        assert!(lake.heron_rows().is_none());
        let mut frame = Frame::new(20, 6);
        lake.draw(&mut frame, theme::default_theme());
        assert!(frame.drawn() > 0);
    }
}
