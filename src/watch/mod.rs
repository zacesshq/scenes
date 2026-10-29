//! The watch mode: bacon runs a job in the current Cargo project and the screen shows how the
//! build stands, a calm sea while it passes and a storm while it fails, with a status bar and an
//! optional tone on each change. Every tone has the same state on screen, so sound is never the
//! only signal.

pub mod bacon;
pub mod view;

use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::app::{HELP_FADING, HELP_SHOWN, RESIZE_SETTLE, Settings};
use crate::frame::Frame;
use crate::report::Report;
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::scenes::{self, rain::Rain};
use crate::sound::{Sound, Tone};
use crate::terminal::{Input, Terminal};
use crate::theme::ThemeData;

use self::bacon::{Bacon, Event};
use self::view::{Phase, View};

/// The jobs the watch mode runs, in the order the number keys pick them.
pub const JOBS: [&str; 4] = ["check", "check-all", "clippy", "test"];

/// The calm scene shown while waiting and while the build passes.
pub const CALM_SCENE: &str = "waves";

/// Lines shown under ALL GOOD, one per passing run in turn.
pub const CAPTIONS: [&str; 6] = [
    "Everything builds. Time to stretch your legs.",
    "A clean run. The borrow checker is content.",
    "Nothing to fix. On to the next thing.",
    "All quiet in the compiler.",
    "Green from end to end.",
    "The build is happy. So is the sea.",
];

/// How often the report file is looked at.
const REPORT_POLL: Duration = Duration::from_millis(250);

/// The Cargo project a folder belongs to: the nearest folder at or above it with a
/// `Cargo.toml`.
pub fn find_project(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| dir.join("Cargo.toml").is_file())
        .map(Path::to_path_buf)
}

/// The package name from a `Cargo.toml`, read without a TOML parser: the first `name` in the
/// `[package]` table.
pub fn package_name(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            if key.trim() == "name" {
                let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

/// The name shown for a project: its package name, otherwise its folder's name, as for a
/// workspace.
pub fn project_name(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("Cargo.toml"))
        .ok()
        .and_then(|text| package_name(&text))
        .or_else(|| dir.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "project".to_string())
}

/// The message for a folder that is not in a Cargo project.
pub fn not_a_project(dir: &Path) -> String {
    format!(
        "{} is not inside a Cargo project: no Cargo.toml here or in any folder above. Run \
         zacess-scenes watch from a Rust project's folder.",
        dir.display()
    )
}

/// The message for a system without bacon.
pub fn bacon_missing() -> String {
    format!(
        "the watch mode runs bacon, which is not installed or not on PATH. Install it with:\n\n    {}\n",
        bacon::INSTALL
    )
}

/// The tone for a finished run, played only when the result differs from the last one shown,
/// or when the run was asked for with a key, so saving a file that still fails stays quiet.
pub fn tone_for(previous: Phase, now: Phase, asked: bool) -> Option<Tone> {
    let changed = previous != now;
    match now {
        Phase::Passing if changed || asked => Some(Tone::Pass),
        Phase::Failing if changed || asked => Some(Tone::Fail),
        _ => None,
    }
}

/// The watch mode's state, apart from the terminal and bacon itself.
pub struct Watch {
    pub project: String,
    pub job: String,
    pub phase: Phase,
    pub running: bool,
    pub report: Option<Report>,
    pub finished_at: Option<Instant>,
    pub message: Option<String>,
    /// How many runs have passed, which picks the caption.
    pub passes: usize,
    /// Set by a key that asks for a run, so its result always sounds.
    pub asked: bool,
}

impl Watch {
    pub fn new(project: String, job: String) -> Self {
        Self {
            project,
            job,
            phase: Phase::Waiting,
            running: true,
            report: None,
            finished_at: None,
            message: None,
            passes: 0,
            asked: false,
        }
    }

    /// Takes in what bacon reported and returns the tone to play, if any.
    pub fn apply(&mut self, event: Event, now: Instant) -> Option<Tone> {
        match event {
            Event::Started => {
                let first = self.phase == Phase::Waiting;
                self.running = true;
                (!first).then_some(Tone::Tick)
            }
            Event::Finished(report) => {
                let previous = self.phase;
                self.phase = if report.failed() {
                    Phase::Failing
                } else {
                    self.passes += 1;
                    Phase::Passing
                };
                self.report = Some(report);
                self.running = false;
                self.finished_at = Some(now);
                let tone = tone_for(previous, self.phase, self.asked);
                self.asked = false;
                tone
            }
            Event::Exited(line) => {
                self.phase = Phase::Stopped;
                self.running = false;
                self.message = Some(line);
                None
            }
        }
    }

    /// Starts over for a new job or a rerun.
    pub fn restart(&mut self, job: &str) {
        if self.job != job {
            // a different job's last result says nothing about this one
            self.phase = Phase::Waiting;
            self.report = None;
            self.finished_at = None;
        } else if self.phase == Phase::Stopped {
            self.phase = Phase::Waiting;
        }
        self.job = job.to_string();
        self.message = None;
        self.running = true;
        self.asked = true;
    }

    pub fn caption(&self) -> &'static str {
        CAPTIONS[self.passes.saturating_sub(1) % CAPTIONS.len()]
    }

    /// The scene this phase calls for: the storm while failing, the calm sea otherwise.
    pub fn wants_storm(&self) -> bool {
        self.phase == Phase::Failing
    }
}

/// Builds the scene for a phase.
fn make_scene(
    storm: bool,
    width: usize,
    height: usize,
    motion: Motion,
    rng: &mut Rng,
) -> Box<dyn Scene> {
    if storm {
        Box::new(Rain::storm(width, height, motion, rng))
    } else {
        scenes::create(CALM_SCENE, width, height, motion, rng)
            .expect("the calm scene is in the registry")
    }
}

/// Everything the run loop needs that is not the terminal.
pub struct Context<'a> {
    pub theme: &'a ThemeData,
    pub settings: Settings,
    pub bacon: PathBuf,
    pub project_dir: PathBuf,
    pub report_path: PathBuf,
    pub sound: Sound,
}

fn start_bacon(context: &Context, job: &str) -> io::Result<Bacon> {
    Bacon::start(
        &context.bacon,
        &context.project_dir,
        job,
        &context.report_path,
    )
}

/// Runs the watch mode until a quit key or a termination signal.
pub fn run(
    context: &mut Context,
    watch: &mut Watch,
    rng: &mut Rng,
    terminal: &mut Terminal,
) -> io::Result<()> {
    let motion = if context.settings.calm {
        Motion::CALM
    } else {
        Motion::FULL
    };
    let mut bacon = Some(start_bacon(context, &watch.job)?);
    let (mut width, mut height) = terminal.size()?;
    let mut storm = watch.wants_storm();
    let mut scene = make_scene(storm, width, height, motion, rng);
    let mut frame = Frame::new(width, height);
    let frame_time = Duration::from_secs_f32(context.settings.frame_time());
    let mut last = Instant::now();
    let mut last_report_poll = Instant::now();
    let mut paused = false;
    let mut hint_age: Option<f32> = Some(0.0);
    let mut pending_resize: Option<(usize, usize, Instant)> = None;

    loop {
        if pending_resize.is_none() {
            if watch.wants_storm() != storm {
                // the scene changes in one step, with no fade or flash between the two
                storm = watch.wants_storm();
                scene = make_scene(storm, width, height, motion, rng);
            }
            let mut notes = Vec::new();
            if paused {
                notes.push("paused");
            }
            if !context.sound.enabled() {
                notes.push("sound off");
            } else if !context.sound.available() {
                notes.push("no sound player");
            }
            let view = View {
                project: &watch.project,
                job: &watch.job,
                phase: watch.phase,
                running: watch.running,
                report: watch.report.as_ref(),
                ago: watch.finished_at.map(|at| at.elapsed().as_secs()),
                caption: watch.caption(),
                message: watch.message.as_deref(),
                notes,
                hint_age,
            };
            frame.clear();
            scene.draw(&mut frame, context.theme);
            view::draw(&mut frame, context.theme, &view);
            terminal.draw(&frame, context.settings.colour)?;
        }

        let deadline = last + frame_time;
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
                Input::Pause | Input::Key('p') => paused = !paused,
                Input::Help => hint_age = Some(0.0),
                Input::Key('s') => context.sound.toggle(),
                Input::Key('r') => {
                    let job = watch.job.clone();
                    // the old bacon is stopped before the new one starts, so they never share
                    // the report file
                    drop(bacon.take());
                    watch.restart(&job);
                    bacon = Some(start_bacon(context, &job)?);
                }
                Input::Key(digit @ '1'..='4') => {
                    let job = JOBS[digit as usize - '1' as usize];
                    drop(bacon.take());
                    watch.restart(job);
                    bacon = Some(start_bacon(context, job)?);
                }
                Input::Resize(w, h) => pending_resize = Some((w, h, Instant::now())),
                Input::Key(_) | Input::None => {}
            }
            let settled = pending_resize.filter(|&(_, _, at)| at.elapsed() >= RESIZE_SETTLE);
            if let Some((w, h, _)) = settled {
                pending_resize = None;
                (width, height) = (w, h);
                scene.resize(w, h, rng);
                frame = Frame::new(w, h);
                terminal.reset()?;
                break;
            }
        }

        if last_report_poll.elapsed() >= REPORT_POLL {
            last_report_poll = Instant::now();
            let events = bacon.as_mut().map(Bacon::poll).unwrap_or_default();
            for event in events {
                let exited = matches!(event, Event::Exited(_));
                if let Some(tone) = watch.apply(event, Instant::now()) {
                    context.sound.play(tone);
                }
                if exited {
                    bacon = None;
                    break;
                }
            }
        }

        let now = Instant::now();
        let elapsed = (now - last)
            .as_secs_f32()
            .min(context.settings.frame_time() * 2.0);
        last = now;
        if !paused {
            scene.step(elapsed * context.settings.speed, rng);
        }
        if let Some(age) = hint_age.as_mut() {
            *age += elapsed;
            if *age >= HELP_SHOWN + HELP_FADING {
                hint_age = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{Item, Kind};

    fn failing() -> Report {
        Report {
            errors: 1,
            exit_code: Some(101),
            items: vec![Item {
                kind: Kind::Error,
                message: "error: oops".into(),
                location: None,
            }],
            ..Report::default()
        }
    }

    #[test]
    fn finds_the_project_from_a_folder_inside_it() {
        let root = std::env::temp_dir().join(format!("zacess-watch-{}", std::process::id()));
        let inner = root.join("src").join("deep");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = []\n\n[package]\nname = \"sample\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        assert_eq!(find_project(&inner).as_deref(), Some(root.as_path()));
        assert_eq!(project_name(&root), "sample");
        let _ = std::fs::remove_dir_all(&root);
        assert!(not_a_project(Path::new("/nowhere")).contains("Cargo.toml"));
        assert!(bacon_missing().contains("cargo install --locked bacon"));
    }

    #[test]
    fn reads_the_package_name_and_skips_other_tables() {
        assert_eq!(
            package_name("[dependencies]\nname = \"x\"\n[package]\nname = 'real'\n"),
            Some("real".into())
        );
        assert_eq!(package_name("[workspace]\nmembers = [\"a\"]\n"), None);
    }

    #[test]
    fn tones_follow_changes_and_requests_only() {
        assert_eq!(
            tone_for(Phase::Waiting, Phase::Passing, false),
            Some(Tone::Pass)
        );
        assert_eq!(
            tone_for(Phase::Passing, Phase::Failing, false),
            Some(Tone::Fail)
        );
        assert_eq!(tone_for(Phase::Failing, Phase::Failing, false), None);
        assert_eq!(tone_for(Phase::Passing, Phase::Passing, false), None);
        assert_eq!(
            tone_for(Phase::Passing, Phase::Passing, true),
            Some(Tone::Pass)
        );
        assert_eq!(tone_for(Phase::Passing, Phase::Stopped, true), None);
    }

    #[test]
    fn the_state_moves_from_waiting_to_passing_to_failing_and_back() {
        let now = Instant::now();
        let mut watch = Watch::new("sample".into(), "check".into());
        assert_eq!(watch.phase, Phase::Waiting);
        assert!(watch.running);
        // the first start is not a rebuild, so it has no tick
        assert_eq!(watch.apply(Event::Started, now), None);
        assert_eq!(
            watch.apply(Event::Finished(Report::default()), now),
            Some(Tone::Pass)
        );
        assert_eq!(watch.phase, Phase::Passing);
        assert!(!watch.running && !watch.wants_storm());
        assert_eq!(watch.caption(), CAPTIONS[0]);
        assert_eq!(watch.apply(Event::Started, now), Some(Tone::Tick));
        assert!(watch.running);
        assert_eq!(
            watch.apply(Event::Finished(failing()), now),
            Some(Tone::Fail)
        );
        assert_eq!(watch.phase, Phase::Failing);
        assert!(watch.wants_storm());
        assert_eq!(watch.apply(Event::Finished(failing()), now), None);
        assert_eq!(
            watch.apply(Event::Finished(Report::default()), now),
            Some(Tone::Pass)
        );
        assert_eq!(watch.caption(), CAPTIONS[1]);
    }

    #[test]
    fn switching_job_forgets_the_old_result_and_a_rerun_keeps_it() {
        let now = Instant::now();
        let mut watch = Watch::new("sample".into(), "check".into());
        watch.apply(Event::Finished(failing()), now);
        watch.restart("check");
        assert_eq!(watch.phase, Phase::Failing);
        assert!(watch.running && watch.asked);
        // a rerun asked for with a key sounds even when nothing changed
        assert_eq!(
            watch.apply(Event::Finished(failing()), now),
            Some(Tone::Fail)
        );
        watch.restart("test");
        assert_eq!(watch.phase, Phase::Waiting);
        assert!(watch.report.is_none() && watch.finished_at.is_none());
        assert_eq!(watch.job, "test");
    }

    #[test]
    fn bacon_stopping_is_shown_and_r_starts_again() {
        let mut watch = Watch::new("sample".into(), "check".into());
        watch.apply(Event::Exited("error: unknown job".into()), Instant::now());
        assert_eq!(watch.phase, Phase::Stopped);
        assert_eq!(watch.message.as_deref(), Some("error: unknown job"));
        watch.restart("check");
        assert_eq!(watch.phase, Phase::Waiting);
        assert!(watch.message.is_none());
    }

    #[test]
    fn the_storm_and_the_calm_scene_build_at_every_size() {
        let mut rng = Rng::new(1);
        for (w, h) in [(1, 1), (20, 8), (80, 24)] {
            for storm in [false, true] {
                let mut scene = make_scene(storm, w, h, Motion::CALM, &mut rng);
                scene.step(0.05, &mut rng);
                let mut frame = Frame::new(w, h);
                scene.draw(&mut frame, crate::theme::default_theme());
            }
        }
    }
}
