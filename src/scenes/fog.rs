//! Fog: a country road at dusk running away from the viewer and bending off to the right, with
//! grass verges and hedges either side, lamp posts along one verge, marker posts with their
//! reflectors, a speed limit sign and a chevron board at the bend, conifers in the fields and a
//! far ridge under a dusk sky. A bank of fog lies in the distance and rolls towards the viewer
//! over the first minute, its front billowing as it drifts. It swallows the ridge and the far
//! lamps first, then the bend and its board, until only the nearest stretch of road is clear,
//! while the lamps it has taken still glow dimly through it.
//!
//! The sky, the ridge, the land, the road and the fog are tints of the background held below the
//! non-text floor. The posts, the signs, the lamps and the near road markings are solid until the
//! fog takes them, so with colour off the scene empties as the fog thickens.

use super::paint::{Blender, billow, ease, held_imagery, noise, solid, stronger};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// Seconds the fog bank takes to roll in from the distance at the standard pace.
pub const ROLL_SECONDS: f32 = 50.0;
/// Columns a second the fog's billows drift by. Its texture is many columns wide, so a pixel
/// takes seconds to go from clear to thick and back.
pub const DRIFT: f32 = 1.2;
/// Columns across one billow of the fog's front.
const PATCH: f32 = 14.0;
/// The fog density from which it hides what is behind it.
const HIDES: f32 = 0.8;
/// How far off the fog's front starts and how close it comes, in multiples of the distance to
/// the bottom of the screen.
const FRONT_FAR: f32 = 8.0;
const FRONT_NEAR: f32 = 1.2;
/// How tall the wall of fog stands, in units of the screen below the horizon seen at the
/// bottom row.
const WALL: f32 = 2.0;

pub const SUMMARY: &str =
    "A road at dusk where a bank of fog rolls in and swallows the lamps and signs along the verge";

pub const CAPTIONS: &[&str] = &[
    "Low visibility. Slow down and read the first error.",
    "The fog lifts one fix at a time.",
    "Cannot see the road? Follow the error message.",
    "Dip your lights and take it steady.",
    "Somewhere in the fog is a clean build.",
];

/// What stands along the road: its distance, which side and how far out (in road half widths)
/// and what it is.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Thing {
    d: f32,
    side: f32,
    kind: Kind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Lamp,
    Marker,
    Speed,
    Chevrons,
    Conifer,
}

/// The road's colours in one theme.
#[cfg_attr(not(test), allow(dead_code))]
struct Colours {
    sky: Ramp,
    glow_sky: Rgb,
    ridge: Rgb,
    field: Rgb,
    verge: Rgb,
    hedge: Rgb,
    road: Ramp,
    edge: Rgb,
    dash: Rgb,
    conifer: (Rgb, Rgb),
    post: Rgb,
    post_lit: Rgb,
    lamp: Rgb,
    halo: Rgb,
    reflector: Rgb,
    plate: Rgb,
    ring: Rgb,
    ink: Rgb,
    board: Rgb,
    chevron: Rgb,
    fog: Rgb,
    fog_lit: Rgb,
}

impl Colours {
    fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let t = |hue: Rgb, s: f32, ceiling: f64| shades.tint(hue, s, ceiling);
        let dusk = mix(theme.blue(), theme.magenta(), 0.35);
        let warm = mix(theme.warning, theme.red(), 0.3);
        let grass = mix(theme.green(), theme.muted, 0.35);
        let post = solid(theme, theme.muted, shades.darkest(), 0.25);
        if theme.dark {
            Self {
                sky: Ramp::new(t(dusk, 0.2, 1.3), t(warm, 0.42, 2.0), 8),
                glow_sky: t(theme.yellow(), 0.5, 2.4),
                ridge: t(dusk, 0.24, 1.38),
                field: t(grass, 0.14, 1.22),
                verge: t(grass, 0.4, 1.75),
                hedge: t(grass, 0.08, 1.12),
                road: Ramp::new(t(theme.muted, 0.14, 1.22), t(theme.muted, 0.24, 1.42), 5),
                edge: t(theme.foreground, 0.5, 2.4),
                dash: theme.foreground,
                conifer: (t(grass, 0.1, 1.15), t(grass, 0.2, 1.32)),
                post,
                post_lit: stronger(theme, post, 0.4),
                lamp: theme.bright_yellow(),
                halo: t(theme.bright_yellow(), 0.6, 2.6),
                reflector: theme.red(),
                plate: theme.foreground,
                ring: theme.red(),
                ink: theme.background,
                board: theme.foreground,
                chevron: theme.red(),
                fog: t(mix(theme.muted, theme.info, 0.3), 0.5, 2.3),
                fog_lit: t(mix(theme.foreground, theme.info, 0.2), 0.6, 2.75),
            }
        } else {
            // early evening in a light theme: a pale sky warming at the horizon, the fog a
            // white that pales everything it touches
            Self {
                sky: Ramp::new(t(theme.blue(), 0.36, 1.6), t(warm, 0.16, 1.2), 8),
                glow_sky: t(theme.yellow(), 0.12, 1.1),
                ridge: t(dusk, 0.4, 1.75),
                field: t(grass, 0.45, 2.0),
                verge: t(grass, 0.32, 1.65),
                hedge: t(grass, 0.72, 2.75),
                road: Ramp::new(t(theme.muted, 0.42, 1.9), t(theme.muted, 0.62, 2.45), 5),
                edge: theme.background,
                dash: theme.background,
                conifer: (t(grass, 0.78, 2.85), t(grass, 0.6, 2.4)),
                post,
                post_lit: mix(post, theme.background, 0.25),
                lamp: theme.yellow(),
                halo: t(theme.yellow(), 0.3, 1.5),
                reflector: theme.red(),
                plate: theme.background,
                ring: theme.red(),
                ink: theme.foreground,
                board: theme.foreground,
                chevron: theme.background,
                fog: t(theme.info, 0.03, 1.04),
                fog_lit: theme.background,
            }
        }
    }
}

pub struct Fog {
    width: usize,
    height: usize,
    motion: Motion,
    /// Seconds of rolling in so far, kept across a resize so the fog never clears.
    rolled: f32,
    /// How far the fog's texture has drifted, in columns.
    drift: f32,
    seed: u32,
    things: Vec<Thing>,
}

/// Where the road and the land are on screen: the horizon row, the vanishing column, the road's
/// half width at the bottom row and how far the far end bends to the right.
#[derive(Clone, Copy, Debug)]
struct View {
    horizon: f32,
    tall: f32,
    centre: f32,
    half: f32,
    bend: f32,
}

impl View {
    /// The distance of the ground at a pixel row, 1 at the bottom of the screen and growing
    /// towards the horizon.
    fn distance(&self, y: f32) -> f32 {
        ((self.tall - self.horizon) / (y - self.horizon).max(0.25)).min(80.0)
    }

    /// The pixel row of the ground at a distance.
    fn row(&self, d: f32) -> f32 {
        self.horizon + (self.tall - self.horizon) / d.max(0.01)
    }

    /// The column of a point `side` road half widths from the centre line at a distance.
    fn column(&self, d: f32, side: f32) -> f32 {
        let away = 1.0 - 1.0 / d.max(1.0);
        self.centre + self.bend * away * away + side * self.half / d.max(0.01)
    }

    /// Pixels per unit of height at a distance: one unit is the height of the screen below the
    /// horizon seen at the bottom row.
    fn scale(&self, d: f32) -> f32 {
        (self.tall - self.horizon) / d.max(0.01)
    }
}

impl Fog {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut fog = Self {
            width: 1,
            height: 1,
            motion,
            rolled: 0.0,
            drift: 0.0,
            seed: (rng.next_u64() >> 32) as u32,
            things: Vec::new(),
        };
        fog.resize(width, height, rng);
        fog
    }

    fn view(&self) -> View {
        let tall = self.height as f32 * 2.0;
        let w = self.width as f32;
        let horizon = (tall * 0.44).floor();
        let below = tall - horizon;
        View {
            horizon,
            tall,
            centre: w * 0.46,
            half: (w * 0.36).clamp(below * 0.9, below * 2.6),
            bend: w * 0.22,
        }
    }

    /// How far off the fog's front is now.
    fn front(&self) -> f32 {
        let done = ease(self.rolled / ROLL_SECONDS);
        FRONT_FAR * (FRONT_NEAR / FRONT_FAR).powf(done)
    }

    /// Where the fog's front stands at a column, as a distance: it billows, so some columns let
    /// it reach nearer than others.
    fn front_at(&self, x: i32) -> f32 {
        let tex = billow((x as f32 - self.drift) / PATCH, 0.7, self.seed);
        self.front() * (0.8 + 0.4 * tex)
    }

    /// The top row of the wall of fog at a column. The bank stands on the ground at its front
    /// and is taller than the lamps, so as it comes nearer it rises up the screen and swallows
    /// everything behind it.
    fn wall_top(&self, x: i32, view: &View) -> f32 {
        let front = self.front_at(x);
        let crest = billow(
            (x as f32 - self.drift * 1.3) / (PATCH * 0.6),
            4.2,
            self.seed ^ 0x3c,
        );
        view.row(front) - WALL * view.scale(front) * (0.75 + 0.5 * crest)
    }

    /// How thick the fog is over a pixel showing something at distance `depth` (infinite for
    /// the sky), from 0 to 1.
    fn density(&self, x: i32, y: i32, depth: f32, view: &View) -> f32 {
        let front = self.front_at(x);
        let top = self.wall_top(x, view);
        // behind the front, the wall hides everything below its billowing top
        let wall = if depth > front {
            ease((y as f32 - top) / 3.0)
        } else {
            0.0
        };
        // in front of it thin wisps run ahead over the road while the far distance stays hazy
        let wisp = ease((depth - front * 0.55) / (front * 0.45 + 0.01)) * 0.45;
        let haze = if depth.is_finite() {
            (1.0 - (-depth / 30.0).exp()) * 0.3
        } else {
            0.0
        };
        wall.max(wisp).max(haze).min(1.0)
    }

    /// Whether a pixel of fog lies along the wall's lit crest.
    fn crest(&self, x: i32, y: i32, view: &View) -> bool {
        let top = self.wall_top(x, view);
        (y as f32) >= top && (y as f32) < top + 2.5
    }

    /// The share of the picture the fog hides.
    #[cfg(test)]
    fn hidden(&self) -> f32 {
        let view = self.view();
        let tall = view.tall as i32;
        let mut count = 0;
        for y in 0..tall {
            let depth = if (y as f32) > view.horizon {
                view.distance(y as f32)
            } else {
                f32::INFINITY
            };
            for x in 0..self.width as i32 {
                if self.density(x, y, depth, &view) >= HIDES {
                    count += 1;
                }
            }
        }
        count as f32 / (self.width as f32 * tall as f32)
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours, theme: &ThemeData) {
        let view = self.view();
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let mut depth = vec![f32::INFINITY; (wide * tall).max(1) as usize];
        let mut put = |canvas: &mut Canvas, x: i32, y: i32, d: f32, colour: Rgb| {
            if x >= 0 && y >= 0 && x < wide && y < tall {
                canvas.set(x, y, colour);
                depth[(y * wide + x) as usize] = d;
            }
        };
        let hz = view.horizon as i32;

        // the dusk sky, with the last of the light low over the bend
        for y in 0..tall {
            canvas.fill_row(y, c.sky.at(y as f32 / view.horizon.max(1.0)));
        }
        let sun = view.column(40.0, 0.0);
        for y in (hz - (view.horizon * 0.35) as i32).max(0)..=hz {
            for x in 0..wide {
                let dx = (x as f32 - sun) / (wide as f32 * 0.3);
                let dy = (view.horizon - y as f32) / (view.horizon * 0.35);
                let g = 1.0 - (dx * dx + dy * dy).sqrt();
                if g > 0.45 {
                    canvas.set(x, y, c.glow_sky);
                } else if g > 0.2 {
                    canvas.set(x, y, mix(c.sky.at(1.0), c.glow_sky, 0.5));
                }
            }
        }
        // a far ridge along the horizon
        for x in 0..wide {
            let lift = noise(x as f32 / 22.0, 1.0, self.seed ^ 0x7) * view.horizon * 0.12 + 1.0;
            for y in (view.horizon - lift).round() as i32..=hz {
                put(canvas, x, y, 60.0, c.ridge);
            }
        }

        // the ground, row by row from the horizon down: fields, verges and the road
        for y in hz + 1..tall {
            let d = view.distance(y as f32 + 0.5);
            let centre = view.column(d, 0.0);
            let half = view.half / d;
            let road = c.road.at((1.0 / d).min(1.0));
            for x in 0..wide {
                let side = (x as f32 + 0.5 - centre) / half.max(0.01);
                let a = side.abs();
                let colour = if a < 1.0 {
                    let edge_line = a > 1.0 - (0.9 / half).max(0.05);
                    let centre_line =
                        a < (0.9 / half).max(0.02) && (d * 1.6).fract() < 0.5 && d < 14.0;
                    if centre_line && d < 4.0 {
                        c.dash
                    } else if edge_line || centre_line {
                        c.edge
                    } else {
                        road
                    }
                } else if a < 1.5 {
                    c.verge
                } else {
                    c.field
                };
                put(canvas, x, y, d, colour);
            }
        }
        // hedges along the far side of each verge, nearer stretches over further ones
        for y in hz + 1..tall {
            let d = view.distance(y as f32 + 0.5);
            let h = (view.scale(d) * 0.07).max(1.0);
            for side in [-1.75_f32, 1.75] {
                let x0 = view.column(d, side).round() as i32;
                let x1 = view.column(d, side * 1.4).round() as i32;
                let (l, r) = (x0.min(x1), x0.max(x1));
                for x in l..=r {
                    let top = (y as f32 - h + 0.3 * noise(x as f32 / 2.0, d, 3)).round() as i32;
                    for py in top..=y {
                        put(canvas, x, py, d, c.hedge);
                    }
                }
            }
        }

        // things along the road, the furthest first
        let mut order: Vec<&Thing> = self.things.iter().collect();
        order.sort_by(|a, b| b.d.total_cmp(&a.d));
        let mut lamps = Vec::new();
        for thing in order {
            let d = thing.d;
            let foot = view.row(d);
            if foot >= view.tall + 2.0 {
                continue;
            }
            let x = view.column(d, thing.side);
            let s = view.scale(d);
            let fy = foot.round() as i32;
            let px = x.round() as i32;
            match thing.kind {
                Kind::Lamp => {
                    let h = (s * 0.9).max(3.0);
                    let thick = if s > 40.0 { 2 } else { 1 };
                    let top = (foot - h).round() as i32;
                    for y in top..=fy {
                        put(canvas, px, y, d, c.post);
                        if thick > 1 {
                            put(canvas, px + 1, y, d, c.post_lit);
                        }
                    }
                    // the arm reaches out over the road to the lamp head
                    let reach = ((s * 0.18).max(2.0)) as i32;
                    let dir = if thing.side > 0.0 { -1 } else { 1 };
                    for k in 0..=reach {
                        put(canvas, px + dir * k, top, d, c.post);
                    }
                    let head = (px + dir * reach, top + 1);
                    let size = if s > 30.0 { 2 } else { 1 };
                    for hx in 0..size {
                        put(canvas, head.0 - dir * hx, head.1, d, c.lamp);
                    }
                    lamps.push((head.0, head.1, d, s));
                }
                Kind::Marker => {
                    let h = (s * 0.16).max(2.0) as i32;
                    for y in fy - h..=fy {
                        put(canvas, px, y, d, c.plate);
                    }
                    put(canvas, px, fy - h + 1, d, c.reflector);
                }
                Kind::Speed => {
                    let r = (s * 0.2).max(2.0);
                    let cy = foot - s * 0.55;
                    for y in cy.round() as i32..=fy {
                        put(canvas, px, y, d, c.post);
                    }
                    self.speed_sign(canvas, &mut put, (x, cy), r, d, c);
                }
                Kind::Chevrons => {
                    let (w, h) = ((s * 0.95).max(5.0), (s * 0.36).max(3.0));
                    let top = foot - s * 0.5;
                    for leg in [-0.4_f32, 0.4] {
                        let lx = (x + leg * w).round() as i32;
                        for y in (top + h).round() as i32..=fy {
                            put(canvas, lx, y, d, c.post);
                        }
                    }
                    self.chevrons(canvas, &mut put, (x - w / 2.0, top), (w, h), d, c);
                }
                Kind::Conifer => {
                    let h = s * 0.75;
                    let w = h * 0.36;
                    let top = foot - h;
                    for y in top.round() as i32..=fy {
                        let v = (y as f32 - top) / h.max(1.0);
                        // tiers: each widens downwards then steps in again
                        let tier = (v * 4.0).fract();
                        let half = w * v * (0.75 + 0.25 * tier);
                        let (l, r) = ((x - half).round() as i32, (x + half).round() as i32);
                        for xx in l..=r {
                            let lit = (xx as f32) < x - half * 0.3;
                            put(
                                canvas,
                                xx,
                                y,
                                d,
                                if lit { c.conifer.1 } else { c.conifer.0 },
                            );
                        }
                    }
                }
            }
        }

        // the fog: everything is mixed towards it by how thick it is at that distance
        let mut mixer = Blender::new(5);
        for y in 0..tall {
            for x in 0..wide {
                let d = depth[(y * wide + x) as usize];
                let amount = self.density(x, y, d, &view);
                if amount > 0.03 {
                    // inside the bank rolling billows catch the last light, so it reads as a
                    // moving body of fog rather than a flat wash
                    let roll = billow(
                        (x as f32 - self.drift * 1.6) / (PATCH * 0.8),
                        y as f32 / 6.0,
                        self.seed ^ 0x9e,
                    );
                    let lit = amount > 0.6 && (self.crest(x, y, &view) || roll > 0.6);
                    let colour = if lit { c.fog_lit } else { c.fog };
                    let strength = if lit && roll > 0.6 { 0.6 } else { 1.0 };
                    let t = (amount / HIDES).min(1.0) * strength;
                    let under = canvas.get(x, y).unwrap_or(c.fog);
                    if t > 0.15 && theme.contrast_of(under) >= IMAGERY_CEILING {
                        // a shape the fog has begun to take drops back to imagery at once, so it
                        // fades as a ghost rather than lingering half solid
                        let keep = ((1.0 - t) * 5.0).round() / 5.0;
                        canvas.set(x, y, held_imagery(theme.background, colour, under, keep));
                    } else {
                        mixer.blend(canvas, x, y, colour, t);
                    }
                }
            }
        }
        // lamps glow through the fog, dimmer the deeper in they are
        for (lx, ly, d, s) in lamps {
            let amount = self.density(lx, ly, d, &view);
            let r = (s * 0.12).clamp(2.0, 9.0) * (1.0 + amount);
            let strength = 0.75 - amount * 0.35;
            let reach = r.ceil() as i32;
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    let q = ((dx * dx + dy * dy) as f32).sqrt() / r;
                    if q < 1.0 {
                        mixer.blend(canvas, lx + dx, ly + dy, c.halo, strength * (1.0 - q));
                    }
                }
            }
            if amount < HIDES * 0.85 {
                canvas.set(lx, ly, c.lamp);
            }
        }
    }

    /// A round speed limit sign: a red ring, a pale face and a 30 when there is room for it.
    fn speed_sign(
        &self,
        canvas: &mut Canvas,
        put: &mut impl FnMut(&mut Canvas, i32, i32, f32, Rgb),
        centre: (f32, f32),
        r: f32,
        d: f32,
        c: &Colours,
    ) {
        let reach = r.ceil() as i32 + 1;
        let (cx, cy) = (centre.0.round() as i32, centre.1.round() as i32);
        for y in cy - reach..=cy + reach {
            for x in cx - reach..=cx + reach {
                let q = (((x - cx) * (x - cx) + (y - cy) * (y - cy)) as f32).sqrt();
                if q <= r + 0.3 {
                    let ring = q > r - (r * 0.28).max(1.0);
                    put(canvas, x, y, d, if ring { c.ring } else { c.plate });
                }
            }
        }
        if r >= 5.0 {
            // a 3 and a 0, three pixels wide and five tall
            const DIGITS: [[&str; 5]; 2] = [
                ["###", "..#", ".##", "..#", "###"],
                ["###", "#.#", "#.#", "#.#", "###"],
            ];
            for (k, digit) in DIGITS.iter().enumerate() {
                let left = cx - 3 + k as i32 * 4;
                for (dy, row) in digit.iter().enumerate() {
                    for (dx, ch) in row.chars().enumerate() {
                        if ch == '#' {
                            put(canvas, left + dx as i32, cy - 2 + dy as i32, d, c.ink);
                        }
                    }
                }
            }
        }
    }

    /// A chevron board warning of the bend: a board with arrowheads pointing the way the road
    /// turns.
    fn chevrons(
        &self,
        canvas: &mut Canvas,
        put: &mut impl FnMut(&mut Canvas, i32, i32, f32, Rgb),
        corner: (f32, f32),
        size: (f32, f32),
        d: f32,
        c: &Colours,
    ) {
        let (x0, y0) = (corner.0.round() as i32, corner.1.round() as i32);
        let (w, h) = (size.0.round() as i32, size.1.round() as i32);
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                put(canvas, x, y, d, c.board);
            }
        }
        if h < 3 {
            return;
        }
        // each chevron is a > whose point sits at the middle row
        let mid = y0 + h / 2;
        let half = (h - 1) / 2;
        let count = (w / (half + 4)).clamp(1, 4);
        let pitch = w / count;
        for k in 0..count {
            let tip = x0 + k * pitch + pitch / 2 + half / 2;
            for dy in -half..=half {
                let x = tip - dy.abs();
                put(canvas, x, mid + dy, d, c.chevron);
                put(canvas, x - 1, mid + dy, d, c.chevron);
            }
        }
    }
}

impl Scene for Fog {
    fn name(&self) -> &'static str {
        "fog"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let mut things = Vec::new();
        // lamps along the right verge, spaced evenly along the road, so they bunch with distance
        let mut d = 1.35;
        while d < 30.0 {
            things.push(Thing {
                d,
                side: 1.3,
                kind: Kind::Lamp,
            });
            d *= 1.55;
        }
        // marker posts along both edges
        let mut d = 1.15;
        while d < 20.0 {
            for side in [-1.12, 1.12] {
                things.push(Thing {
                    d: d * if side < 0.0 { 1.12 } else { 1.0 },
                    side,
                    kind: Kind::Marker,
                });
            }
            d *= 1.35;
        }
        things.push(Thing {
            d: 1.45,
            side: -1.45,
            kind: Kind::Speed,
        });
        things.push(Thing {
            d: 2.6,
            side: -1.9,
            kind: Kind::Chevrons,
        });
        // conifers out in the fields on both sides
        let count = 6 + rng.below(4);
        for _ in 0..count {
            let side = if rng.chance(0.55) { -1.0 } else { 1.0 };
            things.push(Thing {
                d: rng.range(2.2, 16.0),
                side: side * rng.range(2.2, 4.5),
                kind: Kind::Conifer,
            });
        }
        self.things = things;
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        let speed = self.motion.speed();
        self.rolled = (self.rolled + dt * speed).min(ROLL_SECONDS * 2.0);
        // the drift wraps far beyond any texture repeat, so it never jumps on a long run
        self.drift = (self.drift + DRIFT * speed * dt) % 100_000.0;
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = Colours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours, theme);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::paint;
    use crate::scenes::testing::{apart, check_pixel_scene, pixels};
    use crate::scenes::testing::{check_determinism, check_scene, render};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("fog");
        check_determinism("fog");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("fog", 20.0);
    }

    #[test]
    fn the_fog_rolls_in_slowly_and_swallows_the_lamps() {
        let theme = theme::default_theme();
        let c = Colours::new(theme);
        let mut rng = Rng::new(14);
        let mut fog = Fog::new(80, 24, Motion::FULL, &mut rng);
        let clear = render(&mut fog, 80, 24, 1);
        let lamps = |frame: &Frame| pixels(frame).iter().filter(|&&p| p == c.lamp).count();
        let lit = lamps(&clear);
        assert!(lit >= 3, "{lit} lamp pixels");
        assert!(pixels(&clear).contains(&c.ring) && pixels(&clear).contains(&c.chevron));
        // no words or warning signs: the fog itself says what is wrong
        assert!(
            clear
                .cells()
                .iter()
                .all(|cell| cell.is_pixel() || cell.ch == ' ')
        );
        let start = fog.hidden();
        let mut last = start;
        for _ in 0..(3600 * 20) {
            fog.step(0.05, &mut rng);
            if fog.rolled < ROLL_SECONDS * 1.5 {
                let now = fog.hidden();
                // the bank never covers more than a tenth of the screen in a second
                assert!((now - last).abs() < 0.1 / 20.0 * 2.0, "{last} to {now}");
                last = now;
            }
        }
        assert!(
            fog.hidden() > 0.4 && fog.hidden() > start + 0.2,
            "{}",
            fog.hidden()
        );
        let thick = render(&mut fog, 80, 24, 1);
        assert!(lamps(&thick) < lit, "the fog never took a lamp");
        assert!(pixels(&thick).contains(&c.fog) || fog.hidden() > 0.4);
        const { assert!(DRIFT / PATCH < 1.0 / 3.0) };
    }

    #[test]
    fn the_layers_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            apart(theme, c.sky.at(0.0), c.sky.at(1.0), 0.03, "the dusk sky");
            apart(theme, c.ridge, c.sky.at(1.0), 0.02, "the ridge and the sky");
            apart(theme, c.verge, c.field, 0.03, "the verge and the field");
            apart(theme, c.hedge, c.verge, 0.03, "the hedge and the verge");
            apart(
                theme,
                c.road.at(1.0),
                c.verge,
                0.03,
                "the road and the verge",
            );
            apart(theme, c.fog, c.road.at(0.5), 0.05, "the fog and the road");
            apart(
                theme,
                c.edge,
                c.road.at(1.0),
                0.05,
                "the edge line and the road",
            );
            let mut imagery = vec![
                c.glow_sky,
                c.ridge,
                c.field,
                c.verge,
                c.hedge,
                c.halo,
                c.fog,
                c.conifer.0,
                c.conifer.1,
            ];
            imagery.extend(c.sky.steps());
            imagery.extend(c.road.steps());
            for colour in imagery {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "{} in {}",
                    colour.hex(),
                    theme.name
                );
            }
            for colour in [c.post, c.lamp, c.reflector, c.ring] {
                assert!(
                    theme.contrast_of(colour) >= paint::SOLID - 0.01,
                    "{}",
                    theme.name
                );
            }
        }
    }
}
