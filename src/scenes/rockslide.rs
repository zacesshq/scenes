//! Rockslide: a steep mountainside of layered rock falls to a road along the valley floor.
//! High on the face a fresh pale scar shows where the rock has broken away. Boulders come
//! loose from it one after another, tumbling down the slope in heavy hops, turning as they go
//! and throwing up dust where they strike. Each one rolls out onto the road and comes to rest
//! in a heap that grows across both lanes, up against a striped barrier, until the road is
//! blocked and only the odd pebble still skitters down.
//!
//! The sky, the far hills, the mountainside, the scar, the road and the dust are tints of the
//! background held below the non-text floor. The boulders and the barrier are solid, so with
//! colour off the falling rocks and the blocked road still read.

use std::f32::consts::TAU;

use super::paint::{self, Blender, Cloud, Wave, noise, unit};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "Boulders breaking from a fresh scar and tumbling down a mountainside to heap across the road, by day in light themes";
pub const CAPTIONS: &[&str] = &[
    "Something came loose. The road is blocked.",
    "Errors piling up on the path. Clear the first.",
    "Rocks down. Check the first failure.",
];

/// Seconds between boulders breaking loose at the standard pace, while the heap is growing.
pub const ROCK_PERIOD: f32 = 2.4;
/// Seconds between pebbles once the heap is complete.
pub const PEBBLE_PERIOD: f32 = 6.0;
/// How long the dust of a strike takes to settle.
const DUST_SECONDS: f32 = 1.8;
/// Boulders already in the heap when the scene starts, so it reads as a slide from the start.
const START_PILE: usize = 4;
/// Pixels a second squared that a falling rock gathers, at the standard pace.
const GRAVITY: f32 = 46.0;

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

#[derive(Clone, Debug)]
pub(crate) struct Colours {
    sky: Ramp,
    cloud: (Rgb, Rgb),
    hills: Rgb,
    /// The layers of the mountainside, from its lit face to deep in shadow, in two bands.
    rock: Ramp,
    rock_alt: Ramp,
    scar: Rgb,
    ledge: Rgb,
    shrub: Rgb,
    road: Rgb,
    line: Rgb,
    kerb: Rgb,
    verge: Ramp,
    dust: Rgb,
    /// A boulder's shadow, its body, its lit faces and its highlight.
    boulder: [Rgb; 4],
    stripe: Rgb,
    stripe_light: Rgb,
    post: Rgb,
}

impl Colours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let stone = mix(theme.muted, theme.yellow(), 0.25);
        let banded = mix(theme.muted, theme.red(), 0.2);
        let grass = mix(theme.green(), theme.yellow(), 0.25);
        // a dark theme's mountainside is lit out of an evening sky; a light theme's is a darker
        // face against a pale day sky, with the lit faces nearest the background
        let (sky_top, sky_low, hills, lit, deep, lit_alt, deep_alt, scar, road) = if theme.dark {
            (
                shades.tint(theme.blue(), 0.14, 1.2),
                shades.tint(mix(theme.info, theme.yellow(), 0.3), 0.35, 1.65),
                shades.tint(mix(grass, theme.blue(), 0.4), 0.3, 1.4),
                shades.tint(mix(stone, theme.foreground, 0.2), 0.62, 2.6),
                shades.tint(stone, 0.22, 1.3),
                shades.tint(mix(banded, theme.foreground, 0.2), 0.55, 2.35),
                shades.tint(banded, 0.2, 1.25),
                shades.tint(mix(stone, theme.foreground, 0.5), 0.85, 2.9),
                shades.tint(theme.muted, 0.22, 1.35),
            )
        } else {
            (
                shades.tint(theme.blue(), 0.3, 1.5),
                shades.tint(theme.info, 0.1, 1.12),
                shades.tint(mix(grass, theme.blue(), 0.4), 0.3, 1.45),
                shades.tint(stone, 0.35, 1.6),
                shades.tint(stone, 0.75, 2.75),
                shades.tint(banded, 0.42, 1.8),
                shades.tint(banded, 0.8, 2.85),
                shades.tint(stone, 0.25, 1.35),
                shades.tint(theme.muted, 0.62, 2.45),
            )
        };
        let (cloud_body, cloud_shade) = paint::cloud_tones(theme, 2.2);
        let rock_hue = mix(theme.muted, theme.yellow(), 0.12);
        Self {
            sky: Ramp::new(sky_top, sky_low, 9),
            cloud: (cloud_body, cloud_shade),
            hills,
            rock: Ramp::new(lit, deep, 6),
            rock_alt: Ramp::new(lit_alt, deep_alt, 6),
            scar,
            ledge: if theme.dark {
                shades.tint(theme.muted, 0.05, 1.08)
            } else {
                shades.tint(stone, 0.9, 2.95)
            },
            shrub: if theme.dark {
                shades.tint(grass, 0.35, 1.5)
            } else {
                shades.tint(grass, 0.7, 2.6)
            },
            road,
            line: if theme.dark {
                shades.tint(theme.foreground, 0.6, 2.7)
            } else {
                theme.background
            },
            kerb: if theme.dark {
                shades.tint(theme.muted, 0.45, 2.0)
            } else {
                shades.tint(theme.muted, 0.3, 1.5)
            },
            verge: if theme.dark {
                Ramp::new(
                    shades.tint(grass, 0.35, 1.55),
                    shades.tint(grass, 0.2, 1.25),
                    4,
                )
            } else {
                Ramp::new(
                    shades.tint(grass, 0.45, 1.85),
                    shades.tint(grass, 0.65, 2.4),
                    4,
                )
            },
            dust: if theme.dark {
                shades.tint(mix(stone, theme.foreground, 0.4), 0.7, 2.6)
            } else {
                shades.tint(stone, 0.35, 1.55)
            },
            boulder: [
                tone(theme, rock_hue, 0.0),
                at_floor(theme, tone(theme, rock_hue, 0.35)),
                tone(theme, rock_hue, 0.7),
                tone(theme, rock_hue, 1.0),
            ],
            stripe: tone(theme, theme.red(), 0.4),
            stripe_light: tone(theme, theme.foreground, 1.0),
            post: tone(theme, theme.muted, 0.2),
        }
    }

    #[cfg(test)]
    pub(crate) fn palette(theme: &ThemeData) -> crate::scenes::testing::Palette {
        let c = Self::new(theme);
        crate::scenes::testing::Palette {
            imagery: vec![
                ("sky top", c.sky.at(0.0)),
                ("sky low", c.sky.at(1.0)),
                ("hills", c.hills),
                ("lit rock", c.rock.at(0.0)),
                ("deep rock", c.rock.at(1.0)),
                ("scar", c.scar),
                ("road", c.road),
                ("verge", c.verge.at(0.0)),
                ("dust", c.dust),
            ],
            shapes: vec![("boulder", c.boulder[1])],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    /// Hopping down the slope.
    Falling,
    /// Rolling along the road to its place in the heap.
    Rolling,
    /// Still, part of the heap.
    Resting,
}

#[derive(Clone, Debug, PartialEq)]
struct Rock {
    x: f32,
    y: f32,
    r: f32,
    vx: f32,
    vy: f32,
    angle: f32,
    salt: u32,
    phase: Phase,
    /// The place in the heap it is heading for, if any; a pebble has none.
    slot: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
struct Dust {
    x: f32,
    y: f32,
    age: f32,
    size: f32,
}

pub struct Rockslide {
    width: usize,
    height: usize,
    motion: Motion,
    rocks: Vec<Rock>,
    dust: Vec<Dust>,
    /// Places in the heap: centre and radius, bottom row first.
    slots: Vec<(f32, f32, f32)>,
    filled: usize,
    next_rock: f32,
    hills: Wave,
    clouds: Vec<Cloud>,
    seed: u32,
}

impl Rockslide {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut slide = Self {
            width: 1,
            height: 1,
            motion,
            rocks: Vec::new(),
            dust: Vec::new(),
            slots: Vec::new(),
            filled: START_PILE,
            next_rock: 0.8,
            hills: Wave::new(rng, 16.0),
            clouds: Vec::new(),
            seed: rng.next_u64() as u32,
        };
        slide.resize(width, height, rng);
        slide
    }

    fn wide(&self) -> i32 {
        self.width as i32
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The top pixel row of the road.
    fn road(&self) -> i32 {
        ((self.tall() as f32 * 0.74).round() as i32).clamp(1, self.tall())
    }

    fn road_depth(&self) -> i32 {
        (self.tall() / 9).max(2)
    }

    /// The row the boulders come to rest on: the near lane of the road, which is seen from a
    /// little above.
    fn lane(&self) -> f32 {
        (self.road() as f32 + self.road_depth() as f32 * 0.7).round()
    }

    /// The column where the mountainside meets the road.
    fn foot(&self) -> f32 {
        (self.width as f32 * 0.5).max(1.0)
    }

    /// The mountainside's surface row at a column: a high crag at the left falling in a steep,
    /// broken face that eases out into a scree fan at the foot.
    fn slope(&self, x: f32) -> f32 {
        // the crag rises past the top of the view, so the mountain fills its corner
        let top = -self.tall() as f32 * 0.06;
        let road = self.road() as f32;
        let along = (x / self.foot()).clamp(0.0, 1.0);
        // a convex face: a high shoulder that steepens into the fall, then a short scree fan
        let face = top + (road - top) * (0.6 * along.powf(1.5) + 0.4 * along);
        // broken ledges along the face, gentler towards the foot
        let crag = (noise(x / 5.0, 0.0, self.seed) - 0.5) * self.tall() as f32 * 0.05;
        (face + crag * (1.0 - along)).min(road)
    }

    /// Where the boulders break loose: high on the face, under the scar.
    fn source(&self) -> f32 {
        self.foot() * 0.16
    }

    fn lay_out_slots(&mut self) {
        self.slots.clear();
        let road = self.lane();
        let r = (self.tall() as f32 / 16.0).clamp(1.6, 8.0);
        let start = self.foot() + r * 1.2;
        let wide = self.width as f32;
        // a heap: a long bottom row across the road, shorter rows resting in the gaps above
        let mut row = 0;
        let mut count = ((wide * 0.36) / (r * 2.05)).floor().clamp(3.0, 9.0) as i32;
        while count > 0 && row < 3 {
            let size = r * (1.0 - 0.12 * row as f32);
            let y = road + 1.0 - size - row as f32 * size * 1.7;
            for i in 0..count {
                let jitter = unit(i, row, self.seed) - 0.5;
                let x = start + (row as f32 * 1.05 + i as f32 * 2.05) * size + jitter * size * 0.3;
                let rr = size * (0.85 + 0.3 * unit(i, row + 9, self.seed));
                if x + rr < wide - 1.0 {
                    self.slots.push((x, y + (size - rr) * 0.8, rr));
                }
            }
            row += 1;
            count -= 2;
        }
    }

    fn spawn(&mut self, rng: &mut Rng) {
        let slot = (self.filled < self.slots.len()).then_some(self.filled);
        let r = match slot {
            Some(i) => self.slots[i].2,
            None => (self.tall() as f32 / 40.0).clamp(0.8, 2.0),
        };
        if slot.is_some() {
            self.filled += 1;
        }
        let x = self.source() + rng.range(-0.06, 0.06) * self.foot();
        self.rocks.push(Rock {
            x,
            y: self.slope(x) - r,
            r,
            vx: rng.range(2.5, 4.0),
            vy: 0.0,
            angle: rng.range(0.0, TAU),
            salt: rng.next_u64() as u32,
            phase: Phase::Falling,
            slot,
        });
    }

    /// Puts the boulders already in the heap in their places.
    fn settle_pile(&mut self) {
        self.rocks.retain(|r| r.phase != Phase::Resting);
        for i in 0..self.filled.min(self.slots.len()) {
            let (x, y, r) = self.slots[i];
            self.rocks.push(Rock {
                x,
                y,
                r,
                vx: 0.0,
                vy: 0.0,
                angle: unit(i as i32, 2, self.seed) * TAU,
                salt: self.seed.wrapping_add(i as u32 * 31),
                phase: Phase::Resting,
                slot: Some(i),
            });
        }
        // resting rocks are drawn first so moving ones pass in front
        self.rocks
            .sort_by_key(|r| if r.phase == Phase::Resting { 0 } else { 1 });
    }

    fn paint_sky(&self, canvas: &mut Canvas, c: &Colours) {
        let road = self.road();
        for y in 0..road {
            canvas.fill_row(y, c.sky.at(y as f32 / road.max(1) as f32));
        }
        for cloud in &self.clouds {
            cloud.paint(canvas, c.cloud.0, c.cloud.1);
        }
        // far hills along the valley beyond the road
        let span = road as f32 * 0.22;
        for x in 0..self.wide() {
            let top = road as f32 - span * (0.55 + 0.45 * self.hills.at(x as f32));
            canvas.fill_rect(x, top.round() as i32, 1, road - top.round() as i32, c.hills);
        }
    }

    fn paint_mountain(&self, canvas: &mut Canvas, c: &Colours) {
        let road = self.road();
        let foot = self.foot();
        let band = (self.tall() / 10).max(3) as f32;
        let scar_x = self.source();
        let scar_y = self.slope(scar_x);
        for x in 0..(foot.ceil() as i32 + 2).min(self.wide()) {
            let surface = self.slope(x as f32);
            let top = surface.round() as i32;
            for y in top.max(0)..road {
                let depth = (y as f32 - surface).max(0.0);
                // layers of rock dipping gently to the left, lit along the face and darker
                // further in
                let layer = ((y as f32 + x as f32 * 0.3) / band).floor() as i32;
                let lit = (depth / (self.tall() as f32 * 0.22)).min(1.0) * 0.82
                    + noise(x as f32 / 9.0, y as f32 / 7.0, self.seed ^ 1) * 0.18;
                let mut colour = if layer % 3 == 1 {
                    c.rock_alt.at(lit)
                } else {
                    c.rock.at(lit)
                };
                // a ledge of shadow under the lip of each layer where it meets the face
                let in_layer = (y as f32 + x as f32 * 0.3).rem_euclid(band);
                if in_layer < 1.0 && depth < 2.5 {
                    colour = c.ledge;
                }
                // the scar: fresh pale rock where the slide broke away, with a ragged edge
                let (dx, dy) = (
                    (x as f32 - scar_x) / (foot * 0.07),
                    (y as f32 - scar_y - band * 0.4) / (band * 0.7),
                );
                let ragged = 0.75 + 0.5 * noise(x as f32 / 2.0, y as f32 / 2.0, self.seed ^ 3);
                if (dx * dx + dy * dy) * ragged < 1.0 {
                    colour = c.scar;
                }
                canvas.set(x, y, colour);
            }
        }
        // a few shrubs clinging to the ledges, kept clear of the rock fall's path
        let count = (self.width / 24).clamp(2, 8) as i32;
        for k in 0..count {
            let x = (unit(k, 4, self.seed) * foot * 0.9) as i32;
            if (x as f32 - scar_x).abs() < foot * 0.18 {
                continue;
            }
            let y = self.slope(x as f32).round() as i32;
            canvas.fill_rect(x - 1, y - 1, 3, 1, c.shrub);
            canvas.set(x, y - 2, c.shrub);
        }
    }

    fn paint_road(&self, canvas: &mut Canvas, c: &Colours) {
        let (road, depth, tall) = (self.road(), self.road_depth(), self.tall());
        canvas.fill_rect(0, road, self.wide(), depth, c.road);
        canvas.fill_rect(0, road, self.wide(), 1, c.kerb);
        // the dashed line between the two lanes
        let mid = road + depth / 2;
        let dash = (self.width / 16).clamp(3, 8) as i32;
        let mut x = dash / 2;
        while x < self.wide() {
            canvas.fill_rect(x, mid, dash, 1, c.line);
            x += dash * 2;
        }
        for y in road + depth..tall {
            let v = (y - road - depth) as f32 / (tall - road - depth).max(1) as f32;
            canvas.fill_row(y, c.verge.at(v));
        }
        canvas.fill_rect(0, road + depth, self.wide(), 1, c.kerb);
    }

    /// The striped barrier standing in the road beyond the heap.
    fn paint_barrier(&self, canvas: &mut Canvas, c: &Colours) {
        if self.width < 50 || self.height < 12 {
            return;
        }
        let (last_x, _, last_r) = self.slots.iter().fold((0.0_f32, 0.0, 0.0), |m, s| {
            if s.0 + s.2 > m.0 + m.2 { *s } else { m }
        });
        let left = (last_x + last_r + 3.0).round() as i32;
        let width = (self.wide() / 9).clamp(9, 22);
        if left + width >= self.wide() - 1 {
            return;
        }
        let height = (self.tall() / 12).clamp(2, 5);
        let ground = self.lane() as i32 + 1;
        let top = ground - height - (self.tall() / 10).max(3);
        for x in left..left + width {
            for y in top..top + height {
                // stripes leaning to the right, red and white
                let red = (x - left + y - top).div_euclid((height).max(2)) % 2 == 0;
                canvas.set(x, y, if red { c.stripe } else { c.stripe_light });
            }
        }
        for post in [left + 1, left + width - 2] {
            canvas.fill_rect(post, top + height, 1, ground - top - height, c.post);
            canvas.fill_rect(post - 1, ground - 1, 3, 1, c.post);
        }
    }

    fn paint_dust(&self, canvas: &mut Canvas, blender: &mut Blender, c: &Colours) {
        for d in &self.dust {
            let k = d.age / DUST_SECONDS;
            let r = d.size * (0.8 + 1.4 * k);
            let amount = (1.0 - k) * 0.8;
            for y in (d.y - r).floor() as i32..=(d.y + r * 0.4).ceil() as i32 {
                for x in (d.x - r * 1.4).floor() as i32..=(d.x + r * 1.4).ceil() as i32 {
                    let dx = (x as f32 + 0.5 - d.x) / (r * 1.4);
                    let dy = (y as f32 + 0.5 - d.y) / r;
                    let dd = dx * dx + dy * dy;
                    let billow = noise(x as f32 / 2.0, y as f32 / 2.0, 11);
                    if dd < 1.0 && billow > 0.3 {
                        blender.blend(canvas, x, y, c.dust, amount * (1.0 - dd));
                    }
                }
            }
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        let mut blender = Blender::new(6);
        self.paint_sky(canvas, c);
        self.paint_road(canvas, c);
        self.paint_mountain(canvas, c);
        self.paint_barrier(canvas, c);
        for rock in &self.rocks {
            boulder(canvas, rock, c);
        }
        self.paint_dust(canvas, &mut blender, c);
    }

    fn rest(&mut self, i: usize) {
        let rock = &mut self.rocks[i];
        if let Some(s) = rock.slot {
            let (x, y, _) = self.slots[s];
            rock.x = x;
            rock.y = y;
        }
        rock.vx = 0.0;
        rock.vy = 0.0;
        rock.phase = Phase::Resting;
    }
}

/// Paints a boulder: a lumpy, faceted shape turned by its angle, lit from the upper left, with
/// a dark rim on its shaded side and a crack across it, so it reads as heavy rock.
fn boulder(canvas: &mut Canvas, rock: &Rock, c: &Colours) {
    let lobe = |a: f32| {
        let s = rock.salt;
        let p = |k: u32| (s.wrapping_mul(2_654_435_761).wrapping_add(k * 97) % 628) as f32 / 100.0;
        1.0 + 0.12 * (3.0 * a + p(1)).sin() + 0.07 * (5.0 * a + p(2)).sin()
            - 0.05 * (2.0 * a + p(3)).cos()
    };
    let reach = rock.r * 1.25;
    for y in (rock.y - reach).floor() as i32..=(rock.y + reach).ceil() as i32 {
        for x in (rock.x - reach).floor() as i32..=(rock.x + reach).ceil() as i32 {
            let (dx, dy) = (x as f32 + 0.5 - rock.x, y as f32 + 0.5 - rock.y);
            let d = (dx * dx + dy * dy).sqrt();
            let local = dy.atan2(dx) - rock.angle;
            let edge = rock.r * lobe(local);
            if d > edge {
                continue;
            }
            // the lumps turn with the rock while the light stays put on its upper left: a lit
            // crescent there, the body, then shadow and a dark rim on the lower right
            let (nx, ny) = (dx / edge, dy / edge);
            let light = -0.62 * nx - 0.78 * ny;
            // a dark rim all round except where the light catches the upper left, so stones in
            // the heap stay apart from one another
            let colour = if d / edge > 0.8 && light < 0.3 {
                c.boulder[0]
            } else if light > 0.42 {
                c.boulder[3]
            } else if light > 0.05 {
                c.boulder[2]
            } else if light > -0.45 {
                c.boulder[1]
            } else {
                c.boulder[0]
            };
            canvas.set(x, y, colour);
        }
    }
    // a crack across the larger boulders
    if rock.r >= 4.0 {
        let a = rock.angle + 0.9;
        let (ux, uy) = (a.cos(), a.sin());
        let steps = (rock.r * 0.9) as i32;
        for k in -steps / 2..steps / 2 {
            let t = k as f32;
            let wob = (t * 1.3 + rock.salt as f32).sin() * 0.5;
            let x = rock.x + ux * t - uy * wob;
            let y = rock.y + uy * t + ux * wob;
            canvas.set(x.floor() as i32, y.floor() as i32, c.boulder[0]);
        }
    }
}

impl Scene for Rockslide {
    fn name(&self) -> &'static str {
        "rockslide"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.lay_out_slots();
        self.filled = self.filled.min(self.slots.len());
        // rocks in flight are dropped and the heap is rebuilt for the new size
        self.rocks.clear();
        self.dust.clear();
        self.settle_pile();
        self.clouds.clear();
        if self.tall() >= 20 {
            let road = self.road() as f32;
            for _ in 0..(self.width / 50).clamp(1, 4) {
                self.clouds.push(Cloud::random(
                    rng,
                    self.width as f32,
                    (road * 0.12, road * 0.35),
                    (10.0, 22.0),
                    self.motion.speed(),
                ));
            }
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        let speed = self.motion.speed();
        let moved = dt * speed;
        let wide = self.width as f32;
        for cloud in &mut self.clouds {
            cloud.drift(dt, wide);
        }
        self.next_rock -= moved;
        if self.next_rock <= 0.0 {
            let full = self.filled >= self.slots.len();
            // under reduced motion fewer rocks come down, not only slower ones
            let calm = if self.motion.calm { 2.0 } else { 1.0 };
            self.next_rock += calm * if full { PEBBLE_PERIOD } else { ROCK_PERIOD };
            if self.height >= 4 {
                self.spawn(rng);
            }
        }
        let road = self.lane();
        let foot = self.foot();
        let mut strikes = Vec::new();
        let mut done = Vec::new();
        for i in 0..self.rocks.len() {
            let slope_here = self.slope(self.rocks[i].x);
            let rock = &mut self.rocks[i];
            match rock.phase {
                Phase::Falling => {
                    rock.vy += GRAVITY * moved;
                    rock.x += rock.vx * moved;
                    rock.y += rock.vy * moved;
                    rock.angle += rock.vx / rock.r.max(0.5) * moved;
                    if rock.x >= foot {
                        // out onto the road: it lands and rolls on towards its place
                        rock.y = road - rock.r;
                        rock.vy = 0.0;
                        rock.vx = rock.vx.max(3.0);
                        rock.phase = Phase::Rolling;
                        strikes.push((rock.x, road, rock.r));
                    } else if rock.y + rock.r >= slope_here {
                        // it strikes the face and bounces off it, losing much of its fall
                        rock.y = slope_here - rock.r;
                        if rock.vy > 6.0 {
                            strikes.push((rock.x, slope_here, rock.r));
                        }
                        rock.vy = -rock.vy.abs() * 0.42;
                        rock.vx = (rock.vx * 0.92 + 1.2).min(14.0);
                    }
                }
                Phase::Rolling => {
                    rock.x += rock.vx * moved;
                    rock.angle += rock.vx / rock.r.max(0.5) * moved;
                    rock.vx = (rock.vx - 2.5 * moved).max(1.5);
                    let target = match rock.slot {
                        Some(s) => self.slots[s].0,
                        None => foot + rock.r * 2.0,
                    };
                    if rock.x >= target {
                        done.push(i);
                    }
                }
                Phase::Resting => {}
            }
        }
        for i in done {
            if self.rocks[i].slot.is_some() {
                let (x, y, r) = (self.rocks[i].x, self.rocks[i].y, self.rocks[i].r);
                strikes.push((x, y + r, r * 0.8));
                self.rest(i);
            } else {
                // a pebble comes to a stop against the heap and is lost among its stones
                self.rocks[i].phase = Phase::Resting;
                self.rocks[i].r = 0.0;
            }
        }
        self.rocks.retain(|r| r.r > 0.0);
        self.rocks
            .sort_by_key(|r| if r.phase == Phase::Resting { 0 } else { 1 });
        let most_dust = if self.motion.calm { 3 } else { 8 };
        for (x, y, r) in strikes {
            if self.dust.len() < most_dust {
                self.dust.push(Dust {
                    x,
                    y,
                    age: 0.0,
                    size: (r * 1.2).max(1.5),
                });
            }
        }
        let pace = self.motion.pace();
        for d in &mut self.dust {
            d.age += dt * pace;
        }
        self.dust.retain(|d| d.age < DUST_SECONDS);
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
        check_scene("rockslide");
        check_determinism("rockslide");
    }

    #[test]
    fn passes_the_pixel_checks() {
        check_pixel_scene("rockslide", 20.0);
        check_palette(Colours::palette);
    }

    #[test]
    fn the_bands_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            for (a, b, what) in [
                (c.sky.at(0.0), c.sky.at(1.0), "the sky"),
                (c.rock.at(0.0), c.rock.at(1.0), "the lit and deep rock"),
                (
                    c.rock.at(0.0),
                    c.sky.at(0.5),
                    "the mountainside and the sky",
                ),
                (c.road, c.verge.at(0.0), "the road and the verge"),
                (c.scar, c.rock.at(0.5), "the scar and the rock"),
                (
                    c.boulder[1],
                    c.boulder[3],
                    "a boulder's body and its highlight",
                ),
                (c.stripe, c.stripe_light, "the barrier's stripes"),
            ] {
                assert!(distance(a, b) >= 0.03, "{what} in {}", theme.name);
            }
        }
    }

    #[test]
    fn boulders_tumble_down_and_block_the_road_then_the_slide_holds() {
        let mut rng = Rng::new(3);
        let mut slide = Rockslide::new(80, 24, Motion::FULL, &mut rng);
        let slots = slide.slots.len();
        assert!(slots >= 6, "{slots}");
        let road = slide.lane();
        for _ in 0..(5 * 60 * 20) {
            slide.step(0.05, &mut rng);
            for rock in &slide.rocks {
                assert!(
                    rock.y - rock.r <= road + 1.0,
                    "a rock fell through the road"
                );
            }
        }
        assert_eq!(slide.filled, slots);
        let resting = slide
            .rocks
            .iter()
            .filter(|r| r.phase == Phase::Resting)
            .count();
        assert_eq!(resting, slots);
        assert!(slide.rocks.len() <= slots + 3);
    }

    #[test]
    fn a_resize_keeps_the_heap() {
        let mut rng = Rng::new(3);
        let mut slide = Rockslide::new(80, 24, Motion::FULL, &mut rng);
        for _ in 0..(20 * 20) {
            slide.step(0.05, &mut rng);
        }
        let filled = slide.filled;
        slide.resize(120, 40, &mut rng);
        let resting = slide
            .rocks
            .iter()
            .filter(|r| r.phase == Phase::Resting)
            .count();
        assert_eq!(resting, filled.min(slide.slots.len()));
    }
}
