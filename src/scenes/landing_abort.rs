//! A landing called off, painted in solid pixels: the `drone-landing` picture, the same
//! quadcopter over the same rolling country and pad, but the dots its beam maps scatter above
//! and below the ground instead of following it. With no match the drone hovers over the pad,
//! then climbs calmly away and holds high above the field, its lamp turned amber.

use crate::rng::Rng;
use crate::scene::Motion;

pub use super::drone_landing::DroneLanding;

pub const SUMMARY: &str = "The same quadcopter finding no terrain match, hovering over the pad at dusk, then climbing calmly away, by day in light themes";

pub const CAPTIONS: &[&str] = &[
    "No terrain match. The build stays up.",
    "The map and the ground disagree.",
    "Landing called off. Something failed.",
    "Holding position until the build is fixed.",
    "No fix, no landing.",
];

/// The aborted approach, fitted to a size.
pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> DroneLanding {
    DroneLanding::aborted(width, height, motion, rng)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Frame;
    use crate::scene::Scene;
    use crate::scenes::testing::{check_determinism, check_pixel_scene, check_scene};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("landing-abort");
        check_determinism("landing-abort");
        check_pixel_scene("landing-abort", 30.0);
    }

    #[test]
    fn the_scan_finds_no_match_and_the_drone_climbs_away_from_the_pad() {
        let mut rng = Rng::new(3);
        let mut scene = new(80, 24, Motion::FULL, &mut rng);
        assert_eq!(scene.name(), "landing-abort");
        let mut frame = Frame::new(80, 24);
        for _ in 0..(40 * 20) {
            scene.step(0.05, &mut rng);
        }
        scene.draw(&mut frame, theme::default_theme());
        assert!(frame.row_text(1).contains("NO TERRAIN MATCH"));
        assert!(!frame.row_text(1).contains("POSITION FIX GOOD"));
    }
}
