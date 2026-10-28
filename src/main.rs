use std::io::{self, Write};
use std::process::ExitCode;
use std::time::Duration;

use zacess_scenes::cli::{self, Command};
use zacess_scenes::colour::ColourDepth;
use zacess_scenes::rng::Rng;
use zacess_scenes::scene::Motion;
use zacess_scenes::terminal::{self, Terminal};
use zacess_scenes::{VERSION, app, glyphs, motion, scenes, theme};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        // a reader that stops early, such as `head`, is not an error
        Err(message) if message == BROKEN_PIPE => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("zacess-scenes: {message}");
            ExitCode::from(2)
        }
    }
}

const BROKEN_PIPE: &str = "broken pipe";

/// Writes text to standard output, treating a closed pipe as the reader having seen enough.
fn emit(text: &str) -> Result<(), String> {
    let mut out = io::stdout().lock();
    match out.write_all(text.as_bytes()).and_then(|()| out.flush()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Err(BROKEN_PIPE.to_string()),
        Err(error) => Err(format!("could not write the output: {error}")),
    }
}

fn pick_theme(flag: Option<&str>) -> Result<&'static theme::ThemeData, String> {
    let env = std::env::var(theme::ENV_VAR).ok();
    let state_file = theme::read_state_file();
    // the system is only asked when the state file follows it
    let prefers_dark = match &state_file {
        Some(theme::ThemeState::Auto { .. }) => motion::system_prefers_dark(),
        _ => true,
    };
    theme::choose(&theme::Choice {
        flag,
        env: env.as_deref(),
        state_file,
        prefers_dark,
    })
}

fn run() -> Result<(), String> {
    let options = match cli::parse(std::env::args().skip(1))? {
        Command::Help => return emit(&cli::usage()),
        Command::Version => {
            return emit(&format!(
                "zacess-scenes {VERSION} (Zacess palette {})\n",
                theme::PALETTE_VERSION
            ));
        }
        Command::ListScenes => {
            let mut text = String::new();
            for scene in &scenes::SCENES {
                text.push_str(&format!("{:<11} {}\n", scene.name, scene.summary));
            }
            return emit(&text);
        }
        Command::ListThemes => {
            let active = pick_theme(None)?;
            let mut text = String::new();
            for theme in theme::all() {
                let mark = if std::ptr::eq(theme, active) {
                    " (active)"
                } else {
                    ""
                };
                text.push_str(&format!(
                    "{:<21} {:<11} {}{mark}\n",
                    theme.id,
                    theme.short(),
                    theme.name
                ));
            }
            return emit(&text);
        }
        Command::Run(options) => options,
    };

    let theme = pick_theme(options.theme.as_deref())?;
    let env_motion = std::env::var(motion::ENV_VAR).ok();
    // a headless render never asks the system, so the same command draws the same frame anywhere
    let calm = motion::reduced(options.reduced_motion, env_motion.as_deref(), || {
        !options.headless && motion::system_reduced_motion()
    });
    let colour = !std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
    let settings = app::Settings {
        fps: options.fps,
        speed: if options.slow { 0.5 } else { 1.0 },
        calm,
        duration: options
            .duration_minutes
            .map(|minutes| Duration::from_secs_f64(minutes * 60.0)),
        colour,
    };
    let motion = if calm { Motion::CALM } else { Motion::FULL };
    let mut rng = options.seed.map(Rng::new).unwrap_or_else(Rng::from_time);
    let info = scenes::info(&options.scene).ok_or_else(|| format!("no scene {}", options.scene))?;

    if options.headless {
        let (width, height) = options.size.unwrap_or_else(|| terminal::size_or((80, 24)));
        let mut scene = scenes::create(info.name, width, height, motion, &mut rng)
            .ok_or_else(|| format!("no scene {}", info.name))?;
        let frame = app::render_headless(
            scene.as_mut(),
            theme,
            &settings,
            options.frames,
            &mut rng,
            width,
            height,
        );
        return emit(&format!("{}\n", frame.to_text()));
    }

    let intro = format!(
        "Zacess Scenes: {} in {}{}. Space pauses, h shows the keys, q, Esc or Ctrl+C quits.",
        info.name,
        theme.name,
        if calm { " with reduced motion" } else { "" }
    );
    let env = |name: &str| std::env::var(name).ok();
    let depth = options
        .colour_depth
        .unwrap_or_else(|| ColourDepth::detect(env));
    glyphs::set_basic(terminal::limited_font(env));
    let (width, height) = terminal::size_or(terminal::FALLBACK_SIZE);
    let mut scene = scenes::create(info.name, width, height, motion, &mut rng)
        .ok_or_else(|| format!("no scene {}", info.name))?;
    // from here every way out, a panic included, prints the stopped line as the terminal is
    // given back
    let mut terminal = Terminal::open(&intro)
        .map_err(|error| format!("could not take over the terminal: {error}"))?;
    terminal.set_paint(terminal::Paint {
        depth,
        background: theme.background,
        floor: theme.floor(),
    });
    let result = app::run(scene.as_mut(), theme, &settings, &mut rng, &mut terminal);
    drop(terminal);
    result.map_err(|error| format!("terminal error: {error}"))
}
