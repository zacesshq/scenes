//! Pit stop stuck: the same pit lane and the same car as `pit_stop`, where one wheel will not
//! come off. The car slides into its box and goes up on the jacks. The front wheel is changed,
//! but the rear one stays put, its wheel nut marked in the error colour, the gunner still on it.
//! The front gunner comes over to help and the others gather round while the car waits on its
//! jacks with the lollipop held down in front of it. The stop time on the board climbs a second
//! at a time. After the crew have gathered nothing moves but the clock.
//!
//! The drawing is shared with `pit_stop`, so the two stops look the same.

use super::pit_stop::{
    ARRIVE, Crew, JACK, Layout, Pose, START, SWAP, Walk, arriving, draw_pose, jacked, phase,
};
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;

pub const SUMMARY: &str = "The same pit stop with one wheel that will not come off, the crew gathered round and the board climbing, by day in light themes";

pub const CAPTIONS: &[&str] = &[
    "One wheel is stuck. Find the failing step.",
    "The clock is running. Read the first error.",
    "Still in the box. One fix and it is away.",
];

/// Seconds after the guns go on that the rear wheel should have come off and is seen not to.
const STUCK_AFTER: f32 = 1.5;
/// When the front crew start over to the stuck wheel, after the car stopped.
const GATHER_AT: f32 = 6.0;
/// Seconds the front crew take to walk over.
const GATHER: f32 = 3.0;
/// The longest stop the board counts to, in seconds, before it holds.
const BOARD_MAX: u32 = 9 * 60 + 59;

pub struct PitStopStuck {
    width: usize,
    height: usize,
    motion: Motion,
    /// Seconds into the scene at the scene's own speed.
    t: f64,
    /// Seconds into the scene at the pace anything that changes in place keeps, which the board's
    /// clock runs on, so it never ticks more than once a second.
    clock: f64,
    /// Where the scene's own time started.
    start: f64,
}

impl PitStopStuck {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        // under reduced motion the car is already waiting in its box, so the scene opens on
        // the problem instead of a car sliding in
        let start = if motion.calm {
            f64::from(ARRIVE)
        } else {
            START
        };
        let mut scene = Self {
            width: 1,
            height: 1,
            motion,
            t: start,
            clock: start,
            start,
        };
        scene.resize(width, height, rng);
        scene
    }

    fn layout(&self) -> Layout {
        Layout::new(self.width, self.height)
    }

    /// The whole seconds the car has waited in its box.
    fn waited(&self) -> u32 {
        // the clock starts when the car stops, which the scene's own time reaches at `ARRIVE`
        let ratio = f64::from(self.motion.pace() / self.motion.speed());
        let stopped = f64::from(ARRIVE) * ratio + self.start * (1.0 - ratio);
        ((self.clock - stopped).max(0.0) as u32).min(BOARD_MAX)
    }

    fn board(&self) -> String {
        let s = self.waited();
        format!("{}:{:02}", s / 60, s % 60)
    }

    fn pose(&self) -> Pose {
        let l = self.layout();
        let t = self.t as f32;
        let mut pose = Pose::waiting(self.board());
        if t < ARRIVE {
            pose.car_x = Some(arriving(&l, t));
            return pose;
        }
        pose.car_x = Some(l.stop_x());
        pose.lift = jacked(&l, phase(t, ARRIVE, JACK));
        let guns = ARRIVE + JACK;
        let off = guns + 0.3;
        let on = off + SWAP;
        // the front wheel comes off and the new one goes on as it should
        let front = &mut pose.wheels[1];
        front.gun = t >= guns && t < on + SWAP;
        front.off = phase(t, off, SWAP);
        front.on = phase(t, on, SWAP);
        // the rear one never moves and its nut shows why
        let rear = &mut pose.wheels[0];
        rear.gun = t >= guns;
        rear.stuck = t >= guns + STUCK_AFTER;
        pose.new_in_hand = true;
        // the front gunner comes over to the stuck wheel and the rest of the front crew close in
        if l.kit.crew {
            let k = phase(t, ARRIVE + GATHER_AT, GATHER);
            if k > 0.0 {
                let s = l.scale as f32;
                let feet = l.feet as f32;
                let (hub, _, _) = l.wheel_at(l.stop_x(), 0, 0);
                let step_back =
                    (l.spot(Crew::RearOn) + (l.kit.figure_width() + 1) * l.scale) as f32;
                let front_on = (l.spot(Crew::FrontOn) - l.scale) as f32;
                pose.walks = vec![
                    Walk {
                        who: Crew::FrontGun,
                        to: ((hub + 1.5 * s).round(), feet),
                        along: k,
                        wheel: None,
                        kneel: true,
                        right: false,
                    },
                    Walk {
                        who: Crew::RearOn,
                        to: (step_back, feet),
                        along: k,
                        wheel: Some(true),
                        kneel: false,
                        right: false,
                    },
                    Walk {
                        who: Crew::FrontOn,
                        to: (front_on, feet),
                        along: phase(t, ARRIVE + GATHER_AT + 1.0, GATHER),
                        wheel: None,
                        kneel: false,
                        right: false,
                    },
                ];
            }
        }
        pose
    }
}

impl Scene for PitStopStuck {
    fn name(&self) -> &'static str {
        "pit-stop-stuck"
    }

    fn resize(&mut self, width: usize, height: usize, _rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt * self.motion.speed());
        self.clock += f64::from(dt * self.motion.pace());
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        draw_pose(frame, theme, self.layout(), &self.pose());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testing::{check_determinism, check_pixel_scene, check_scene};

    #[test]
    fn passes_the_shared_checks() {
        check_scene("pit-stop-stuck");
        check_determinism("pit-stop-stuck");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("pit-stop-stuck", 30.0);
    }

    #[test]
    fn the_board_climbs_no_faster_than_once_a_second_and_the_wheel_stays() {
        for motion in Motion::ALL {
            let mut rng = Rng::new(1);
            let mut scene = PitStopStuck::new(100, 30, motion, &mut rng);
            let mut last = scene.board();
            let mut changed_at = -10.0_f32;
            let mut changes = 0;
            for i in 0..(60 * 20) {
                scene.step(0.05, &mut rng);
                let board = scene.board();
                if board != last {
                    let now = i as f32 * 0.05;
                    assert!(now - changed_at >= 0.95, "{motion:?} ticked at {now}");
                    changed_at = now;
                    changes += 1;
                    last = board;
                }
            }
            assert!(changes >= 15, "{motion:?} the board barely moved");
            let pose = scene.pose();
            let l = scene.layout();
            assert_eq!(pose.car_x, Some(l.stop_x()), "the car is still in its box");
            assert!(pose.wheels[0].stuck && pose.wheels[0].off == 0.0);
            assert_eq!(pose.wheels[1].on, 1.0, "the front wheel was changed");
            assert_eq!(pose.lollipop, 0.0, "the lollipop stays down");
        }
    }
}
