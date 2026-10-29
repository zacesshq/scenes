//! Broken bridge: a bridge over a river with its middle span gone. Girders hang from the broken
//! ends, the fallen span lies in the water, a barrier and a warning sign stop the road and a car
//! waits behind them. The river keeps flowing and now and then a fragment drops from an end.

use super::wrap;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::{error_colour, warning_colour};

pub const SUMMARY: &str =
    "A bridge with its middle span gone, girders hanging, a barrier and a waiting car over a river";

pub const CAPTIONS: &[&str] = &[
    "A gap in the build. Bridge the first error.",
    "The road stops here for now. Read the sign below.",
    "One missing piece. The rest is still standing.",
    "Something did not connect. Check the first error.",
    "Closed for repairs. The list below is the plan.",
];

/// The car on the deck, facing the gap.
const CAR: [&str; 2] = [" ,-=-. ", "(o___o)"];
/// Seconds between two fragments falling from a broken end at the standard pace.
pub const FRAGMENT_PERIOD: f32 = 4.0;
/// How long a splash shows where a fragment lands. The splash cell changes twice in this time.
pub const SPLASH_LENGTH: f32 = 0.7;

#[derive(Clone, Debug)]
struct Fragment {
    x: i32,
    y: f32,
    vy: f32,
}

pub struct BrokenBridge {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    fragments: Vec<Fragment>,
    splashes: Vec<(i32, f32)>,
    since_fragment: f32,
    clouds: Vec<(f32, i32, usize)>,
}

impl BrokenBridge {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut bridge = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            fragments: Vec::new(),
            splashes: Vec::new(),
            since_fragment: 0.0,
            clouds: Vec::new(),
        };
        bridge.resize(width, height, rng);
        bridge
    }

    fn deck(&self) -> i32 {
        ((self.height as f32 * 0.45) as i32).max(2)
    }

    /// The first row of water.
    fn river(&self) -> i32 {
        let h = self.height as i32;
        (h - (h / 4).max(3)).max(self.deck() + 2)
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
}

impl Scene for BrokenBridge {
    fn name(&self) -> &'static str {
        "broken-bridge"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.fragments.clear();
        self.splashes.clear();
        let count = self.motion.scaled((self.width / 30).max(1));
        let sky = (self.deck() - 2).max(1) as usize;
        self.clouds = (0..count)
            .map(|_| {
                (
                    rng.range(0.0, self.width as f32),
                    rng.below(sky) as i32,
                    6 + rng.below(9),
                )
            })
            .collect();
    }

    fn step(&mut self, dt: f32, rng: &mut Rng) {
        self.t += f64::from(dt);
        let speed = self.motion.speed();
        let width = self.width as f32;
        for cloud in &mut self.clouds {
            cloud.0 = wrap(cloud.0 + 0.35 * speed * dt + 16.0, width + 32.0) - 16.0;
        }
        self.since_fragment += dt;
        if self.since_fragment >= self.fragment_period() {
            self.since_fragment = 0.0;
            let (left, right) = self.gap();
            let x = if rng.chance(0.5) { left } else { right };
            self.fragments.push(Fragment {
                x,
                y: (self.deck() + 1) as f32,
                vy: 0.5,
            });
        }
        let river = self.river() as f32;
        let gravity = 10.0 * speed;
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
        let width = self.width as i32;
        let height = self.height as i32;
        let deck = self.deck();
        let river = self.river();
        let bank = self.bank_width();
        let (gap_left, gap_right) = self.gap();
        let speed = self.motion.speed();

        for &(x, y, len) in &self.clouds {
            for i in 0..len as i32 {
                frame.put(x.round() as i32 + i, y, '~', Some(theme.grey()));
            }
        }

        // the river, drifting downstream from left to right
        for y in river..height {
            let depth = (y - river) as f32;
            let drift = super::cycle(self.t, (0.9 + 0.3 * depth) * speed / 7.0);
            for x in 0..width {
                // two waves at different lengths, so the surface never settles into stripes
                let x = x as f32;
                let long = 6.0 + (y % 3) as f32 * 1.5;
                let v = 0.6 * (x / long - drift + depth * 2.39).sin()
                    + 0.4 * (x / 2.7 - drift * 1.6 + depth * 4.1).sin();
                let x = x as i32;
                if v > 0.6 {
                    frame.put(x, y, '~', Some(theme.cyan()));
                } else if v > 0.15 {
                    frame.put(x, y, '-', Some(theme.blue()));
                }
            }
        }

        // the banks, sloping down to the water
        let ground = Some(theme.green());
        let earth = Some(theme.muted);
        for y in deck + 1..river {
            let step = y - deck - 1;
            let left_edge = bank + step;
            let right_edge = width - 1 - bank - step;
            frame.put(left_edge, y, '\\', ground);
            frame.put(right_edge, y, '/', ground);
            for x in 0..left_edge {
                frame.put(x, y, theme.faint_shade(), earth);
            }
            for x in right_edge + 1..width {
                frame.put(x, y, theme.faint_shade(), earth);
            }
        }
        for x in 0..bank {
            frame.put(x, deck, '_', ground);
            frame.put(width - 1 - x, deck, '_', ground);
        }

        // piers under the deck, away from the gap
        let span = (width / 6).max(8);
        let mut x = bank + span / 2;
        while x < width - bank {
            if x < gap_left - 2 || x > gap_right + 2 {
                for y in deck + 1..river {
                    frame.put(x, y, '|', Some(theme.grey()));
                    frame.put(x + 1, y, '|', Some(theme.grey()));
                }
            }
            x += span;
        }

        // the deck and its railing, with the middle span missing
        for x in bank..width - bank {
            if (gap_left..=gap_right).contains(&x) {
                continue;
            }
            frame.put(x, deck, '=', Some(theme.foreground));
            if deck >= 1 {
                let post = (x - bank) % 4 == 0;
                frame.put(
                    x,
                    deck - 1,
                    if post { '+' } else { '-' },
                    Some(theme.grey()),
                );
            }
        }

        // girders hanging from both broken ends
        let hang = ((river - deck - 1) / 2).clamp(1, 4);
        for i in 0..hang {
            frame.put(
                gap_left + i,
                deck + 1 + i,
                '\\',
                Some(warning_colour(theme)),
            );
            frame.put(
                gap_right - i,
                deck + 1 + i,
                '/',
                Some(warning_colour(theme)),
            );
        }
        frame.put(gap_left, deck, '%', Some(warning_colour(theme)));
        frame.put(gap_right, deck, '%', Some(warning_colour(theme)));

        // the fallen span lying in the river, the water running over it
        let fallen = (gap_right - gap_left - 1).max(2);
        for i in 0..fallen {
            let x = gap_left + 1 + i;
            let ch = if i == 0 {
                '/'
            } else if i == fallen - 1 {
                '\\'
            } else {
                '='
            };
            frame.put(x, river, ch, Some(theme.grey()));
        }

        for fragment in &self.fragments {
            frame.put(
                fragment.x,
                fragment.y.floor() as i32,
                '.',
                Some(theme.grey()),
            );
        }
        for &(x, age) in &self.splashes {
            if age < SPLASH_LENGTH / 2.0 {
                frame.put(x, river, 'o', Some(theme.bright_cyan()));
            } else {
                frame.put(x - 1, river, '(', Some(theme.cyan()));
                frame.put(x + 1, river, ')', Some(theme.cyan()));
            }
        }

        // the barrier, its sign and the car waiting behind it
        let barrier = gap_left - 3;
        if deck >= 2 && barrier > bank + 1 {
            frame.put_str(barrier - 1, deck - 1, "|X|", Some(error_colour(theme)));
            if deck >= 3 {
                frame.put_str(barrier - 1, deck - 2, "/!\\", Some(warning_colour(theme)));
            }
            let car_x = barrier - 10;
            if car_x >= bank && deck >= 3 && self.width >= 40 {
                frame.put_sprite(car_x, deck - 2, &CAR, Some(theme.bright_blue()));
            }
            if deck >= 4 && self.width >= 60 {
                let text = "BRIDGE OUT";
                frame.put_str(
                    barrier - text.len() as i32 / 2,
                    deck - 3,
                    text,
                    Some(error_colour(theme)),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_scene, render};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("broken-bridge");
        check_determinism("broken-bridge");
    }

    #[test]
    fn the_deck_has_a_gap_with_hanging_girders_and_a_barrier() {
        let mut rng = Rng::new(5);
        let mut bridge = BrokenBridge::new(100, 30, Motion::FULL, &mut rng);
        let frame = render(&mut bridge, 100, 30, 100);
        let deck = frame.row_text(bridge.deck() as usize);
        let (left, right) = bridge.gap();
        assert!(deck.contains('='));
        for x in left + 1..right {
            assert_ne!(frame.get(x, bridge.deck()).unwrap().ch, '=');
        }
        let text = frame.to_text();
        assert!(text.contains("|X|") && text.contains("/!\\") && text.contains("BRIDGE OUT"));
        assert!(text.contains("(o___o)"));
        assert!(frame.row_text(bridge.deck() as usize + 1).contains('\\'));
        let small = render(
            &mut BrokenBridge::new(20, 8, Motion::CALM, &mut rng),
            20,
            8,
            20,
        );
        assert!(small.to_text().contains('='));
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
    }
}
