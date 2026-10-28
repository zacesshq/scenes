//! The sea: fish of several sizes, bubbles, seaweed, a boat on the surface and, now and then, a
//! whale crossing the deep water.

use std::f32::consts::TAU;

use super::{count_for, wrap};
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Size {
    Small,
    Medium,
    Large,
}

impl Size {
    fn sprite(self, right: bool) -> &'static str {
        match (self, right) {
            (Size::Small, true) => "><>",
            (Size::Small, false) => "<><",
            (Size::Medium, true) => "><(o>",
            (Size::Medium, false) => "<o)><",
            (Size::Large, true) => "><(((o>",
            (Size::Large, false) => "<o)))><",
        }
    }

    fn len(self) -> f32 {
        match self {
            Size::Small => 3.0,
            Size::Medium => 5.0,
            Size::Large => 7.0,
        }
    }
}

#[derive(Clone, Debug)]
struct Fish {
    x: f32,
    y: f32,
    /// Cells per second, negative when swimming left.
    speed: f32,
    size: Size,
    colour: usize,
    wobble_phase: f32,
}

#[derive(Clone, Debug)]
struct Bubble {
    x: f32,
    y: f32,
    speed: f32,
    big: bool,
}

#[derive(Clone, Debug)]
struct Whale {
    x: f32,
    y: i32,
    speed: f32,
}

const WHALE_RIGHT: [&str; 4] = [
    "        __________    ",
    "   ,___/          \\__ ",
    "  <____  o          _)",
    "       \\__________/   ",
];
const WHALE_LEFT: [&str; 4] = [
    "    __________        ",
    " __/          \\___,   ",
    "(_          o  ____>  ",
    "   \\__________/       ",
];
const WHALE_WIDTH: f32 = 22.0;

const BOAT_TALL: [&str; 3] = ["   |\\", "   |_\\", " \\_____/"];
const BOAT_SHORT: [&str; 2] = ["  |\\", "\\___/"];

pub struct Sea {
    width: usize,
    height: usize,
    motion: Motion,
    t: f32,
    fish: Vec<Fish>,
    bubbles: Vec<Bubble>,
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

    fn surface_row(&self) -> i32 {
        if self.height >= 10 { 2 } else { 1 }
    }

    /// The first and last rows fish may swim in (`None` when the screen is too short).
    fn water(&self) -> Option<(i32, i32)> {
        let top = self.surface_row() + 1;
        let bottom = self.height as i32 - 2;
        (bottom >= top).then_some((top, bottom))
    }

    fn whale_fits(&self) -> bool {
        self.width >= 30 && self.water().is_some_and(|(top, bottom)| bottom - top >= 5)
    }

    fn spawn_fish(&self, rng: &mut Rng, enter_from_edge: bool) -> Option<Fish> {
        let (top, bottom) = self.water()?;
        let size = match rng.below(6) {
            0..=2 => Size::Small,
            3..=4 => Size::Medium,
            _ => Size::Large,
        };
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
        Some(Fish {
            x,
            y: rng.range(top as f32, bottom as f32 + 0.99),
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
}

impl Scene for Sea {
    fn name(&self) -> &'static str {
        "sea"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = count_for(self.width, self.height, 40, 2, 30);
        self.fish.clear();
        for _ in 0..wanted {
            if let Some(fish) = self.spawn_fish(rng, false) {
                self.fish.push(fish);
            }
        }
        self.bubbles.clear();
        self.seaweed.clear();
        if let Some((top, bottom)) = self.water() {
            let max_height = (((bottom - top) / 2) as usize).clamp(1, 3);
            let mut x = rng.below(4);
            while x < self.width {
                let height = 1 + rng.below(max_height);
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
        self.t += dt;
        let speed = self.motion.speed();
        let width = self.width as f32;

        // the boat drifts slowly and comes back round
        self.boat_x = wrap(self.boat_x + 0.8 * speed * dt + 9.0, width + 18.0) - 9.0;

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
                        y: fish.y,
                        speed: rng.range(1.2, 2.2) * speed,
                        big: false,
                    });
                }
            }
            let bed_rate =
                0.4 * (self.width as f32 / 40.0) * if self.motion.calm { 0.5 } else { 1.0 };
            if self.bubbles.len() < bubble_cap && rng.chance(bed_rate * dt) {
                self.bubbles.push(Bubble {
                    x: rng.range(0.0, width),
                    y: self.height as f32 - 2.0,
                    speed: rng.range(1.0, 1.8) * speed,
                    big: rng.chance(0.3),
                });
            }
            for bubble in &mut self.bubbles {
                bubble.y -= bubble.speed * dt;
                bubble.x += (self.t * 2.0 + bubble.y).sin() * 0.3 * dt;
            }
            let surface = top as f32;
            self.bubbles.retain(|b| b.y >= surface - 0.5);
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
                    y: bottom - 3,
                    speed: if right { 2.5 } else { -2.5 } * speed,
                });
                self.whales_seen += 1;
            }
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let surface = self.surface_row();
        let bed = self.height as i32 - 1;
        let speed = self.motion.speed();

        for x in 0..self.width as i32 {
            let v = (x as f32 * 0.8 + self.t * 2.0 * speed).sin();
            let ch = if v > 0.3 { '~' } else { '-' };
            frame.put(x, surface, ch, Some(theme.cyan()));
        }

        if bed > surface {
            for x in 0..self.width as i32 {
                let ch = match x % 5 {
                    0 => '.',
                    2 => ',',
                    3 => '\'',
                    _ => '.',
                };
                frame.put(x, bed, ch, Some(theme.yellow()));
            }
            for &(x, height, phase) in &self.seaweed {
                for i in 0..height as i32 {
                    let y = bed - 1 - i;
                    if y <= surface {
                        break;
                    }
                    let sway = (self.t * 0.8 * speed + phase + i as f32 * 0.9).sin();
                    let ch = if sway > 0.0 { '(' } else { ')' };
                    frame.put(x as i32, y, ch, Some(theme.green()));
                }
            }
        }

        for bubble in &self.bubbles {
            let ch = if bubble.big { 'O' } else { 'o' };
            frame.put(
                bubble.x.round() as i32,
                bubble.y.round() as i32,
                ch,
                Some(theme.muted),
            );
        }

        let colours = [
            theme.yellow(),
            theme.magenta(),
            theme.bright_cyan(),
            theme.red(),
            theme.bright_blue(),
        ];
        if let Some((top, bottom)) = self.water() {
            for fish in &self.fish {
                let wobble = (self.t * 1.3 * speed + fish.wobble_phase).sin() * 0.6;
                let y = (fish.y + wobble).round() as i32;
                let y = y.clamp(top, bottom);
                frame.put_str(
                    fish.x.round() as i32,
                    y,
                    fish.size.sprite(fish.speed > 0.0),
                    Some(colours[fish.colour % colours.len()]),
                );
            }
        }

        if let Some(whale) = &self.whale {
            let rows = if whale.speed > 0.0 {
                &WHALE_RIGHT
            } else {
                &WHALE_LEFT
            };
            frame.put_sprite(whale.x.round() as i32, whale.y, rows, Some(theme.blue()));
        }

        let boat_x = self.boat_x.round() as i32;
        if surface >= 2 {
            frame.put_sprite(boat_x, surface - 2, &BOAT_TALL, Some(theme.foreground));
        } else {
            frame.put_sprite(boat_x, surface - 1, &BOAT_SHORT, Some(theme.foreground));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("sea");
        check_determinism("sea");
    }

    #[test]
    fn the_surface_carries_the_boat_and_the_bed_grows_seaweed() {
        let mut rng = Rng::new(5);
        let mut sea = Sea::new(60, 20, Motion::FULL, &mut rng);
        let frame = render(&mut sea, 60, 20, 10);
        let surface = frame.row_text(2);
        assert!(
            surface.contains('~') && surface.contains("\\_____/"),
            "{surface}"
        );
        assert!(frame.row_text(19).contains('.'));
        let seaweed = (0..60)
            .filter(|&x| matches!(frame.get(x, 18).map(|c| c.ch), Some('(' | ')')))
            .count();
        assert!(seaweed >= 5, "{seaweed}");
        let sprites: Vec<&str> = ["><>", "<><", "><(o>", "<o)><", "><(((o>", "<o)))><"].to_vec();
        let text = frame.to_text();
        assert!(sprites.iter().any(|s| text.contains(s)), "{text}");
    }

    #[test]
    fn a_whale_crosses_a_big_enough_sea_and_never_a_small_one() {
        let mut rng = Rng::new(11);
        let mut sea = Sea::new(80, 24, Motion::FULL, &mut rng);
        let mut seen_in_frame = false;
        for _ in 0..(60 * 20) {
            sea.step(0.05, &mut rng);
            if sea.whale.is_some() {
                let mut frame = Frame::new(80, 24);
                sea.draw(&mut frame, theme::default_theme());
                seen_in_frame |= frame.to_text().contains("__________");
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
    fn fish_stay_in_the_water() {
        let mut rng = Rng::new(3);
        let mut sea = Sea::new(40, 12, Motion::CALM, &mut rng);
        for _ in 0..400 {
            sea.step(0.05, &mut rng);
            let (top, bottom) = sea.water().unwrap();
            for fish in &sea.fish {
                assert!(fish.y >= top as f32 && fish.y < bottom as f32 + 1.0);
            }
        }
        let frame = render(&mut sea, 40, 12, 1);
        assert_eq!(frame.row_text(0).trim(), frame.row_text(0).trim());
        assert!(frame.drawn() > 40);
    }
}
