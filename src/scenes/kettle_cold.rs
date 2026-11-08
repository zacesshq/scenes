//! A kettle that never boils, painted in solid pixels: the `kettle-boil` counter, window, kettle
//! and mug, but the lamp on the base goes out again after a few seconds with the water barely
//! stirring. No steam rises, the mug stays empty and a note pinned to the tiles by it says to try
//! again.

use crate::rng::Rng;
use crate::scene::Motion;

pub use super::kettle_boil::Kettle;

pub const SUMMARY: &str = "The same kettle clicking off before it boils, no steam, with a note by the mug to try again, in the evening or by day in light themes";

pub const CAPTIONS: &[&str] = &[
    "The kettle clicked off. The build did not boil.",
    "No steam. Something failed.",
    "Cold water and a failing build.",
    "Try again once the build is fixed.",
    "Not quite tea time yet.",
];

/// The cold kettle, fitted to a size.
pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Kettle {
    Kettle::cold(width, height, motion, rng)
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
        check_scene("kettle-cold");
        check_determinism("kettle-cold");
        check_pixel_scene("kettle-cold", 14.0);
    }

    #[test]
    fn the_kettle_clicks_off_cold_and_a_note_says_to_try_again() {
        let mut rng = Rng::new(3);
        let mut kettle = new(80, 24, Motion::FULL, &mut rng);
        assert_eq!(kettle.name(), "kettle-cold");
        let mut frame = Frame::new(80, 24);
        for _ in 0..(30 * 20) {
            kettle.step(0.05, &mut rng);
        }
        kettle.draw(&mut frame, theme::default_theme());
        let text: String = (0..24).map(|y| frame.row_text(y)).collect();
        assert!(text.contains("try again"));
    }
}
