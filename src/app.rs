//! The run loop shared by the interactive and the headless modes, plus the help line.

use std::io;
use std::time::{Duration, Instant};

use crate::ambient::Ambient;
use crate::catalogue::Catalogue;
use crate::config::Rotation;
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::terminal::{Input, Terminal};
use crate::theme::ThemeData;
use crate::watch::Lineup;

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
/// The shortest time between two scene changes from the keyboard, so a held or hammered key
/// never replaces the whole picture more than three times a second.
pub const SCENE_DEBOUNCE: Duration = Duration::from_millis(400);
/// Seconds a scene's name stays on the bottom row after `n` or `b`, before it dims and goes.
pub const LABEL_SHOWN: f32 = 2.5;
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
    } else if width >= 84 {
        "Space pauses, n and b change the scene, h shows this again, q, Esc or Ctrl+C quits."
    } else if width >= 58 {
        "Space pauses, n next scene, h shows this again, q quits."
    } else if width >= 36 {
        "Space pauses, n next, q or Esc quits"
    } else if width >= 30 {
        "Space pauses, q or Esc quits"
    } else {
        "Space pause, q quit"
    }
}

/// Draws the help or paused line on the bottom row. `help_age` is the seconds since the help was
/// asked for (`None` once it has faded). The line is drawn in the text colour, then the muted
/// colour, then not at all, so it never flashes. A label, such as a new scene's name, takes the
/// help line's place while it shows.
pub fn overlay(
    frame: &mut Frame,
    theme: &ThemeData,
    paused: bool,
    help_age: Option<f32>,
    label: Option<(&str, f32)>,
) {
    let row = frame.height() as i32 - 1;
    if !paused {
        if let Some((text, age)) = label.filter(|(_, age)| *age < LABEL_SHOWN + HELP_FADING) {
            let colour = if age < LABEL_SHOWN {
                theme.foreground
            } else {
                theme.muted
            };
            let text: String = text.chars().take(frame.width()).collect();
            frame.fill_row(row, ' ', None);
            frame.put_str(0, row, &text, Some(colour));
            return;
        }
    }
    let colour = if paused {
        theme.foreground
    } else {
        match help_age {
            Some(age) if age < HELP_SHOWN => theme.foreground,
            Some(age) if age < HELP_SHOWN + HELP_FADING => theme.muted,
            _ => return,
        }
    };
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

/// The shortest time between scenes on a timer.
pub const MIN_EVERY: Duration = Duration::from_secs(3);

/// Moving to another scene on a timer, through a list of scenes.
pub struct Auto {
    pub rotation: Rotation,
    pub every: Duration,
    pub lineup: Lineup,
}

impl Auto {
    /// A timer over `names` that starts on `first` when it is in the list. Fixed rotation is
    /// taken as cycle, since a timer that never moves would do nothing.
    pub fn new(rotation: Rotation, every: Duration, names: Vec<String>, first: &str) -> Self {
        let rotation = if rotation == Rotation::Fixed {
            Rotation::Cycle
        } else {
            rotation
        };
        // each change replaces the whole picture, so however small the asked for time, scenes
        // never change faster than this
        let every = every.max(MIN_EVERY);
        let mut lineup = Lineup::new(names, first);
        if let Some(at) = lineup.names.iter().position(|n| n == first) {
            lineup.index = at;
        }
        Self {
            rotation,
            every,
            lineup,
        }
    }
}

/// The scenes the standalone mode steps through with `n` and `b` and the one showing.
pub struct Playlist<'a> {
    pub catalogue: &'a mut Catalogue,
    pub names: Vec<String>,
    pub index: usize,
    pub motion: Motion,
    /// Moving on by itself, when asked for.
    pub auto: Option<Auto>,
    /// The background sound, when there is a player for it.
    pub ambient: Option<Ambient>,
}

impl Playlist<'_> {
    /// Builds the scene at the index, skipping any user scene whose script fails and retiring
    /// it. Returns the scene and a note about anything skipped.
    fn build(
        &mut self,
        width: usize,
        height: usize,
        rng: &mut Rng,
    ) -> (Box<dyn Scene>, Option<String>) {
        let mut note = None;
        for _ in 0..self.names.len().max(1) {
            let name = self.names[self.index % self.names.len()].clone();
            match self
                .catalogue
                .create(&name, width, height, self.motion, rng)
            {
                Ok(scene) => return (scene, note),
                Err(error) => {
                    note = Some(error);
                    self.catalogue.retire(&name);
                    self.names.retain(|n| *n != name);
                    if self.names.is_empty() {
                        break;
                    }
                    self.index %= self.names.len();
                }
            }
        }
        self.names = vec!["sea".to_string()];
        self.index = 0;
        let scene = crate::scenes::create("sea", width, height, self.motion, rng)
            .expect("the default scene is built in");
        (scene, note)
    }

    fn step(&mut self, by: isize) {
        let len = self.names.len().max(1) as isize;
        self.index = (self.index as isize + by).rem_euclid(len) as usize;
    }

    fn current(&self) -> &str {
        &self.names[self.index % self.names.len()]
    }
}

/// The line shown when the scene changes: its name and summary (why a scene was skipped instead).
fn scene_label(playlist: &Playlist, note: Option<String>) -> String {
    if let Some(note) = note {
        return format!("scene error: {note}");
    }
    let summary = playlist
        .catalogue
        .find(playlist.current())
        .map(|e| e.summary.clone())
        .unwrap_or_default();
    format!("{}: {summary}", playlist.current())
}

/// Runs scenes in the terminal until a quit key, a termination signal or the duration ends. `n`
/// and `b` move through the playlist.
pub fn run(
    playlist: &mut Playlist,
    theme: &ThemeData,
    settings: &Settings,
    rng: &mut Rng,
    terminal: &mut Terminal,
) -> io::Result<()> {
    let (width, height) = terminal.size()?;
    let (mut scene, mut label) = {
        let (scene, note) = playlist.build(width, height, rng);
        (scene, note.map(|n| (format!("scene error: {n}"), 0.0)))
    };
    let mut frame = Frame::new(width, height);
    let frame_time = Duration::from_secs_f32(settings.frame_time());
    let start = Instant::now();
    let mut last = Instant::now();
    let mut paused = false;
    let mut last_pause_press: Option<Instant> = None;
    let mut last_scene_press: Option<Instant> = None;
    let mut help_age: Option<f32> = Some(0.0);
    let mut pending_resize: Option<(usize, usize, Instant)> = None;
    let mut needs_draw = true;
    let mut picked_at = Instant::now();

    loop {
        if let Some(ambient) = playlist.ambient.as_mut() {
            let name = playlist.names[playlist.index % playlist.names.len()].clone();
            ambient.set_scene(&name);
            ambient.pause(paused);
            ambient.tick();
        }
        let due = playlist
            .auto
            .as_ref()
            .is_some_and(|auto| !paused && picked_at.elapsed() >= auto.every);
        if due {
            picked_at = Instant::now();
            let showing = playlist.current().to_string();
            if let Some(auto) = playlist.auto.as_mut() {
                // a pick that is the scene already showing, after n or b or a retired scene, is
                // passed over so the timer never spends a turn on no change
                for _ in 0..auto.lineup.names.len().max(1) {
                    auto.lineup.rotate(auto.rotation, rng);
                    let next = auto.lineup.current();
                    if next != showing && playlist.names.iter().any(|n| n == next) {
                        break;
                    }
                }
                let next = auto.lineup.current().to_string();
                if let Some(at) = playlist.names.iter().position(|n| *n == next) {
                    playlist.index = at;
                }
            }
            let (w, h) = (frame.width(), frame.height());
            let (next, note) = playlist.build(w, h, rng);
            scene = next;
            label = Some((scene_label(playlist, note), 0.0));
            needs_draw = true;
        }
        if let Some(problem) = scene.problem() {
            let name = playlist.current().to_string();
            playlist.catalogue.retire(&name);
            playlist.names.retain(|n| *n != name);
            if playlist.names.is_empty() {
                playlist.names.push("sea".to_string());
            }
            playlist.index %= playlist.names.len();
            let (w, h) = (frame.width(), frame.height());
            scene = playlist.build(w, h, rng).0;
            label = Some((format!("scene error: {problem}"), 0.0));
            needs_draw = true;
        }
        // paused with the help and the label gone, the picture cannot change, so it is not
        // redrawn
        let still = paused && help_age.is_none() && label.is_none();
        if pending_resize.is_none() && (needs_draw || !still) {
            frame.clear();
            scene.draw(&mut frame, theme);
            overlay(
                &mut frame,
                theme,
                paused,
                help_age,
                label.as_ref().map(|(t, a)| (t.as_str(), *a)),
            );
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
                Input::Key(key @ ('n' | 'b')) => {
                    let now = Instant::now();
                    let accepted =
                        last_scene_press.is_none_or(|at| now.duration_since(at) >= SCENE_DEBOUNCE);
                    last_scene_press = Some(now);
                    if accepted {
                        playlist.step(if key == 'n' { 1 } else { -1 });
                        let (w, h) = (frame.width(), frame.height());
                        let (next, note) = playlist.build(w, h, rng);
                        scene = next;
                        label = Some((scene_label(playlist, note), 0.0));
                        // a scene picked by hand gets a full turn before the timer moves on
                        picked_at = Instant::now();
                        needs_draw = true;
                        if idle {
                            break;
                        }
                    }
                }
                Input::Key(key @ ('a' | 'm' | '+' | '-' | '[' | ']')) => {
                    let text = match playlist.ambient.as_mut() {
                        None => "no sound player for the background sound".to_string(),
                        Some(ambient) if matches!(key, 'a' | 'm') => {
                            ambient.toggle();
                            let state = if ambient.enabled() { "on" } else { "off" };
                            format!("background sound {state}")
                        }
                        Some(ambient) => {
                            let volume = ambient.change_volume(matches!(key, '+' | ']'));
                            format!("background sound volume {volume}")
                        }
                    };
                    label = Some((text, 0.0));
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
        if let Some((_, age)) = label.as_mut() {
            *age += elapsed;
            if *age >= LABEL_SHOWN + HELP_FADING {
                label = None;
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
        for width in [20, 30, 36, 48, 58, 84, 120] {
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
        overlay(&mut frame, theme, false, Some(0.0), None);
        assert!(frame.row_text(9).starts_with("Space pauses"));
        assert_eq!(frame.get(0, 9).unwrap().colour, Some(theme.foreground));

        frame.clear();
        overlay(&mut frame, theme, false, Some(HELP_SHOWN + 0.5), None);
        assert_eq!(frame.get(0, 9).unwrap().colour, Some(theme.muted));

        frame.clear();
        overlay(&mut frame, theme, false, None, None);
        assert_eq!(frame.drawn(), 0);

        frame.clear();
        overlay(&mut frame, theme, true, None, None);
        assert!(frame.row_text(9).starts_with("Paused"));

        frame.clear();
        overlay(
            &mut frame,
            theme,
            false,
            Some(0.0),
            Some(("koi-pond: a pond", 0.5)),
        );
        assert!(frame.row_text(9).starts_with("koi-pond"));
        frame.clear();
        overlay(
            &mut frame,
            theme,
            false,
            None,
            Some(("koi-pond", LABEL_SHOWN + 0.5)),
        );
        assert_eq!(frame.get(0, 9).unwrap().colour, Some(theme.muted));
        frame.clear();
        overlay(
            &mut frame,
            theme,
            false,
            None,
            Some(("koi-pond", LABEL_SHOWN + HELP_FADING)),
        );
        assert_eq!(frame.drawn(), 0);
    }

    #[test]
    fn the_timer_never_changes_scenes_faster_than_the_floor_and_always_moves() {
        let names = vec!["sea".to_string(), "rain".to_string(), "snow".to_string()];
        let auto = Auto::new(Rotation::Fixed, Duration::from_millis(10), names, "rain");
        assert_eq!(auto.every, MIN_EVERY);
        assert_eq!(
            auto.rotation,
            Rotation::Cycle,
            "a timer that never moves does nothing"
        );
        assert_eq!(
            auto.lineup.current(),
            "rain",
            "it starts on the scene asked for"
        );
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
