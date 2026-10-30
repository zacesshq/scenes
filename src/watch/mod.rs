//! The watch mode: a project's check runs and the screen shows how it stands, a calm scene while
//! it passes and a serious one while it fails, with a status bar, an optional tone and an
//! optional desktop notification on each change. Every tone and notification has the same state
//! on screen, so neither sound nor colour is ever the only signal.
//!
//! The results come from one of three sources: bacon in a Rust project, the project's own
//! command in any other language or a shell being followed, where every command counts.

pub mod bacon;
pub mod follow;
pub mod runner;
pub mod transition;
pub mod view;

use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::ambient::Ambient;
use crate::app::{HELP_FADING, HELP_SHOWN, RESIZE_SETTLE, Settings};
use crate::catalogue::Catalogue;
use crate::config::Rotation;
use crate::frame::Frame;
use crate::notify::{self, Change};
use crate::report::Report;
use crate::rng::Rng;
use crate::scene::{Moment, Motion, Scene};
use crate::sound::{Sound, Tone};
use crate::terminal::{Input, Terminal};
use crate::theme::ThemeData;

use self::bacon::{Bacon, Event};
use self::follow::Follow;
use self::runner::Runner;
use self::view::{Phase, View};

/// Where the results come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    /// bacon running one of its jobs in a Rust project.
    #[default]
    Bacon,
    /// The project's own command, run again whenever its files change.
    Command,
    /// Each command that finishes in a followed shell.
    Follow,
}

/// How to start the source of results.
#[derive(Clone, Debug)]
pub enum How {
    Bacon(PathBuf),
    Command(String),
    Follow { log: PathBuf, min_seconds: f64 },
}

impl How {
    pub fn mode(&self) -> Mode {
        match self {
            How::Bacon(_) => Mode::Bacon,
            How::Command(_) => Mode::Command,
            How::Follow { .. } => Mode::Follow,
        }
    }
}

/// A running source of results.
pub enum Source {
    Bacon(Bacon),
    Command(Runner),
    Follow(Follow),
}

impl Source {
    pub fn poll(&mut self) -> Vec<Event> {
        match self {
            Source::Bacon(bacon) => bacon.poll(),
            Source::Command(runner) => runner.poll(),
            Source::Follow(follow) => follow.poll(),
        }
    }
}

/// The jobs the watch mode runs, in the order the number keys pick them.
pub const JOBS: [&str; 4] = ["check", "check-all", "clippy", "test"];

/// The scene shown while waiting and while the build passes, unless the settings name others.
pub const CALM_SCENE: &str = "waves";
/// The scene shown while the build fails, unless the settings name others.
pub const FAIL_SCENE: &str = "storm";

/// Lines shown under ALL GOOD just after a failing build is fixed.
pub const RECOVERED_CAPTIONS: [&str; 5] = [
    "Fixed. The storm has passed.",
    "Back to green. Nicely done.",
    "Recovered. Calm water again.",
    "That did it. All good.",
    "Mended. On to the next thing.",
];

/// How long the status bar says recovered and the recovered caption shows.
pub const RECOVERED_SHOWN: Duration = Duration::from_secs(8);
/// How long a note about a change of scene, sound set or volume stays in the status bar.
pub const NOTICE_SHOWN: Duration = Duration::from_secs(3);
/// How long a user scene's error stays in the status bar after the built-in scene takes over.
pub const ERROR_SHOWN: Duration = Duration::from_secs(20);
/// The shortest time between two scene changes from the keyboard. Each change replaces the
/// whole picture, so a held or hammered key could otherwise change it faster than three times a
/// second.
pub const SCENE_DEBOUNCE: Duration = Duration::from_millis(400);

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

/// The tone for a finished run. A fix plays the recovered tone, a change or a run asked for with
/// a key plays the pass or fail tone, new warnings on a passing build play the warnings tone and
/// a build that keeps failing plays the quiet still failing tone, which is off unless turned on.
pub fn tone_for(previous: Phase, now: Phase, asked: bool, new_warnings: bool) -> Option<Tone> {
    let changed = previous != now;
    match now {
        Phase::Passing if previous == Phase::Failing => Some(Tone::Recovered),
        Phase::Passing if changed || asked => Some(Tone::Pass),
        Phase::Passing if new_warnings => Some(Tone::Warnings),
        Phase::Failing if changed || asked => Some(Tone::Fail),
        Phase::Failing => Some(Tone::StillFailing),
        _ => None,
    }
}

/// The notification for a finished run: one when a build starts failing and one when it is
/// fixed. A first pass is not a change, so it stays quiet.
pub fn change_for(previous: Phase, now: Phase) -> Option<Change> {
    match (previous, now) {
        (Phase::Failing, Phase::Passing) => Some(Change::Recovered),
        (Phase::Failing, Phase::Failing) => None,
        (_, Phase::Failing) => Some(Change::Failing),
        _ => None,
    }
}

/// What a finished run or a start of one asks for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub tone: Option<Tone>,
    pub change: Option<Change>,
    /// The moment a scene may offer a caption for.
    pub moment: Option<Moment>,
}

/// The scenes for one state and which is showing.
#[derive(Clone, Debug, PartialEq)]
pub struct Lineup {
    pub names: Vec<String>,
    pub index: usize,
    /// Whether the state has been shown before, so its first showing keeps the first scene.
    shown: bool,
}

impl Lineup {
    pub fn new(names: Vec<String>, fallback: &str) -> Self {
        let names = if names.is_empty() {
            vec![fallback.to_string()]
        } else {
            names
        };
        Self {
            names,
            index: 0,
            shown: false,
        }
    }

    pub fn current(&self) -> &str {
        &self.names[self.index % self.names.len()]
    }

    /// Moves by `by` places, wrapping round.
    pub fn step(&mut self, by: isize) {
        let len = self.names.len() as isize;
        self.index = ((self.index as isize + by).rem_euclid(len)) as usize;
    }

    /// Picks the scene for a new showing of the state.
    pub fn rotate(&mut self, rotation: Rotation, rng: &mut Rng) {
        match rotation {
            Rotation::Fixed => {}
            Rotation::Cycle => self.step(1),
            Rotation::Shuffle if self.names.len() > 1 => {
                // never the same scene twice running
                let offset = 1 + rng.below(self.names.len() - 1);
                self.step(offset as isize);
            }
            Rotation::Shuffle => {}
        }
    }

    /// Called each time the state starts showing.
    fn enter(&mut self, rotation: Rotation, rng: &mut Rng) {
        if self.shown {
            self.rotate(rotation, rng);
        }
        self.shown = true;
    }

    /// Takes a scene out, after its script failed. The fallback stands in if nothing is left.
    pub fn remove(&mut self, name: &str, fallback: &str) {
        let current = self.current().to_string();
        self.names.retain(|n| n != name);
        if self.names.is_empty() {
            self.names.push(fallback.to_string());
        }
        self.index = self.names.iter().position(|n| *n == current).unwrap_or(0);
    }
}

/// The watch mode's state, apart from the terminal and bacon itself.
pub struct Watch {
    pub project: String,
    pub job: String,
    pub mode: Mode,
    pub phase: Phase,
    pub running: bool,
    pub report: Option<Report>,
    pub finished_at: Option<Instant>,
    pub message: Option<String>,
    /// How many runs have passed, which picks the caption.
    pub passes: usize,
    /// How many runs have failed, which picks the fail caption.
    pub fails: usize,
    /// Set by a key that asks for a run, so its result always sounds.
    pub asked: bool,
    pub pass: Lineup,
    pub fail: Lineup,
    pub rotation: Rotation,
    /// Time between scene changes while the state stays the same, when rotating.
    pub interval: Option<Duration>,
    picked_at: Instant,
    /// When the build was last fixed.
    pub recovered_at: Option<Instant>,
    /// Set when the next scene change is a fix, which gets its own transition.
    pub recovering: bool,
    pub captions: bool,
    pub banner: bool,
    /// A caption a user scene's hook offered for the latest result.
    pub hook_caption: Option<String>,
    rng: Rng,
}

impl Watch {
    pub fn new(project: String, job: String) -> Self {
        Self {
            project,
            job,
            mode: Mode::Bacon,
            phase: Phase::Waiting,
            running: true,
            report: None,
            finished_at: None,
            message: None,
            passes: 0,
            fails: 0,
            asked: false,
            pass: Lineup::new(vec![CALM_SCENE.to_string()], CALM_SCENE),
            fail: Lineup::new(vec![FAIL_SCENE.to_string()], FAIL_SCENE),
            rotation: Rotation::Fixed,
            interval: None,
            picked_at: Instant::now(),
            recovered_at: None,
            recovering: false,
            captions: true,
            banner: true,
            hook_caption: None,
            rng: Rng::from_time(),
        }
    }

    /// The scene lists, the rotation and the interval in minutes (0 for none).
    pub fn with_scenes(
        mut self,
        pass: Vec<String>,
        fail: Vec<String>,
        rotation: Rotation,
        interval_minutes: f64,
    ) -> Self {
        self.pass = Lineup::new(pass, CALM_SCENE);
        self.fail = Lineup::new(fail, FAIL_SCENE);
        self.rotation = rotation;
        self.interval = (interval_minutes > 0.0 && rotation != Rotation::Fixed)
            .then(|| Duration::from_secs_f64(interval_minutes * 60.0));
        self
    }

    /// Where the results come from. A followed shell has nothing running until a command
    /// finishes.
    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self.running = mode != Mode::Follow;
        self
    }

    /// Whether the latest result is a command that was not found, outside bacon.
    pub fn typo(&self) -> bool {
        self.mode == Mode::Follow
            && self.phase == Phase::Failing
            && self.report.as_ref().is_some_and(follow::is_typo)
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng = Rng::new(seed);
        self
    }

    /// Takes in what bacon reported and returns what it asks for.
    pub fn apply(&mut self, event: Event, now: Instant) -> Outcome {
        match event {
            Event::Started => {
                let first = self.phase == Phase::Waiting;
                self.running = true;
                Outcome {
                    tone: (!first).then_some(Tone::Tick),
                    ..Outcome::default()
                }
            }
            Event::Finished(report) => {
                let previous = self.phase;
                let warnings_before = self.report.as_ref().map_or(0, |r| r.warnings);
                self.phase = if report.failed() {
                    self.fails += 1;
                    Phase::Failing
                } else {
                    self.passes += 1;
                    Phase::Passing
                };
                let new_warnings = report.warnings > warnings_before;
                let typo = self.mode == Mode::Follow && follow::is_typo(&report);
                self.report = Some(report);
                self.running = false;
                self.finished_at = Some(now);
                self.hook_caption = None;
                // a mistyped command gets its own soft tone each time, never the fail phrase
                let tone = if typo {
                    Some(Tone::Typo)
                } else {
                    tone_for(previous, self.phase, self.asked, new_warnings)
                };
                self.asked = false;
                if previous != self.phase {
                    match self.phase {
                        // the waiting screen already shows the first pass scene
                        Phase::Passing if previous == Phase::Waiting => self.pass.shown = true,
                        Phase::Passing => self.pass.enter(self.rotation, &mut self.rng),
                        Phase::Failing => self.fail.enter(self.rotation, &mut self.rng),
                        _ => {}
                    }
                    self.picked_at = now;
                }
                let fixed = previous == Phase::Failing && self.phase == Phase::Passing;
                if fixed {
                    self.recovered_at = Some(now);
                    self.recovering = true;
                } else {
                    self.recovered_at = None;
                }
                let moment = match self.phase {
                    Phase::Passing if fixed => Moment::Recovered,
                    Phase::Passing => Moment::Pass,
                    _ => Moment::Fail,
                };
                Outcome {
                    tone,
                    change: change_for(previous, self.phase),
                    moment: Some(moment),
                }
            }
            Event::Exited(line) => {
                self.phase = Phase::Stopped;
                self.running = false;
                self.message = Some(line);
                Outcome::default()
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
            self.recovered_at = None;
        } else if self.phase == Phase::Stopped {
            self.phase = Phase::Waiting;
        }
        self.job = job.to_string();
        self.message = None;
        self.running = true;
        self.asked = true;
    }

    /// Whether the build was fixed within the last few seconds.
    pub fn recently_recovered(&self) -> bool {
        self.phase == Phase::Passing
            && self
                .recovered_at
                .is_some_and(|at| at.elapsed() < RECOVERED_SHOWN)
    }

    fn lineup(&self) -> &Lineup {
        if self.wants_failing() {
            &self.fail
        } else {
            &self.pass
        }
    }

    fn lineup_mut(&mut self) -> &mut Lineup {
        if self.wants_failing() {
            &mut self.fail
        } else {
            &mut self.pass
        }
    }

    /// The scene the state calls for.
    pub fn wanted_scene(&self) -> &str {
        self.lineup().current()
    }

    /// Moves to the next or the previous scene by key. A list of one widens to every scene of
    /// its kind, so the key always has somewhere to go. Returns the new scene.
    pub fn change_scene(&mut self, by: isize, catalogue: &Catalogue) -> String {
        let failing = self.wants_failing();
        let lineup = self.lineup_mut();
        if lineup.names.len() == 1 {
            let current = lineup.names[0].clone();
            let all = catalogue.names_for(!failing);
            if all.len() > 1 {
                lineup.index = all.iter().position(|n| *n == current).unwrap_or(0);
                lineup.names = all;
            }
        }
        lineup.step(by);
        self.picked_at = Instant::now();
        self.hook_caption = None;
        self.recovering = false;
        self.wanted_scene().to_string()
    }

    /// Moves the scene on once the interval has passed, when rotating.
    pub fn tick(&mut self, now: Instant) {
        let Some(interval) = self.interval else {
            return;
        };
        if now.duration_since(self.picked_at) >= interval {
            self.picked_at = now;
            let rotation = self.rotation;
            let mut rng = self.rng.clone();
            self.lineup_mut().rotate(rotation, &mut rng);
            self.rng = rng;
        }
    }

    /// Drops a scene whose script failed from both lists.
    pub fn drop_scene(&mut self, name: &str) {
        self.pass.remove(name, CALM_SCENE);
        self.fail.remove(name, FAIL_SCENE);
    }

    /// The line under the banner: a hook's caption, the recovered caption just after a fix or
    /// the scene's own caption, changing with each run. `None` with captions off.
    pub fn caption(&self, catalogue: &Catalogue) -> Option<String> {
        if !self.captions {
            return None;
        }
        if let Some(hook) = &self.hook_caption {
            return Some(hook.clone());
        }
        match self.phase {
            Phase::Passing if self.recently_recovered() => Some(
                RECOVERED_CAPTIONS[self.passes.saturating_sub(1) % RECOVERED_CAPTIONS.len()]
                    .to_string(),
            ),
            Phase::Passing | Phase::Failing => {
                let captions = catalogue.find(self.wanted_scene())?.captions.clone();
                let count = if self.phase == Phase::Passing {
                    self.passes
                } else {
                    self.fails
                };
                (!captions.is_empty())
                    .then(|| captions[count.saturating_sub(1) % captions.len()].clone())
            }
            _ => None,
        }
    }

    /// Whether the phase calls for a fail scene.
    pub fn wants_failing(&self) -> bool {
        self.phase == Phase::Failing
    }
}

/// Everything the run loop needs that is not the terminal.
pub struct Context<'a> {
    pub theme: &'a ThemeData,
    pub settings: Settings,
    pub motion: Motion,
    pub how: How,
    pub project_dir: PathBuf,
    pub report_path: PathBuf,
    pub sound: Sound,
    pub ambient: Ambient,
    pub catalogue: Catalogue,
    pub notify: bool,
}

fn start_source(context: &Context, job: &str) -> io::Result<Source> {
    Ok(match &context.how {
        How::Bacon(program) => Source::Bacon(Bacon::start(
            program,
            &context.project_dir,
            job,
            &context.report_path,
        )?),
        How::Command(command) => Source::Command(Runner::start(&context.project_dir, command)?),
        How::Follow { log, min_seconds } => Source::Follow(Follow::start(log, *min_seconds)?),
    })
}

/// How long a tone takes, so the ambient sound can make room for it.
const TONE_ROOM: Duration = Duration::from_millis(1600);

/// Plays an event's tone, with the ambient sound stepping aside while it does.
fn sound_tone(context: &mut Context, tone: Tone) {
    if context.sound.would_play(tone) {
        context.ambient.duck(TONE_ROOM);
    }
    context.sound.play(tone);
}

/// A scene by name (or the built-in scene of its kind when a user scene fails to start) with the
/// error to show.
fn build(
    catalogue: &Catalogue,
    name: &str,
    failing: bool,
    size: (usize, usize),
    motion: Motion,
    rng: &mut Rng,
) -> (Box<dyn Scene>, String, Option<String>) {
    match catalogue.create(name, size.0, size.1, motion, rng) {
        Ok(scene) => (scene, name.to_string(), None),
        Err(error) => {
            let fallback = if failing { FAIL_SCENE } else { CALM_SCENE };
            let scene = crate::scenes::create(fallback, size.0, size.1, motion, rng)
                .expect("the fallback scenes are built in");
            (scene, fallback.to_string(), Some(error))
        }
    }
}

/// Runs the watch mode until a quit key or a termination signal.
pub fn run(
    context: &mut Context,
    watch: &mut Watch,
    rng: &mut Rng,
    terminal: &mut Terminal,
) -> io::Result<()> {
    let motion = context.motion;
    let mode = context.how.mode();
    let mut source = Some(start_source(context, &watch.job)?);
    let (mut width, mut height) = terminal.size()?;
    let (mut scene, mut current, mut scene_error) = build(
        &context.catalogue,
        watch.wanted_scene(),
        watch.wants_failing(),
        (width, height),
        motion,
        rng,
    );
    let mut error_at = Instant::now();
    if let Some(error) = &scene_error {
        watch.drop_scene(&error_name(error, watch.wanted_scene()));
    }
    let mut outgoing: Option<(Box<dyn Scene>, f32)> = None;
    let mut frame = Frame::new(width, height);
    let mut behind = Frame::new(width, height);
    let frame_time = Duration::from_secs_f32(context.settings.frame_time());
    let mut last = Instant::now();
    let mut last_report_poll = Instant::now();
    let mut last_scene_key: Option<Instant> = None;
    let mut paused = false;
    let mut hint_age: Option<f32> = Some(0.0);
    let mut pending_resize: Option<(usize, usize, Instant)> = None;
    let mut pending_moment: Option<Moment> = None;
    let mut notice: Option<(String, Instant)> = None;

    loop {
        if pending_resize.is_none() {
            if let Some(problem) = scene.problem() {
                // a user scene that fails or flashes is swapped for the built-in one of its kind
                context.catalogue.retire(&current);
                watch.drop_scene(&current);
                scene_error = Some(problem);
                error_at = Instant::now();
            }
            if watch.wanted_scene() != current || scene.problem().is_some() {
                let (next, name, error) = build(
                    &context.catalogue,
                    watch.wanted_scene(),
                    watch.wants_failing(),
                    (width, height),
                    motion,
                    rng,
                );
                if let Some(error) = error {
                    watch.drop_scene(&error_name(&error, watch.wanted_scene()));
                    scene_error = Some(error);
                    error_at = Instant::now();
                }
                let old = std::mem::replace(&mut scene, next);
                // a fix is the one change that gets a transition: the new scene opens out from
                // the middle; every other change happens in one step
                if std::mem::take(&mut watch.recovering) {
                    outgoing = Some((old, 0.0));
                } else {
                    outgoing = None;
                }
                current = name;
            }
            if let Some(moment) = pending_moment.take() {
                if let Some(caption) = scene.caption_for(moment) {
                    watch.hook_caption = Some(caption);
                }
            }
            let mut notes: Vec<String> = Vec::new();
            if scene_error.is_some() && error_at.elapsed() < ERROR_SHOWN {
                notes.push("scene error".to_string());
            }
            if let Some((text, _)) = notice
                .as_ref()
                .filter(|(_, at)| at.elapsed() < NOTICE_SHOWN)
            {
                notes.push(text.clone());
            }
            if paused {
                notes.push("paused".to_string());
            }
            context.ambient.set_scene(&current);
            context.ambient.pause(paused);
            context.ambient.tick();
            if !context.sound.enabled() {
                notes.push("sound off".to_string());
            } else if !context.sound.available() {
                notes.push("no sound player".to_string());
            } else if context.sound.quiet_now() {
                notes.push("quiet hours".to_string());
            }
            let caption = watch.caption(&context.catalogue);
            let alert = scene_error
                .as_ref()
                .filter(|_| error_at.elapsed() < ERROR_SHOWN)
                .map(|error| format!("Showing {current}, a scene stopped: {error}"));
            let view = View {
                project: &watch.project,
                job: &watch.job,
                mode,
                typo: watch.typo(),
                phase: watch.phase,
                running: watch.running,
                report: watch.report.as_ref(),
                ago: watch.finished_at.map(|at| at.elapsed().as_secs()),
                caption: caption.as_deref(),
                banner: watch.banner,
                recovered: watch.recently_recovered(),
                alert: alert.as_deref(),
                message: watch.message.as_deref(),
                notes: notes.iter().map(String::as_str).collect(),
                hint_age,
            };
            frame.clear();
            scene.draw(&mut frame, context.theme);
            if let Some((old, age)) = &outgoing {
                behind.clear();
                old.draw(&mut behind, context.theme);
                let progress = age / transition::length(motion);
                transition::compose(&mut frame, &behind, progress, context.theme, motion);
            }
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
                Input::Key('a') => {
                    context.ambient.toggle();
                    let state = if context.ambient.enabled() {
                        "on"
                    } else {
                        "off"
                    };
                    notice = Some((format!("ambient {state}"), Instant::now()));
                }
                Input::Key(key @ ('[' | ']')) => {
                    let volume = context.ambient.change_volume(key == ']');
                    notice = Some((format!("ambient volume {volume}"), Instant::now()));
                }
                Input::Key(key @ ('n' | 'N' | 'b')) => {
                    let now = Instant::now();
                    if last_scene_key.is_none_or(|at| now.duration_since(at) >= SCENE_DEBOUNCE) {
                        let by = if key == 'n' { 1 } else { -1 };
                        let name = watch.change_scene(by, &context.catalogue);
                        notice = Some((
                            format!("scene {name} \u{b7} sounds {}", context.sound.set().name()),
                            now,
                        ));
                    }
                    last_scene_key = Some(now);
                }
                Input::Key('m') => {
                    let set = context.sound.next_set();
                    notice = Some((
                        format!("sounds {} \u{b7} scene {current}", set.name()),
                        Instant::now(),
                    ));
                    sound_tone(context, Tone::Pass);
                }
                Input::Key(key @ ('+' | '-')) => {
                    let volume = context.sound.change_volume(key == '+');
                    notice = Some((format!("volume {volume}"), Instant::now()));
                    sound_tone(context, Tone::Tick);
                }
                // a followed shell runs nothing itself, so there is nothing to rerun
                Input::Key('r') if mode != Mode::Follow => {
                    let job = watch.job.clone();
                    match source.as_mut() {
                        Some(Source::Command(runner)) => {
                            watch.restart(&job);
                            runner.rerun();
                        }
                        _ => {
                            // the old source is stopped before the new one starts, so two
                            // bacons never share the report file
                            drop(source.take());
                            watch.restart(&job);
                            source = Some(start_source(context, &job)?);
                        }
                    }
                }
                Input::Key(digit @ '1'..='4') if mode == Mode::Bacon => {
                    let job = JOBS[digit as usize - '1' as usize];
                    drop(source.take());
                    watch.restart(job);
                    source = Some(start_source(context, job)?);
                }
                Input::Resize(w, h) => pending_resize = Some((w, h, Instant::now())),
                Input::Key(_) | Input::None => {}
            }
            let settled = pending_resize.filter(|&(_, _, at)| at.elapsed() >= RESIZE_SETTLE);
            if let Some((w, h, _)) = settled {
                pending_resize = None;
                (width, height) = (w, h);
                scene.resize(w, h, rng);
                outgoing = None;
                frame = Frame::new(w, h);
                behind = Frame::new(w, h);
                terminal.reset()?;
                break;
            }
        }

        if last_report_poll.elapsed() >= REPORT_POLL {
            last_report_poll = Instant::now();
            let events = source.as_mut().map(Source::poll).unwrap_or_default();
            for event in events {
                let exited = matches!(event, Event::Exited(_));
                let outcome = watch.apply(event, Instant::now());
                if let Some(tone) = outcome.tone {
                    sound_tone(context, tone);
                }
                if let Some(change) = outcome.change.filter(|_| context.notify) {
                    let problems = watch.report.as_ref().map_or(0, |r| r.errors + r.test_fails);
                    notify::send(
                        notify::TITLE,
                        &notify::message(change, &watch.project, &watch.job, problems),
                    );
                }
                if outcome.moment.is_some() {
                    pending_moment = outcome.moment;
                }
                if exited {
                    source = None;
                    break;
                }
            }
        }

        let now = Instant::now();
        let elapsed = (now - last)
            .as_secs_f32()
            .min(context.settings.frame_time() * 2.0);
        last = now;
        watch.tick(now);
        if !paused {
            scene.step(elapsed * context.settings.speed, rng);
            if let Some((old, age)) = outgoing.as_mut() {
                old.step(elapsed * context.settings.speed, rng);
                *age += elapsed;
                if *age >= transition::length(motion) {
                    outgoing = None;
                }
            }
        }
        if let Some(age) = hint_age.as_mut() {
            *age += elapsed;
            if *age >= HELP_SHOWN + HELP_FADING {
                hint_age = None;
            }
        }
    }
}

/// The scene a start error is about: the name before the first colon, which is how user scene
/// errors begin, otherwise the scene that was asked for.
fn error_name(error: &str, wanted: &str) -> String {
    error
        .split_once(':')
        .map_or(wanted, |(name, _)| name.trim())
        .to_string()
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

    fn warned() -> Report {
        Report {
            warnings: 1,
            ..Report::default()
        }
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
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
    fn tones_follow_changes_fixes_warnings_and_requests() {
        use Phase::*;
        assert_eq!(tone_for(Waiting, Passing, false, false), Some(Tone::Pass));
        assert_eq!(tone_for(Passing, Failing, false, false), Some(Tone::Fail));
        assert_eq!(tone_for(Waiting, Failing, false, false), Some(Tone::Fail));
        assert_eq!(
            tone_for(Failing, Passing, false, false),
            Some(Tone::Recovered)
        );
        assert_eq!(
            tone_for(Failing, Failing, false, false),
            Some(Tone::StillFailing)
        );
        assert_eq!(tone_for(Failing, Failing, true, false), Some(Tone::Fail));
        assert_eq!(tone_for(Passing, Passing, false, false), None);
        assert_eq!(
            tone_for(Passing, Passing, false, true),
            Some(Tone::Warnings)
        );
        assert_eq!(tone_for(Passing, Passing, true, false), Some(Tone::Pass));
        assert_eq!(tone_for(Passing, Stopped, true, false), None);
    }

    #[test]
    fn notifications_mark_a_failure_and_a_fix_only() {
        use Phase::*;
        assert_eq!(change_for(Waiting, Passing), None);
        assert_eq!(change_for(Passing, Passing), None);
        assert_eq!(change_for(Waiting, Failing), Some(Change::Failing));
        assert_eq!(change_for(Passing, Failing), Some(Change::Failing));
        assert_eq!(change_for(Failing, Failing), None);
        assert_eq!(change_for(Failing, Passing), Some(Change::Recovered));
    }

    #[test]
    fn the_state_moves_from_waiting_to_passing_to_failing_and_back() {
        let now = Instant::now();
        let catalogue = Catalogue::built_in();
        let mut watch = Watch::new("sample".into(), "check".into());
        assert_eq!(watch.phase, Phase::Waiting);
        assert_eq!(watch.wanted_scene(), CALM_SCENE);
        // the first start is not a rebuild, so it has no tick
        assert_eq!(watch.apply(Event::Started, now).tone, None);
        let first = watch.apply(Event::Finished(Report::default()), now);
        assert_eq!(first.tone, Some(Tone::Pass));
        assert_eq!(first.change, None);
        assert_eq!(first.moment, Some(Moment::Pass));
        assert_eq!(watch.phase, Phase::Passing);
        assert_eq!(
            watch.caption(&catalogue).as_deref(),
            Some(crate::scenes::info("waves").unwrap().captions[0])
        );
        assert_eq!(watch.apply(Event::Started, now).tone, Some(Tone::Tick));
        let broke = watch.apply(Event::Finished(failing()), now);
        assert_eq!(broke.tone, Some(Tone::Fail));
        assert_eq!(broke.change, Some(Change::Failing));
        assert!(watch.wants_failing());
        assert_eq!(watch.wanted_scene(), FAIL_SCENE);
        assert!(watch.caption(&catalogue).is_some());
        assert_eq!(
            watch.apply(Event::Finished(failing()), now).tone,
            Some(Tone::StillFailing)
        );
        let fixed = watch.apply(Event::Finished(Report::default()), now);
        assert_eq!(fixed.tone, Some(Tone::Recovered));
        assert_eq!(fixed.change, Some(Change::Recovered));
        assert_eq!(fixed.moment, Some(Moment::Recovered));
        assert!(watch.recovering && watch.recently_recovered());
        assert!(RECOVERED_CAPTIONS.contains(&watch.caption(&catalogue).unwrap().as_str()));
        assert_eq!(
            watch.apply(Event::Finished(warned()), now).tone,
            Some(Tone::Warnings)
        );
        assert!(!watch.recently_recovered());
        watch.captions = false;
        assert_eq!(watch.caption(&catalogue), None);
    }

    #[test]
    fn rotation_picks_the_scene_on_each_new_state() {
        let now = Instant::now();
        let mut cycle = Watch::new("s".into(), "check".into()).with_scenes(
            names(&["tea", "orbit", "garden"]),
            names(&["flood", "fog"]),
            Rotation::Cycle,
            0.0,
        );
        assert_eq!(cycle.wanted_scene(), "tea");
        cycle.apply(Event::Finished(Report::default()), now);
        assert_eq!(cycle.wanted_scene(), "tea");
        cycle.apply(Event::Finished(failing()), now);
        assert_eq!(cycle.wanted_scene(), "flood");
        cycle.apply(Event::Finished(Report::default()), now);
        assert_eq!(cycle.wanted_scene(), "orbit");
        cycle.apply(Event::Finished(failing()), now);
        assert_eq!(cycle.wanted_scene(), "fog");
        cycle.apply(Event::Finished(Report::default()), now);
        assert_eq!(cycle.wanted_scene(), "garden");

        let mut fixed = Watch::new("s".into(), "check".into()).with_scenes(
            names(&["tea", "orbit"]),
            names(&["flood", "fog"]),
            Rotation::Fixed,
            5.0,
        );
        assert_eq!(fixed.interval, None);
        for report in [Report::default(), failing(), Report::default()] {
            fixed.apply(Event::Finished(report), now);
        }
        assert_eq!(fixed.wanted_scene(), "tea");

        let mut shuffle = Watch::new("s".into(), "check".into())
            .with_scenes(
                names(&["tea", "orbit", "garden", "tiles"]),
                vec![],
                Rotation::Shuffle,
                0.0,
            )
            .with_seed(4);
        shuffle.apply(Event::Finished(failing()), now);
        shuffle.apply(Event::Finished(Report::default()), now);
        let mut last = shuffle.wanted_scene().to_string();
        for _ in 0..20 {
            shuffle.apply(Event::Finished(failing()), now);
            shuffle.apply(Event::Finished(Report::default()), now);
            assert_ne!(shuffle.wanted_scene(), last, "shuffle repeated a scene");
            last = shuffle.wanted_scene().to_string();
        }
        assert_eq!(shuffle.fail.names, names(&[FAIL_SCENE]));
    }

    #[test]
    fn the_interval_moves_the_scene_while_the_state_stays() {
        let start = Instant::now();
        let mut watch = Watch::new("s".into(), "check".into()).with_scenes(
            names(&["tea", "orbit"]),
            vec![],
            Rotation::Cycle,
            0.5,
        );
        watch.apply(Event::Finished(Report::default()), start);
        watch.tick(start + Duration::from_secs(10));
        assert_eq!(watch.wanted_scene(), "tea");
        watch.tick(start + Duration::from_secs(31));
        assert_eq!(watch.wanted_scene(), "orbit");
    }

    #[test]
    fn keys_step_through_the_list_or_every_scene_of_the_kind() {
        let catalogue = Catalogue::built_in();
        let mut watch = Watch::new("s".into(), "check".into());
        assert_eq!(watch.change_scene(1, &catalogue), "zacess");
        assert_eq!(watch.change_scene(-1, &catalogue), "waves");
        assert!(watch.pass.names.len() > 25);
        assert!(!watch.pass.names.contains(&"storm".to_string()));
        watch.apply(Event::Finished(failing()), Instant::now());
        let next = watch.change_scene(1, &catalogue);
        assert_eq!(
            catalogue.find(&next).unwrap().kind,
            crate::scenes::Kind::Fail
        );
        let mut listed = Watch::new("s".into(), "check".into()).with_scenes(
            names(&["tea", "orbit"]),
            vec![],
            Rotation::Fixed,
            0.0,
        );
        assert_eq!(listed.change_scene(1, &catalogue), "orbit");
        assert_eq!(listed.change_scene(1, &catalogue), "tea");
        assert_eq!(listed.change_scene(-1, &catalogue), "orbit");
    }

    #[test]
    fn a_failed_scene_leaves_both_lists() {
        let mut watch = Watch::new("s".into(), "check".into()).with_scenes(
            names(&["mine", "tea"]),
            names(&["mine"]),
            Rotation::Fixed,
            0.0,
        );
        watch.drop_scene("mine");
        assert_eq!(watch.pass.names, names(&["tea"]));
        assert_eq!(watch.fail.names, names(&[FAIL_SCENE]));
        assert_eq!(error_name("mine: lantern.lua:3: boom", "x"), "mine");
        assert_eq!(error_name("no colon", "x"), "x");
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
            watch.apply(Event::Finished(failing()), now).tone,
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
    fn only_a_followed_shell_calls_a_missing_command_a_typo() {
        let missing = Report {
            errors: 1,
            exit_code: Some(follow::NOT_FOUND),
            ..Report::default()
        };
        let mut followed =
            Watch::new("following".into(), "your shell".into()).with_mode(Mode::Follow);
        assert!(
            !followed.running,
            "a followed shell runs nothing until a command finishes"
        );
        let outcome = followed.apply(Event::Finished(missing.clone()), Instant::now());
        assert_eq!(outcome.tone, Some(Tone::Typo));
        assert!(followed.typo());
        let fixed = followed.apply(Event::Finished(Report::default()), Instant::now());
        assert_eq!(fixed.tone, Some(Tone::Recovered));

        // a project command whose tool is missing exits 127 too, but that is a failure to read
        let mut watched = Watch::new("p".into(), "pnpm test".into()).with_mode(Mode::Command);
        let outcome = watched.apply(Event::Finished(missing), Instant::now());
        assert_eq!(outcome.tone, Some(Tone::Fail));
        assert!(!watched.typo());
    }

    #[test]
    fn a_broken_user_scene_falls_back_to_the_built_in_one() {
        let catalogue = Catalogue::built_in();
        let mut rng = Rng::new(1);
        let (scene, name, error) =
            build(&catalogue, "nope", true, (40, 12), Motion::FULL, &mut rng);
        assert_eq!(name, FAIL_SCENE);
        assert_eq!(scene.name(), FAIL_SCENE);
        assert!(error.is_some());
        for (w, h) in [(1, 1), (20, 8), (80, 24)] {
            for failing in [false, true] {
                let wanted = if failing { FAIL_SCENE } else { CALM_SCENE };
                let (mut scene, _, error) =
                    build(&catalogue, wanted, failing, (w, h), Motion::CALM, &mut rng);
                assert!(error.is_none());
                scene.step(0.05, &mut rng);
                scene.draw(&mut Frame::new(w, h), crate::theme::default_theme());
            }
        }
    }
}
