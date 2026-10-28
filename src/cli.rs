//! The command line: a scene name and a handful of flags, parsed by hand so the binary stays
//! small.

use crate::scenes::SCENES;

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
}

/// The settings for a run.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub scene: String,
    pub theme: Option<String>,
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
}

impl Default for Options {
    fn default() -> Self {
        Self {
            scene: DEFAULT_SCENE.to_string(),
            theme: None,
            fps: DEFAULT_FPS,
            slow: false,
            reduced_motion: false,
            duration_minutes: None,
            seed: None,
            headless: false,
            frames: DEFAULT_FRAMES,
            size: None,
        }
    }
}

/// The help text.
pub fn usage() -> String {
    let mut text = String::from(
        "Usage: zacess-scenes [scene] [options]\n\nCalm animated scenes for the terminal in the Zacess themes.\n\nScenes:\n",
    );
    for scene in &SCENES {
        text.push_str(&format!("  {:<11} {}\n", scene.name, scene.summary));
    }
    text.push_str(&format!(
        "\nThe default scene is {DEFAULT_SCENE}.\n\nOptions:\n\
  --theme <name>       A Zacess theme by name, id or short name\n\
  --list-themes        Print the themes and which one is active\n\
  --list-scenes        Print the scenes\n\
  --fps <n>            Frames per second, 1 to {MAX_FPS} (default {DEFAULT_FPS})\n\
  --slow               Half speed\n\
  --reduced-motion     Fewer and slower movements, no lightning and no flicker\n\
  --duration <minutes> Stop after this many minutes\n\
  --seed <n>           Seed the random numbers, so a run can be repeated\n\
  --headless           Draw without a terminal and print the last frame\n\
  --frames <n>         Frames to draw headless (default {DEFAULT_FRAMES})\n\
  --size <WxH>         Columns and rows for a headless render\n\
  -h, --help           This help\n\
  -V, --version        The version\n\
\nKeys while a scene runs: Space pauses and resumes, h shows the help line again and q, Esc or\n\
Ctrl+C quits.\n\nThe theme is taken from --theme, then ZACESS_THEME, then the Zacess theme switcher's state\n\
file, otherwise Zacess Deep Contrast Dark. ZACESS_REDUCED_MOTION=1 or the system's reduce motion\n\
setting also turns reduced motion on. NO_COLOR draws in the terminal's own text colour.\n"
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

/// Parses the arguments after the program name.
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Command, String> {
    let mut options = Options::default();
    let mut scene: Option<String> = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((flag, inline)) if flag.starts_with("--") => (flag.to_string(), Some(inline)),
            _ => (arg.clone(), None),
        };
        match flag.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "--list-scenes" => return Ok(Command::ListScenes),
            "--list-themes" => return Ok(Command::ListThemes),
            "--slow" => options.slow = true,
            "--reduced-motion" => options.reduced_motion = true,
            "--headless" => options.headless = true,
            "--theme" => options.theme = Some(value(&flag, inline, &mut args)?),
            "--fps" => {
                let text = value(&flag, inline, &mut args)?;
                let fps: u32 = text
                    .parse()
                    .map_err(|_| format!("--fps wants a whole number, not \"{text}\""))?;
                if !(1..=MAX_FPS).contains(&fps) {
                    return Err(format!("--fps must be between 1 and {MAX_FPS}, not {fps}"));
                }
                options.fps = fps;
            }
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
            "--size" => options.size = Some(parse_size(&value(&flag, inline, &mut args)?)?),
            _ if flag.starts_with('-') => {
                return Err(format!(
                    "unknown option {flag}. Run with --help for the options."
                ));
            }
            _ => {
                if scene.is_some() {
                    return Err(format!(
                        "only one scene can run at a time; \"{arg}\" is a second one"
                    ));
                }
                scene = Some(arg.trim().to_ascii_lowercase().replace('_', "-"));
            }
        }
    }
    if let Some(name) = scene {
        if !SCENES.iter().any(|s| s.name == name) {
            return Err(format!(
                "\"{name}\" is not a scene. Run with --list-scenes to see them."
            ));
        }
        options.scene = name;
    }
    Ok(Command::Run(options))
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
        assert_eq!(options.scene, "sea");
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
        ]);
        assert_eq!(options.scene, "snow");
        assert_eq!(options.theme.as_deref(), Some("cream"));
        assert_eq!(options.fps, 12);
        assert!(options.slow && options.reduced_motion && options.headless);
        assert_eq!(options.duration_minutes, Some(1.5));
        assert_eq!(options.seed, Some(7));
        assert_eq!(options.frames, 3);
        assert_eq!(options.size, Some((20, 8)));
    }

    #[test]
    fn scene_names_are_normalised() {
        assert_eq!(run(&["Code_Rain"]).scene, "code-rain");
        assert_eq!(run(&["ZACESS"]).scene, "zacess");
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
    }

    #[test]
    fn unknown_options_and_scenes_are_errors() {
        assert!(parse_args(&["--wibble"]).unwrap_err().contains("--wibble"));
        assert!(parse_args(&["lava"]).unwrap_err().contains("--list-scenes"));
        assert!(
            parse_args(&["sea", "snow"])
                .unwrap_err()
                .contains("one scene")
        );
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
}
