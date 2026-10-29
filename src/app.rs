//! The run loop shared by the interactive and the headless modes, plus the help line.

use std::io;
use std::time::{Duration, Instant};

use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::Scene;
use crate::terminal::{Input, Terminal};
use crate::theme::ThemeData;

/// The frame rate ceiling under reduced motion.
pub const CALM_FPS: u32 = 12;
/// Seconds the help line stays fully visible after the start or after `h`.
pub const HELP_SHOWN: f32 = 4.0;
/// Seconds it then stays in the muted colour before it goes.
pub const HELP_FADING: f32 = 1.5;
/// How long the terminal must stay one size before the scene is laid out again, so dragging a
/// window edge never makes the screen blink between layouts.
pub const RESIZE_SETTLE: Duration = Duration::from_millis(150);
/// The shortest time between two pause toggles. Without it a held Space key, which most
/// terminals send as a stream of presses, would flash the paused line on and off.
pub const PAUSE_DEBOUNCE: Duration = Duration::from_millis(300);
/// How long the loop waits for a key while nothing on screen can change: paused with the help
/// line gone. Short enough that a termination signal is still acted on promptly.
pub const IDLE_POLL: Duration = Duration::from_millis(500);

/// The settings for one run.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub fps: u32,
    /// A multiplier on simulated time: 1 normally, 0.5 with `--slow`.
    pub speed: f32,
    /// Reduced motion.
    pub calm: bool,
    pub duration: Option<Duration>,
    /// False under `NO_COLOR`, when every cell is drawn in the terminal's own text colour.
    pub colour: bool,
}

impl Settings {
    /// The frame rate actually used: reduced motion caps it.
    pub fn frame_rate(&self) -> u32 {
        if self.calm {
            self.fps.min(CALM_FPS)
        } else {
            self.fps
        }
        .max(1)
    }

    /// Seconds between frames.
    pub fn frame_time(&self) -> f32 {
        1.0 / self.frame_rate() as f32
    }
}

/// The one-line help for a terminal width or the paused line.
pub fn help_text(width: usize, paused: bool) -> &'static str {
    if paused {
        if width >= 48 {
            "Paused. Space resumes, q, Esc or Ctrl+C quits."
        } else if width >= 22 {
            "Paused. Space resumes"
        } else {
            "Paused: Space"
        }
    } else if width >= 58 {
        "Space pauses, h shows this again, q, Esc or Ctrl+C quits."
    } else if width >= 30 {
        "Space pauses, q or Esc quits"
    } else {
        "Space pause, q quit"
    }
}

/// Draws the help or paused line on the bottom row. `help_age` is the seconds since the help was
/// asked for (`None` once it has faded). The line is drawn in the text colour, then the muted
/// colour, then not at all, so it never flashes.
pub fn overlay(frame: &mut Frame, theme: &ThemeData, paused: bool, help_age: Option<f32>) {
    let colour = if paused {
        theme.foreground
    } else {
        match help_age {
            Some(age) if age < HELP_SHOWN => theme.foreground,
            Some(age) if age < HELP_SHOWN + HELP_FADING => theme.muted,
            _ => return,
        }
    };
    let row = frame.height() as i32 - 1;
    frame.fill_row(row, ' ', None);
    frame.put_str(0, row, help_text(frame.width(), paused), Some(colour));
}

/// Steps a scene a fixed number of frames with no terminal and returns the last frame.
pub fn render_headless(
    scene: &mut dyn Scene,
    theme: &ThemeData,
    settings: &Settings,
    frames: u32,
    rng: &mut Rng,
    width: usize,
    height: usize,
) -> Frame {
    let dt = settings.frame_time() * settings.speed;
    scene.resize(width, height, rng);
    for _ in 0..frames {
        scene.step(dt, rng);
    }
    let mut frame = Frame::new(width, height);
    scene.draw(&mut frame, theme);
    frame
}

/// Whether a pause key press at `now` toggles the pause. Every press, accepted or not, restarts
/// the debounce window, so a held key toggles once and then nothing until it is let go.
pub fn pause_accepted(last_press: &mut Option<Instant>, now: Instant) -> bool {
    let accepted = last_press.is_none_or(|last| now.duration_since(last) >= PAUSE_DEBOUNCE);
    *last_press = Some(now);
    accepted
}

/// Runs a scene in the terminal until a quit key, a termination signal or the duration ends.
pub fn run(
    scene: &mut dyn Scene,
    theme: &ThemeData,
    settings: &Settings,
    rng: &mut Rng,
    terminal: &mut Terminal,
) -> io::Result<()> {
    let (width, height) = terminal.size()?;
    scene.resize(width, height, rng);
    let mut frame = Frame::new(width, height);
    let frame_time = Duration::from_secs_f32(settings.frame_time());
    let start = Instant::now();
    let mut last = Instant::now();
    let mut paused = false;
    let mut last_pause_press: Option<Instant> = None;
    let mut help_age: Option<f32> = Some(0.0);
    let mut pending_resize: Option<(usize, usize, Instant)> = None;
    let mut needs_draw = true;

    loop {
        // paused with the help gone, the picture cannot change, so it is not redrawn
        let still = paused && help_age.is_none();
        if pending_resize.is_none() && (needs_draw || !still) {
            frame.clear();
            scene.draw(&mut frame, theme);
            overlay(&mut frame, theme, paused, help_age);
            terminal.draw(&frame, settings.colour)?;
            needs_draw = false;
        }

        let idle = still && pending_resize.is_none();
        let deadline = if idle {
            Instant::now() + IDLE_POLL
        } else {
            last + frame_time
        };
        loop {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            let mut wait = deadline - now;
            if let Some((_, _, at)) = pending_resize {
                wait = wait.min((at + RESIZE_SETTLE).saturating_duration_since(now));
            }
            match terminal.poll(wait)? {
                Input::Quit => return Ok(()),
                Input::Pause => {
                    if pause_accepted(&mut last_pause_press, Instant::now()) {
                        paused = !paused;
                        needs_draw = true;
                        if idle {
                            break;
                        }
                    }
                }
                Input::Help => {
                    help_age = Some(0.0);
                    needs_draw = true;
                    if idle {
                        break;
                    }
                }
                Input::Resize(w, h) => pending_resize = Some((w, h, Instant::now())),
                Input::Key(_) | Input::None => {}
            }
            let settled = pending_resize.filter(|&(_, _, at)| at.elapsed() >= RESIZE_SETTLE);
            if let Some((w, h, _)) = settled {
                pending_resize = None;
                scene.resize(w, h, rng);
                frame = Frame::new(w, h);
                terminal.reset()?;
                needs_draw = true;
                break;
            }
        }

        let now = Instant::now();
        // a late frame advances at most two frames' worth, so a stall never makes things jump
        let elapsed = (now - last).as_secs_f32().min(settings.frame_time() * 2.0);
        last = now;
        if !paused {
            scene.step(elapsed * settings.speed, rng);
        }
        if let Some(age) = help_age.as_mut() {
            *age += elapsed;
            if *age >= HELP_SHOWN + HELP_FADING {
                help_age = None;
                needs_draw = true;
            }
        }
        if settings
            .duration
            .is_some_and(|limit| start.elapsed() >= limit)
        {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Motion;
    use crate::scenes;
    use crate::theme;

    fn settings(calm: bool) -> Settings {
        Settings {
            fps: 20,
            speed: 1.0,
            calm,
            duration: None,
            colour: true,
        }
    }

    #[test]
    fn reduced_motion_caps_the_frame_rate() {
        assert_eq!(settings(false).frame_rate(), 20);
        assert_eq!(settings(true).frame_rate(), CALM_FPS);
        let slow = Settings {
            fps: 5,
            ..settings(true)
        };
        assert_eq!(slow.frame_rate(), 5);
        assert!((settings(false).frame_time() - 0.05).abs() < 1e-6);
    }

    #[test]
    fn the_help_line_fits_every_width() {
        for width in [20, 30, 48, 58, 120] {
            for paused in [false, true] {
                let text = help_text(width, paused);
                assert!(text.len() <= width, "{text} does not fit {width}");
            }
        }
        assert!(help_text(120, false).contains("Ctrl+C"));
        assert!(help_text(120, true).starts_with("Paused"));
    }

    #[test]
    fn a_held_pause_key_toggles_once() {
        let start = Instant::now();
        let mut last = None;
        assert!(pause_accepted(&mut last, start));
        // key repeat at 30 a second for two seconds never toggles again
        for i in 1..60 {
            let at = start + Duration::from_millis(33 * i);
            assert!(!pause_accepted(&mut last, at), "toggled at {i}");
        }
        let released = start + Duration::from_millis(33 * 59) + PAUSE_DEBOUNCE;
        assert!(pause_accepted(&mut last, released));
    }

    #[test]
    fn the_paused_line_says_how_to_resume_at_every_width() {
        for width in 20..=120 {
            assert!(help_text(width, true).contains("Space"), "{width}");
        }
    }

    #[test]
    fn the_overlay_shows_then_dims_then_goes() {
        let theme = theme::default_theme();
        let mut frame = Frame::new(60, 10);
        overlay(&mut frame, theme, false, Some(0.0));
        assert!(frame.row_text(9).starts_with("Space pauses"));
        assert_eq!(frame.get(0, 9).unwrap().colour, Some(theme.foreground));

        frame.clear();
        overlay(&mut frame, theme, false, Some(HELP_SHOWN + 0.5));
        assert_eq!(frame.get(0, 9).unwrap().colour, Some(theme.muted));

        frame.clear();
        overlay(&mut frame, theme, false, None);
        assert_eq!(frame.drawn(), 0);

        frame.clear();
        overlay(&mut frame, theme, true, None);
        assert!(frame.row_text(9).starts_with("Paused"));
    }

    #[test]
    fn the_headless_render_is_the_right_size_and_repeatable() {
        let theme = theme::default_theme();
        let mut outputs = Vec::new();
        for _ in 0..2 {
            let mut rng = Rng::new(5);
            let mut scene = scenes::create("snow", 20, 8, Motion::FULL, &mut rng).unwrap();
            let frame =
                render_headless(scene.as_mut(), theme, &settings(false), 30, &mut rng, 20, 8);
            assert_eq!((frame.width(), frame.height()), (20, 8));
            assert!(frame.drawn() > 0);
            outputs.push(frame.to_text());
        }
        assert_eq!(outputs[0], outputs[1]);
    }
}
