//! The command line: a scene name and a handful of flags or `watch` and its job. It is parsed by
//! hand so the binary stays small.

use std::path::PathBuf;

use crate::colour::ColourDepth;
use crate::config::{self, Flags};
use crate::scene::Level;
use crate::scenes::SCENES;
use crate::sound::{self, SoundSet};
use crate::watch::JOBS;

pub const DEFAULT_SCENE: &str = "sea";
pub const DEFAULT_FPS: u32 = 20;
/// The ceiling for `--fps`. Nothing in a scene needs more and a lower rate is calmer.
pub const MAX_FPS: u32 = 30;
pub const DEFAULT_FRAMES: u32 = 60;
/// The largest headless size, so a typo cannot ask for a frame of gigabytes.
pub const MAX_SIZE: usize = 1000;

/// What the command line asked for.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Run(Options),
    Help,
    Version,
    ListScenes,
    ListThemes,
    Watch(WatchOptions),
    WatchHelp,
    /// `list`: every scene and sound set.
    List,
    /// `config init`, refusing to replace a file unless forced.
    ConfigInit {
        force: bool,
    },
    /// `config path`.
    ConfigPath,
    /// `sounds play <set> [event]`.
    SoundsPlay {
        set: SoundSet,
        event: Option<sound::Tone>,
    },
    /// `scene new <name>`, refusing to replace a file unless forced.
    SceneNew {
        name: String,
        force: bool,
    },
    /// `snapshot <scene>`.
    Snapshot(SnapshotOptions),
    SnapshotHelp,
}

/// What a watch mode screen shows in a snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotState {
    Pass,
    Fail,
}

/// The settings for `snapshot`.
#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotOptions {
    pub scene: String,
    pub theme: Option<String>,
    pub size: (usize, usize),
    /// Seconds of the scene to run before the still is taken.
    pub time: f32,
    /// With a number of frames, the output is the frames as JSON instead of an SVG still.
    pub frames: Option<u32>,
    pub fps: u32,
    pub out: Option<PathBuf>,
    pub state: Option<SnapshotState>,
    pub seed: u64,
    pub motion: Option<Level>,
    pub reduced_motion: bool,
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        Self {
            scene: DEFAULT_SCENE.to_string(),
            theme: None,
            size: (80, 24),
            time: 6.0,
            frames: None,
            fps: 12,
            out: None,
            state: None,
            seed: 1,
            motion: None,
            reduced_motion: false,
        }
    }
}

/// The settings for the watch mode.
#[derive(Clone, Debug, PartialEq)]
pub struct WatchOptions {
    /// The bacon job: `check`, `check-all`, `clippy` or `test`.
    pub job: String,
    pub fps: u32,
    pub colour_depth: Option<ColourDepth>,
    /// The theme, scenes, sounds, motion and notification flags, combined later with the
    /// variables and the config file.
    pub flags: Flags,
}

impl Default for WatchOptions {
    fn default() -> Self {
        Self {
            job: JOBS[0].to_string(),
            fps: DEFAULT_FPS,
            colour_depth: None,
            flags: Flags::default(),
        }
    }
}

/// The settings for a run.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// The scene named on the command line, `None` for the default or the config's first pass
    /// scene.
    pub scene: Option<String>,
    pub theme: Option<String>,
    pub motion: Option<Level>,
    pub fps: u32,
    pub slow: bool,
    pub reduced_motion: bool,
    /// Minutes to run before stopping on its own.
    pub duration_minutes: Option<f64>,
    pub seed: Option<u64>,
    pub headless: bool,
    pub frames: u32,
    /// Columns and rows for a headless render.
    pub size: Option<(usize, usize)>,
    /// The colour depth asked for with `--colour`, otherwise detected from the terminal.
    pub colour_depth: Option<ColourDepth>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            scene: None,
            theme: None,
            motion: None,
            fps: DEFAULT_FPS,
            slow: false,
            reduced_motion: false,
            duration_minutes: None,
            seed: None,
            headless: false,
            frames: DEFAULT_FRAMES,
            size: None,
            colour_depth: None,
        }
    }
}

/// The scene names wrapped to fit the help, indented.
fn wrapped_names(names: &[&str]) -> String {
    let mut out = String::new();
    let mut line = String::from("  ");
    for name in names {
        if line.len() + name.len() + 1 > 78 && !line.trim().is_empty() {
            out.push_str(line.trim_end());
            out.push('\n');
            line = String::from("  ");
        }
        line.push_str(name);
        line.push(' ');
    }
    out.push_str(line.trim_end());
    out.push('\n');
    out
}

/// The help text.
pub fn usage() -> String {
    let names = |kind: crate::scenes::Kind| -> Vec<&str> {
        SCENES
            .iter()
            .filter(|s| s.kind == kind)
            .map(|s| s.name)
            .collect()
    };
    let mut text = String::from(
        "Usage: zacess-scenes [scene] [options]\n       zacess-scenes <command>\n\n\
Calm animated scenes for the terminal in the Zacess themes.\n\nThe original scenes:\n",
    );
    text.push_str(&wrapped_names(&names(crate::scenes::Kind::General)));
    text.push_str("Pass scenes, also shown while a build passes:\n");
    text.push_str(&wrapped_names(&names(crate::scenes::Kind::Pass)));
    text.push_str("Fail scenes, shown while a build fails:\n");
    text.push_str(&wrapped_names(&names(crate::scenes::Kind::Fail)));
    text.push_str(&format!(
        "\nYour own Lua scenes in the scenes folder beside the config file work too. The default\n\
scene is {DEFAULT_SCENE}.\n\nCommands:\n\
  watch [job]               Show the state of a Rust build as a scene (watch --help)\n\
  list                      Every scene (pass, fail or general) and the sound sets\n\
  config init [--force]     Write a fully commented config file\n\
  config path               Print where the config file is\n\
  sounds play <set> [event] Play a sound set's tone (event: pass, fail, tick, recovered,\n\
                            still_failing or warnings; pass by default)\n\
  scene new <name>          Write a commented starter Lua scene into the scenes folder\n\
  snapshot <scene>          Draw a scene into an SVG still or JSON frames (snapshot --help)\n\
\nOptions:\n\
  --theme <name>       A Zacess theme by name, id or short name\n\
  --scene <name>       The scene, as an alternative to naming it first\n\
  --motion <level>     calm, standard or lively (lively never flashes either)\n\
  --list-themes        Print the themes and which one is active\n\
  --list-scenes        Print the scenes\n\
  --fps <n>            Frames per second, 1 to {MAX_FPS} (default {DEFAULT_FPS})\n\
  --slow               Half speed\n\
  --reduced-motion     Fewer and slower movements, no lightning and no flicker\n\
  --duration <minutes> Stop after this many minutes\n\
  --colour <depth>     truecolor or 256, when the terminal's colour depth is detected wrongly\n\
  --seed <n>           Seed the random numbers, so a run can be repeated\n\
  --headless           Draw without a terminal and print the last frame\n\
  --frames <n>         Frames to draw headless (default {DEFAULT_FRAMES})\n\
  --size <WxH>         Columns and rows for a headless render\n\
  -h, --help           This help\n\
  -V, --version        The version\n\
\nKeys while a scene runs: Space pauses and resumes, n shows the next scene and b the one\n\
before, h shows the help line again and q, Esc or Ctrl+C quits.\n\nThe theme is taken from\n\
--theme, then ZACESS_THEME, then the config file, then the theme state file\n\
($XDG_STATE_HOME/zacess/theme), otherwise Zacess Deep Contrast Dark. ZACESS_REDUCED_MOTION=1\n\
or the system's reduce motion setting also turns reduced motion on. ZACESS_MOTION sets the\n\
level. NO_COLOR draws in the terminal's own text colour.\n"
    ));
    text
}

fn value(
    flag: &str,
    inline: Option<&str>,
    rest: &mut impl Iterator<Item = String>,
) -> Result<String, String> {
    match inline {
        Some(v) => Ok(v.to_string()),
        None => rest.next().ok_or_else(|| format!("{flag} needs a value")),
    }
}

fn parse_size(text: &str) -> Result<(usize, usize), String> {
    let error = || format!("--size wants columns x rows, such as 80x24, not \"{text}\"");
    let (w, h) = text
        .to_ascii_lowercase()
        .split_once('x')
        .map(|(w, h)| (w.trim().to_string(), h.trim().to_string()))
        .ok_or_else(error)?;
    let width: usize = w.parse().map_err(|_| error())?;
    let height: usize = h.parse().map_err(|_| error())?;
    if width == 0 || height == 0 || width > MAX_SIZE || height > MAX_SIZE {
        return Err(format!(
            "--size must be between 1x1 and {MAX_SIZE}x{MAX_SIZE}, not {width}x{height}"
        ));
    }
    Ok((width, height))
}

/// The help text for the watch mode.
pub fn watch_usage() -> String {
    format!(
        "Usage: zacess-scenes watch [job] [options]\n\n\
Runs bacon in the current Cargo project and shows the state of the build as a scene: a calm\n\
scene and ALL GOOD while it passes, a serious one and BUILD FAILED with the first errors while\n\
it fails. A short opening marks a failing build being fixed.\n\n\
Jobs: {jobs} (default {default}).\n\n\
Options:\n\
  --scene <names>      Pass scenes, separated by commas (default waves, then every pass scene)\n\
  --fail-scene <names> Fail scenes, separated by commas (default storm, then every fail scene)\n\
  --sounds <set>       The sound set: {sets} (default bells)\n\
  --motion <level>     calm, standard or lively\n\
  --notify             A desktop notification when the build fails or is fixed\n\
  --no-notify          No notifications, whatever the config says\n\
  --theme <name>       A Zacess theme by name, id or short name\n\
  --fps <n>            Frames per second, 1 to {MAX_FPS} (default {DEFAULT_FPS})\n\
  --reduced-motion     Fewer and slower movements and no lightning\n\
  --colour <depth>     truecolor or 256, when the terminal's colour depth is detected wrongly\n\
  --no-sound           No tones (the screen shows every state either way)\n\
  --volume <0-100>     The volume of the tones (default {volume})\n\
  -h, --help           This help\n\
\nKeys: q quits, 1 to 4 switch to {jobs}, r reruns the job, n shows the next\n\
scene and N or b the one before, m changes the sound set, + and - change the volume, s turns\n\
sound on or off, p or Space pauses the animation and h shows the keys again.\n\
\nEvery setting can also go in the config file (zacess-scenes config path shows where). A flag\n\
wins over an environment variable, which wins over the file, which wins over the default. The\n\
variables are ZACESS_SCENE, ZACESS_FAIL_SCENE, ZACESS_SOUNDS, ZACESS_MOTION, ZACESS_NOTIFY,\n\
ZACESS_SOUND=0 to turn sound off and ZACESS_VOLUME. Needs bacon, installed with\n\
cargo install --locked bacon\n",
        jobs = JOBS.join(", "),
        default = JOBS[0],
        volume = sound::DEFAULT_VOLUME,
        sets = SoundSet::ALL.map(|s| s.name()).join(", "),
    )
}

/// The help text for `snapshot`.
pub fn snapshot_usage() -> String {
    "Usage: zacess-scenes snapshot <scene> [options]\n\n\
Draws a scene with the real renderer and no terminal into a self-contained SVG still or, with\n\
--frames into JSON frames for a web page to play.\n\n\
Options:\n\
  --theme <name>       A Zacess theme (default as for the scenes)\n\
  --size <WxH>         Columns and rows (default 80x24)\n\
  --time <seconds>     How far into the scene the still is taken (default 6)\n\
  --frames <n>         Write n frames as JSON instead of a still\n\
  --fps <n>            Frames a second for --frames, 1 to 30 (default 12)\n\
  --state <pass|fail>  Draw the watch mode's banner, caption and status bar with sample data\n\
  --seed <n>           Seed the random numbers (default 1)\n\
  --motion <level>     calm, standard or lively\n\
  --reduced-motion     Reduced motion\n\
  --out <path>         Write to a file instead of standard output\n"
        .to_string()
}

fn parse_level(flag: &str, text: &str) -> Result<Level, String> {
    Level::parse(text)
        .ok_or_else(|| format!("{flag} wants calm, standard or lively, not \"{text}\""))
}

fn parse_set(flag: &str, text: &str) -> Result<SoundSet, String> {
    SoundSet::parse(text).ok_or_else(|| {
        format!(
            "{flag} wants a sound set ({}), not \"{text}\"",
            SoundSet::ALL.map(|s| s.name()).join(", ")
        )
    })
}

fn parse_names(flag: &str, text: &str) -> Result<Vec<String>, String> {
    let names = config::split_names(text);
    if names.is_empty() {
        return Err(format!("{flag} needs at least one scene name"));
    }
    Ok(names)
}

/// Splits `--flag=value` into the flag and its value.
fn split_flag(arg: &str) -> (String, Option<&str>) {
    match arg.split_once('=') {
        Some((flag, inline)) if flag.starts_with("--") => (flag.to_string(), Some(inline)),
        _ => (arg.to_string(), None),
    }
}

fn parse_fps(text: &str) -> Result<u32, String> {
    let fps: u32 = text
        .parse()
        .map_err(|_| format!("--fps wants a whole number, not \"{text}\""))?;
    if !(1..=MAX_FPS).contains(&fps) {
        return Err(format!("--fps must be between 1 and {MAX_FPS}, not {fps}"));
    }
    Ok(fps)
}

fn parse_colour(flag: &str, text: &str) -> Result<ColourDepth, String> {
    ColourDepth::parse(text).ok_or_else(|| format!("{flag} wants truecolor or 256, not \"{text}\""))
}

/// Parses the arguments after `watch`.
fn parse_watch(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut options = WatchOptions::default();
    let mut job: Option<String> = None;
    let mut args = args;
    while let Some(arg) = args.next() {
        let (flag, inline) = split_flag(&arg);
        let flags = &mut options.flags;
        match flag.as_str() {
            "-h" | "--help" => return Ok(Command::WatchHelp),
            "--reduced-motion" => flags.reduced_motion = true,
            "--no-sound" => flags.no_sound = true,
            "--notify" => flags.notify = Some(true),
            "--no-notify" => flags.notify = Some(false),
            "--theme" => flags.theme = Some(value(&flag, inline, &mut args)?),
            "--scene" => flags.scene = Some(parse_names(&flag, &value(&flag, inline, &mut args)?)?),
            "--fail-scene" => {
                flags.fail_scene = Some(parse_names(&flag, &value(&flag, inline, &mut args)?)?)
            }
            "--sounds" => flags.sounds = Some(parse_set(&flag, &value(&flag, inline, &mut args)?)?),
            "--motion" => {
                flags.motion = Some(parse_level(&flag, &value(&flag, inline, &mut args)?)?)
            }
            "--fps" => options.fps = parse_fps(&value(&flag, inline, &mut args)?)?,
            "--colour" | "--color" => {
                options.colour_depth =
                    Some(parse_colour(&flag, &value(&flag, inline, &mut args)?)?);
            }
            "--volume" => {
                let text = value(&flag, inline, &mut args)?;
                flags.volume = Some(sound::parse_volume(&text).ok_or_else(|| {
                    format!("--volume wants a whole number from 0 to 100, not \"{text}\"")
                })?);
            }
            _ if flag.starts_with('-') => {
                return Err(format!(
                    "unknown option {flag}. Run zacess-scenes watch --help for the options."
                ));
            }
            _ => {
                if job.is_some() {
                    return Err(format!(
                        "only one job can run at a time; \"{arg}\" is a second one"
                    ));
                }
                let name = arg.trim().to_ascii_lowercase().replace('_', "-");
                if !JOBS.contains(&name.as_str()) {
                    return Err(format!(
                        "\"{name}\" is not a job the watch mode runs. Use one of {}.",
                        JOBS.join(", ")
                    ));
                }
                job = Some(name);
            }
        }
    }
    if let Some(job) = job {
        options.job = job;
    }
    Ok(Command::Watch(options))
}

/// Parses `config init` and `config path`.
fn parse_config(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let args: Vec<String> = args.collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["init"] => Ok(Command::ConfigInit { force: false }),
        ["init", "--force"] | ["--force", "init"] => Ok(Command::ConfigInit { force: true }),
        ["path"] => Ok(Command::ConfigPath),
        _ => Err("config takes init (with --force to replace a file) or path".to_string()),
    }
}

/// Parses `sounds play <set> [event]`.
fn parse_sounds(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let args: Vec<String> = args.collect();
    match args.as_slice() {
        [play, set] if play == "play" => Ok(Command::SoundsPlay {
            set: parse_set("sounds play", set)?,
            event: None,
        }),
        [play, set, event] if play == "play" => Ok(Command::SoundsPlay {
            set: parse_set("sounds play", set)?,
            event: Some(sound::Tone::parse(event).ok_or_else(|| {
                format!(
                    "\"{event}\" is not an event. Use one of {}.",
                    sound::Tone::ALL.map(|t| t.name()).join(", ")
                )
            })?),
        }),
        _ => Err(
            "sounds takes play <set> [event], such as sounds play marimba recovered".to_string(),
        ),
    }
}

/// Parses `scene new <name> [--force]`.
fn parse_scene(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut force = false;
    let mut words = Vec::new();
    for arg in args {
        if arg == "--force" {
            force = true;
        } else {
            words.push(arg);
        }
    }
    match words.as_slice() {
        [new, name] if new == "new" => {
            let name = config::normalise_name(name);
            let ok = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            if !ok {
                return Err(format!(
                    "\"{name}\" is not a scene name: use letters, digits and hyphens"
                ));
            }
            if SCENES.iter().any(|s| s.name == name) {
                return Err(format!(
                    "{name} is a built-in scene already. Pick another name."
                ));
            }
            Ok(Command::SceneNew { name, force })
        }
        _ => Err("scene takes new <name>, such as scene new lighthouse".to_string()),
    }
}

/// Parses `snapshot <scene> [options]`.
fn parse_snapshot(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut options = SnapshotOptions::default();
    let mut scene: Option<String> = None;
    let mut args = args;
    while let Some(arg) = args.next() {
        let (flag, inline) = split_flag(&arg);
        match flag.as_str() {
            "-h" | "--help" => return Ok(Command::SnapshotHelp),
            "--theme" => options.theme = Some(value(&flag, inline, &mut args)?),
            "--size" => options.size = parse_size(&value(&flag, inline, &mut args)?)?,
            "--time" => {
                let text = value(&flag, inline, &mut args)?;
                let time: f32 = text
                    .parse()
                    .ok()
                    .filter(|t: &f32| t.is_finite() && *t >= 0.0 && *t <= 3600.0)
                    .ok_or_else(|| {
                        format!("--time wants seconds from 0 to 3600, not \"{text}\"")
                    })?;
                options.time = time;
            }
            "--frames" => {
                let text = value(&flag, inline, &mut args)?;
                let frames: u32 = text
                    .parse()
                    .ok()
                    .filter(|f| (1..=3600).contains(f))
                    .ok_or_else(|| format!("--frames wants 1 to 3600, not \"{text}\""))?;
                options.frames = Some(frames);
            }
            "--fps" => options.fps = parse_fps(&value(&flag, inline, &mut args)?)?,
            "--out" => options.out = Some(PathBuf::from(value(&flag, inline, &mut args)?)),
            "--state" => {
                let text = value(&flag, inline, &mut args)?;
                options.state = Some(match text.trim().to_ascii_lowercase().as_str() {
                    "pass" => SnapshotState::Pass,
                    "fail" => SnapshotState::Fail,
                    _ => return Err(format!("--state wants pass or fail, not \"{text}\"")),
                });
            }
            "--seed" => {
                let text = value(&flag, inline, &mut args)?;
                options.seed = text
                    .parse()
                    .map_err(|_| format!("--seed wants a whole number, not \"{text}\""))?;
            }
            "--motion" => {
                options.motion = Some(parse_level(&flag, &value(&flag, inline, &mut args)?)?)
            }
            "--reduced-motion" => options.reduced_motion = true,
            _ if flag.starts_with('-') => {
                return Err(format!(
                    "unknown option {flag}. Run zacess-scenes snapshot --help for the options."
                ));
            }
            _ => {
                if scene.is_some() {
                    return Err(format!(
                        "only one scene at a time; \"{arg}\" is a second one"
                    ));
                }
                scene = Some(config::normalise_name(&arg));
            }
        }
    }
    options.scene = scene.ok_or("snapshot needs a scene, such as snapshot koi-pond")?;
    Ok(Command::Snapshot(options))
}

/// Parses the arguments after the program name.
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Command, String> {
    let mut options = Options::default();
    let mut args = args.into_iter().peekable();
    match args.peek().map(String::as_str) {
        Some("watch") => {
            args.next();
            return parse_watch(args);
        }
        Some("list") => {
            args.next();
            return match args.next() {
                None => Ok(Command::List),
                Some(extra) => Err(format!("list takes no arguments, not \"{extra}\"")),
            };
        }
        Some("config") => {
            args.next();
            return parse_config(args);
        }
        Some("sounds") => {
            args.next();
            return parse_sounds(args);
        }
        Some("scene") => {
            args.next();
            return parse_scene(args);
        }
        Some("snapshot") => {
            args.next();
            return parse_snapshot(args);
        }
        _ => {}
    }
    while let Some(arg) = args.next() {
        let (flag, inline) = split_flag(&arg);
        match flag.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "--list-scenes" => return Ok(Command::ListScenes),
            "--list-themes" => return Ok(Command::ListThemes),
            "--slow" => options.slow = true,
            "--reduced-motion" => options.reduced_motion = true,
            "--headless" => options.headless = true,
            "--theme" => options.theme = Some(value(&flag, inline, &mut args)?),
            "--motion" => {
                options.motion = Some(parse_level(&flag, &value(&flag, inline, &mut args)?)?)
            }
            "--fps" => options.fps = parse_fps(&value(&flag, inline, &mut args)?)?,
            "--duration" => {
                let text = value(&flag, inline, &mut args)?;
                let minutes: f64 = text
                    .parse()
                    .map_err(|_| format!("--duration wants minutes, not \"{text}\""))?;
                if minutes <= 0.0 || !minutes.is_finite() {
                    return Err(format!(
                        "--duration must be more than 0 minutes, not {text}"
                    ));
                }
                options.duration_minutes = Some(minutes);
            }
            "--seed" => {
                let text = value(&flag, inline, &mut args)?;
                options.seed = Some(
                    text.parse()
                        .map_err(|_| format!("--seed wants a whole number, not \"{text}\""))?,
                );
            }
            "--frames" => {
                let text = value(&flag, inline, &mut args)?;
                let frames: u32 = text
                    .parse()
                    .map_err(|_| format!("--frames wants a whole number, not \"{text}\""))?;
                if frames == 0 {
                    return Err("--frames must be at least 1".to_string());
                }
                options.frames = frames;
            }
            "--colour" | "--color" => {
                options.colour_depth =
                    Some(parse_colour(&flag, &value(&flag, inline, &mut args)?)?);
            }
            "--size" => options.size = Some(parse_size(&value(&flag, inline, &mut args)?)?),
            "--scene" => {
                let name = value(&flag, inline, &mut args)?;
                set_scene(&mut options, &name)?;
            }
            _ if flag.starts_with('-') => {
                return Err(format!(
                    "unknown option {flag}. Run with --help for the options."
                ));
            }
            _ => set_scene(&mut options, &arg)?,
        }
    }
    Ok(Command::Run(options))
}

fn set_scene(options: &mut Options, name: &str) -> Result<(), String> {
    if options.scene.is_some() {
        return Err(format!(
            "only one scene can run at a time; \"{name}\" is a second one"
        ));
    }
    options.scene = Some(config::normalise_name(name));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Result<Command, String> {
        parse(args.iter().map(|a| a.to_string()))
    }

    fn run(args: &[&str]) -> Options {
        match parse_args(args) {
            Ok(Command::Run(options)) => options,
            other => panic!("expected a run, got {other:?}"),
        }
    }

    #[test]
    fn no_arguments_runs_the_default_scene() {
        let options = run(&[]);
        assert_eq!(options, Options::default());
        assert_eq!(options.scene, None);
        assert_eq!(options.fps, 20);
    }

    #[test]
    fn every_flag_parses_in_both_spellings() {
        let options = run(&[
            "snow",
            "--theme",
            "cream",
            "--fps=12",
            "--slow",
            "--reduced-motion",
            "--duration",
            "1.5",
            "--seed=7",
            "--headless",
            "--frames",
            "3",
            "--size=20x8",
            "--colour",
            "256",
        ]);
        assert_eq!(options.scene.as_deref(), Some("snow"));
        assert_eq!(options.theme.as_deref(), Some("cream"));
        assert_eq!(options.fps, 12);
        assert!(options.slow && options.reduced_motion && options.headless);
        assert_eq!(options.duration_minutes, Some(1.5));
        assert_eq!(options.seed, Some(7));
        assert_eq!(options.frames, 3);
        assert_eq!(options.size, Some((20, 8)));
        assert_eq!(options.colour_depth, Some(ColourDepth::Ansi256));
        assert_eq!(
            run(&["--color=truecolor"]).colour_depth,
            Some(ColourDepth::TrueColour)
        );
    }

    #[test]
    fn scene_names_are_normalised() {
        assert_eq!(run(&["Code_Rain"]).scene.as_deref(), Some("code-rain"));
        assert_eq!(run(&["ZACESS"]).scene.as_deref(), Some("zacess"));
        assert_eq!(
            run(&["--scene", "Koi Pond"]).scene.as_deref(),
            Some("koi-pond")
        );
        assert_eq!(run(&["--motion=lively"]).motion, Some(Level::Lively));
    }

    #[test]
    fn informational_flags_win() {
        assert_eq!(parse_args(&["--help"]), Ok(Command::Help));
        assert_eq!(parse_args(&["-h"]), Ok(Command::Help));
        assert_eq!(parse_args(&["--version"]), Ok(Command::Version));
        assert_eq!(
            parse_args(&["sea", "--list-scenes"]),
            Ok(Command::ListScenes)
        );
        assert_eq!(parse_args(&["--list-themes"]), Ok(Command::ListThemes));
    }

    #[test]
    fn bad_values_are_errors_with_the_flag_named() {
        assert!(parse_args(&["--fps", "0"]).unwrap_err().contains("--fps"));
        assert!(parse_args(&["--fps", "31"]).unwrap_err().contains("--fps"));
        assert!(parse_args(&["--fps", "ten"]).unwrap_err().contains("--fps"));
        assert!(
            parse_args(&["--duration", "0"])
                .unwrap_err()
                .contains("--duration")
        );
        assert!(
            parse_args(&["--duration", "-1"])
                .unwrap_err()
                .contains("--duration")
        );
        assert!(
            parse_args(&["--duration"])
                .unwrap_err()
                .contains("--duration")
        );
        assert!(parse_args(&["--seed", "x"]).unwrap_err().contains("--seed"));
        assert!(
            parse_args(&["--frames", "0"])
                .unwrap_err()
                .contains("--frames")
        );
        assert!(
            parse_args(&["--size", "80"])
                .unwrap_err()
                .contains("--size")
        );
        assert!(
            parse_args(&["--size", "0x8"])
                .unwrap_err()
                .contains("--size")
        );
        assert!(
            parse_args(&["--size", "5000x8"])
                .unwrap_err()
                .contains("--size")
        );
        assert!(parse_args(&["--theme"]).unwrap_err().contains("--theme"));
        assert!(
            parse_args(&["--colour", "16"])
                .unwrap_err()
                .contains("--colour")
        );
    }

    #[test]
    fn unknown_options_and_scenes_are_errors() {
        assert!(parse_args(&["--wibble"]).unwrap_err().contains("--wibble"));
        // a scene name is checked against the catalogue later, so user scenes work too
        assert_eq!(run(&["lava"]).scene.as_deref(), Some("lava"));
        assert!(
            parse_args(&["--motion", "fast"])
                .unwrap_err()
                .contains("--motion")
        );
        assert!(
            parse_args(&["sea", "snow"])
                .unwrap_err()
                .contains("one scene")
        );
    }

    fn watch(args: &[&str]) -> WatchOptions {
        let mut all = vec!["watch"];
        all.extend_from_slice(args);
        match parse_args(&all) {
            Ok(Command::Watch(options)) => options,
            other => panic!("expected a watch, got {other:?}"),
        }
    }

    #[test]
    fn watch_defaults_to_the_check_job_with_sound() {
        let options = watch(&[]);
        assert_eq!(options, WatchOptions::default());
        assert_eq!(options.job, "check");
        assert!(!options.flags.no_sound);
        assert_eq!(options.flags.volume, None);
    }

    #[test]
    fn watch_takes_a_job_and_its_flags() {
        for job in JOBS {
            assert_eq!(watch(&[job]).job, job);
        }
        assert_eq!(watch(&["Check_All"]).job, "check-all");
        let options = watch(&[
            "clippy",
            "--no-sound",
            "--volume=30",
            "--theme",
            "cream",
            "--fps",
            "10",
            "--reduced-motion",
            "--colour",
            "256",
        ]);
        assert_eq!(options.job, "clippy");
        assert!(options.flags.no_sound && options.flags.reduced_motion);
        assert_eq!(options.flags.volume, Some(30));
        assert_eq!(options.flags.theme.as_deref(), Some("cream"));
        assert_eq!(options.fps, 10);
        assert_eq!(options.colour_depth, Some(ColourDepth::Ansi256));
        assert_eq!(watch(&["--volume", "0"]).flags.volume, Some(0));
        let options = watch(&[
            "--scene",
            "Koi_Pond, tea",
            "--fail-scene=flood",
            "--sounds",
            "Marimba",
            "--motion",
            "calm",
            "--notify",
        ]);
        assert_eq!(
            options.flags.scene,
            Some(vec!["koi-pond".to_string(), "tea".to_string()])
        );
        assert_eq!(options.flags.fail_scene, Some(vec!["flood".to_string()]));
        assert_eq!(options.flags.sounds, Some(SoundSet::Marimba));
        assert_eq!(options.flags.motion, Some(Level::Calm));
        assert_eq!(options.flags.notify, Some(true));
        assert_eq!(watch(&["--no-notify"]).flags.notify, Some(false));
        assert!(
            parse_args(&["watch", "--sounds", "tuba"])
                .unwrap_err()
                .contains("--sounds")
        );
        assert!(
            parse_args(&["watch", "--scene", ","])
                .unwrap_err()
                .contains("--scene")
        );
        assert_eq!(parse_args(&["watch", "--help"]), Ok(Command::WatchHelp));
    }

    #[test]
    fn watch_rejects_bad_jobs_and_values() {
        assert!(
            parse_args(&["watch", "doc"])
                .unwrap_err()
                .contains("check-all")
        );
        assert!(
            parse_args(&["watch", "check", "test"])
                .unwrap_err()
                .contains("one job")
        );
        for bad in ["101", "-1", "loud"] {
            assert!(
                parse_args(&["watch", "--volume", bad])
                    .unwrap_err()
                    .contains("--volume")
            );
        }
        assert!(
            parse_args(&["watch", "--volume"])
                .unwrap_err()
                .contains("--volume")
        );
        assert!(
            parse_args(&["watch", "--slow"])
                .unwrap_err()
                .contains("--slow")
        );
        // a scene flag before the word makes it a scene name
        assert_eq!(
            parse_args(&["--slow", "watch"]).map(|c| matches!(c, Command::Run(_))),
            Ok(true)
        );
    }

    #[test]
    fn the_watch_help_names_every_job_key_and_variable() {
        let text = watch_usage();
        for job in JOBS {
            assert!(text.contains(job), "{job}");
        }
        for needle in [
            "--no-sound",
            "--volume",
            "--scene",
            "--fail-scene",
            "--sounds",
            "--motion",
            "--notify",
            "ZACESS_SCENE",
            "ZACESS_NOTIFY",
            "ZACESS_SOUND",
            "ZACESS_VOLUME",
            "cargo install --locked bacon",
        ] {
            assert!(text.contains(needle), "{needle}");
        }
        assert!(usage().contains("watch"));
    }

    #[test]
    fn the_help_names_every_scene_and_key() {
        let text = usage();
        for scene in &SCENES {
            assert!(text.contains(scene.name), "{}", scene.name);
        }
        assert!(text.contains("Space"));
        assert!(text.contains("Ctrl+C"));
        assert!(text.contains("ZACESS_THEME"));
        assert!(text.contains("NO_COLOR"));
    }

    #[test]
    fn the_commands_parse() {
        assert_eq!(parse_args(&["list"]), Ok(Command::List));
        assert!(parse_args(&["list", "x"]).is_err());
        assert_eq!(
            parse_args(&["config", "init"]),
            Ok(Command::ConfigInit { force: false })
        );
        assert_eq!(
            parse_args(&["config", "init", "--force"]),
            Ok(Command::ConfigInit { force: true })
        );
        assert_eq!(parse_args(&["config", "path"]), Ok(Command::ConfigPath));
        assert!(parse_args(&["config"]).unwrap_err().contains("init"));
        assert_eq!(
            parse_args(&["sounds", "play", "kalimba"]),
            Ok(Command::SoundsPlay {
                set: SoundSet::Kalimba,
                event: None
            })
        );
        assert_eq!(
            parse_args(&["sounds", "play", "chip", "still-failing"]),
            Ok(Command::SoundsPlay {
                set: SoundSet::Chip,
                event: Some(sound::Tone::StillFailing)
            })
        );
        assert!(
            parse_args(&["sounds", "play", "chip", "boom"])
                .unwrap_err()
                .contains("event")
        );
        assert!(parse_args(&["sounds", "play", "tuba"]).is_err());
        assert_eq!(
            parse_args(&["scene", "new", "Light_House"]),
            Ok(Command::SceneNew {
                name: "light-house".into(),
                force: false
            })
        );
        assert!(
            parse_args(&["scene", "new", "waves"])
                .unwrap_err()
                .contains("built-in")
        );
        assert!(parse_args(&["scene", "new", "a/b"]).is_err());
    }

    #[test]
    fn snapshot_takes_its_options() {
        let Ok(Command::Snapshot(options)) = parse_args(&[
            "snapshot",
            "koi-pond",
            "--theme",
            "light",
            "--size=100x30",
            "--time",
            "2.5",
            "--frames",
            "72",
            "--fps",
            "12",
            "--state",
            "pass",
            "--seed",
            "9",
            "--out",
            "x.json",
        ]) else {
            panic!("expected a snapshot");
        };
        assert_eq!(options.scene, "koi-pond");
        assert_eq!(options.size, (100, 30));
        assert_eq!(options.time, 2.5);
        assert_eq!(options.frames, Some(72));
        assert_eq!(options.state, Some(SnapshotState::Pass));
        assert_eq!(options.seed, 9);
        assert_eq!(options.out, Some(PathBuf::from("x.json")));
        assert_eq!(
            parse_args(&["snapshot", "--help"]),
            Ok(Command::SnapshotHelp)
        );
        assert!(
            parse_args(&["snapshot"])
                .unwrap_err()
                .contains("needs a scene")
        );
        assert!(parse_args(&["snapshot", "sea", "--state", "x"]).is_err());
        assert!(parse_args(&["snapshot", "sea", "--time", "-1"]).is_err());
        assert!(snapshot_usage().contains("--frames"));
    }
}
