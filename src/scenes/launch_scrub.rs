//! Launch scrub: the coastal pad from `rocket_launch`, the same rocket standing on its
//! platform beside the gantry, but the count has stopped. The board on the apron reads HOLD,
//! the access arm swings slowly back in to the rocket and steam keeps venting from its sides
//! and drifting off on the wind, with a low mist round the foot of the platform. Nothing lights
//! and nothing lifts; once the arm is home only the steam, the sea and the clouds move.
//!
//! Everything is painted by the launch's own functions with its colours, so the pair match.
//! The steam is imagery below the non-text floor; the rocket, the gantry and the board's frame
//! are solid. HOLD is written in the theme's error colour on the board's plain face.

use super::paint::{Cloud, unit};
use super::rocket_launch::write_board;
use super::rocket_launch::{Colours, Moment, Pad, Puff, between, paint_moment, sky_clouds};
use crate::frame::Frame;
use crate::pixels::Canvas;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::error_colour;

pub const SUMMARY: &str = "The same coastal pad with the rocket held, venting steam as the gantry arm swings back and the board reads HOLD, by day in light themes";

pub const CAPTIONS: &[&str] = &[
    "Launch on hold. Check the first error.",
    "Holding the count. Something needs a look.",
    "Scrubbed for now. Read the log and try again.",
];

/// When the count stops and the board turns to HOLD, on the scene's clock.
pub const HOLD_AT: f32 = 3.0;
/// When the arm starts to swing back in to the rocket and when it is home.
pub const ARM_BACK: (f32, f32) = (5.0, 17.0);
/// Seconds between two puffs of steam from one vent at the standard pace.
pub const VENT_PERIOD: f32 = 0.8;
/// How long a puff of steam lives as it drifts away and thins.
const VENT_LIFE: f32 = 5.0;
/// Seconds for the mist round the platform to swell and settle once.
const MIST_PERIOD: f32 = 9.0;

pub struct LaunchScrub {
    width: usize,
    height: usize,
    motion: Motion,
    /// The scene's clock, on the motion pace.
    clock: f64,
    /// Time on the motion speed, for the sea and the surf.
    drift: f64,
    clouds: Vec<Cloud>,
}

impl LaunchScrub {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut scene = Self {
            width: 1,
            height: 1,
            motion,
            clock: 0.0,
            drift: 0.0,
            clouds: Vec::new(),
        };
        scene.resize(width, height, rng);
        scene
    }

    fn pad(&self) -> Pad {
        Pad::new(self.width, self.height)
    }

    fn now(&self) -> f32 {
        // the clock is narrowed only once it is folded into a long repeat, so the steam's
        // timing stays smooth however long the scene runs
        let span = f64::from(VENT_PERIOD * MIST_PERIOD * 100.0);
        let held = self.clock.min(f64::from(ARM_BACK.1 + 1.0));
        if self.clock <= held {
            held as f32
        } else {
            (f64::from(ARM_BACK.1 + 1.0) + (self.clock - held).rem_euclid(span)) as f32
        }
    }

    fn board_text(&self) -> &'static str {
        if self.now() < HOLD_AT { "T-31" } else { "HOLD" }
    }

    /// How far the arm reaches towards the rocket, from swung away to home.
    fn arm(&self) -> f32 {
        let swung = 0.25;
        swung + (1.0 - swung) * between(self.now(), ARM_BACK.0, ARM_BACK.1)
    }

    /// The steam venting from the rocket's sides and the mist round its foot. Each puff's
    /// place comes from how long ago its vent let it go, so it drifts smoothly without state.
    fn puffs(&self, pad: &Pad) -> Vec<Puff> {
        let mut out = Vec::new();
        if pad.h < 12 {
            return out;
        }
        let s = self.now();
        let rw = pad.rw as f32;
        let top = pad.body_bottom() - pad.rh;
        let right = pad.px as f32 + 0.5 + rw / 2.0;
        let left = pad.px as f32 + 0.5 - rw / 2.0;
        // vents at the stage line and lower on the first stage blow off downwind to the right, one
        // low on the left side curls away to the left
        let vents = [
            (
                right,
                (top + (pad.rh as f32 * 0.4) as i32 + 2) as f32,
                1.0,
                0.0,
            ),
            (
                right,
                (top + (pad.rh as f32 * 0.6) as i32) as f32,
                1.0,
                0.33,
            ),
            (
                left,
                (top + (pad.rh as f32 * 0.72) as i32) as f32,
                -1.0,
                0.66,
            ),
        ];
        let scale = rw / 7.0;
        for (v, &(x, y, side, offset)) in vents.iter().enumerate() {
            let newest = ((s / VENT_PERIOD) - offset).floor() as i32;
            let oldest = newest - (VENT_LIFE / VENT_PERIOD).ceil() as i32;
            for k in oldest..=newest {
                let born = (k as f32 + offset) * VENT_PERIOD;
                let age = s - born;
                if !(0.0..VENT_LIFE).contains(&age) {
                    continue;
                }
                let thin = 1.0 - ((age - VENT_LIFE * 0.6) / (VENT_LIFE * 0.4)).clamp(0.0, 1.0);
                // each puff leaves the vent as a thin jet and swells as it drifts and rises
                let r = (0.6 + age * 0.5) * scale.max(0.6) * thin;
                let wobble = (unit(k, v as i32, 17) - 0.5) * age.min(1.0);
                out.push(Puff {
                    x: x + side * (age * 3.2 * scale + r * 0.3),
                    y: y - age * 0.6 * scale + wobble,
                    r,
                    seed: 500 + (k.rem_euclid(64) as u32) * 3 + v as u32,
                });
            }
        }
        // a low mist round the platform's foot, swelling and settling very slowly
        let breath = (s / MIST_PERIOD * std::f32::consts::TAU).sin();
        for i in -3..=3_i32 {
            let n = i.unsigned_abs() as f32;
            let r = rw
                * (0.85 - n * 0.12)
                * (0.92 + 0.08 * breath * if i % 2 == 0 { 1.0 } else { -1.0 });
            out.push(Puff {
                x: pad.px as f32 + 0.5 + i as f32 * rw * 0.6,
                y: pad.foot as f32 - r * 0.35,
                r,
                seed: 700 + (i + 3) as u32,
            });
        }
        out
    }

    fn moment(&self, pad: &Pad) -> Moment {
        Moment {
            platform: pad.px as f32 + 0.5,
            rocket: Some(0.0),
            arm: self.arm(),
            burn: 0.0,
            waver: 0.0,
            puffs: self.puffs(pad),
            drift: self.drift as f32,
        }
    }
}

impl Scene for LaunchScrub {
    fn name(&self) -> &'static str {
        "launch-scrub"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.clouds = sky_clouds(&self.pad(), self.motion, rng);
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.clock += f64::from(dt * self.motion.pace());
        self.drift = (self.drift + f64::from(dt * self.motion.speed())).rem_euclid(100_000.0);
        let w = self.width as f32;
        for cloud in &mut self.clouds {
            cloud.drift(dt, w);
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = Colours::new(theme);
        let pad = self.pad();
        let mut canvas = Canvas::for_frame(frame, theme.background);
        paint_moment(
            &mut canvas,
            &pad,
            &colours,
            theme,
            &self.clouds,
            &self.moment(&pad),
        );
        canvas.blit(frame);
        let text = self.board_text();
        let colour = if text == "HOLD" {
            error_colour(theme)
        } else {
            theme.foreground
        };
        write_board(frame, &pad, text, colour);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::render;
    use crate::scenes::testing::{check_determinism, check_pixel_scene, check_scene, pixels};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("launch-scrub");
        check_determinism("launch-scrub");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("launch-scrub", 8.0);
    }

    #[test]
    fn the_board_holds_the_arm_swings_home_and_the_rocket_never_lifts() {
        for motion in Motion::ALL {
            let mut rng = Rng::new(2);
            let mut scene = LaunchScrub::new(100, 30, motion, &mut rng);
            let pad = scene.pad();
            let mut arm = scene.arm();
            for i in 0..(120 * 20) {
                scene.step(0.05, &mut rng);
                let m = scene.moment(&pad);
                assert_eq!(m.rocket, Some(0.0), "{motion:?}");
                assert_eq!(m.burn, 0.0);
                // the arm only ever swings in, a little at a time
                assert!(m.arm >= arm && m.arm - arm < 0.02, "{motion:?}");
                arm = m.arm;
                if i > 10 * 20 {
                    assert_eq!(scene.board_text(), "HOLD");
                }
            }
            assert_eq!(arm, 1.0, "{motion:?}");
        }
    }

    #[test]
    fn steam_drifts_slowly_from_the_rocket() {
        let mut rng = Rng::new(2);
        let mut scene = LaunchScrub::new(100, 30, Motion::LIVELY, &mut rng);
        let pad = scene.pad();
        for _ in 0..(30 * 20) {
            let before = scene.puffs(&pad);
            scene.step(0.05, &mut rng);
            let after = scene.puffs(&pad);
            assert!(!after.is_empty());
            for p in &after {
                if let Some(q) = before.iter().find(|q| q.seed == p.seed) {
                    let moved = ((p.x - q.x).powi(2) + (p.y - q.y).powi(2)).sqrt();
                    assert!(moved < 0.5, "a puff jumped {moved} pixels");
                }
            }
        }
    }

    #[test]
    fn the_scene_matches_the_launch_and_reads_hold() {
        let theme = theme::default_theme();
        let c = Colours::new(theme);
        let mut rng = Rng::new(5);
        let mut scene = LaunchScrub::new(100, 30, Motion::FULL, &mut rng);
        let frame = render(&mut scene, 100, 30, 20 * 20);
        let all = pixels(&frame);
        for colour in [c.hull[1], c.trim.0, c.steel[1], c.smoke[1], c.beacon] {
            assert!(all.contains(&colour), "{}", colour.hex());
        }
        assert!(!all.contains(&c.flame.0));
        assert!(frame.to_text().contains("HOLD"));
    }
}
