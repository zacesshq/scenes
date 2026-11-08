//! The sea, painted in solid pixels: a sky gradient down to a horizon glow, water that deepens
//! as it goes down, wave highlights drifting across the surface, fish of several sizes, bubbles,
//! seaweed on a sandy bed, a boat on the surface and, now and then, a whale crossing the deep.
//!
//! Every colour comes from the active theme. The sky, water, sand and seaweed are tints of the
//! background held below the non-text floor, so they read as background; the fish, the boat and
//! the sun keep the theme's own drawing colours, so they stand out as shapes; they keep their
//! outline when colour is off.

use std::f32::consts::TAU;

use super::{count_for, cycle, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, IMAGERY_CEILING, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Size {
    Small,
    Medium,
    Large,
}

// fish swimming right, drawn at pixel resolution: `t` is the forked tail and the fins, `h` the
// lit back, `b` the body, `s` the shaded belly and `e` the eye. Every fin touches the body and
// the tail joins it through a narrow stem, so each fish is one solid shape with no stray pixels
const SMALL: [&str; 4] = ["....tt...", "tthhhhhh.", ".tbbbbbeb", "ttsssss.."];
const MEDIUM: [&str; 6] = [
    ".....ttt....",
    "tt..hhhhhh..",
    ".tthbbbbbbb.",
    "..tbbbbbbbeb",
    ".ttsssssss..",
    "tt...tt.....",
];
const LARGE: [&str; 8] = [
    "......tttt.......",
    "tt...hhhhhhhh....",
    ".tt.hhhhhhhhhhh..",
    "..ttbbbbbbbbbbbb.",
    "...tbbbbbbbbbbebb",
    "..ttsssssssssss..",
    ".tt..sssssssss...",
    "tt.....tt........",
];

impl Size {
    /// The pixel rows of a fish swimming right. A fish swimming left is the same, mirrored.
    fn rows(self) -> &'static [&'static str] {
        match self {
            Size::Small => &SMALL,
            Size::Medium => &MEDIUM,
            Size::Large => &LARGE,
        }
    }

    fn len(self) -> f32 {
        self.rows()[0].len() as f32
    }

    fn tall(self) -> i32 {
        self.rows().len() as i32
    }

    /// The row of the snout, where bubbles leave.
    fn mouth(self) -> i32 {
        self.rows()
            .iter()
            .position(|row| row.contains('e'))
            .unwrap_or(0) as i32
    }
}

const WHALE: [&str; 5] = [
    "#.......#######.....",
    "##...###########o#..",
    "####################",
    "##...#############..",
    "#.......########....",
];
const WHALE_WIDTH: f32 = 20.0;

const BOAT: [&str; 6] = [
    "....|.....",
    "....|s....",
    "....|ss...",
    "....|sss..",
    "hhhhhhhhhh",
    ".hhhhhhhh.",
];
const BOAT_WIDTH: f32 = 10.0;

/// Mirrors a sprite, for a fish or whale swimming left.
fn mirrored(rows: &[&str]) -> Vec<String> {
    rows.iter().map(|row| row.chars().rev().collect()).collect()
}

#[derive(Clone, Debug, PartialEq)]
struct Fish {
    x: f32,
    /// The top row of the sprite, in pixels.
    y: f32,
    /// Pixels per second, negative when swimming left.
    speed: f32,
    size: Size,
    colour: usize,
    wobble_phase: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct Bubble {
    x: f32,
    y: f32,
    speed: f32,
    big: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct Whale {
    x: f32,
    y: i32,
    speed: f32,
}

/// Steps in the depth and highlight ramps. Few steps keep the colour count low; so the escape
/// output, small while still reading as a smooth gradient.
const DEPTH_STEPS: usize = 12;
const HIGHLIGHT_STEPS: usize = 3;

/// The sea's colours in one theme, every one derived from the theme's own.
/// Some are kept only so the tests can check the bands against each other.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct SeaColours {
    pub sky_top: Rgb,
    pub sky_low: Rgb,
    pub glow: Rgb,
    pub surface: Rgb,
    pub deep: Rgb,
    pub highlight: Rgb,
    pub sand: Rgb,
    pub grain: Rgb,
    pub weed: Rgb,
    pub whale: Rgb,
    pub sun: Rgb,
    /// The soft ring around the sun on a light theme, where the sun is a pale disc.
    pub halo: Rgb,
    dark: bool,
    pub hull: Rgb,
    pub sail: Rgb,
    pub mast: Rgb,
    pub eye: Rgb,
    pub fish: [Rgb; 5],
    /// For each fish colour its lit back, body, shaded belly and fins.
    pub fish_shades: [[Rgb; 4]; 5],
    sky: Ramp,
    /// For each depth step, the highlight ramp from the water to the brightest highlight.
    water: Vec<Ramp>,
}

impl SeaColours {
    pub(crate) fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        // a night sky on a dark theme brightens towards the horizon; a day sky on a light theme
        // pales towards it, which also keeps it apart from the water below
        let (sky_top, sky_low) = if theme.dark {
            (
                shades.tint(theme.info, 0.16, 1.4),
                shades.tint(theme.info, 0.42, 2.0),
            )
        } else {
            (
                shades.tint(theme.info, 0.42, 2.0),
                shades.tint(theme.info, 0.14, 1.3),
            )
        };
        let glow = shades.tint(theme.warning, 0.5, 2.4);
        // deeper water is darker in every theme: on a dark theme it sinks towards the
        // background, on a light one it moves away from it
        let (surface, deep) = if theme.dark {
            (
                shades.tint(theme.blue(), 0.62, 2.6),
                shades.tint(theme.blue(), 0.22, 1.35),
            )
        } else {
            (
                shades.tint(theme.blue(), 0.3, 1.9),
                shades.tint(theme.blue(), 0.75, 2.8),
            )
        };
        let highlight = if theme.dark {
            mix(surface, theme.bright_cyan(), 0.55)
        } else {
            mix(surface, theme.background, 0.7)
        };
        let sand = shades.tint(theme.yellow(), 0.5, 2.6);
        let grain = mix(sand, deep, 0.4);
        let weed = shades.tint(theme.green(), 0.7, IMAGERY_CEILING - 0.2);
        let whale = mix(theme.muted, deep, 0.45);
        let fish = [
            theme.yellow(),
            theme.magenta(),
            theme.bright_cyan(),
            theme.red(),
            theme.bright_blue(),
        ];
        // the shading moves each body colour a little towards the light above and the deep water
        // below, held at the non-text floor so every part of a fish keeps its outline
        let fish_shades = fish.map(|body| {
            [
                shades.held(body, shades.lightest(), 0.35, IMAGERY_CEILING + 0.5),
                body,
                shades.held(body, deep, 0.3, IMAGERY_CEILING + 0.5),
                shades.held(body, deep, 0.5, IMAGERY_CEILING + 1.0),
            ]
        });
        let depth = Ramp::new(surface, deep, DEPTH_STEPS);
        let water = depth
            .steps()
            .iter()
            .map(|&base| Ramp::new(base, highlight, HIGHLIGHT_STEPS))
            .collect();
        Self {
            sky_top,
            sky_low,
            glow,
            surface,
            deep,
            highlight,
            sand,
            grain,
            weed,
            whale,
            // a daylit sky on a light theme has a pale sun in a warm halo, never a dark disc
            sun: if theme.dark {
                theme.yellow()
            } else {
                theme.background
            },
            halo: mix(theme.background, glow, 0.6),
            dark: theme.dark,
            hull: theme.red(),
            sail: theme.foreground,
            mast: theme.muted,
            eye: theme.background,
            fish,
            fish_shades,
            sky: Ramp::new(sky_top, sky_low, DEPTH_STEPS),
            water,
        }
    }

    /// The water at a depth from 0 at the surface to 1 at the bed, lifted by a highlight from 0
    /// to 1.
    fn water(&self, depth: f32, highlight: f32) -> Rgb {
        let i = (depth.clamp(0.0, 1.0) * (DEPTH_STEPS - 1) as f32).round() as usize;
        self.water[i.min(DEPTH_STEPS - 1)].at(highlight)
    }
}

pub struct Sea {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    fish: Vec<Fish>,
    bubbles: Vec<Bubble>,
    /// Seaweed as (column, height in pixels, sway phase).
    seaweed: Vec<(usize, usize, f32)>,
    whale: Option<Whale>,
    next_whale: f32,
    whales_seen: u32,
    boat_x: f32,
}

impl Sea {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut sea = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            fish: Vec::new(),
            bubbles: Vec::new(),
            seaweed: Vec::new(),
            whale: None,
            next_whale: rng.range(8.0, 20.0),
            whales_seen: 0,
            boat_x: 0.0,
        };
        sea.resize(width, height, rng);
        sea
    }

    /// The height in pixels, two to a row.
    fn pixels_tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The pixel row of the horizon, where the sky meets the water.
    fn horizon(&self) -> i32 {
        let tall = self.pixels_tall();
        ((tall as f32 * 0.3).round() as i32).clamp(1, (tall - 1).max(1))
    }

    /// The first pixel row of the sandy bed, below the water. A short screen has no bed.
    fn bed(&self) -> i32 {
        let tall = self.pixels_tall();
        if tall >= 12 { tall - 2 } else { tall }
    }

    /// The first and last pixel rows a fish may occupy (`None` when the screen is too short).
    fn water(&self) -> Option<(i32, i32)> {
        let top = self.horizon() + 2;
        let bottom = self.bed() - 2;
        (bottom - top + 1 >= Size::Small.tall()).then_some((top, bottom))
    }

    fn whale_fits(&self) -> bool {
        self.width >= 30
            && self
                .water()
                .is_some_and(|(top, bottom)| bottom - top + 1 >= WHALE.len() as i32 + 4)
    }

    fn spawn_fish(&self, rng: &mut Rng, enter_from_edge: bool) -> Option<Fish> {
        let (top, bottom) = self.water()?;
        let mut size = match rng.below(6) {
            0..=2 => Size::Small,
            3..=4 => Size::Medium,
            _ => Size::Large,
        };
        if bottom - top + 1 < size.tall() {
            size = Size::Small;
        }
        let right = rng.chance(0.5);
        let base = match size {
            Size::Small => rng.range(3.0, 6.0),
            Size::Medium => rng.range(2.5, 4.5),
            Size::Large => rng.range(1.5, 3.0),
        };
        let speed = if right { base } else { -base } * self.motion.speed();
        let x = if enter_from_edge {
            if right {
                -size.len()
            } else {
                self.width as f32
            }
        } else {
            rng.range(0.0, self.width as f32)
        };
        let lowest = (bottom - size.tall() + 1) as f32;
        Some(Fish {
            x,
            y: rng.range(top as f32, lowest + 0.99),
            speed,
            size,
            colour: rng.below(5),
            wobble_phase: rng.range(0.0, TAU),
        })
    }

    #[cfg(test)]
    pub(crate) fn whales_seen(&self) -> u32 {
        self.whales_seen
    }

    fn paint(&self, canvas: &mut Canvas, colours: &SeaColours) {
        let horizon = self.horizon();
        let bed = self.bed();
        let tall = canvas.height() as i32;
        let wide = canvas.width() as i32;
        let pace = self.motion.pace();
        let speed = self.motion.speed();

        // the sky, darkest at the top and warming to a glow just above the horizon
        for y in 0..horizon.min(tall) {
            let t = y as f32 / horizon.max(1) as f32;
            let colour = if y == horizon - 1 && horizon >= 4 {
                mix(colours.sky_low, colours.glow, 0.35)
            } else {
                colours.sky.at(t)
            };
            canvas.fill_row(y, colour);
        }
        // a still sun: it never changes, so it can never flash
        if horizon >= 8 && wide >= 20 {
            let (cx, cy) = ((wide as f32 * 0.8) as i32, horizon * 2 / 5);
            let r = (horizon / 5).clamp(1, 4);
            for dy in -r - 1..=r + 1 {
                for dx in -r - 1..=r + 1 {
                    let d = ((dx * dx + dy * dy) as f32).sqrt();
                    if d <= r as f32 {
                        canvas.set(cx + dx, cy + dy, colours.sun);
                    } else if d <= r as f32 + 1.0 {
                        if colours.dark {
                            canvas.blend(cx + dx, cy + dy, colours.sun, 0.35);
                        } else {
                            canvas.set(cx + dx, cy + dy, colours.halo);
                        }
                    }
                }
            }
        }

        // the water, deepening downwards, with highlights that drift across it and fade with
        // depth; fewer and slower under reduced motion
        let phase = cycle(self.t, 0.8 * pace);
        let threshold = if self.motion.calm { 1.25 } else { 1.05 };
        let span = (bed - horizon).max(1) as f32;
        for y in horizon..bed.min(tall) {
            let depth = (y - horizon) as f32 / span;
            let fade = (1.0 - depth).powf(1.5);
            for x in 0..wide {
                let (xf, yf) = (x as f32, y as f32);
                let v = (xf * 0.31 + yf * 1.3 - phase).sin()
                    + 0.7 * (xf * 0.17 - yf * 0.8 + phase * 0.6 + 1.0).sin();
                let mut lift = ((v - threshold) / 0.45).clamp(0.0, 1.0) * fade;
                if y == horizon {
                    // the surface line always catches some light
                    lift = lift.max(0.5);
                }
                canvas.set(x, y, colours.water(depth.powf(0.8), lift));
            }
        }

        // the sandy bed with a fixed scatter of grains
        for y in bed..tall {
            for x in 0..wide {
                let h = (x as u32).wrapping_mul(0x9e37_79b1) ^ (y as u32).wrapping_mul(0x85eb_ca77);
                let colour = if (h >> 7) % 5 == 0 {
                    colours.grain
                } else {
                    colours.sand
                };
                canvas.set(x, y, colour);
            }
        }
        if bed < tall {
            for &(x, height, sway_phase) in &self.seaweed {
                for i in 0..height as i32 {
                    let y = bed - 1 - i;
                    if y <= horizon {
                        break;
                    }
                    let sway = (cycle(self.t, 0.8 * speed) + sway_phase + i as f32 * 0.6).sin();
                    let dx = (sway * i as f32 / height as f32 * 1.2).round() as i32;
                    canvas.set(x as i32 + dx, y, colours.weed);
                }
            }
        }

        for bubble in &self.bubbles {
            let (x, y) = (bubble.x.round() as i32, bubble.y.round() as i32);
            canvas.set(x, y, colours.highlight);
            if bubble.big {
                canvas.set(x, y - 1, colours.highlight);
            }
        }

        if let Some(whale) = &self.whale {
            let rows = if whale.speed > 0.0 {
                WHALE.iter().map(|r| r.to_string()).collect()
            } else {
                mirrored(&WHALE)
            };
            let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
            let palette = [('#', colours.whale), ('o', colours.eye)];
            canvas.sprite(whale.x.round() as i32, whale.y, &rows, &palette);
        }

        if let Some((top, bottom)) = self.water() {
            // smaller fish swim further off, so a big fish passing one is drawn in front of it
            let mut order: Vec<&Fish> = self.fish.iter().collect();
            order.sort_by_key(|f| f.size as u8);
            for fish in order {
                let wobble = (cycle(self.t, 1.3 * pace) + fish.wobble_phase).sin();
                let lowest = bottom - fish.size.tall() + 1;
                let y = ((fish.y + wobble).round() as i32).clamp(top, lowest.max(top));
                let rows: Vec<String> = if fish.speed > 0.0 {
                    fish.size.rows().iter().map(|r| r.to_string()).collect()
                } else {
                    mirrored(fish.size.rows())
                };
                let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
                let [lit, body, belly, fin] =
                    colours.fish_shades[fish.colour % colours.fish_shades.len()];
                canvas.sprite(
                    fish.x.round() as i32,
                    y,
                    &rows,
                    &[
                        ('h', lit),
                        ('b', body),
                        ('s', belly),
                        ('t', fin),
                        ('e', colours.eye),
                    ],
                );
            }
        }

        // the boat sits on the surface, its hull dipping one pixel into the water
        let palette = [
            ('h', colours.hull),
            ('s', colours.sail),
            ('|', colours.mast),
        ];
        canvas.sprite(
            self.boat_x.round() as i32,
            horizon - BOAT.len() as i32 + 2,
            &BOAT,
            &palette,
        );
    }
}

impl Scene for Sea {
    fn name(&self) -> &'static str {
        "sea"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = count_for(self.width, self.height, 90, 2, 16);
        self.fish.clear();
        for _ in 0..wanted {
            if let Some(fish) = self.spawn_fish(rng, false) {
                self.fish.push(fish);
            }
        }
        self.bubbles.clear();
        self.seaweed.clear();
        if let Some((top, bottom)) = self.water() {
            let max_height = ((bottom - top) as usize).clamp(2, 7);
            let mut x = rng.below(4);
            while x < self.width {
                let height = 2 + rng.below(max_height - 1);
                self.seaweed.push((x, height, rng.range(0.0, TAU)));
                x += 4 + rng.below(5);
            }
        }
        if !self.whale_fits() {
            self.whale = None;
        }
        self.boat_x = rng.range(0.0, self.width as f32);
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        let width = self.width as f32;

        // the boat drifts slowly and comes back round
        self.boat_x = wrap(
            self.boat_x + 0.8 * speed * dt + BOAT_WIDTH,
            width + 2.0 * BOAT_WIDTH,
        ) - BOAT_WIDTH;

        for i in 0..self.fish.len() {
            let fish = &mut self.fish[i];
            fish.x += fish.speed * dt;
            let len = fish.size.len();
            if fish.x > width + 1.0 || fish.x < -len - 1.0 {
                if let Some(fresh) = self.spawn_fish(rng, true) {
                    self.fish[i] = fresh;
                }
            }
        }

        let bubble_cap = count_for(self.width, self.height, 30, 2, 40);
        if let Some((top, _)) = self.water() {
            let rate = if self.motion.calm { 0.05 } else { 0.15 };
            for i in 0..self.fish.len() {
                if self.bubbles.len() < bubble_cap && rng.chance(rate * dt) {
                    let fish = &self.fish[i];
                    let mouth = if fish.speed > 0.0 {
                        fish.x + fish.size.len()
                    } else {
                        fish.x - 1.0
                    };
                    self.bubbles.push(Bubble {
                        x: mouth,
                        y: fish.y + fish.size.mouth() as f32,
                        speed: rng.range(1.4, 2.4) * speed,
                        big: false,
                    });
                }
            }
            let bed_rate =
                0.4 * (self.width as f32 / 40.0) * if self.motion.calm { 0.5 } else { 1.0 };
            if self.bubbles.len() < bubble_cap && rng.chance(bed_rate * dt) {
                self.bubbles.push(Bubble {
                    x: rng.range(0.0, width),
                    y: self.bed() as f32 - 1.0,
                    speed: rng.range(1.2, 2.0) * speed,
                    big: rng.chance(0.3),
                });
            }
            for bubble in &mut self.bubbles {
                bubble.y -= bubble.speed * dt;
                bubble.x += (cycle(self.t, 2.0 * speed) + bubble.y / 2.0).sin() * 0.3 * dt;
            }
            let surface = (top - 1) as f32;
            self.bubbles.retain(|b| b.y >= surface);
        }

        if let Some(whale) = &mut self.whale {
            whale.x += whale.speed * dt;
            if whale.x > width + 2.0 || whale.x < -WHALE_WIDTH - 2.0 {
                self.whale = None;
                self.next_whale = rng.range(20.0, 50.0);
            }
        } else if self.whale_fits() {
            self.next_whale -= dt;
            if self.next_whale <= 0.0 {
                let right = rng.chance(0.5);
                let (_, bottom) = self.water().expect("checked by whale_fits");
                self.whale = Some(Whale {
                    x: if right { -WHALE_WIDTH } else { width },
                    y: bottom - WHALE.len() as i32 - 1,
                    speed: if right { 2.5 } else { -2.5 } * speed,
                });
                self.whales_seen += 1;
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = SeaColours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colour::xterm_colour;
    use crate::flash::{MAX_CHANGES_PER_SECOND, most_changes_per_second, regions};
    use crate::pixels::distance;
    use crate::scenes::testing::{
        check_determinism, check_pixel_scene, check_scene, max_changes_per_second, render,
    };
    use crate::terminal::pixel_256;
    use crate::theme;

    fn drawn(sea: &Sea, width: usize, height: usize, theme: &ThemeData) -> Frame {
        let mut frame = Frame::new(width, height);
        sea.draw(&mut frame, theme);
        frame
    }

    /// Every pixel in a frame, top then bottom of each cell.
    fn pixels(frame: &Frame) -> Vec<Rgb> {
        frame
            .cells()
            .iter()
            .filter_map(|c| c.pixel_pair())
            .flat_map(|(a, b)| [a, b])
            .collect()
    }

    #[test]
    fn passes_the_shared_checks() {
        check_scene("sea");
        check_determinism("sea");
        check_pixel_scene("sea", 6.0);
    }

    #[test]
    fn the_same_seed_paints_the_same_pixels() {
        let run = || {
            let mut rng = Rng::new(2024);
            let mut sea = Sea::new(60, 20, Motion::FULL, &mut rng);
            for _ in 0..200 {
                sea.step(0.05, &mut rng);
            }
            drawn(&sea, 60, 20, theme::default_theme())
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn fish_have_a_forked_tail_a_dorsal_fin_an_eye_and_face_their_way() {
        for size in [Size::Small, Size::Medium, Size::Large] {
            let rows = size.rows();
            let eye = rows.iter().position(|r| r.contains('e')).unwrap();
            let eye_x = rows[eye].find('e').unwrap();
            // the eye is in the front third and the tail fins at the back
            assert!(eye_x * 3 >= rows[0].len() * 2, "{rows:?}");
            let tail_tips: Vec<usize> = rows
                .iter()
                .enumerate()
                .filter(|(_, r)| r.starts_with('t'))
                .map(|(i, _)| i)
                .collect();
            // the tail is forked: its tips sit above and below the eye's row, not on it
            assert!(tail_tips.iter().any(|&i| i < eye) && tail_tips.iter().any(|&i| i > eye));
            assert!(!rows[eye].starts_with('t'), "{rows:?}");
            // a dorsal fin rises above the body
            assert!(rows[0].contains('t') && !rows[0].contains('b'), "{rows:?}");
            // the body tapers: the eye's row is the longest stretch of body
            let body = |r: &str| r.chars().filter(|c| "hbse".contains(*c)).count();
            assert!(
                rows.iter().all(|r| body(r) <= body(rows[eye]) + 1),
                "{rows:?}"
            );
            assert_eq!(size.mouth() as usize, eye);
        }
        assert!(Size::Small.len() < Size::Medium.len() && Size::Medium.len() < Size::Large.len());
    }

    #[test]
    fn sprites_are_rectangular() {
        for rows in [&SMALL[..], &MEDIUM[..], &LARGE[..], &WHALE[..], &BOAT[..]] {
            assert!(rows.iter().all(|r| r.len() == rows[0].len()), "{rows:?}");
        }
        assert_eq!(WHALE[0].len() as f32, WHALE_WIDTH);
        assert_eq!(BOAT[0].len() as f32, BOAT_WIDTH);
        assert_eq!(mirrored(&["ab."]), vec![".ba".to_string()]);
    }

    #[test]
    fn the_ramps_stay_distinct_and_in_order_in_every_theme() {
        for theme in theme::all() {
            let c = SeaColours::new(theme);
            let name = theme.name;
            let apart = |a: Rgb, b: Rgb, what: &str| {
                assert!(
                    distance(a, b) >= 0.03,
                    "{what} collapse in {name}: {} and {} are {:.3} apart",
                    a.hex(),
                    b.hex(),
                    distance(a, b)
                );
            };
            apart(c.sky_top, c.sky_low, "the sky's top and bottom");
            apart(c.sky_low, c.surface, "the sky and the water");
            apart(c.glow, c.sky_low, "the horizon glow and the sky");
            apart(c.surface, c.deep, "the shallow and deep water");
            apart(c.surface, c.highlight, "the water and its highlights");
            apart(c.deep, c.sand, "the deep water and the sand");
            apart(
                c.highlight,
                theme.background,
                "the highlights and the background",
            );
            apart(c.sky_top, theme.background, "the sky and the background");
            // deeper water is darker and highlights are lighter, on dark and light themes alike
            assert!(c.deep.luminance() < c.surface.luminance(), "{name}");
            assert!(c.highlight.luminance() > c.surface.luminance(), "{name}");
            assert!(c.highlight.contrast(c.surface) >= 1.2, "{name}");
            assert!(c.surface.contrast(c.deep) >= 1.2, "{name}");
            // the background imagery reads as background: below the non-text floor
            for (what, colour) in [
                ("sky top", c.sky_top),
                ("sky low", c.sky_low),
                ("glow", c.glow),
                ("surface", c.surface),
                ("deep", c.deep),
                ("sand", c.sand),
                ("weed", c.weed),
            ] {
                assert!(
                    theme.contrast_of(colour) < IMAGERY_CEILING,
                    "the {what} in {name} is {:.2}:1",
                    theme.contrast_of(colour)
                );
            }
            // the shapes keep the theme's floor, so they stand out from it and from the water
            let sun = theme.dark.then_some(&c.sun);
            for colour in c.fish.iter().chain([&c.hull, &c.sail]).chain(sun) {
                assert!(theme.contrast_of(*colour) >= theme.floor(), "{name}");
                assert!(distance(*colour, c.surface) >= 0.1, "{name}");
                assert!(distance(*colour, c.deep) >= 0.1, "{name}");
            }
            assert!(
                distance(c.whale, c.deep) >= 0.08,
                "the whale vanishes in {name}"
            );
            // every part of a fish keeps its outline and the shading is a real step
            for [lit, body, belly, fin] in c.fish_shades {
                for part in [lit, body, belly, fin] {
                    assert!(theme.contrast_of(part) >= IMAGERY_CEILING, "{name}");
                    assert!(
                        distance(part, c.surface) >= 0.08,
                        "{name}: {} by {}",
                        part.hex(),
                        c.surface.hex()
                    );
                }
                assert!(
                    distance(lit, belly) >= 0.03,
                    "the shading is flat in {name}"
                );
            }
        }
    }

    #[test]
    fn every_theme_paints_a_sky_a_horizon_and_deepening_water() {
        for theme in theme::all() {
            let c = SeaColours::new(theme);
            let mut rng = Rng::new(4);
            let mut sea = Sea::new(80, 24, Motion::FULL, &mut rng);
            for _ in 0..100 {
                sea.step(0.05, &mut rng);
            }
            let frame = drawn(&sea, 80, 24, theme);
            let used = {
                let mut p = pixels(&frame);
                p.sort_unstable();
                p.dedup();
                p
            };
            assert!(
                used.len() >= 20,
                "{} uses {} colours",
                theme.name,
                used.len()
            );
            assert!(
                used.len() <= 120,
                "{} uses {} colours",
                theme.name,
                used.len()
            );
            // the top-left cell is sky; the water below the horizon gets darker downwards
            let at = |y: usize| {
                let (top, bottom) = frame.get(2, y as i32).unwrap().pixel_pair().unwrap();
                (top, bottom)
            };
            assert_eq!(at(0).0, c.sky_top, "{}", theme.name);
            let horizon = sea.horizon() as usize;
            let shallow = c.water(((2.0 / 34.0) as f32).powf(0.8), 0.0);
            let deep_row = (sea.bed() as usize - 1) / 2;
            let (_, deep_pixel) = at(deep_row);
            assert!(
                deep_pixel.luminance() < shallow.luminance(),
                "{}: {} is not darker than {}",
                theme.name,
                deep_pixel.hex(),
                shallow.hex()
            );
            assert!(horizon > 0);
            // the bed is sand
            let (_, sand) = at(23);
            assert!(sand == c.sand || sand == c.grain, "{}", theme.name);
        }
    }

    #[test]
    fn text_over_the_sea_keeps_the_floor_in_every_theme() {
        use crate::catalogue::Catalogue;
        use crate::cli::SnapshotState;
        use crate::snapshot::Shot;
        let catalogue = Catalogue::built_in();
        for theme in theme::all() {
            for state in [SnapshotState::Pass, SnapshotState::Fail] {
                let shot = Shot {
                    catalogue: &catalogue,
                    scene: "sea",
                    theme,
                    size: (80, 24),
                    seed: 1,
                    motion: Motion::FULL,
                    state: Some(state),
                    clock: crate::clock::SNAPSHOT_START,
                };
                let frame = shot.still(3.0).unwrap();
                let mut text = 0;
                for cell in frame.cells().iter().filter(|c| !c.is_pixel()) {
                    if cell.ch == ' ' {
                        continue;
                    }
                    text += 1;
                    let fg = cell.colour.unwrap_or(theme.foreground);
                    let bg = cell.bg.unwrap_or(theme.background);
                    assert!(
                        fg.contrast(bg) >= theme.floor(),
                        "{:?} in {} is {:.2}:1",
                        cell.ch,
                        theme.name,
                        fg.contrast(bg)
                    );
                }
                assert!(text > 10, "no banner or caption in {}", theme.name);
            }
        }
    }

    #[test]
    fn the_256_colour_fallback_stays_close_and_keeps_the_bands_apart() {
        for theme in theme::all() {
            let c = SeaColours::new(theme);
            let near = |colour: Rgb| xterm_colour(pixel_256(colour, theme.background));
            let mut rng = Rng::new(8);
            let mut sea = Sea::new(80, 24, Motion::FULL, &mut rng);
            for _ in 0..60 {
                sea.step(0.05, &mut rng);
            }
            let frame = drawn(&sea, 80, 24, theme);
            let mut worst = 0.0_f32;
            for colour in pixels(&frame) {
                worst = worst.max(distance(colour, near(colour)));
            }
            assert!(worst <= 0.15, "{} is {worst:.3} off at worst", theme.name);
            // the table is always at least as close as the theme's own colours would be
            let mut palette = theme.drawable();
            palette.push(theme.background);
            let nearest_theme = |colour: Rgb| {
                palette
                    .iter()
                    .map(|&p| distance(colour, p))
                    .fold(f32::MAX, f32::min)
            };
            for colour in [c.sky_top, c.sky_low, c.surface, c.deep, c.highlight, c.sand] {
                assert!(
                    distance(colour, near(colour)) <= nearest_theme(colour) + 0.01,
                    "{} in {}",
                    colour.hex(),
                    theme.name
                );
            }
            // the main bands still read as different things
            for (a, b, what) in [
                (c.sky_low, c.surface, "sky and water"),
                (c.surface, c.highlight, "water and highlight"),
                (c.deep, c.sand, "deep water and sand"),
            ] {
                assert_ne!(near(a), near(b), "{what} collapse in {}", theme.name);
            }
            for fish in c.fish {
                assert_ne!(near(fish), near(c.surface), "{}", theme.name);
            }
        }
    }

    #[test]
    fn every_theme_survives_tiny_and_huge_sizes() {
        let sizes = [
            (1, 1),
            (2, 2),
            (5, 3),
            (20, 8),
            (33, 9),
            (300, 90),
            (400, 120),
        ];
        for theme in theme::all() {
            let mut rng = Rng::new(6);
            let mut sea = Sea::new(80, 24, Motion::LIVELY, &mut rng);
            for &(w, h) in sizes.iter().chain(sizes.iter().rev()) {
                sea.resize(w, h, &mut rng);
                for _ in 0..30 {
                    sea.step(0.05, &mut rng);
                }
                let frame = drawn(&sea, w, h, theme);
                assert!(frame.drawn() > 0, "{w}x{h} in {}", theme.name);
            }
        }
    }

    #[test]
    fn calm_and_the_flash_limits_hold_in_every_theme() {
        for theme in theme::all() {
            for motion in Motion::ALL {
                let mut rng = Rng::new(12);
                let mut sea = Sea::new(60, 20, motion, &mut rng);
                let mut measured = Vec::new();
                for _ in 0..(20 * 20) {
                    sea.step(0.05, &mut rng);
                    measured.push(regions(&drawn(&sea, 60, 20, theme), theme));
                }
                let most = most_changes_per_second(&measured, 20);
                assert!(
                    most <= MAX_CHANGES_PER_SECOND,
                    "{} {motion:?}: {most}",
                    theme.name
                );
            }
        }
    }

    #[test]
    fn reduced_motion_changes_less_and_more_slowly() {
        let theme = theme::default_theme();
        let run = |motion: Motion| {
            let mut rng = Rng::new(9);
            let mut sea = Sea::new(60, 20, motion, &mut rng);
            (0..120)
                .map(|_| {
                    sea.step(0.05, &mut rng);
                    drawn(&sea, 60, 20, theme)
                })
                .collect::<Vec<_>>()
        };
        let changed = |frames: &[Frame]| -> usize {
            frames
                .windows(2)
                .map(|w| {
                    w[0].cells()
                        .iter()
                        .zip(w[1].cells())
                        .filter(|(a, b)| a != b)
                        .count()
                })
                .sum()
        };
        let full = run(Motion::FULL);
        let calm = run(Motion::CALM);
        assert!(
            changed(&calm) * 3 < changed(&full) * 2,
            "{} vs {}",
            changed(&calm),
            changed(&full)
        );
        // the fastest any one cell changes is no faster under reduced motion; the shapes that
        // cross a cell are moving, so the region meter above is what holds them
        assert!(max_changes_per_second(&calm, 20) <= max_changes_per_second(&full, 20));
    }

    #[test]
    fn a_whale_crosses_a_big_enough_sea_and_never_a_small_one() {
        let mut rng = Rng::new(11);
        let mut sea = Sea::new(80, 24, Motion::FULL, &mut rng);
        let mut seen_in_frame = false;
        for _ in 0..(60 * 20) {
            sea.step(0.05, &mut rng);
            if sea.whale.is_some() {
                let c = SeaColours::new(theme::default_theme());
                let frame = drawn(&sea, 80, 24, theme::default_theme());
                seen_in_frame |= pixels(&frame).contains(&c.whale);
            }
        }
        assert!(sea.whales_seen() >= 1);
        assert!(seen_in_frame);

        let mut small = Sea::new(20, 8, Motion::FULL, &mut rng);
        for _ in 0..(60 * 20) {
            small.step(0.05, &mut rng);
        }
        assert_eq!(small.whales_seen(), 0);
    }

    #[test]
    fn fish_stay_in_the_water_and_the_boat_rides_the_surface() {
        let mut rng = Rng::new(3);
        let mut sea = Sea::new(40, 12, Motion::CALM, &mut rng);
        for _ in 0..400 {
            sea.step(0.05, &mut rng);
            let (top, bottom) = sea.water().unwrap();
            for fish in &sea.fish {
                assert!(fish.y >= top as f32);
                assert!(fish.y < (bottom - fish.size.tall() + 2) as f32);
            }
        }
        let theme = theme::default_theme();
        let c = SeaColours::new(theme);
        let mut sea = Sea::new(60, 20, Motion::FULL, &mut rng);
        sea.boat_x = 20.0;
        let frame = render(&mut sea, 60, 20, 1);
        let all = pixels(&frame);
        assert!(all.contains(&c.hull) && all.contains(&c.sail));
        assert!(c.fish.iter().any(|f| all.contains(f)));
    }
}
