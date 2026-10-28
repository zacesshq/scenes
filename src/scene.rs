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
}

/// How much a scene may move. Calm is the reduced motion setting: slower, fewer things moving,
/// no lightning and no flicker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Motion {
    pub calm: bool,
}

impl Motion {
    pub const FULL: Motion = Motion { calm: false };
    pub const CALM: Motion = Motion { calm: true };

    /// The speed factor a scene multiplies its movement by.
    pub fn speed(self) -> f32 {
        if self.calm { 0.5 } else { 1.0 }
    }
}
