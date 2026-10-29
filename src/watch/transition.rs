//! The recovered moment: when a failing build is fixed, the pass scene opens out from the middle
//! of the screen over the fail scene, its edge traced by a ring of small sparkles. The ring moves
//! outwards like any moving shape, so each cell changes once as it passes and no region flashes.
//! Under reduced motion it is slower and has no sparkles.

use crate::frame::Frame;
use crate::scene::Motion;
use crate::theme::ThemeData;

/// Seconds the opening takes.
pub const LENGTH: f32 = 2.0;
/// Seconds it takes under reduced motion.
pub const CALM_LENGTH: f32 = 3.5;

/// How long the transition takes for a motion setting.
pub fn length(motion: Motion) -> f32 {
    if motion.calm { CALM_LENGTH } else { LENGTH }
}

/// A smooth start and end, so the opening eases in and settles.
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A fixed choice per cell, so a sparkle never changes while it is on screen.
fn hash(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9e37_79b1) ^ (y as u32).wrapping_mul(0x85eb_ca77);
    h ^= h >> 15;
    h.wrapping_mul(0xc2b2_ae3d)
}

/// Draws the opening over `frame`, which already holds the new scene: cells outside the circle
/// still show `behind` (the old scene) and the circle's edge carries the sparkles.
pub fn compose(
    frame: &mut Frame,
    behind: &Frame,
    progress: f32,
    theme: &ThemeData,
    motion: Motion,
) {
    let width = frame.width() as f32;
    let height = frame.height() as f32;
    let (cx, cy) = (width / 2.0, height / 2.0);
    // cells are about twice as tall as they are wide, so x distances are halved to keep the
    // opening round
    let reach = ((cx / 2.0).powi(2) + cy.powi(2)).sqrt() + 1.5;
    let radius = ease(progress) * reach;
    let colours = [theme.green(), theme.bright_yellow(), theme.bright_cyan()];
    for y in 0..frame.height() as i32 {
        for x in 0..frame.width() as i32 {
            let dx = (x as f32 + 0.5 - cx) / 2.0;
            let dy = y as f32 + 0.5 - cy;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > radius + 0.6 {
                if let Some(cell) = behind.get(x, y) {
                    frame.put(x, y, cell.ch, cell.colour);
                }
            } else if distance > radius - 0.6 && !motion.calm && progress < 1.0 {
                let h = hash(x, y);
                if h % 3 == 0 {
                    let ch = ['*', '+', '.'][(h / 3 % 3) as usize];
                    frame.put(x, y, ch, Some(colours[(h / 9 % 3) as usize]));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flash::{MAX_CHANGES_PER_SECOND, most_changes_per_second, regions};
    use crate::rng::Rng;
    use crate::scenes;
    use crate::theme;

    fn run(motion: Motion, theme: &ThemeData) -> (Vec<Vec<f32>>, Vec<Frame>) {
        let (w, h) = (80, 24);
        let mut rng = Rng::new(3);
        let mut old = scenes::create("storm", w, h, motion, &mut rng).unwrap();
        let mut new = scenes::create("waves", w, h, motion, &mut rng).unwrap();
        let mut measured = Vec::new();
        let mut frames = Vec::new();
        let steps = (length(motion) / 0.05) as usize + 20;
        for step in 0..steps {
            old.step(0.05, &mut rng);
            new.step(0.05, &mut rng);
            let mut frame = Frame::new(w, h);
            let mut behind = Frame::new(w, h);
            new.draw(&mut frame, theme);
            old.draw(&mut behind, theme);
            compose(
                &mut frame,
                &behind,
                step as f32 * 0.05 / length(motion),
                theme,
                motion,
            );
            measured.push(regions(&frame, theme));
            frames.push(frame);
        }
        (measured, frames)
    }

    #[test]
    fn the_opening_never_flashes_and_ends_on_the_new_scene() {
        for theme in theme::all() {
            for motion in [Motion::FULL, Motion::CALM, Motion::LIVELY] {
                let (measured, frames) = run(motion, theme);
                let most = most_changes_per_second(&measured, 20);
                assert!(
                    most <= MAX_CHANGES_PER_SECOND,
                    "{} {motion:?}: {most}",
                    theme.name
                );
                for colour in frames.iter().flat_map(|f| f.colours()) {
                    assert!(theme.contrast_of(colour) >= theme.floor(), "{}", theme.name);
                }
            }
        }
    }

    #[test]
    fn sparkles_trace_the_edge_except_under_reduced_motion() {
        let theme = theme::default_theme();
        let (w, h) = (60, 20);
        let behind = Frame::new(w, h);
        let mut full = Frame::new(w, h);
        compose(&mut full, &behind, 0.5, theme, Motion::FULL);
        assert!(full.drawn() > 0);
        let mut calm = Frame::new(w, h);
        compose(&mut calm, &behind, 0.5, theme, Motion::CALM);
        assert_eq!(calm.drawn(), 0);
        // at the start the old scene fills the screen and at the end the new one does
        let mut old = Frame::new(w, h);
        old.fill_row(5, '~', Some(theme.cyan()));
        let mut start = Frame::new(w, h);
        compose(&mut start, &old, 0.0, theme, Motion::CALM);
        assert_eq!(start.row_text(5).trim_matches('~').len(), 0);
        let mut end = Frame::new(w, h);
        compose(&mut end, &old, 1.0, theme, Motion::FULL);
        assert_eq!(end.drawn(), 0);
        const { assert!(CALM_LENGTH > LENGTH) };
    }
}
