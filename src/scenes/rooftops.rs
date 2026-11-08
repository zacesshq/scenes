//! Rooftops at dusk in solid pixels, seen from a high window: a sky fading from deep blue to a
//! warm glow where the sun has just gone down, a crescent moon, a far skyline of towers and a
//! spire, a terrace of houses with pitched roofs and windows lighting up one at a time, then the
//! big slate roofs close by with their chimney stacks and pots, smoke curling up and drifting
//! away on the breeze, television aerials and pigeons on the ridges, one now and then flying
//! across to another roof.
//!
//! In a light theme it is the same street in late afternoon: a pale sky warming to the horizon,
//! a low sun, the far towers pale with distance and the sun catching the window panes.
//!
//! The sky, the skyline, the walls, the roofs, the chimneys and the smoke are tints of the
//! background held below the non-text floor; the lit windows, the chimney pots, the aerials, the
//! moon and the pigeons keep the theme's own drawing colours, so they stand out as shapes and
//! keep their outline when colour is off.

use super::paint::{Blender, disc, noise, tint, unit};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "Rooftops at dusk with chimney smoke, pigeons on the ridges and windows lighting up, in late afternoon in light themes";
pub const CAPTIONS: &[&str] = &[
    "Lights coming on. The build is fine.",
    "A quiet evening over a clean build.",
    "Everything in its place up here.",
];

/// The shortest time between two windows changing at the standard pace.
pub const WINDOW_GAP: f32 = 2.4;
/// One beat of a flying pigeon's wings: up, then down.
pub const WING_PERIOD: f32 = 0.8;
/// Seconds between pigeons taking off, at the least.
const FLIGHT_GAP: f32 = 9.0;
/// Seconds a puff of smoke rises before it has faded.
const SMOKE_LIFE: f32 = 7.0;

/// A pigeon perched facing right: a small head and beak, a rounded breast, the folded wing a
/// shade darker and the tail low behind.
const PERCHED: [&str; 4] = ["...hb", ".hhh.", "wwhh.", ".l.l."];
const WINGS_UP: [&str; 3] = ["w...w", ".whw.", "..h.."];
const WINGS_DOWN: [&str; 3] = ["..h..", ".whw.", "w...w"];

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct Colours {
    pub sky_top: Rgb,
    pub sky_mid: Rgb,
    pub sky_low: Rgb,
    pub glow: Rgb,
    pub far: Rgb,
    pub far_window: Rgb,
    pub wall: Rgb,
    pub wall_shade: Rgb,
    pub roof: Rgb,
    pub roof_rim: Rgb,
    pub near: Rgb,
    pub near_tile: Rgb,
    pub near_rim: Rgb,
    pub brick: Rgb,
    pub brick_lit: Rgb,
    pub glass: Rgb,
    pub smoke: Rgb,
    /// A lit window: lamplight at dusk and the sun on the glass in a light theme.
    pub window: Rgb,
    pub pot: Rgb,
    pub aerial: Rgb,
    pub moon: Rgb,
    pub pigeon: Rgb,
    pub pigeon_wing: Rgb,
    sky: Ramp,
}

impl Colours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let dark = theme.dark;
        let t = |hue: Rgb, s: f32, ceiling: f64| tint(theme, hue, s, ceiling);
        let warm = mix(theme.warning, theme.red(), 0.3);
        if dark {
            // dusk: the houses are silhouettes against the glow, each row nearer a step darker,
            // with a warm rim of light along every ridge facing the sunset
            let sky_low = t(warm, 0.5, 2.3);
            Self {
                sky_top: t(theme.blue(), 0.22, 1.3),
                sky_mid: t(theme.magenta(), 0.22, 1.5),
                sky_low,
                glow: t(theme.yellow(), 0.6, 2.8),
                far: t(theme.blue(), 0.3, 1.55),
                far_window: t(theme.yellow(), 0.5, 2.1),
                wall: t(theme.red(), 0.18, 1.3),
                wall_shade: t(theme.red(), 0.1, 1.15),
                roof: t(theme.muted, 0.2, 1.22),
                roof_rim: t(warm, 0.55, 2.2),
                near: t(theme.blue(), 0.08, 1.09),
                near_tile: t(theme.muted, 0.26, 1.3),
                near_rim: t(warm, 0.45, 1.9),
                brick: t(theme.red(), 0.3, 1.45),
                brick_lit: t(warm, 0.55, 2.1),
                glass: t(theme.blue(), 0.12, 1.12),
                smoke: t(theme.grey(), 0.6, 2.2),
                window: theme.yellow(),
                pot: theme.red(),
                aerial: theme.muted,
                moon: theme.bright(),
                pigeon: theme.grey(),
                pigeon_wing: mix(theme.grey(), theme.muted, 0.5),
                sky: Ramp::new(t(theme.blue(), 0.22, 1.3), sky_low, 8),
            }
        } else {
            // late afternoon: a pale sky, the far towers paler still with distance and the
            // nearest slates the strongest tints, so depth reads from back to front
            let sky_top = t(theme.blue(), 0.32, 1.55);
            let sky_low = t(warm, 0.12, 1.15);
            Self {
                sky_top,
                sky_mid: t(theme.info, 0.16, 1.28),
                sky_low,
                glow: t(theme.yellow(), 0.1, 1.08),
                far: t(theme.blue(), 0.3, 1.5),
                far_window: t(theme.info, 0.12, 1.25),
                wall: t(theme.red(), 0.42, 2.0),
                wall_shade: t(theme.red(), 0.55, 2.35),
                roof: t(theme.muted, 0.5, 2.2),
                roof_rim: t(theme.muted, 0.28, 1.6),
                near: t(theme.muted, 0.68, 2.75),
                near_tile: t(theme.muted, 0.55, 2.35),
                near_rim: t(theme.muted, 0.35, 1.75),
                brick: t(theme.red(), 0.62, 2.7),
                brick_lit: t(warm, 0.42, 2.0),
                glass: t(theme.blue(), 0.5, 2.4),
                smoke: t(theme.grey(), 0.22, 1.35),
                // in daylight a window is lit by the sun on its glass, a warm gold
                window: t(theme.yellow(), 0.55, 2.3),
                pot: theme.red(),
                aerial: theme.muted,
                moon: t(theme.yellow(), 0.25, 1.3),
                pigeon: theme.grey(),
                pigeon_wing: mix(theme.grey(), theme.foreground, 0.4),
                sky: Ramp::new(sky_top, sky_low, 8),
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn palette(theme: &ThemeData) -> crate::scenes::testing::Palette {
        let c = Self::new(theme);
        let mut imagery = vec![
            ("sky top", c.sky_top),
            ("sky middle", c.sky_mid),
            ("sky low", c.sky_low),
            ("glow", c.glow),
            ("far skyline", c.far),
            ("far window", c.far_window),
            ("wall", c.wall),
            ("wall in shade", c.wall_shade),
            ("roof", c.roof),
            ("roof rim", c.roof_rim),
            ("near roof", c.near),
            ("near tiles", c.near_tile),
            ("near rim", c.near_rim),
            ("brick", c.brick),
            ("lit brick", c.brick_lit),
            ("glass", c.glass),
            ("smoke", c.smoke),
        ];
        let mut shapes = vec![
            ("chimney pot", c.pot),
            ("aerial", c.aerial),
            ("pigeon", c.pigeon),
        ];
        if theme.dark {
            shapes.push(("moon", c.moon));
            shapes.push(("lit window", c.window));
        } else {
            imagery.push(("sun", c.moon));
            imagery.push(("sunlit window", c.window));
        }
        crate::scenes::testing::Palette { imagery, shapes }
    }
}

/// A house in the terrace across the middle of the view.
#[derive(Clone, Debug, PartialEq)]
struct House {
    x: i32,
    w: i32,
    /// The top of the front wall and the ridge above it.
    eaves: i32,
    ridge: i32,
    /// A small chimney on the ridge, as a column.
    chimney: Option<i32>,
}

/// A big roof close by: a slate slope from its ridge down past the bottom of the view.
#[derive(Clone, Debug, PartialEq)]
struct Roof {
    x: i32,
    w: i32,
    ridge: i32,
    /// Chimney stacks on the ridge as (column, width, height, pots, aerial).
    stacks: Vec<(i32, i32, i32, i32, bool)>,
    /// A dormer window part way down the slope, as its column.
    dormer: Option<i32>,
}

/// A flight from one perch to another and how far along it is, from 0 to 1.
type Flight = ((f32, f32), (f32, f32), f32);

#[derive(Clone, Debug, PartialEq)]
struct Pigeon {
    x: f32,
    y: f32,
    flight: Option<Flight>,
    facing_right: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct Puff {
    x: f32,
    y: f32,
    age: f32,
    seed: u32,
}

pub struct Rooftops {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// The far skyline's top row at each column.
    skyline: Vec<i32>,
    /// Lit windows in the far towers, as (column, row).
    far_windows: Vec<(i32, i32)>,
    houses: Vec<House>,
    roofs: Vec<Roof>,
    /// Every terrace window as (column, row, lit).
    windows: Vec<(i32, i32, bool)>,
    /// The size of a terrace window in pixels.
    pane: i32,
    pigeons: Vec<Pigeon>,
    puffs: Vec<Puff>,
    sun: (f32, f32),
    next_window: f32,
    next_flight: f32,
    next_puff: f32,
    puff_count: u32,
    changes: u32,
}

impl Rooftops {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut roofs = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            skyline: Vec::new(),
            far_windows: Vec::new(),
            houses: Vec::new(),
            roofs: Vec::new(),
            windows: Vec::new(),
            pane: 2,
            pigeons: Vec::new(),
            puffs: Vec::new(),
            sun: (0.0, 0.0),
            next_window: WINDOW_GAP,
            next_flight: FLIGHT_GAP,
            next_puff: 0.0,
            puff_count: 0,
            changes: 0,
        };
        roofs.resize(width, height, rng);
        roofs
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The ridge row at a column, for perching: the ridge of whichever near roof is there,
    /// clear of its chimney stacks.
    fn ridge_at(&self, x: i32) -> Option<i32> {
        let wide = PERCHED[0].len() as i32;
        self.roofs
            .iter()
            .find(|r| {
                let clear = r
                    .stacks
                    .iter()
                    .all(|&(cx, w, ..)| x + wide < cx - 1 || x > cx + w + 1);
                x > r.x + 2 && x + wide < r.x + r.w - 2 && clear
            })
            .map(|r| r.ridge)
    }

    fn perch(&self, rng: &mut Rng) -> Option<(f32, f32)> {
        for _ in 0..12 {
            let x = rng.below(self.width.max(1)) as i32;
            // pigeons keep a little room between them
            let crowded = self.pigeons.iter().any(|p| (p.x - x as f32).abs() < 7.0);
            if crowded {
                continue;
            }
            if let Some(ridge) = self.ridge_at(x) {
                return Some((x as f32, (ridge - PERCHED.len() as i32) as f32));
            }
        }
        None
    }

    /// Where smoke leaves each chimney pot.
    fn pot_tops(&self) -> Vec<(i32, i32)> {
        let mut tops = Vec::new();
        for roof in &self.roofs {
            for &(cx, _, h, pots, _) in &roof.stacks {
                for k in 0..pots {
                    let px = cx + 1 + k * 3;
                    tops.push((px, roof.ridge - h - 3));
                }
            }
        }
        for house in &self.houses {
            if let Some(cx) = house.chimney {
                tops.push((cx + 1, house.ridge - 5));
            }
        }
        tops
    }

    fn paint_sky(&self, canvas: &mut Canvas, c: &Colours, blender: &mut Blender, dark: bool) {
        let (wide, tall) = (canvas.width() as i32, canvas.height() as i32);
        let horizon = (tall as f32 * 0.46).max(1.0);
        for y in 0..tall {
            let v = y as f32 / horizon;
            canvas.fill_row(y, c.sky.at(v));
            // a band of rose between the blue above and the glow below
            let rose = (1.0 - ((v - 0.55) / 0.25).abs()).max(0.0) * 0.5;
            if rose > 0.0 {
                for x in 0..wide {
                    blender.blend(canvas, x, y, c.sky_mid, rose);
                }
            }
        }
        // the glow where the sun has set, widest along the horizon
        let (sx, sy) = self.sun;
        let reach = (wide as f32 * 0.45).max(tall as f32 * 0.8);
        for y in 0..tall {
            for x in 0..wide {
                let dx = (x as f32 - sx) / reach;
                let dy = (y as f32 - sy) / (reach * 0.4);
                let g = (1.0 - (dx * dx + dy * dy).sqrt()).max(0.0);
                if g > 0.05 {
                    blender.blend(canvas, x, y, c.glow, g * 0.8);
                }
            }
        }
        // long thin clouds lit from below, drifting slowly
        let drift = self.t as f32 * 0.4 * self.motion.speed();
        let band = (tall as f32 * 0.42) as i32;
        for y in (tall as f32 * 0.08) as i32..band {
            for x in 0..wide {
                let n = noise((x as f32 + drift) / 26.0, y as f32 / 3.2, 71);
                let d = ((n - 0.7) / 0.1).clamp(0.0, 1.0);
                if d > 0.1 {
                    let under = noise((x as f32 + drift) / 26.0, (y as f32 + 1.5) / 3.2, 71);
                    let colour = if under < n { c.roof_rim } else { c.sky_mid };
                    blender.blend(canvas, x, y, colour, d * 0.7);
                }
            }
        }
        // a crescent moon high on the dark side of the sky; the low sun in a light theme
        if dark {
            let r = (tall as f32 * 0.07).clamp(2.5, 7.0);
            let (mx, my) = (wide as f32 * 0.82, tall as f32 * 0.16);
            disc(canvas, mx, my, r, c.moon);
            // too small a moon for a crescent to read stays full
            if r >= 3.5 {
                let behind = canvas
                    .get((mx + r * 2.0) as i32, my as i32)
                    .unwrap_or(c.sky_top);
                disc(canvas, mx + r * 0.55, my - r * 0.3, r * 0.9, behind);
            }
        } else {
            disc(canvas, sx, sy - 2.0, 4.0, c.moon);
        }
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours, dark: bool) {
        let mut blender = Blender::new(3);
        let tall = canvas.height() as i32;
        self.paint_sky(canvas, c, &mut blender, dark);

        // the far skyline, flat with distance, with a few windows already lit
        for (x, &top) in self.skyline.iter().enumerate() {
            for y in top.max(0)..tall {
                canvas.set(x as i32, y, c.far);
            }
        }
        for &(x, y) in &self.far_windows {
            canvas.set(x, y, c.far_window);
        }

        // smoke behind the nearest roofs drifts over the skyline and the terrace
        for puff in &self.puffs {
            let age = (puff.age / SMOKE_LIFE).clamp(0.0, 1.0);
            let r = 0.8 + age * 3.2;
            let strength = (1.0 - age) * 0.85;
            for k in 0..3 {
                let wob = unit(k, 0, puff.seed) - 0.5;
                let (cx, cy) = (puff.x + wob * r, puff.y - k as f32 * r * 0.5);
                let pr = r * (0.8 - 0.15 * k as f32);
                soft_disc(canvas, &mut blender, (cx, cy), pr, c.smoke, strength);
            }
        }

        // the terrace: walls with a shaded right end and pitched roofs with a lit rim along the
        // left of each ridge, then its windows in rows
        for house in &self.houses {
            for y in house.eaves.max(0)..tall {
                for x in house.x..house.x + house.w {
                    let shade = x >= house.x + house.w - 2 || x == house.x;
                    canvas.set(x, y, if shade { c.wall_shade } else { c.wall });
                }
            }
            for y in house.ridge.max(0)..house.eaves {
                let inset = (house.eaves - y) * 3 / 4;
                for x in house.x + inset - 1..house.x + house.w - inset + 1 {
                    let edge = y == house.ridge || x == house.x + inset - 1;
                    canvas.set(x, y, if edge { c.roof_rim } else { c.roof });
                }
            }
            if let Some(cx) = house.chimney {
                let top = house.ridge - 4;
                canvas.fill_rect(cx, top, 3, house.ridge - top + 1, c.brick);
                canvas.set(cx, top, c.brick_lit);
                canvas.fill_rect(cx, top - 1, 1, 1, c.pot);
                canvas.fill_rect(cx + 2, top - 1, 1, 1, c.pot);
            }
        }
        for &(x, y, lit) in &self.windows {
            let p = self.pane;
            canvas.fill_rect(x, y, p, p, if lit { c.window } else { c.glass });
            // a sill below each window catches the light
            canvas.fill_rect(x - 1, y + p, p + 2, 1, c.roof_rim);
        }

        // the near roofs: slate slopes with rows of tiles, a lit ridge, dormers and stacks
        for roof in &self.roofs {
            let course_tone = blender.mix(c.near, c.near_tile, 0.5);
            for y in roof.ridge.max(0)..tall {
                let down = y - roof.ridge;
                for x in roof.x..roof.x + roof.w {
                    // the ridge tiles catch the glow, the slates just below them still hold
                    // some of the sky and the rest of the slope falls into shadow, with the
                    // courses of slates picked out a step lighter
                    let colour = if down < 2 {
                        c.near_rim
                    } else if down < 4 {
                        c.near_tile
                    } else if down % 3 == 0 || (x + (down / 3) * 4) % 7 == 0 && down % 3 == 1 {
                        course_tone
                    } else {
                        c.near
                    };
                    canvas.set(x, y, colour);
                }
            }
            // a brick party wall between this roof and the next, its coping above the ridge
            let wall = roof.x + roof.w;
            for y in (roof.ridge - 2).max(0)..tall {
                let colour = if y < roof.ridge { c.near_rim } else { c.brick };
                canvas.set(wall, y, colour);
                canvas.set(
                    wall + 1,
                    y,
                    if y < roof.ridge { c.near_rim } else { c.brick },
                );
            }
            if let Some(dx) = roof.dormer {
                let top = roof.ridge + 5;
                let (w, h) = (9, 8);
                canvas.fill_rect(dx - 1, top - 2, w + 2, 2, c.near_rim);
                canvas.fill_rect(dx, top, w, h, c.wall_shade);
                canvas.fill_rect(dx + 2, top + 2, w - 4, h - 3, c.window);
                canvas.fill_rect(dx + w / 2, top + 2, 1, h - 3, c.wall_shade);
            }
            for &(cx, w, h, pots, aerial) in &roof.stacks {
                let top = roof.ridge - h;
                // the stack's left face catches the last of the sun
                for y in top..roof.ridge + 3 {
                    for x in cx..cx + w {
                        let lit = x < cx + 2;
                        let course = (y - top) % 3 == 2;
                        let colour = if lit {
                            c.brick_lit
                        } else if course {
                            blender.mix(c.brick, c.near, 0.5)
                        } else {
                            c.brick
                        };
                        canvas.set(x, y, colour);
                    }
                }
                // a projecting cap and a pot or two above it
                canvas.fill_rect(cx - 1, top, w + 2, 1, c.near_rim);
                for k in 0..pots {
                    let px = cx + 1 + k * 3;
                    canvas.fill_rect(px, top - 3, 2, 3, c.pot);
                }
                if aerial {
                    // a television aerial: a mast and a boom with short elements across it
                    let mast = cx + w - 1;
                    let boom = top - 10;
                    canvas.line((mast, top - 1), (mast, boom), c.aerial);
                    canvas.line((mast - 4, boom), (mast + 3, boom), c.aerial);
                    for k in [-4, -2, 0, 2] {
                        canvas.set(mast + k, boom - 1, c.aerial);
                        canvas.set(mast + k, boom + 1, c.aerial);
                    }
                }
            }
        }

        let up = (self.t as f32 * self.motion.pace() / WING_PERIOD).fract() < 0.5;
        let palette = [
            ('h', c.pigeon),
            ('b', c.pot),
            ('w', c.pigeon_wing),
            ('l', c.pot),
        ];
        for pigeon in &self.pigeons {
            let rows: Vec<String> = match (&pigeon.flight, up) {
                (Some(_), true) => WINGS_UP.iter().map(|r| r.to_string()).collect(),
                (Some(_), false) => WINGS_DOWN.iter().map(|r| r.to_string()).collect(),
                (None, _) if pigeon.facing_right => PERCHED.iter().map(|r| r.to_string()).collect(),
                (None, _) => super::paint::mirrored(&PERCHED),
            };
            let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
            canvas.sprite(
                pigeon.x.round() as i32,
                pigeon.y.round() as i32,
                &rows,
                &palette,
            );
        }
    }
}

/// A puff of smoke: a disc mixed over what is behind it, thinner towards its rim.
fn soft_disc(
    canvas: &mut Canvas,
    blender: &mut Blender,
    centre: (f32, f32),
    r: f32,
    colour: Rgb,
    strength: f32,
) {
    let reach = r.ceil() as i32 + 1;
    let (px, py) = (centre.0.round() as i32, centre.1.round() as i32);
    for y in py - reach..=py + reach {
        for x in px - reach..=px + reach {
            let d = ((x as f32 - centre.0).powi(2) + (y as f32 - centre.1).powi(2)).sqrt();
            if d <= r + 0.3 {
                let edge = if d > r - 1.0 { 0.6 } else { 1.0 };
                blender.blend(canvas, x, y, colour, strength * edge);
            }
        }
    }
}

impl Scene for Rooftops {
    fn name(&self) -> &'static str {
        "rooftops"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let (w, tall) = (self.width as i32, self.tall());
        // everything is laid out in proportion to the height, so the street keeps its shape
        // from a phone to a wide display
        let scale = (tall as f32 / 68.0).clamp(0.45, 2.2);
        self.sun = (w as f32 * 0.3, tall as f32 * 0.42);

        let far_base = tall as f32 * 0.46;
        self.skyline = vec![tall; w as usize];
        self.far_windows.clear();
        let mut x = 0;
        while x < w {
            let bw = ((rng.range(5.0, 12.0) * scale) as i32).max(3);
            let rise = (rng.range(0.02, 0.16) * tall as f32) as i32;
            let top = far_base as i32 - rise;
            let spire = rng.chance(0.12) && bw >= 5;
            for bx in x..(x + bw).min(w) {
                self.skyline[bx as usize] = top;
            }
            if spire {
                let mid = x + bw / 2;
                let tip = (8.0 * scale) as i32;
                for k in 0..tip {
                    let half = (tip - k) / 4;
                    for sx in mid - half..=mid + half {
                        if (0..w).contains(&sx) {
                            let row = top - k;
                            let i = sx as usize;
                            self.skyline[i] = self.skyline[i].min(row);
                        }
                    }
                }
            }
            // a few far windows, sparse so the towers stay calm
            let mut wy = top + 2;
            while wy < far_base as i32 + 6 {
                let mut wx = x + 1;
                while wx < x + bw - 1 {
                    if rng.chance(0.12) {
                        self.far_windows.push((wx, wy));
                    }
                    wx += 2;
                }
                wy += 3;
            }
            x += bw;
        }

        // the terrace across the middle
        self.houses.clear();
        self.windows.clear();
        self.pane = if scale >= 1.2 { 3 } else { 2 };
        let eaves_base = (tall as f32 * 0.54) as i32;
        let mut x = -(rng.below(6) as i32);
        while x < w {
            let hw = ((rng.range(12.0, 18.0) * scale) as i32).max(8);
            let eaves = eaves_base + rng.below(3) as i32;
            let ridge = eaves - ((hw as f32 * 0.3) as i32).clamp(3, (10.0 * scale) as i32 + 3);
            let chimney = (hw >= 10 && rng.chance(0.5))
                .then(|| x + 3 + rng.below((hw - 8).max(1) as usize) as i32);
            self.houses.push(House {
                x,
                w: hw,
                eaves,
                ridge,
                chimney,
            });
            let p = self.pane;
            let mut wy = eaves + 3;
            while wy + p < tall {
                let mut wx = x + 3;
                while wx + p < x + hw - 2 {
                    self.windows.push((wx, wy, rng.chance(0.35)));
                    wx += p + 3;
                }
                wy += p + 4;
            }
            x += hw;
        }

        // the near roofs, close enough to fill the bottom of the view
        self.roofs.clear();
        let ridge_base = (tall as f32 * 0.82) as i32;
        let mut x = -(rng.below(10) as i32);
        while x < w {
            let rw = ((rng.range(26.0, 44.0) * scale) as i32).max(14);
            let ridge = ridge_base - rng.below((4.0 * scale) as usize + 1) as i32;
            let mut stacks = Vec::new();
            if rw >= 14 {
                let count = if rw >= 34 && rng.chance(0.4) { 2 } else { 1 };
                for k in 0..count {
                    let sw = ((rng.range(6.0, 8.0) * scale) as i32).clamp(4, 14);
                    let sh = ((rng.range(9.0, 13.0) * scale) as i32).max(4);
                    let span = (rw / count).max(sw + 2);
                    let cx = x + k * span + 3 + rng.below((span - sw - 4).max(1) as usize) as i32;
                    let pots = ((sw - 1) / 3).max(1);
                    stacks.push((cx, sw, sh, pots, scale >= 0.8 && rng.chance(0.4)));
                }
            }
            let dormer = (rw >= 26 && scale >= 0.8 && rng.chance(0.6)).then(|| {
                // away from the stacks, so a stack never stands in front of a dormer
                let mut dx = x + 3 + rng.below((rw - 14).max(1) as usize) as i32;
                if stacks
                    .iter()
                    .any(|&(cx, sw, ..)| dx < cx + sw + 2 && dx + 11 > cx)
                {
                    dx = stacks
                        .first()
                        .map(
                            |&(cx, sw, ..)| {
                                if cx - x > rw / 2 { x + 3 } else { cx + sw + 3 }
                            },
                        )
                        .unwrap_or(dx);
                }
                dx
            });
            self.roofs.push(Roof {
                x,
                w: rw,
                ridge,
                stacks,
                dormer,
            });
            x += rw + 2;
        }

        self.pigeons.clear();
        let count = self.motion.scaled((self.width / 24).clamp(1, 6));
        for _ in 0..count {
            if let Some((px, py)) = self.perch(rng) {
                self.pigeons.push(Pigeon {
                    x: px,
                    y: py,
                    flight: None,
                    facing_right: rng.chance(0.5),
                });
            }
        }
        // smoke already on its way up, so the first frame is not bare
        self.puffs.clear();
        let tops = self.pot_tops();
        if !tops.is_empty() {
            for i in 0..(tops.len() * 6) {
                let (px, py) = tops[i % tops.len()];
                let age = (i / tops.len()) as f32 * 1.1 + rng.range(0.0, 0.5);
                self.puffs.push(Puff {
                    x: px as f32 + 0.7 * age,
                    y: py as f32 - 1.1 * age,
                    age,
                    seed: rng.next_u64() as u32,
                });
            }
        }
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let moved = dt * self.motion.speed();

        // one window at a time, never quicker than the gap; calm halves the pace
        self.next_window -= dt * self.motion.pace();
        if self.next_window <= 0.0 {
            self.next_window = WINDOW_GAP + rng.range(0.0, 2.0);
            if !self.windows.is_empty() {
                let i = rng.below(self.windows.len());
                self.windows[i].2 = !self.windows[i].2;
                self.changes += 1;
            }
        }

        self.next_puff -= moved;
        if self.next_puff <= 0.0 {
            self.next_puff = 1.1;
            for (x, y) in self.pot_tops() {
                self.puff_count = self.puff_count.wrapping_add(1);
                self.puffs.push(Puff {
                    x: x as f32,
                    y: y as f32,
                    age: 0.0,
                    seed: self.puff_count.wrapping_mul(2_654_435_761) ^ rng.next_u64() as u32,
                });
            }
        }
        for puff in &mut self.puffs {
            puff.age += moved;
            // smoke rises straight at first, then leans with the breeze as it slows
            let lean = (puff.age / SMOKE_LIFE).min(1.0);
            puff.y -= (1.2 - 0.5 * lean) * moved;
            puff.x += (0.3 + 1.2 * lean) * moved;
        }
        self.puffs.retain(|p| p.age < SMOKE_LIFE);

        self.next_flight -= moved;
        if self.next_flight <= 0.0 {
            self.next_flight = FLIGHT_GAP + rng.range(0.0, 8.0);
            if !self.pigeons.is_empty() {
                let i = rng.below(self.pigeons.len());
                if self.pigeons[i].flight.is_none() {
                    if let Some(to) = self.perch(rng) {
                        let from = (self.pigeons[i].x, self.pigeons[i].y);
                        self.pigeons[i].facing_right = to.0 >= from.0;
                        self.pigeons[i].flight = Some((from, to, 0.0));
                    }
                }
            }
        }
        for pigeon in &mut self.pigeons {
            if let Some((from, to, done)) = pigeon.flight {
                let distance = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2))
                    .sqrt()
                    .max(1.0);
                let done = (done + 7.0 * moved / distance).min(1.0);
                let arc = (done * std::f32::consts::PI).sin() * (distance * 0.25).min(10.0);
                pigeon.x = from.0 + (to.0 - from.0) * done;
                pigeon.y = from.1 + (to.1 - from.1) * done - arc;
                pigeon.flight = if done >= 1.0 {
                    None
                } else {
                    Some((from, to, done))
                };
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = Colours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours, theme.dark);
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
        check_scene("rooftops");
        check_determinism("rooftops");
    }

    #[test]
    fn passes_the_pixel_checks() {
        check_pixel_scene("rooftops", 6.0);
        check_palette(Colours::palette);
    }

    #[test]
    fn the_evening_stays_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            for (a, b, what) in [
                (c.sky_top, c.sky_low, "the sky"),
                (c.far, c.sky_low, "the skyline and the sky"),
                (c.far, c.wall, "the skyline and the terrace"),
                (c.roof, c.roof_rim, "a roof and its lit rim"),
                (c.near, c.near_tile, "the slates and their courses"),
                (c.wall, c.glass, "the walls and the dark windows"),
                (c.brick, c.near, "the chimneys and the roofs"),
                (c.brick, c.brick_lit, "the lit face of a stack"),
            ] {
                assert!(distance(a, b) >= 0.03, "{what} in {}", theme.name);
            }
        }
        for rows in [&PERCHED[..], &WINGS_UP[..], &WINGS_DOWN[..]] {
            assert!(rows.iter().all(|r| r.len() == 5));
        }
    }

    #[test]
    fn windows_change_one_at_a_time_and_pigeons_land_on_ridges() {
        for motion in Motion::ALL {
            let mut rng = Rng::new(6);
            let mut roofs = Rooftops::new(80, 24, motion, &mut rng);
            let mut times = Vec::new();
            let mut last = roofs.changes;
            for i in 0..(90 * 20) {
                roofs.step(0.05, &mut rng);
                if roofs.changes != last {
                    assert_eq!(roofs.changes, last + 1);
                    times.push(i as f32 * 0.05);
                    last = roofs.changes;
                }
            }
            let gap = WINDOW_GAP / motion.pace();
            for w in times.windows(2) {
                assert!(w[1] - w[0] >= gap - 0.06, "{motion:?}: {w:?}");
            }
            assert!(times.len() >= 5);
            for pigeon in roofs.pigeons.iter().filter(|p| p.flight.is_none()) {
                let ridge = roofs.ridge_at(pigeon.x as i32).expect("on a roof");
                assert_eq!(pigeon.y as i32, ridge - PERCHED.len() as i32);
            }
        }
    }

    #[test]
    fn every_chimney_pot_smokes() {
        let mut rng = Rng::new(4);
        let mut roofs = Rooftops::new(120, 34, Motion::FULL, &mut rng);
        for _ in 0..200 {
            roofs.step(0.05, &mut rng);
        }
        for (x, y) in roofs.pot_tops() {
            let near = roofs
                .puffs
                .iter()
                .any(|p| (p.x - x as f32).abs() < 4.0 && (p.y - y as f32).abs() < 4.0);
            assert!(near, "no smoke over the pot at {x},{y}");
        }
    }
}
