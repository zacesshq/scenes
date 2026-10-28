use std::process::ExitCode;
use std::time::Duration;

use zacess_scenes::cli::{self, Command};
use zacess_scenes::rng::Rng;
use zacess_scenes::scene::Motion;
use zacess_scenes::terminal::{self, Terminal};
use zacess_scenes::{VERSION, app, motion, scenes, theme};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("zacess-scenes: {message}");
            ExitCode::from(2)
        }
    }
}

fn pick_theme(flag: Option<&str>) -> Result<&'static theme::ThemeData, String> {
    let env = std::env::var(theme::ENV_VAR).ok();
    let switcher = theme::read_switcher_state();
    // the system is only asked when the switcher follows it
    let prefers_dark = match &switcher {
        Some(theme::SwitcherState::Auto { .. }) => motion::system_prefers_dark(),
        _ => true,
    };
    theme::choose(&theme::Choice {
        flag,
        env: env.as_deref(),
        switcher,
        prefers_dark,
    })
}

fn run() -> Result<(), String> {
    let options = match cli::parse(std::env::args().skip(1))? {
        Command::Help => {
            print!("{}", cli::usage());
            return Ok(());
        }
        Command::Version => {
            println!(
                "zacess-scenes {VERSION} (Zacess palette {})",
                theme::PALETTE_VERSION
            );
            return Ok(());
        }
        Command::ListScenes => {
            for scene in &scenes::SCENES {
                println!("{:<11} {}", scene.name, scene.summary);
            }
            return Ok(());
        }
        Command::ListThemes => {
            let active = pick_theme(None)?;
            for theme in theme::all() {
                let mark = if std::ptr::eq(theme, active) {
                    " (active)"
                } else {
                    ""
                };
                println!(
                    "{:<21} {:<11} {}{mark}",
                    theme.id,
                    theme.short(),
                    theme.name
                );
            }
            return Ok(());
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
        println!("{}", frame.to_text());
        return Ok(());
    }

    let intro = format!(
        "Zacess Scenes: {} in {}{}. Space pauses, h shows the keys, q, Esc or Ctrl+C quits.",
        info.name,
        theme.name,
        if calm { " with reduced motion" } else { "" }
    );
    let mut terminal = Terminal::open(&intro)
        .map_err(|error| format!("could not take over the terminal: {error}"))?;
    let (width, height) = terminal
        .size()
        .map_err(|error| format!("could not read the terminal size: {error}"))?;
    let mut scene = scenes::create(info.name, width, height, motion, &mut rng)
        .ok_or_else(|| format!("no scene {}", info.name))?;
    let result = app::run(scene.as_mut(), theme, &settings, &mut rng, &mut terminal);
    drop(terminal);
    println!("Zacess Scenes stopped.");
    result.map_err(|error| format!("terminal error: {error}"))
}
