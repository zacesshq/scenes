//! Flood: a street of houses under a grey sky and a steady drizzle. The street is already awash
//! when the scene opens, with a parked car standing in the water to its hubs; over the first
//! minute the muddy water rises past the doors, the lower windows and the car's sills, then
//! settles into a gentle swell with debris bobbing along the surface.
//!
//! The sky, cloud, rain and water are tints of the background held below the non-text floor.
//! The houses, the car and the debris are solid, so with colour off the water line shows as the
//! place where the houses vanish.

use std::f32::consts::TAU;

use super::paint::{self, Blender, Drop, Overcast, clouds, ease, gradient, shaded, solid};
use super::{count_for, cycle, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

/// Seconds the water takes to reach its highest level at the standard pace.
pub const RISE_SECONDS: f32 = 45.0;
/// Seconds of one slow swell of the risen water. The surface moves at most twice a swell.
pub const SWELL_PERIOD: f32 = 12.0;
/// How far the swell lifts the water, in pixels.
const SWELL_PIXELS: f32 = 0.9;
/// The share of its highest level the water already stands at when the scene opens, so the
/// first glance shows a flooded street rather than a dry one.
const START_LEVEL: f32 = 0.35;
/// The most pixels the opening water stands at, low enough to leave the car's windows clear.
const START_PIXELS: f32 = 3.0;

/// A small hatchback seen side on, facing left: `r` the lit upper body, `R` its shaded lower
/// half, `w` the windows and `k` the tyres.
const CAR: [&str; 5] = [
    "...rrrrr...",
    "..rwwrwwr..",
    "rrrrrrrrrrr",
    "RRRRRRRRRRR",
    ".kk.....kk.",
];

/// The same car for a wide screen, where the small one would be lost.
const CAR_LARGE: [&str; 7] = [
    "....rrrrrrr.....",
    "...rwwwrwwwwr...",
    "..rwwwwrwwwwwr..",
    "rrrrrrrrrrrrrrrr",
    "RRRRRRRRRRRRRRRR",
    ".RkkkRRRRRRkkkR.",
    "..kkk......kkk..",
];

pub const SUMMARY: &str = "A street where muddy water rises slowly past the doors and a parked car under a grey drizzle, with debris bobbing on the surface";

pub const CAPTIONS: &[&str] = &[
    "The water is rising. Bail out the first error.",
    "Wet feet, not a lost house. Start with the top error.",
    "Sandbags first: one fix at a time.",
    "High water in the build. It goes down as fast.",
    "Keep your head above the error list.",
];

/// Debris sprites: a crate, a barrel, a plank, a ball and a tyre.
const DEBRIS: [&[&str]; 5] = [
    &["cCcC", "CcCc"],
    &["bb", "BB", "bb"],
    &["pppppp"],
    &[".o.", "ooo"],
    &[".tt.", "t..t", ".tt."],
];

#[derive(Clone, Debug)]
struct House {
    /// The left column.
    x: i32,
    width: i32,
    /// The height of the walls in pixels, under the roof.
    wall: i32,
    tone: usize,
}

#[derive(Clone, Debug)]
struct Debris {
    x: f32,
    speed: f32,
    sprite: usize,
    phase: f32,
}

/// The flood's colours in one theme.
struct Colours {
    overcast: Overcast,
    walls: [(Rgb, Rgb, Rgb); 4],
    roof: (Rgb, Rgb, Rgb),
    window: Rgb,
    frame: Rgb,
    door: Rgb,
    street: Ramp,
    water: Ramp,
    crest: Rgb,
    debris: [(Rgb, Rgb); 5],
    car: (Rgb, Rgb, Rgb),
    glass: Rgb,
}

impl Colours {
    fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let tone = |hue: Rgb| {
            let base = solid(theme, mix(theme.muted, hue, 0.45), shades.darkest(), 0.3);
            shaded(theme, base)
        };
        let muddy = mix(theme.blue(), theme.yellow(), 0.35);
        let (surface, deep) = if theme.dark {
            (shades.tint(muddy, 0.55, 2.4), shades.tint(muddy, 0.3, 1.5))
        } else {
            (shades.tint(muddy, 0.35, 1.8), shades.tint(muddy, 0.7, 2.7))
        };
        let crest = if theme.dark {
            shades.tint(theme.bright_cyan(), 0.7, 2.9)
        } else {
            mix(surface, theme.background, 0.6)
        };
        let (street_top, street_low) = if theme.dark {
            (
                shades.tint(theme.muted, 0.3, 1.7),
                shades.tint(theme.muted, 0.18, 1.35),
            )
        } else {
            (
                shades.tint(theme.muted, 0.55, 2.3),
                shades.tint(theme.muted, 0.75, 2.8),
            )
        };
        let pair = |c: Rgb| {
            let (dark, base, _) = shaded(theme, c);
            (base, dark)
        };
        Self {
            overcast: Overcast::new(theme),
            walls: [
                tone(theme.muted),
                tone(theme.yellow()),
                tone(theme.red()),
                tone(theme.cyan()),
            ],
            roof: shaded(theme, theme.red()),
            window: theme.yellow(),
            frame: theme.foreground,
            door: solid(theme, theme.muted, shades.darkest(), 0.55),
            street: Ramp::new(street_top, street_low, 4),
            water: Ramp::new(surface, deep, 8),
            crest,
            car: shaded(theme, theme.blue()),
            // glass a step from the body towards the light, so the cabin reads as windows and
            // never as a hole through the car
            glass: {
                let (_, body, light) = shaded(theme, theme.blue());
                if theme.dark {
                    mix(light, theme.foreground, 0.5)
                } else {
                    mix(body, theme.background, 0.45)
                }
            },
            debris: [
                pair(theme.yellow()),
                pair(theme.magenta()),
                pair(theme.foreground),
                pair(theme.bright_red()),
                pair(theme.muted),
            ],
        }
    }
}

pub struct Flood {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    seed: u32,
    /// Seconds of rising so far, kept across a resize so the water never drains away.
    risen: f32,
    houses: Vec<House>,
    car_x: Option<i32>,
    debris: Vec<Debris>,
    drops: Vec<Drop>,
}

impl Flood {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut flood = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            seed: (rng.next_u64() >> 32) as u32,
            risen: 0.0,
            houses: Vec::new(),
            car_x: None,
            debris: Vec::new(),
            drops: Vec::new(),
        };
        flood.resize(width, height, rng);
        flood
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The first pixel row of the street the houses stand on.
    fn ground(&self) -> i32 {
        (self.tall() - 2).max(1)
    }

    /// The most pixels the water rises: past the doors and the lower windows, always leaving the
    /// roofs and the sky clear.
    fn max_level(&self) -> f32 {
        let lowest = self.houses.iter().map(|h| h.wall).min().unwrap_or(6) as f32;
        (lowest * 0.62)
            .min(self.tall() as f32 * 0.4)
            .clamp(1.0, (self.tall() as f32 - 6.0).max(1.0))
    }

    /// The water's height above the street, in pixels, before rounding.
    fn level(&self) -> f32 {
        let start = (START_PIXELS / self.max_level().round().max(1.0)).min(START_LEVEL);
        let rise = start + (1.0 - start) * ease(self.risen / RISE_SECONDS);
        // the swell rises from a whole pixel and never quite reaches the next but one, so the
        // surface crosses one pixel boundary up and one down each swell
        let wave = cycle(self.t, TAU / SWELL_PERIOD * self.motion.pace()).sin();
        let swell = SWELL_PIXELS * rise * (1.0 + wave) / 2.0;
        (self.max_level().round() * rise + swell).max(0.0)
    }

    /// The pixel row of the water's surface. `None` before it has risen a pixel.
    fn surface(&self) -> Option<i32> {
        let rise = self.level().round() as i32;
        (rise >= 1).then(|| (self.ground() - rise).max(2))
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        let mut mixer = Blender::new(5);
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let ground = self.ground();
        let surface = self.surface();
        gradient(canvas, &c.overcast.sky, 0, ground);
        let drift = (self.t % 100_000.0) as f32 * 0.6 * self.motion.speed();
        clouds(
            canvas,
            &mut mixer,
            &c.overcast.cloud,
            -2,
            (tall as f32 * 0.32) as i32,
            drift,
            self.seed,
            0.7,
        );
        for y in ground..tall {
            canvas.fill_row(y, c.street.at((y - ground) as f32 / 2.0));
        }

        for house in &self.houses {
            let (dark, wall, light) = c.walls[house.tone % c.walls.len()];
            let top = ground - house.wall;
            let right = house.x + house.width - 1;
            canvas.fill_rect(house.x, top, house.width, house.wall, wall);
            // light from the left: a lit edge on one side and a shadow down the other
            canvas.fill_rect(house.x, top, 1, house.wall, light);
            canvas.fill_rect(right, top, 1, house.wall, dark);
            // a pitched roof overhanging the walls by a pixel, lit on its left slope
            let ridge = house.x + house.width / 2;
            let roof_rows = (house.width / 2).min(house.wall).max(2);
            for r in 0..roof_rows {
                let y = top - 1 - r;
                let half = house.width / 2 + 1 - r;
                for x in ridge - half..=ridge + half - i32::from(house.width % 2 == 0) {
                    let colour = if x < ridge { c.roof.2 } else { c.roof.1 };
                    canvas.set(x, y, colour);
                }
            }
            for x in house.x - 1..=right + 1 {
                canvas.set(x, top - 1, c.roof.0);
            }
            // a chimney rising out of the shaded slope
            let chimney = ridge + house.width / 4;
            let rows_there = (house.width / 2 + 1 - (chimney - ridge)).clamp(1, roof_rows);
            let slope = top - rows_there;
            canvas.fill_rect(chimney, slope - 3, 2, 4, dark);
            // the door in the middle and windows on two floors either side of it
            let door = house.x + house.width / 2 - 1;
            canvas.fill_rect(door, ground - 5, 3, 5, c.door);
            canvas.set(door + 2, ground - 3, c.frame);
            let mut wy = top + 2;
            while wy + 3 <= ground - 2 {
                for wx in [house.x + 2, right - 3] {
                    if wx > house.x && wx + 2 <= right && (wx + 2 < door || wx > door + 2) {
                        canvas.fill_rect(wx - 1, wy - 1, 4, 4, c.frame);
                        canvas.fill_rect(wx, wy, 2, 2, c.window);
                    }
                }
                wy += 6;
            }
        }

        // the car parked in the gap between the houses, its tyres on the street
        if let Some(x) = self.car_x {
            let (dark, body, _) = c.car;
            let tyre = c.door;
            let rows: &[&str] = if self.width >= 80 { &CAR_LARGE } else { &CAR };
            canvas.sprite(
                x - rows[0].len() as i32 / 2,
                ground - rows.len() as i32,
                rows,
                &[('r', body), ('R', dark), ('w', c.glass), ('k', tyre)],
            );
        }

        // the drizzle falls to the water or the street
        let floor = surface.unwrap_or(ground);
        // the rain blends in two steps only, which keeps the number of colours it adds small
        paint::rain(
            canvas,
            &mut Blender::new(2),
            &self.drops,
            &c.overcast,
            0.3,
            |_| floor,
        );

        let Some(surface) = surface else {
            return;
        };
        // under the water everything is muddied, more so the deeper it lies
        let depth = (ground - surface).max(1) as f32;
        for y in surface..tall {
            let d = ((y - surface) as f32 / depth).min(1.0);
            let water = c.water.at(d);
            for x in 0..wide {
                mixer.blend(canvas, x, y, water, 0.7 + 0.3 * d);
            }
        }
        // the surface catches the light in crests that drift along it
        let phase = cycle(self.t, 0.9 * self.motion.pace());
        for x in 0..wide {
            let v = (x as f32 * 0.45 - phase).sin() + 0.6 * (x as f32 * 0.17 + phase * 0.5).sin();
            let colour = if v > 0.6 { c.crest } else { c.water.at(0.0) };
            canvas.set(x, surface, colour);
        }
        for item in &self.debris {
            let rows = DEBRIS[item.sprite];
            let bob = (cycle(self.t, 1.3 * self.motion.pace()) + item.phase).sin();
            let y = surface - rows.len() as i32 + 1 + i32::from(bob > 0.3);
            let (light, dark) = c.debris[item.sprite];
            let palette = [
                ('c', light),
                ('C', dark),
                ('b', light),
                ('B', dark),
                ('p', light),
                ('o', light),
                ('t', dark),
            ];
            canvas.sprite(item.x.round() as i32, y, rows, &palette);
        }
    }
}

impl Scene for Flood {
    fn name(&self) -> &'static str {
        "flood"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let w = self.width as i32;
        let tall = self.tall();
        let tallest = ((tall as f32 * 0.5) as i32).clamp(8, 44);
        let narrow = self.width < 40;
        self.car_x = (self.width >= 24 && self.height >= 10).then_some(w / 5);
        // the car stands in a gap of its own, so it never sits across a house
        let half = if self.width >= 80 { 9 } else { 6 };
        let clearing = self.car_x.map(|x| (x - half, x + half));
        self.houses.clear();
        let mut x = rng.below(3) as i32;
        let mut tone = rng.below(4);
        while x < w {
            let width = if narrow {
                9 + rng.below(3) as i32
            } else {
                12 + rng.below(6) as i32
            };
            let mut width = width;
            if let Some((left, right)) = clearing {
                if x <= right && x + width > left {
                    // a narrower house still fits before the gap, so the street never starts
                    // with an empty lot
                    let room = left - 1 - x;
                    if room >= if narrow { 9 } else { 12 } && x < left {
                        width = room;
                    } else {
                        x = right + 1;
                        continue;
                    }
                }
            }
            let wall = (tallest - rng.below((tallest / 3).max(1) as usize + 1) as i32).max(8);
            tone += 1 + rng.below(2);
            self.houses.push(House {
                x,
                width,
                wall: wall.min((tall - 6).max(4)),
                tone,
            });
            x += width + 2 + rng.below(3) as i32;
        }
        let wanted = self
            .motion
            .scaled(count_for(self.width, self.height, 500, 2, 8));
        self.debris = (0..wanted)
            .map(|i| Debris {
                x: rng.range(0.0, self.width as f32),
                speed: rng.range(0.5, 1.4) * if i % 2 == 0 { 1.0 } else { -1.0 },
                sprite: rng.below(DEBRIS.len()),
                phase: rng.range(0.0, TAU),
            })
            .collect();
        let drops = self
            .motion
            .scaled(count_for(self.width, self.height, 40, 3, 160));
        let drops = if self.motion.calm { drops / 2 } else { drops };
        self.drops = (0..drops)
            .map(|_| Drop {
                x: rng.range(0.0, self.width as f32),
                y: rng.range(0.0, tall as f32),
                speed: rng.range(12.0, 18.0),
                near: rng.chance(0.3),
            })
            .collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        self.risen = (self.risen + dt * speed).min(RISE_SECONDS * 2.0);
        let width = self.width as f32;
        for item in &mut self.debris {
            item.x = wrap(item.x + item.speed * speed * dt + 8.0, width + 16.0) - 8.0;
        }
        let floor = self.surface().unwrap_or(self.ground()) as f32;
        for drop in &mut self.drops {
            drop.y += drop.speed * speed * dt;
            drop.x = wrap(drop.x + 1.6 * speed * dt, width);
            if drop.y >= floor {
                drop.y = -rng.range(0.0, 4.0);
                drop.x = rng.range(0.0, width);
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
        check_scene("flood");
        check_determinism("flood");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("flood", 40.0);
    }

    #[test]
    fn the_water_rises_slowly_then_holds_below_the_roofs() {
        let mut rng = Rng::new(4);
        let mut flood = Flood::new(80, 24, Motion::FULL, &mut rng);
        // the street is already awash, but no higher than the doors
        let first = flood.surface().expect("awash from the start");
        assert!(first >= flood.ground() - 5, "{first}");
        // the car's roof stands clear of the early water
        let theme = theme::default_theme();
        let start = render(&mut flood, 80, 24, 1);
        assert!(pixels(&start).contains(&Colours::new(theme).car.1));
        for _ in 0..(10 * 20) {
            flood.step(0.05, &mut rng);
        }
        let early = flood.surface().unwrap_or(flood.ground());
        assert!(early >= flood.ground() - 5, "{early}");
        // a simulated hour
        let mut rows = Vec::new();
        for _ in 0..(3600 * 20) {
            flood.step(0.05, &mut rng);
            rows.push(flood.surface().expect("risen"));
        }
        let top = flood.ground() - flood.max_level().round() as i32;
        for &row in &rows[rows.len() - 1200..] {
            assert!((top - 1..=top + 1).contains(&row), "{row} against {top}");
        }
        // the roofs stay above the water
        for house in &flood.houses {
            assert!(flood.ground() - house.wall < top, "{house:?}");
        }
        let frame = render(&mut flood, 80, 24, 1);
        assert!(pixels(&frame).contains(&Colours::new(theme).crest));
    }

    #[test]
    fn the_surface_moves_at_most_twice_a_swell() {
        let mut rng = Rng::new(9);
        let mut flood = Flood::new(60, 20, Motion::LIVELY, &mut rng);
        for _ in 0..(120 * 20) {
            flood.step(0.05, &mut rng);
        }
        let mut changes = 0;
        let mut last = flood.surface();
        let seconds = SWELL_PERIOD * 5.0;
        for _ in 0..((seconds * 20.0) as usize) {
            flood.step(0.05, &mut rng);
            if flood.surface() != last {
                changes += 1;
                last = flood.surface();
            }
        }
        assert!(changes <= 2 * 5 + 1, "{changes}");
    }

    #[test]
    fn a_resize_keeps_the_water_risen() {
        let mut rng = Rng::new(2);
        let mut flood = Flood::new(80, 24, Motion::FULL, &mut rng);
        for _ in 0..(90 * 20) {
            flood.step(0.05, &mut rng);
        }
        flood.resize(40, 12, &mut rng);
        flood.step(0.05, &mut rng);
        assert!(flood.surface().is_some());
    }

    #[test]
    fn the_layers_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            apart(
                theme,
                c.water.at(0.0),
                c.water.at(1.0),
                0.03,
                "shallow and deep water",
            );
            apart(
                theme,
                c.crest,
                c.water.at(0.0),
                0.03,
                "the crests and the water",
            );
            apart(
                theme,
                c.water.at(0.0),
                c.overcast.sky.at(1.0),
                0.03,
                "the water and the sky",
            );
            apart(
                theme,
                c.roof.1,
                c.overcast.sky.at(0.5),
                0.1,
                "the roofs and the sky",
            );
            apart(
                theme,
                c.window,
                c.walls[0].1,
                0.08,
                "the windows and the walls",
            );
            for colour in c.water.steps().iter().chain([&c.crest]) {
                assert!(
                    theme.contrast_of(*colour) < IMAGERY_CEILING,
                    "{}",
                    theme.name
                );
            }
            for (_, wall, _) in c.walls {
                assert!(
                    theme.contrast_of(wall) >= paint::SOLID - 0.01,
                    "{}",
                    theme.name
                );
            }
        }
    }
}
