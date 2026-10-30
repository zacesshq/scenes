use std::io::{self, Write};
use std::process::ExitCode;
use std::time::Duration;

use zacess_scenes::ambient::Ambient;
use zacess_scenes::catalogue::Catalogue;
use zacess_scenes::cli::{self, Command};
use zacess_scenes::colour::ColourDepth;
use zacess_scenes::config::Rotation;
use zacess_scenes::config::{self, Config, Flags};
use zacess_scenes::rng::Rng;
use zacess_scenes::scene::Motion;
use zacess_scenes::sound::{self, SoundSet, Tone};
use zacess_scenes::terminal::{self, Terminal};
use zacess_scenes::watch::{How, follow, runner};
use zacess_scenes::{VERSION, app, glyphs, lua, motion, snapshot, theme, watch};

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

/// Minutes between scenes with --shuffle when nothing else sets them.
const DEFAULT_EVERY: f64 = 5.0;

/// Writes text to standard output, treating a closed pipe as the reader having seen enough.
fn emit(text: &str) -> Result<(), String> {
    let mut out = io::stdout().lock();
    match out.write_all(text.as_bytes()).and_then(|()| out.flush()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Err(BROKEN_PIPE.to_string()),
        Err(error) => Err(format!("could not write the output: {error}")),
    }
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// The config file's settings. A broken file is an error that names the line, so a typo is
/// never silently ignored.
fn load_config() -> Result<Config, String> {
    match config::config_path() {
        Some(path) => Config::load(&path),
        None => Ok(Config::default()),
    }
}

/// The scenes on offer: the built-in ones and the user's Lua scenes.
fn load_catalogue() -> Catalogue {
    Catalogue::load(config::scenes_dir().as_deref())
}

/// Picks the theme: the flag, the variable, the config file, the state file, the default.
fn pick_theme(resolved: &config::Resolved) -> Result<&'static theme::ThemeData, String> {
    let state_file = theme::read_state_file();
    // the system is only asked when the state file follows it
    let prefers_dark = match &state_file {
        Some(theme::ThemeState::Auto { .. }) => motion::system_prefers_dark(),
        _ => true,
    };
    let (flag, env) = if resolved.theme_from_env {
        (None, resolved.theme.as_deref())
    } else {
        (resolved.theme.as_deref(), None)
    };
    theme::choose(&theme::Choice {
        flag,
        env,
        state_file,
        prefers_dark,
    })
}

fn list(catalogue: &Catalogue) -> String {
    let mut text = String::from(
        "Scenes, marked pass, fail or general (general scenes also show while a build passes):\n",
    );
    let width = catalogue
        .entries
        .iter()
        .map(|e| e.name.len())
        .max()
        .unwrap_or(0);
    for entry in &catalogue.entries {
        let mine = if entry.user.is_some() { " (yours)" } else { "" };
        text.push_str(&format!(
            "  {:<width$}  {:<7}  {}{mine}\n",
            entry.name,
            entry.kind.name(),
            entry.summary
        ));
    }
    let pass = catalogue.entries.iter().filter(|e| e.kind.passes()).count();
    let fail = catalogue.entries.len() - pass;
    text.push_str(&format!(
        "\n{} scenes: {pass} for a passing build and {fail} for a failing one.\n",
        catalogue.entries.len()
    ));
    for error in &catalogue.errors {
        text.push_str(&format!("Not loaded: {error}\n"));
    }
    text.push_str("\nSound sets (preview with zacess-scenes sounds play <set> [event]):\n");
    for set in SoundSet::ALL {
        text.push_str(&format!("  {:<8} {}\n", set.name(), set.summary()));
    }
    text.push_str(&format!(
        "\nEvents: {}.\n",
        Tone::ALL.map(|t| t.name()).join(", ")
    ));
    text
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
            let catalogue = load_catalogue();
            let mut text = String::new();
            for entry in &catalogue.entries {
                text.push_str(&format!("{:<15} {}\n", entry.name, entry.summary));
            }
            return emit(&text);
        }
        Command::ListThemes => {
            let resolved = config::resolve(&Flags::default(), env, &load_config()?)?;
            let active = pick_theme(&resolved)?;
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
        Command::List => return emit(&list(&load_catalogue())),
        Command::ConfigPath => {
            let path =
                config::config_path().ok_or("there is no home folder to keep a config in")?;
            let exists = if path.exists() {
                ""
            } else {
                " (not created yet)"
            };
            return emit(&format!("{}{exists}\n", path.display()));
        }
        Command::ConfigInit { force } => {
            let path =
                config::config_path().ok_or("there is no home folder to keep a config in")?;
            config::write_starter(&path, force)?;
            return emit(&format!(
                "Wrote {}. Every setting in it is commented out; remove the # to change one.\n",
                path.display()
            ));
        }
        Command::SceneNew { name, force } => return scene_new(&name, force),
        Command::SoundsPlay { set, event } => return sounds_play(set, event),
        Command::WatchHelp => return emit(&cli::watch_usage()),
        Command::SnapshotHelp => return emit(&cli::snapshot_usage()),
        Command::Snapshot(options) => return run_snapshot(options),
        Command::Watch(options) => return run_watch(options, false),
        Command::FollowHelp => return emit(&cli::follow_usage()),
        Command::FollowInit { shell } => {
            return emit(follow::hook(&shell).ok_or("there is no hook for that shell")?);
        }
        Command::Follow(options) => return run_watch(options, true),
        Command::Run(options) => options,
    };

    let config = load_config()?;
    let flags = Flags {
        theme: options.theme.clone(),
        motion: options.motion,
        reduced_motion: options.reduced_motion,
        ambient: options.ambient,
        ambient_volume: options.ambient_volume,
        ..Flags::default()
    };
    let resolved = config::resolve(&flags, env, &config)?;
    let theme = pick_theme(&resolved)?;
    let mut catalogue = load_catalogue();
    // a scene named on the command line wins; then the config's first pass scene
    let wanted = options
        .scene
        .clone()
        .or_else(|| resolved.pass.as_ref().and_then(|p| p.first().cloned()))
        .unwrap_or_else(|| cli::DEFAULT_SCENE.to_string());
    if catalogue.find(&wanted).is_none() {
        let hint = catalogue
            .errors
            .iter()
            .find(|e| e.contains(&wanted))
            .map(|e| format!(" It did not load: {e}"))
            .unwrap_or_default();
        return Err(format!(
            "\"{wanted}\" is not a scene. Run zacess-scenes list to see them.{hint}"
        ));
    }
    // a headless render never asks the system, so the same command draws the same frame anywhere
    let calm = resolved.reduced_motion || (!options.headless && motion::system_reduced_motion());
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
    let scene_motion = Motion::new(calm, resolved.motion);
    let mut rng = options.seed.map(Rng::new).unwrap_or_else(Rng::from_time);

    if options.headless {
        let (width, height) = options.size.unwrap_or_else(|| terminal::size_or((80, 24)));
        let mut scene = catalogue.create(&wanted, width, height, scene_motion, &mut rng)?;
        let frame = app::render_headless(
            scene.as_mut(),
            theme,
            &settings,
            options.frames,
            &mut rng,
            width,
            height,
        );
        if let Some(problem) = scene.problem() {
            return Err(problem);
        }
        return emit(&format!("{}\n", frame.to_text()));
    }

    let names = catalogue.names();
    let index = names.iter().position(|n| *n == wanted).unwrap_or(0);
    let intro = format!(
        "Zacess Scenes: {wanted} in {}{}. Space pauses, n and b change the scene, h shows the \
         keys, q, Esc or Ctrl+C quits.",
        theme.name,
        if calm { " with reduced motion" } else { "" }
    );
    let depth = options
        .colour_depth
        .unwrap_or_else(|| ColourDepth::detect(env));
    glyphs::set_basic(terminal::limited_font(env));
    // from here every way out, a panic included, prints the stopped line as the terminal is
    // given back
    let mut terminal = Terminal::open(&intro)
        .map_err(|error| format!("could not take over the terminal: {error}"))?;
    terminal.set_paint(terminal::Paint {
        depth,
        background: theme.background,
        floor: theme.floor(),
    });
    // the timer runs when a flag asks for it (from the config only when no scene was named)
    let from_config =
        options.scene.is_none() && resolved.rotation != Rotation::Fixed && resolved.interval > 0.0;
    let auto = if options.shuffle || options.every_minutes.is_some() || from_config {
        // --every on its own goes through the list in order, as its help says; the config's
        // rotation applies only when the timer comes from the config
        let rotation = if options.shuffle {
            Rotation::Shuffle
        } else if options.every_minutes.is_some() {
            Rotation::Cycle
        } else {
            resolved.rotation
        };
        let minutes = options
            .every_minutes
            .or((resolved.interval > 0.0).then_some(resolved.interval))
            .unwrap_or(DEFAULT_EVERY);
        let pool = match resolved.pass.as_ref() {
            Some(list) => {
                catalogue.check_list(list, true, "the pass scene list")?;
                list.clone()
            }
            None => catalogue.names_for(true),
        };
        Some(app::Auto::new(
            rotation,
            Duration::from_secs_f64(minutes * 60.0),
            pool,
            &wanted,
        ))
    } else {
        None
    };
    let ambient = Ambient::new(resolved.ambient, resolved.ambient_volume)
        .with_quiet_hours(resolved.quiet_hours);
    let mut playlist = app::Playlist {
        catalogue: &mut catalogue,
        names,
        index,
        motion: scene_motion,
        auto,
        ambient: ambient.available().then_some(ambient),
    };
    let result = app::run(&mut playlist, theme, &settings, &mut rng, &mut terminal);
    drop(terminal);
    result.map_err(|error| format!("terminal error: {error}"))
}

fn scene_new(name: &str, force: bool) -> Result<(), String> {
    let dir = config::scenes_dir().ok_or("there is no home folder to keep scenes in")?;
    let path = dir.join(format!("{name}.lua"));
    if path.exists() && !force {
        return Err(format!(
            "{} already exists. Add --force to replace it.",
            path.display()
        ));
    }
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    std::fs::write(&path, lua::starter(name))
        .map_err(|e| format!("could not write {}: {e}", path.display()))?;
    emit(&format!(
        "Wrote {}. Run zacess-scenes {name} to see it and edit the file to make it yours.\n",
        path.display()
    ))
}

fn sounds_play(set: SoundSet, event: Option<Tone>) -> Result<(), String> {
    let config = load_config()?;
    let resolved = config::resolve(&Flags::default(), env, &config)?;
    let sound = sound::Sound::new(true, resolved.volume);
    if !sound.available() {
        return Err(
            "no sound player was found (afplay, pw-play, paplay, aplay or PowerShell)".into(),
        );
    }
    let tone = event.unwrap_or(Tone::Pass);
    emit(&format!(
        "Playing the {} tone of {} at volume {}.\n",
        tone.name(),
        set.name(),
        resolved.volume
    ))?;
    if let Some(mut child) = sound.play_now(set, tone) {
        let _ = child.wait();
    }
    Ok(())
}

fn run_snapshot(options: cli::SnapshotOptions) -> Result<(), String> {
    let config = load_config()?;
    let flags = Flags {
        theme: options.theme.clone(),
        motion: options.motion,
        reduced_motion: options.reduced_motion,
        ..Flags::default()
    };
    let resolved = config::resolve(&flags, env, &config)?;
    let theme = pick_theme(&resolved)?;
    let catalogue = load_catalogue();
    let entry = catalogue.find(&options.scene).ok_or_else(|| {
        format!(
            "\"{}\" is not a scene. Run zacess-scenes list to see them.",
            options.scene
        )
    })?;
    let shot = snapshot::Shot {
        catalogue: &catalogue,
        scene: &options.scene,
        theme,
        size: options.size,
        seed: options.seed,
        motion: Motion::new(resolved.reduced_motion, resolved.motion),
        state: options.state,
    };
    let text = match options.frames {
        Some(count) => {
            let frames = shot.frames(options.time, count, options.fps)?;
            snapshot::frames_json(&frames, options.fps, theme)
        }
        None => {
            let frame = shot.still(options.time)?;
            let state = match options.state {
                Some(cli::SnapshotState::Pass) => " \u{b7} watch mode passing",
                Some(cli::SnapshotState::Fail) => " \u{b7} watch mode failing",
                None => "",
            };
            let title = format!("{}{state} \u{b7} {}", entry.name, theme.name);
            let label = snapshot::label(&entry.name, &entry.summary, theme, options.state);
            snapshot::svg(&frame, theme, &title, &label)
        }
    };
    match &options.out {
        Some(path) => {
            if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
                std::fs::create_dir_all(dir)
                    .map_err(|e| format!("could not create {}: {e}", dir.display()))?;
            }
            std::fs::write(path, text)
                .map_err(|e| format!("could not write {}: {e}", path.display()))
        }
        None => emit(&text),
    }
}

/// A scene list from the settings (checked against the catalogue) or every scene of the kind
/// with the default first.
fn scene_list(
    catalogue: &Catalogue,
    chosen: Option<&Vec<String>>,
    passing: bool,
) -> Result<Vec<String>, String> {
    if let Some(list) = chosen {
        let what = if passing {
            "the pass scene list"
        } else {
            "the fail scene list"
        };
        catalogue.check_list(list, passing, what)?;
        return Ok(list.clone());
    }
    let first = if passing {
        watch::CALM_SCENE
    } else {
        watch::FAIL_SCENE
    };
    let mut all = vec![first.to_string()];
    all.extend(
        catalogue
            .names_for(passing)
            .into_iter()
            .filter(|n| n != first),
    );
    Ok(all)
}

/// Where the watch mode's results come from and what to call them: a command named with
/// `--cmd`, bacon in a Rust project (when it is installed or a job was named), otherwise the
/// project's own command.
fn watch_source(
    options: &cli::WatchOptions,
    here: &std::path::Path,
) -> Result<(How, std::path::PathBuf, String, String), String> {
    let folder_name = |dir: &std::path::Path| {
        dir.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "project".to_string())
    };
    if let Some(command) = &options.command {
        let dir = runner::detect(here).map_or_else(|| here.to_path_buf(), |d| d.dir);
        return Ok((
            How::Command(command.clone()),
            dir.clone(),
            folder_name(&dir),
            command.clone(),
        ));
    }
    let detected = runner::detect(here);
    let cargo = watch::find_project(here);
    // bacon is used for the nearest project only when that project is the Rust one
    let rust_nearest = match (&cargo, &detected) {
        (Some(cargo), Some(found)) => found.dir == *cargo,
        (Some(_), None) => true,
        _ => false,
    };
    if rust_nearest {
        let cargo = cargo.expect("checked above");
        match sound::find_program("bacon") {
            Some(bacon) => {
                return Ok((
                    How::Bacon(bacon),
                    cargo.clone(),
                    watch::project_name(&cargo),
                    options.job.clone(),
                ));
            }
            None if options.job_given => return Err(watch::bacon_missing()),
            None => {}
        }
    }
    let found = detected.ok_or_else(|| runner::not_detected(here))?;
    let name = if found.kind == "Rust" {
        watch::project_name(&found.dir)
    } else {
        folder_name(&found.dir)
    };
    Ok((
        How::Command(found.command.clone()),
        found.dir,
        name,
        found.command,
    ))
}

fn run_watch(options: cli::WatchOptions, following: bool) -> Result<(), String> {
    let config = load_config()?;
    let resolved = config::resolve(&options.flags, env, &config)?;
    let catalogue = load_catalogue();
    let pass = scene_list(&catalogue, resolved.pass.as_ref(), true)?;
    let fail = scene_list(&catalogue, resolved.fail.as_ref(), false)?;
    let here = std::env::current_dir()
        .map_err(|error| format!("could not read the current folder: {error}"))?;
    let (how, project_dir, project, job) = if following {
        let channel = options
            .channel
            .clone()
            .unwrap_or_else(|| follow::channel(env));
        let dir = follow::log_dir_from(env).ok_or("there is no home folder to keep a log in")?;
        let label = if channel == follow::DEFAULT_CHANNEL {
            "your shell".to_string()
        } else {
            format!("the {channel} session")
        };
        let how = How::Follow {
            log: follow::log_path(&dir, &channel),
            min_seconds: options.min_seconds.unwrap_or(resolved.follow_min_seconds),
        };
        // the status bar reads "following | the dev session", so the channel is said once
        (how, here.clone(), "following".to_string(), label)
    } else {
        watch_source(&options, &here)?
    };
    let mode = how.mode();
    let theme = pick_theme(&resolved)?;
    let calm = resolved.reduced_motion || motion::system_reduced_motion();
    let colour = !std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
    let settings = app::Settings {
        fps: options.fps,
        speed: 1.0,
        calm,
        duration: None,
        colour,
    };
    let report_path = std::env::temp_dir()
        .join(format!("zacess-scenes-watch-{}", std::process::id()))
        .join("report.json");
    let (pass_first, fail_first) = (pass[0].clone(), fail[0].clone());
    let mut state = watch::Watch::new(project.clone(), job.clone())
        .with_scenes(pass, fail, resolved.rotation, resolved.interval)
        .with_mode(mode);
    state.captions = resolved.captions;
    state.banner = resolved.banner;
    let sound = sound::Sound::new(resolved.sound, resolved.volume)
        .with_set(resolved.sound_set)
        .with_events(resolved.events.clone())
        .with_quiet_hours(resolved.quiet_hours);
    let ambient = Ambient::new(resolved.ambient, resolved.ambient_volume)
        .with_quiet_hours(resolved.quiet_hours);
    let mut context = watch::Context {
        theme,
        settings,
        motion: Motion::new(calm, resolved.motion),
        how,
        ambient,
        project_dir,
        report_path: report_path.clone(),
        sound,
        catalogue,
        notify: resolved.notify,
    };
    let mut rng = Rng::from_time();

    let doing = match mode {
        watch::Mode::Bacon => format!("watching {project} with bacon, running the {job} job"),
        watch::Mode::Command => format!("watching {project}, running {job} on each change"),
        watch::Mode::Follow => format!("following {job}, where every command counts"),
    };
    let keys = match mode {
        watch::Mode::Bacon => "q quits, 1 to 4 switch the job, r reruns, ",
        watch::Mode::Command => "q quits, r reruns, ",
        watch::Mode::Follow => "q quits, ",
    };
    let intro = format!(
        "Zacess Scenes is {doing} in {}{}, showing {pass_first} while it passes and {fail_first} \
         while it fails, with the {} sounds{}. {keys}n and b change the scene, m the sounds, + \
         and - the volume, s turns sound on or off, a the background sound and p pauses the \
         animation.",
        theme.name,
        if calm { " with reduced motion" } else { "" },
        resolved.sound_set.name(),
        if resolved.notify {
            " and desktop notifications"
        } else {
            ""
        }
    );
    let depth = options
        .colour_depth
        .unwrap_or_else(|| ColourDepth::detect(env));
    glyphs::set_basic(terminal::limited_font(env));
    let mut terminal = Terminal::open(&intro)
        .map_err(|error| format!("could not take over the terminal: {error}"))?;
    terminal.set_paint(terminal::Paint {
        depth,
        background: theme.background,
        floor: theme.floor(),
    });
    terminal.set_keys(terminal::interpret_watch);
    let result = watch::run(&mut context, &mut state, &mut rng, &mut terminal);
    drop(terminal);
    if let Some(dir) = report_path.parent() {
        let _ = std::fs::remove_dir_all(dir);
    }
    result.map_err(|error| format!("watch stopped: {error}"))
}
