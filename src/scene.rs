//! The trait every scene implements and the motion setting it is built with.

use crate::frame::Frame;
use crate::rng::Rng;
use crate::theme::ThemeData;

/// A scene: a small simulation that advances in time and draws itself into a frame.
pub trait Scene {
    /// The name used on the command line.
    fn name(&self) -> &'static str;

    /// Fits the scene to a terminal size. Called before the first frame and on every resize.
    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng);

    /// Advances the simulation by `dt` seconds.
    fn step(&mut self, dt: f32, rng: &mut Rng);

    /// Draws the current state into a cleared frame with the theme's colours.
    fn draw(&self, frame: &mut Frame, theme: &ThemeData);

    /// A problem that means the scene should be swapped for a built-in one, such as an error in
    /// a user scene's script. Built-in scenes never have one.
    fn problem(&self) -> Option<String> {
        None
    }

    /// A caption the scene offers for a moment in the watch mode, from a user scene's hook.
    fn caption_for(&mut self, _moment: Moment) -> Option<String> {
        None
    }
}

/// A moment in the watch mode a scene can offer a caption for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moment {
    Pass,
    Fail,
    Recovered,
}

/// How busy and quick the scenes are, chosen with `--motion` or the config file. Lively is faster
/// and busier but never changes anything in place faster than standard does, so no level can
/// break the flash limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Level {
    Calm,
    #[default]
    Standard,
    Lively,
}

impl Level {
    pub const ALL: [Level; 3] = [Level::Calm, Level::Standard, Level::Lively];

    pub fn name(self) -> &'static str {
        match self {
            Level::Calm => "calm",
            Level::Standard => "standard",
            Level::Lively => "lively",
        }
    }

    pub fn parse(text: &str) -> Option<Level> {
        let text = text.trim().to_ascii_lowercase();
        Level::ALL.into_iter().find(|level| level.name() == text)
    }
}

/// How much a scene may move. `calm` is the reduced motion setting: slower, fewer things moving,
/// no lightning and no flicker. The level sets the pace on top of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Motion {
    pub calm: bool,
    pub level: Level,
}

impl Motion {
    pub const FULL: Motion = Motion {
        calm: false,
        level: Level::Standard,
    };
    pub const CALM: Motion = Motion {
        calm: true,
        level: Level::Standard,
    };
    pub const LIVELY: Motion = Motion {
        calm: false,
        level: Level::Lively,
    };
    pub const GENTLE: Motion = Motion {
        calm: false,
        level: Level::Calm,
    };

    /// Every combination the tests check.
    pub const ALL: [Motion; 6] = [
        Motion::FULL,
        Motion::CALM,
        Motion::LIVELY,
        Motion::GENTLE,
        Motion {
            calm: true,
            level: Level::Lively,
        },
        Motion {
            calm: true,
            level: Level::Calm,
        },
    ];

    pub fn new(reduced: bool, level: Level) -> Self {
        Motion {
            calm: reduced,
            level,
        }
    }

    fn level_factor(self) -> f32 {
        match self.level {
            Level::Calm => 0.7,
            Level::Standard => 1.0,
            Level::Lively => 1.35,
        }
    }

    /// The speed factor a scene multiplies its movement by.
    pub fn speed(self) -> f32 {
        self.level_factor() * if self.calm { 0.5 } else { 1.0 }
    }

    /// The factor for anything that changes in place, such as a pulse, a wing beat or a flicker.
    /// It is never above the standard pace, so lively cannot make a cell change faster.
    pub fn pace(self) -> f32 {
        self.speed().min(if self.calm { 0.5 } else { 1.0 })
    }

    /// The factor a scene multiplies its counts by: fewer things on calm, more on lively.
    pub fn busy(self) -> f32 {
        match self.level {
            Level::Calm => 0.75,
            Level::Standard => 1.0,
            Level::Lively if self.calm => 1.0,
            Level::Lively => 1.3,
        }
    }

    /// A count scaled by `busy`, at least one.
    pub fn scaled(self, count: usize) -> usize {
        ((count as f32 * self.busy()).round() as usize).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_parse_and_keep_in_place_changes_at_the_standard_pace() {
        assert_eq!(Level::parse(" Lively "), Some(Level::Lively));
        assert_eq!(Level::parse("fast"), None);
        assert_eq!(Motion::FULL.speed(), 1.0);
        assert_eq!(Motion::CALM.speed(), 0.5);
        assert!(Motion::LIVELY.speed() > 1.0);
        for motion in Motion::ALL {
            assert!(motion.pace() <= 1.0);
            if motion.calm {
                assert!(motion.pace() <= 0.5 && motion.speed() < 1.0);
            }
        }
        assert!(Motion::LIVELY.busy() > 1.0 && Motion::GENTLE.busy() < 1.0);
        assert_eq!(Motion::GENTLE.scaled(1), 1);
    }
}
