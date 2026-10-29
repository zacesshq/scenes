//! Measuring flashes. A frame is split into regions and each region's brightness is worked out
//! from how much of each cell its character covers and how far its colour's luminance is from the
//! background. A large change in any region (or in the whole frame) between two frames counts as
//! one flash step. Moving things keep a region's brightness about level, so a drop of rain or a
//! drifting cloud never counts, while a large area lighting up or going dark does.
//!
//! The tests hold every built-in scene to the limit and the run loop holds user scenes to it
//! while they run, damping a frame that would break it.

use std::collections::VecDeque;

use crate::frame::{Cell, Frame};
use crate::theme::ThemeData;

/// The most large changes allowed in any one second. Two opposing changes make one flash, so
/// this is at most one and a half flashes a second, half the three a second the guidelines allow.
pub const MAX_CHANGES_PER_SECOND: usize = 3;
/// The change in a region's brightness that counts as large, as a share of the most a region can
/// change: a tenth of it filled with solid blocks or about half of it gaining text.
pub const LARGE_CHANGE: f32 = 0.1;
/// Columns and rows of regions the frame is split into, besides the frame as a whole.
const GRID: (usize, usize) = (4, 3);

/// How much of its cell a character covers, roughly.
pub fn coverage(ch: char) -> f32 {
    match ch {
        ' ' => 0.0,
        '\u{2588}' => 1.0,
        '\u{2593}' => 0.75,
        '\u{2592}' | '\u{2580}' | '\u{2584}' | '\u{258c}' | '\u{2590}' | '\u{25e3}'
        | '\u{25e5}' => 0.5,
        '\u{2591}' => 0.25,
        '.' | ',' | '\'' | '`' | '\u{b7}' | '-' | '_' | ':' => 0.1,
        _ => 0.2,
    }
}

/// How bright a cell looks against the background, from 0 to 1.
fn ink(cell: Cell, theme: &ThemeData) -> f32 {
    let cover = coverage(cell.ch);
    if cover == 0.0 {
        return 0.0;
    }
    let colour = cell.colour.unwrap_or(theme.foreground);
    let difference = (colour.luminance() - theme.background.luminance()).abs() as f32;
    cover * difference
}

/// The mean brightness of each region and then of the whole frame.
pub fn regions(frame: &Frame, theme: &ThemeData) -> Vec<f32> {
    let (columns, rows) = GRID;
    let width = frame.width();
    let height = frame.height();
    let mut sums = vec![0.0_f32; columns * rows + 1];
    let mut counts = vec![0_usize; columns * rows + 1];
    for (y, row) in frame.rows().enumerate() {
        let ry = (y * rows / height).min(rows - 1);
        for (x, cell) in row.iter().enumerate() {
            let rx = (x * columns / width).min(columns - 1);
            let value = ink(*cell, theme);
            let i = ry * columns + rx;
            sums[i] += value;
            counts[i] += 1;
            sums[columns * rows] += value;
            counts[columns * rows] += 1;
        }
    }
    sums.iter()
        .zip(&counts)
        .map(|(sum, count)| {
            if *count == 0 {
                0.0
            } else {
                sum / *count as f32
            }
        })
        .collect()
}

/// Whether two measurements differ by a large change in any region.
pub fn large_change(before: &[f32], after: &[f32]) -> bool {
    before
        .iter()
        .zip(after)
        .any(|(a, b)| (a - b).abs() >= LARGE_CHANGE)
}

/// The most large changes in any one second of `frames` drawn at `fps`.
pub fn most_changes_per_second(frames: &[Vec<f32>], fps: usize) -> usize {
    let steps: Vec<usize> = (1..frames.len())
        .filter(|&i| large_change(&frames[i - 1], &frames[i]))
        .collect();
    let mut most = 0;
    let mut start = 0;
    for end in 0..steps.len() {
        while steps[end] - steps[start] >= fps.max(1) {
            start += 1;
        }
        most = most.max(end - start + 1);
    }
    most
}

/// What the meter decided about a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The frame can be shown.
    Show,
    /// The frame would break the limit, so the last frame shown stays instead.
    Hold,
}

/// Watches frames as they are shown and holds back any that would change a region too often.
#[derive(Clone, Debug, Default)]
pub struct Meter {
    /// The measurement of the last frame shown.
    last: Option<Vec<f32>>,
    /// When the recent large changes were shown, in seconds of the meter's clock.
    changes: VecDeque<f64>,
    /// When recent frames were held back.
    held: VecDeque<f64>,
    now: f64,
}

impl Meter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Moves the meter's clock on.
    pub fn advance(&mut self, dt: f32) {
        self.now += f64::from(dt.max(0.0));
        while self.changes.front().is_some_and(|&t| self.now - t >= 1.0) {
            self.changes.pop_front();
        }
        while self.held.front().is_some_and(|&t| self.now - t >= 10.0) {
            self.held.pop_front();
        }
    }

    /// Forgets the last frame, after a resize or a new scene, which are a single change of their
    /// own.
    pub fn reset(&mut self) {
        self.last = None;
    }

    /// Judges a frame about to be shown.
    pub fn judge(&mut self, frame: &Frame, theme: &ThemeData) -> Verdict {
        let now = regions(frame, theme);
        let Some(last) = &self.last else {
            self.last = Some(now);
            return Verdict::Show;
        };
        if !large_change(last, &now) {
            self.last = Some(now);
            return Verdict::Show;
        }
        if self.changes.len() >= MAX_CHANGES_PER_SECOND {
            self.held.push_back(self.now);
            return Verdict::Hold;
        }
        self.changes.push_back(self.now);
        self.last = Some(now);
        Verdict::Show
    }

    /// How many frames were held back in the last ten seconds.
    pub fn held_recently(&self) -> usize {
        self.held.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme;

    fn filled(width: usize, height: usize, ch: char) -> Frame {
        let mut frame = Frame::new(width, height);
        let colour = Some(theme::default_theme().foreground);
        for y in 0..height as i32 {
            frame.fill_row(y, ch, colour);
        }
        frame
    }

    #[test]
    fn a_whole_screen_flashing_is_caught() {
        let theme = theme::default_theme();
        let on = regions(&filled(40, 12, '\u{2588}'), theme);
        let off = regions(&Frame::new(40, 12), theme);
        assert!(large_change(&on, &off));
        let frames: Vec<Vec<f32>> = (0..40)
            .map(|i| if i % 2 == 0 { on.clone() } else { off.clone() })
            .collect();
        assert!(most_changes_per_second(&frames, 20) > MAX_CHANGES_PER_SECOND);
    }

    #[test]
    fn one_corner_flashing_is_caught_too() {
        let theme = theme::default_theme();
        let mut corner = Frame::new(80, 24);
        for y in 0..8 {
            corner.put_str(0, y, &"\u{2588}".repeat(20), Some(theme.foreground));
        }
        assert!(large_change(
            &regions(&Frame::new(80, 24), theme),
            &regions(&corner, theme)
        ));
    }

    #[test]
    fn a_small_moving_shape_is_not_a_flash() {
        let theme = theme::default_theme();
        let mut a = Frame::new(80, 24);
        let mut b = Frame::new(80, 24);
        a.put_str(10, 5, "><(((o>", Some(theme.foreground));
        b.put_str(11, 5, "><(((o>", Some(theme.foreground));
        assert!(!large_change(&regions(&a, theme), &regions(&b, theme)));
    }

    #[test]
    fn the_meter_holds_frames_past_the_limit() {
        let theme = theme::default_theme();
        let on = filled(40, 12, '\u{2588}');
        let off = Frame::new(40, 12);
        let mut meter = Meter::new();
        let mut shown = Vec::new();
        for i in 0..200 {
            meter.advance(0.05);
            let frame = if i % 2 == 0 { &on } else { &off };
            if meter.judge(frame, theme) == Verdict::Show {
                shown.push(regions(frame, theme));
            } else if let Some(last) = shown.last().cloned() {
                shown.push(last);
            }
        }
        assert!(most_changes_per_second(&shown, 20) <= MAX_CHANGES_PER_SECOND);
        assert!(meter.held_recently() > 0);
    }
}
