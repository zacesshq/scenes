//! Calm animated scenes for the terminal in the Zacess high contrast themes.
//!
//! The crate is a library so every part can run headless in tests: a scene draws into a
//! [`frame::Frame`], the frame renders to a string and the terminal layer in [`terminal`] is the
//! only module that touches a real terminal.

pub mod ambient;
pub mod app;
pub mod catalogue;
pub mod cli;
pub mod colour;
pub mod config;
pub mod flash;
pub mod frame;
pub mod glyphs;
pub mod json;
pub mod lua;
pub mod motion;
pub mod notify;
pub mod palette;
pub mod report;
pub mod rng;
pub mod scene;
pub mod scenes;
pub mod snapshot;
pub mod sound;
pub mod terminal;
pub mod theme;
pub mod watch;

/// The crate version, shown by `--version`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
