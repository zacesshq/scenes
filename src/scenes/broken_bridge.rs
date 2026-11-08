//! Broken bridge: a concrete bridge on stone piers over a river, its middle span gone. Bent
//! girders hang from the jagged broken ends, the fallen span lies tilted in the water, a
//! striped barrier with a BRIDGE OUT board on a post stops the road and a car waits behind
//! them. The river keeps
//! flowing under a grey sky and now and then a fragment drops from an end and splashes.
//!
//! The sky, the hills, the banks and the river are tints of the background held below the
//! non-text floor. The bridge, the barrier, the board and the car are solid, so with colour off
//! the gap in the deck still shows.

use super::paint::{self, Blender, Overcast, clouds, gradient, line, shaded, solid, unit};
use super::{cycle, wrap};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::{error_colour, warning_colour};

pub const SUMMARY: &str = "A concrete bridge with its middle span gone, girders hanging, a striped barrier and a waiting car over a river";

pub const CAPTIONS: &[&str] = &[
    "A gap in the build. Bridge the first error.",
    "The road stops here for now. Read the sign below.",
    "One missing piece. The rest is still standing.",
    "Something did not connect. Check the first error.",
    "Closed for repairs. The list below is the plan.",
];

/// The words on the board above the barrier.
const BOARD_TEXT: &str = "BRIDGE OUT";

/// The car on the deck, facing the gap: body, windows, wheels and its lights.
const CAR: [&str; 5] = [
    "...bbbbb....",
    "..bwwbwwb...",
    ".bbbbbbbbbbb",
    "rbbbbbbbbbbh",
    "..kk....kk..",
];
/// Seconds between two fragments falling from a broken end at the standard pace.
pub const FRAGMENT_PERIOD: f32 = 4.0;
/// How long a splash shows where a fragment lands. Its pixels change twice in this time.
pub const SPLASH_LENGTH: f32 = 0.7;

#[derive(Clone, Debug)]
struct Fragment {
    x: i32,
    y: f32,
    vy: f32,
}

/// The bridge's colours in one theme.
struct Colours {
    overcast: Overcast,
    hills: Rgb,
    grass: Rgb,
    earth: Ramp,
    river: Ramp,
    crest: Rgb,
    deck: (Rgb, Rgb, Rgb),
    rail: Rgb,
    stone: (Rgb, Rgb, Rgb),
    girder: Rgb,
    stripe: (Rgb, Rgb),
    car: Rgb,
    glass: Rgb,
    wheel: Rgb,
    head: Rgb,
}

impl Colours {
    fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let (hills, grass, earth_top, earth_low, river_top, river_deep) = if theme.dark {
            (
                shades.tint(theme.muted, 0.18, 1.35),
                shades.tint(theme.green(), 0.42, 2.0),
                shades.tint(mix(theme.yellow(), theme.muted, 0.6), 0.3, 1.7),
                shades.tint(mix(theme.yellow(), theme.muted, 0.6), 0.16, 1.3),
                shades.tint(theme.blue(), 0.55, 2.4),
                shades.tint(theme.blue(), 0.25, 1.4),
            )
        } else {
            (
                shades.tint(theme.muted, 0.5, 2.2),
                shades.tint(theme.green(), 0.55, 2.4),
                shades.tint(mix(theme.yellow(), theme.muted, 0.6), 0.45, 2.0),
                shades.tint(mix(theme.yellow(), theme.muted, 0.6), 0.7, 2.7),
                shades.tint(theme.blue(), 0.3, 1.7),
                shades.tint(theme.blue(), 0.7, 2.75),
            )
        };
        let crest = if theme.dark {
            shades.tint(theme.bright_cyan(), 0.75, 2.9)
        } else {
            mix(river_top, theme.background, 0.65)
        };
        let concrete = solid(theme, theme.muted, shades.darkest(), 0.2);
        let stone = solid(
            theme,
            mix(theme.muted, theme.yellow(), 0.25),
            shades.darkest(),
            0.35,
        );
        Self {
            overcast: Overcast::new(theme),
            hills,
            grass,
            earth: Ramp::new(earth_top, earth_low, 5),
            river: Ramp::new(river_top, river_deep, 8),
            crest,
            deck: shaded(theme, concrete),
            rail: theme.foreground,
            stone: shaded(theme, stone),
            girder: warning_colour(theme),
            stripe: (error_colour(theme), theme.foreground),
            car: theme.bright_blue(),
            glass: solid(theme, theme.info, shades.darkest(), 0.5),
            wheel: solid(theme, theme.muted, shades.darkest(), 0.6),
            head: theme.bright_yellow(),
        }
    }
}

pub struct BrokenBridge {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    seed: u32,
    fragments: Vec<Fragment>,
    splashes: Vec<(i32, f32)>,
    since_fragment: f32,
    cloud_drift: f32,
}

impl BrokenBridge {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut bridge = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            seed: (rng.next_u64() >> 32) as u32,
            fragments: Vec::new(),
            splashes: Vec::new(),
            since_fragment: 0.0,
            cloud_drift: 0.0,
        };
        bridge.resize(width, height, rng);
        bridge
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The pixel row of the road on the deck.
    fn deck(&self) -> i32 {
        ((self.tall() as f32 * 0.45) as i32).max(4)
    }

    /// The first pixel row of water.
    fn river(&self) -> i32 {
        let tall = self.tall();
        (tall - (tall / 4).max(5)).max(self.deck() + 4)
    }

    fn bank_width(&self) -> i32 {
        (self.width as i32 / 9).max(3)
    }

    /// The columns of the gap, left and right inclusive.
    fn gap(&self) -> (i32, i32) {
        let w = self.width as i32;
        let gap = (w / 6).clamp(4, 16);
        let left = w / 2 - gap / 2;
        (left, left + gap - 1)
    }

    fn fragment_period(&self) -> f32 {
        FRAGMENT_PERIOD / self.motion.pace()
    }

    /// The column where the barrier stands, if there is room for it on the deck.
    fn barrier(&self) -> Option<i32> {
        let x = self.gap().0 - 4;
        (self.deck() >= 6 && x > self.bank_width() + 1).then_some(x)
    }

    /// The text row of the board above the barrier, if the screen is wide and tall enough for
    /// its words, with the board's frame clear of the top edge.
    fn board_row(&self) -> Option<i32> {
        let row = (self.deck() - 12) / 2;
        (self.barrier().is_some() && row >= 1 && self.width >= 60 && self.deck() >= 16)
            .then_some(row)
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours, theme: &ThemeData) {
        let mut mixer = Blender::new(8);
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let deck = self.deck();
        let river = self.river();
        let bank = self.bank_width();
        let (gap_left, gap_right) = self.gap();
        gradient(canvas, &c.overcast.sky, 0, river);
        clouds(
            canvas,
            &mut mixer,
            &c.overcast.cloud,
            -2,
            (deck as f32 * 0.8) as i32,
            self.cloud_drift,
            self.seed,
            0.6,
        );
        // far hills along the horizon
        for x in 0..wide {
            let lift = 2.0 * (x as f32 * 0.05 + 1.0).sin() + (x as f32 * 0.13).sin();
            let top = deck + 2 - lift.round() as i32;
            for y in top.max(0)..river {
                canvas.set(x, y, c.hills);
            }
        }

        // the river, drifting downstream from left to right with highlights on it
        let speed = self.motion.speed();
        let span = (tall - river).max(1) as f32;
        for y in river..tall {
            let depth = (y - river) as f32 / span;
            let drift = cycle(self.t, (0.9 + 0.5 * depth) * speed / 4.0);
            for x in 0..wide {
                let xf = x as f32;
                let v = 0.6 * (xf / 5.0 - drift + depth * 7.0).sin()
                    + 0.4 * (xf / 2.3 - drift * 1.6 + depth * 11.0).sin();
                let lift = ((v - 0.55) / 0.4).clamp(0.0, 1.0) * (1.0 - depth * 0.5);
                canvas.set(x, y, mixer.mix(c.river.at(depth), c.crest, lift));
            }
        }

        // the banks, sloping down to the water, grass along their tops
        for y in deck + 1..river + 2 {
            let step = y - deck - 1;
            let left_edge = bank + step;
            let right_edge = wide - 1 - bank - step;
            let v = step as f32 / (river - deck).max(1) as f32;
            for x in 0..=left_edge.min(wide - 1) {
                canvas.set(x, y, c.earth.at(v + unit(x, y, self.seed) * 0.2));
            }
            for x in right_edge.max(0)..wide {
                canvas.set(x, y, c.earth.at(v + unit(x, y, self.seed) * 0.2));
            }
            canvas.set(left_edge, y, c.grass);
            canvas.set(right_edge, y, c.grass);
        }
        for x in 0..bank {
            canvas.set(x, deck, c.grass);
            canvas.set(wide - 1 - x, deck, c.grass);
        }

        // stone piers under the deck, away from the gap, lit on their left
        let (dark, stone, light) = c.stone;
        let span = (wide / 6).max(8);
        let mut x = bank + span / 2;
        while x < wide - bank {
            if x < gap_left - 3 || x > gap_right + 3 {
                canvas.fill_rect(x, deck + 2, 3, tall - deck - 2, stone);
                canvas.fill_rect(x, deck + 2, 1, tall - deck - 2, light);
                canvas.fill_rect(x + 2, deck + 2, 1, tall - deck - 2, dark);
                // the water muddies the foot of each pier
                for y in river..tall {
                    for px in x..x + 3 {
                        mixer.blend(canvas, px, y, c.river.at(0.5), 0.6);
                    }
                }
                for px in x - 1..x + 4 {
                    canvas.set(px, river, c.crest);
                }
            }
            x += span;
        }

        // the fallen span, tilted into the water, which runs over its low end
        let fallen = (gap_right - gap_left - 1).max(2) as f32;
        let (from, to) = (
            (gap_left as f32 + 1.0, river as f32 - 2.0),
            (gap_left as f32 + fallen, river as f32 + 1.0),
        );
        line(canvas, from, to, 2, c.deck.1);
        line(
            canvas,
            (from.0, from.1 - 1.0),
            (to.0, to.1 - 1.0),
            1,
            c.deck.2,
        );
        for px in gap_left..=gap_right {
            mixer.blend(canvas, px, river + 1, c.river.at(0.2), 0.5);
        }

        // the deck and its railing, with the middle span missing and the ends broken jaggedly
        let (under, top, lit) = c.deck;
        for x in bank..wide - bank {
            if (gap_left..=gap_right).contains(&x) {
                continue;
            }
            // near the break the slab crumbles away from below
            let near = (gap_left - 1 - x).max(x - gap_right - 1).max(0);
            let bite = if near < 2 {
                (unit(x, 1, self.seed) * 2.0) as i32
            } else {
                0
            };
            canvas.set(x, deck, lit);
            canvas.set(x, deck + 1, top);
            if bite < 1 {
                canvas.set(x, deck + 2, under);
            }
            if (x - bank) % 5 == 0 {
                canvas.set(x, deck - 1, c.rail);
                canvas.set(x, deck - 2, c.rail);
            }
            canvas.set(x, deck - 3, c.rail);
        }
        // girders hanging from both broken ends
        let hang = ((river - deck - 2) as f32 * 0.55).clamp(2.0, 9.0);
        line(
            canvas,
            (gap_left as f32, deck as f32 + 1.0),
            (gap_left as f32 + hang * 0.5, deck as f32 + 1.0 + hang),
            1,
            c.girder,
        );
        line(
            canvas,
            (gap_right as f32, deck as f32 + 1.0),
            (
                gap_right as f32 - hang * 0.4,
                deck as f32 + 1.0 + hang * 0.8,
            ),
            1,
            c.girder,
        );
        line(
            canvas,
            (gap_left as f32, deck as f32 - 3.0),
            (gap_left as f32 + 2.0, deck as f32 - 1.0),
            1,
            c.girder,
        );

        for fragment in &self.fragments {
            canvas.fill_rect(fragment.x, fragment.y.floor() as i32, 2, 2, under);
        }
        for &(x, age) in &self.splashes {
            if age < SPLASH_LENGTH / 2.0 {
                canvas.fill_rect(x, river - 1, 2, 2, c.crest);
            } else {
                canvas.set(x - 2, river - 1, c.crest);
                canvas.set(x + 3, river - 1, c.crest);
            }
        }

        // the barrier, its sign and the car waiting behind it
        if let Some(bx) = self.barrier() {
            for (i, x) in (bx - 3..=bx + 3).enumerate() {
                let colour = if (i / 2) % 2 == 0 {
                    c.stripe.0
                } else {
                    c.stripe.1
                };
                canvas.set(x, deck - 3, colour);
                canvas.set(x, deck - 4, colour);
            }
            canvas.fill_rect(bx - 3, deck - 2, 1, 2, c.rail);
            canvas.fill_rect(bx + 3, deck - 2, 1, 2, c.rail);
            // a board on a post above the barrier, framed in the stripe colour, its middle left
            // plain for the words written over it after the canvas
            if let Some(row) = self.board_row() {
                let len = BOARD_TEXT.len() as i32;
                let left = bx - len / 2 - 2;
                let top = 2 * row - 1;
                canvas.fill_rect(left, top, len + 4, 4, c.stripe.0);
                canvas.fill_rect(left + 1, top + 1, len + 2, 2, theme.background);
                canvas.fill_rect(bx, top + 4, 1, deck - 4 - (top + 4), c.rail);
            }
            let car_x = bx - 6 - CAR[0].len() as i32;
            if car_x >= bank && self.width >= 40 {
                let palette = [
                    ('b', c.car),
                    ('w', c.glass),
                    ('k', c.wheel),
                    ('r', c.stripe.0),
                    ('h', c.head),
                ];
                canvas.sprite(car_x, deck - CAR.len() as i32, &CAR, &palette);
            }
        }
    }
}

impl Scene for BrokenBridge {
    fn name(&self) -> &'static str {
        "broken-bridge"
    }

    fn resize(&mut self, width: usize, height: usize, _rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.fragments.clear();
        self.splashes.clear();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        self.cloud_drift = wrap(self.cloud_drift + 0.7 * speed * dt, 100_000.0);
        self.since_fragment += dt;
        if self.since_fragment >= self.fragment_period() {
            self.since_fragment = 0.0;
            let (left, right) = self.gap();
            let x = if rng.chance(0.5) { left } else { right - 1 };
            self.fragments.push(Fragment {
                x,
                y: (self.deck() + 2) as f32,
                vy: 1.0,
            });
        }
        let river = self.river() as f32;
        let gravity = 20.0 * speed;
        let mut landed = Vec::new();
        for fragment in &mut self.fragments {
            fragment.vy += gravity * dt;
            fragment.y += fragment.vy * dt;
            if fragment.y >= river {
                landed.push(fragment.x);
            }
        }
        self.fragments.retain(|f| f.y < river);
        for x in landed {
            self.splashes.push((x, 0.0));
        }
        let pace = self.motion.pace();
        for splash in &mut self.splashes {
            splash.1 += dt * pace;
        }
        self.splashes.retain(|s| s.1 < SPLASH_LENGTH);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = Colours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours, theme);
        canvas.blit(frame);
        if let (Some(bx), Some(row)) = (self.barrier(), self.board_row()) {
            paint::label(
                frame,
                bx - BOARD_TEXT.len() as i32 / 2,
                row,
                BOARD_TEXT,
                colours.stripe.0,
            );
        }
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
        check_scene("broken-bridge");
        check_determinism("broken-bridge");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("broken-bridge", 40.0);
    }

    #[test]
    fn the_deck_has_a_gap_with_hanging_girders_and_a_barrier() {
        let theme = theme::default_theme();
        let c = Colours::new(theme);
        let mut rng = Rng::new(5);
        let mut bridge = BrokenBridge::new(100, 30, Motion::FULL, &mut rng);
        let frame = render(&mut bridge, 100, 30, 100);
        let deck = bridge.deck();
        let pixel = |x: i32, y: i32| {
            let (top, bottom) = frame.get(x, y / 2).unwrap().pixel_pair().unwrap();
            if y % 2 == 0 { top } else { bottom }
        };
        let (left, right) = bridge.gap();
        assert_eq!(pixel(left - 3, deck), c.deck.2);
        for x in left + 1..right {
            assert_ne!(pixel(x, deck), c.deck.2);
        }
        let all = pixels(&frame);
        for colour in [c.girder, c.stripe.0, c.car, c.head] {
            assert!(all.contains(&colour), "{}", colour.hex());
        }
        assert!(frame.to_text().contains("BRIDGE OUT"));
        let small = render(
            &mut BrokenBridge::new(20, 8, Motion::CALM, &mut rng),
            20,
            8,
            20,
        );
        assert!(pixels(&small).contains(&c.deck.2));
    }

    #[test]
    fn fragments_fall_slowly_and_splash_once_each() {
        let mut rng = Rng::new(1);
        for motion in Motion::ALL {
            let bridge = BrokenBridge::new(80, 24, motion, &mut rng);
            assert!(bridge.fragment_period() >= FRAGMENT_PERIOD);
        }
        let mut bridge = BrokenBridge::new(80, 24, Motion::LIVELY, &mut rng);
        let mut splashed = false;
        for _ in 0..(60 * 20) {
            bridge.step(0.05, &mut rng);
            splashed |= !bridge.splashes.is_empty();
            assert!(bridge.fragments.len() <= 2);
        }
        assert!(splashed);
        const { assert!(SPLASH_LENGTH / 2.0 > 1.0 / 3.0) };
    }

    #[test]
    fn the_layers_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            apart(
                theme,
                c.river.at(0.0),
                c.river.at(1.0),
                0.03,
                "shallow and deep river",
            );
            apart(
                theme,
                c.crest,
                c.river.at(0.3),
                0.03,
                "the crests and the river",
            );
            apart(
                theme,
                c.earth.at(0.5),
                c.river.at(0.2),
                0.03,
                "the banks and the river",
            );
            apart(
                theme,
                c.deck.1,
                c.overcast.sky.at(0.8),
                0.1,
                "the deck and the sky",
            );
            apart(theme, c.stripe.0, c.stripe.1, 0.1, "the barrier's stripes");
            apart(theme, c.car, c.deck.1, 0.1, "the car and the deck");
            for colour in c
                .river
                .steps()
                .iter()
                .chain(c.earth.steps())
                .chain([&c.hills])
            {
                assert!(
                    theme.contrast_of(*colour) < IMAGERY_CEILING,
                    "{}",
                    theme.name
                );
            }
            for colour in [c.deck.1, c.stone.1, c.girder, c.car] {
                assert!(
                    theme.contrast_of(colour) >= paint::SOLID - 0.01,
                    "{}",
                    theme.name
                );
            }
        }
    }
}
