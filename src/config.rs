//! The config file (`scenes.toml` in the platform's config folder) and how its settings combine
//! with the flags, the environment variables and the defaults: a flag wins, then a variable,
//! then the file, then the default.
//!
//! The file is read by a small parser of our own for the part of TOML it needs: tables, bare
//! keys, strings, whole and decimal numbers, booleans and arrays, with comments anywhere a line
//! allows. Pulling in a full TOML library for a dozen settings would more than double the
//! dependencies the program builds.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::scene::Level;
use crate::sound::{self, QuietHours, SoundSet, Tone};

/// The environment variable that points at a config file other than the usual one.
pub const CONFIG_ENV: &str = "ZACESS_CONFIG";
/// The environment variable that names the pass scenes, separated by commas.
pub const SCENE_ENV: &str = "ZACESS_SCENE";
/// The environment variable that names the fail scenes, separated by commas.
pub const FAIL_SCENE_ENV: &str = "ZACESS_FAIL_SCENE";
/// The environment variable that sets the motion level.
pub const MOTION_ENV: &str = "ZACESS_MOTION";
/// The environment variable that turns notifications on or off.
pub const NOTIFY_ENV: &str = "ZACESS_NOTIFY";
/// Turns the ambient sound on or off, overriding the config file.
pub const AMBIENT_ENV: &str = "ZACESS_AMBIENT";
/// The ambient sound's volume when nothing sets it: well under the tones, so a pass or a fail is
/// always heard over it.
pub const AMBIENT_DEFAULT_VOLUME: u8 = 20;

/// A value in the file.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<Value>),
}

impl Value {
    fn kind(&self) -> &'static str {
        match self {
            Value::String(_) => "a string",
            Value::Integer(_) => "a whole number",
            Value::Float(_) => "a number",
            Value::Bool(_) => "true or false",
            Value::Array(_) => "a list",
        }
    }
}

/// A parsed file: every key under its table, as `table.key`, with the line it was on.
pub type Document = BTreeMap<String, (Value, usize)>;

/// Parses the TOML subset. Errors name the line.
pub fn parse_toml(text: &str) -> Result<Document, String> {
    let mut doc = Document::new();
    let mut table = String::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let number = i + 1;
        let line = strip_comment(lines[i]).trim().to_string();
        i += 1;
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            let name = rest
                .strip_suffix(']')
                .ok_or_else(|| format!("line {number}: a table name needs a closing ]"))?
                .trim();
            if name.is_empty() || !name.split('.').all(is_bare_key) {
                return Err(format!("line {number}: \"{name}\" is not a table name"));
            }
            table = name.to_string();
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("line {number}: expected key = value"))?;
        let key = key.trim();
        if !is_bare_key(key) {
            return Err(format!("line {number}: \"{key}\" is not a key"));
        }
        let mut value = value.trim().to_string();
        // an array may run over several lines until its brackets balance
        if value.starts_with('[') {
            while !brackets_closed(&value) {
                let Some(next) = lines.get(i) else {
                    return Err(format!("line {number}: the list is never closed with ]"));
                };
                value.push(' ');
                value.push_str(strip_comment(next).trim());
                i += 1;
            }
        }
        let mut reader = Reader {
            chars: value.chars().collect(),
            at: 0,
            line: number,
        };
        let parsed = reader.value()?;
        reader.skip_space();
        if reader.at < reader.chars.len() {
            return Err(format!("line {number}: unexpected text after the value"));
        }
        let full = if table.is_empty() {
            key.to_string()
        } else {
            format!("{table}.{key}")
        };
        if doc.insert(full.clone(), (parsed, number)).is_some() {
            return Err(format!("line {number}: {full} is set twice"));
        }
    }
    Ok(doc)
}

fn is_bare_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// A line without its comment. A `#` inside a string is kept.
fn strip_comment(line: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (i, c) in line.char_indices() {
        match quote {
            Some('"') if escaped => escaped = false,
            Some('"') if c == '\\' => escaped = true,
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '#' => return &line[..i],
            None => {}
        }
    }
    line
}

/// Whether every `[` outside a string has its `]`.
fn brackets_closed(text: &str) -> bool {
    let mut depth = 0_i32;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for c in text.chars() {
        match quote {
            Some('"') if escaped => escaped = false,
            Some('"') if c == '\\' => escaped = true,
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None => match c {
                '"' | '\'' => quote = Some(c),
                '[' => depth += 1,
                ']' => depth -= 1,
                _ => {}
            },
        }
    }
    depth <= 0
}

struct Reader {
    chars: Vec<char>,
    at: usize,
    line: usize,
}

impl Reader {
    fn error(&self, what: &str) -> String {
        format!("line {}: {what}", self.line)
    }

    fn skip_space(&mut self) {
        while self.chars.get(self.at).is_some_and(|c| c.is_whitespace()) {
            self.at += 1;
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        self.skip_space();
        match self.chars.get(self.at) {
            None => Err(self.error("a value is missing")),
            Some('"') => self.basic_string().map(Value::String),
            Some('\'') => self.literal_string().map(Value::String),
            Some('[') => self.array(),
            Some(_) => self.bare(),
        }
    }

    fn basic_string(&mut self) -> Result<String, String> {
        self.at += 1;
        let mut out = String::new();
        loop {
            let Some(&c) = self.chars.get(self.at) else {
                return Err(self.error("a string is never closed with \""));
            };
            self.at += 1;
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let Some(&e) = self.chars.get(self.at) else {
                        return Err(self.error("a string ends in a lone \\"));
                    };
                    self.at += 1;
                    match e {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'u' => {
                            let hex: String = self.chars.iter().skip(self.at).take(4).collect();
                            let code = u32::from_str_radix(&hex, 16)
                                .ok()
                                .and_then(char::from_u32)
                                .filter(|_| hex.len() == 4)
                                .ok_or_else(|| self.error("\\u needs four hex digits"))?;
                            self.at += 4;
                            out.push(code);
                        }
                        other => return Err(self.error(&format!("unknown escape \\{other}"))),
                    }
                }
                c => out.push(c),
            }
        }
    }

    fn literal_string(&mut self) -> Result<String, String> {
        self.at += 1;
        let start = self.at;
        while let Some(&c) = self.chars.get(self.at) {
            self.at += 1;
            if c == '\'' {
                return Ok(self.chars[start..self.at - 1].iter().collect());
            }
        }
        Err(self.error("a string is never closed with '"))
    }

    fn array(&mut self) -> Result<Value, String> {
        self.at += 1;
        let mut items = Vec::new();
        loop {
            self.skip_space();
            match self.chars.get(self.at) {
                Some(']') => {
                    self.at += 1;
                    return Ok(Value::Array(items));
                }
                None => return Err(self.error("a list is never closed with ]")),
                _ => {}
            }
            items.push(self.value()?);
            self.skip_space();
            match self.chars.get(self.at) {
                Some(',') => self.at += 1,
                Some(']') => {}
                _ => return Err(self.error("list items are separated by commas")),
            }
        }
    }

    fn bare(&mut self) -> Result<Value, String> {
        let start = self.at;
        while self
            .chars
            .get(self.at)
            .is_some_and(|c| !c.is_whitespace() && *c != ',' && *c != ']')
        {
            self.at += 1;
        }
        let word: String = self.chars[start..self.at].iter().collect();
        let digits = word.replace('_', "");
        match word.as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => {
                if let Ok(n) = digits.parse::<i64>() {
                    Ok(Value::Integer(n))
                } else if let Some(f) = digits.parse::<f64>().ok().filter(|f| f.is_finite()) {
                    Ok(Value::Float(f))
                } else {
                    Err(self.error(&format!(
                        "\"{word}\" is not a value; put text in double quotes"
                    )))
                }
            }
        }
    }
}

/// How the scene changes while the build stays in one state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Rotation {
    /// Always the first scene of the list.
    #[default]
    Fixed,
    /// The next scene of the list each time.
    Cycle,
    /// A scene of the list at random, never the same twice running.
    Shuffle,
}

impl Rotation {
    pub fn name(self) -> &'static str {
        match self {
            Rotation::Fixed => "fixed",
            Rotation::Cycle => "cycle",
            Rotation::Shuffle => "shuffle",
        }
    }

    pub fn parse(text: &str) -> Option<Rotation> {
        match text.trim().to_ascii_lowercase().as_str() {
            "fixed" => Some(Rotation::Fixed),
            "cycle" => Some(Rotation::Cycle),
            "shuffle" => Some(Rotation::Shuffle),
            _ => None,
        }
    }
}

/// The settings a config file holds. Anything left out is `None` and falls through to the
/// default.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub theme: Option<String>,
    pub pass: Option<Vec<String>>,
    pub fail: Option<Vec<String>>,
    pub rotation: Option<Rotation>,
    /// Minutes between scene changes while the state stays the same; 0 for only on a new result.
    pub interval: Option<f64>,
    pub motion: Option<Level>,
    pub reduced_motion: Option<bool>,
    pub captions: Option<bool>,
    pub banner: Option<bool>,
    pub sound: Option<bool>,
    pub sound_set: Option<SoundSet>,
    pub volume: Option<u8>,
    pub quiet_hours: Option<QuietHours>,
    /// Per event switches, in the order of `Tone::ALL`.
    pub events: [Option<bool>; Tone::ALL.len()],
    pub notify: Option<bool>,
    pub ambient: Option<bool>,
    pub ambient_volume: Option<u8>,
    /// Seconds a followed command has to run for before its result counts.
    pub follow_min_seconds: Option<f64>,
}

/// Every key the file may hold.
pub const KEYS: [&str; 24] = [
    "scenes.theme",
    "scenes.pass",
    "scenes.fail",
    "scenes.rotation",
    "scenes.interval",
    "scenes.motion",
    "scenes.reduced_motion",
    "scenes.captions",
    "scenes.banner",
    "sound.enabled",
    "sound.set",
    "sound.volume",
    "sound.quiet_hours",
    "sound.events.pass",
    "sound.events.fail",
    "sound.events.tick",
    "sound.events.recovered",
    "sound.events.still_failing",
    "sound.events.warnings",
    "sound.events.typo",
    "notifications.enabled",
    "ambient.enabled",
    "ambient.volume",
    "follow.min_seconds",
];

fn want_string(key: &str, value: &Value, line: usize) -> Result<String, String> {
    match value {
        Value::String(s) => Ok(s.trim().to_string()),
        other => Err(format!(
            "line {line}: {key} should be text in quotes, not {}",
            other.kind()
        )),
    }
}

fn want_bool(key: &str, value: &Value, line: usize) -> Result<bool, String> {
    match value {
        Value::Bool(b) => Ok(*b),
        other => Err(format!(
            "line {line}: {key} should be true or false, not {}",
            other.kind()
        )),
    }
}

fn want_names(key: &str, value: &Value, line: usize) -> Result<Vec<String>, String> {
    let list = match value {
        Value::String(s) => vec![Value::String(s.clone())],
        Value::Array(items) => items.clone(),
        other => {
            return Err(format!(
                "line {line}: {key} should be a list of scene names, not {}",
                other.kind()
            ));
        }
    };
    let names: Vec<String> = list
        .iter()
        .map(|item| want_string(key, item, line).map(|s| normalise_name(&s)))
        .collect::<Result<_, _>>()?;
    if names.is_empty() {
        return Err(format!("line {line}: {key} needs at least one scene"));
    }
    Ok(names)
}

/// A scene name as the registry spells it.
pub fn normalise_name(name: &str) -> String {
    name.trim().to_ascii_lowercase().replace(['_', ' '], "-")
}

/// Splits a comma separated list of scene names, as the flags and variables take them.
pub fn split_names(text: &str) -> Vec<String> {
    text.split(',')
        .map(normalise_name)
        .filter(|s| !s.is_empty())
        .collect()
}

impl Config {
    /// Reads the settings out of a parsed file.
    pub fn from_document(doc: &Document) -> Result<Config, String> {
        let mut config = Config::default();
        for (key, (value, line)) in doc {
            let line = *line;
            match key.as_str() {
                "scenes.theme" => config.theme = Some(want_string(key, value, line)?),
                "scenes.pass" => config.pass = Some(want_names(key, value, line)?),
                "scenes.fail" => config.fail = Some(want_names(key, value, line)?),
                "scenes.rotation" => {
                    let text = want_string(key, value, line)?;
                    config.rotation = Some(Rotation::parse(&text).ok_or_else(|| {
                        format!("line {line}: rotation is fixed, cycle or shuffle, not \"{text}\"")
                    })?);
                }
                "scenes.interval" => {
                    let minutes = match value {
                        Value::Integer(n) => *n as f64,
                        Value::Float(f) => *f,
                        other => {
                            return Err(format!(
                                "line {line}: interval should be a number of minutes, not {}",
                                other.kind()
                            ));
                        }
                    };
                    if minutes < 0.0 {
                        return Err(format!("line {line}: interval cannot be below 0"));
                    }
                    config.interval = Some(minutes);
                }
                "scenes.motion" => {
                    let text = want_string(key, value, line)?;
                    config.motion = Some(Level::parse(&text).ok_or_else(|| {
                        format!("line {line}: motion is calm, standard or lively, not \"{text}\"")
                    })?);
                }
                "scenes.reduced_motion" => {
                    config.reduced_motion = Some(want_bool(key, value, line)?)
                }
                "scenes.captions" => config.captions = Some(want_bool(key, value, line)?),
                "scenes.banner" => config.banner = Some(want_bool(key, value, line)?),
                "sound.enabled" => config.sound = Some(want_bool(key, value, line)?),
                "sound.set" => {
                    let text = want_string(key, value, line)?;
                    config.sound_set = Some(SoundSet::parse(&text).ok_or_else(|| {
                        format!(
                            "line {line}: \"{text}\" is not a sound set. Run zacess-scenes list to see them."
                        )
                    })?);
                }
                "sound.volume" => match value {
                    Value::Integer(n) if (0..=100).contains(n) => config.volume = Some(*n as u8),
                    _ => {
                        return Err(format!(
                            "line {line}: volume should be a whole number from 0 to 100"
                        ));
                    }
                },
                "sound.quiet_hours" => {
                    let text = want_string(key, value, line)?;
                    config.quiet_hours = if text.is_empty() {
                        None
                    } else {
                        Some(QuietHours::parse(&text).ok_or_else(|| {
                            format!(
                                "line {line}: quiet_hours should look like \"22:00-07:00\", not \"{text}\""
                            )
                        })?)
                    };
                }
                "notifications.enabled" => config.notify = Some(want_bool(key, value, line)?),
                "ambient.enabled" => config.ambient = Some(want_bool(key, value, line)?),
                "ambient.volume" => match value {
                    Value::Integer(n) if (0..=100).contains(n) => {
                        config.ambient_volume = Some(*n as u8)
                    }
                    _ => {
                        return Err(format!(
                            "line {line}: ambient volume should be a whole number from 0 to 100"
                        ));
                    }
                },
                "follow.min_seconds" => {
                    let seconds = match value {
                        Value::Integer(n) => *n as f64,
                        Value::Float(f) => *f,
                        other => {
                            return Err(format!(
                                "line {line}: min_seconds should be a number of seconds, not {}",
                                other.kind()
                            ));
                        }
                    };
                    if seconds < 0.0 || !seconds.is_finite() {
                        return Err(format!("line {line}: min_seconds cannot be below 0"));
                    }
                    config.follow_min_seconds = Some(seconds);
                }
                _ => {
                    let event = key
                        .strip_prefix("sound.events.")
                        .and_then(Tone::parse)
                        .and_then(|tone| Tone::ALL.iter().position(|&t| t == tone));
                    match event {
                        Some(i) => config.events[i] = Some(want_bool(key, value, line)?),
                        None => {
                            return Err(format!(
                                "line {line}: {key} is not a setting. The settings are listed in the file zacess-scenes config init writes."
                            ));
                        }
                    }
                }
            }
        }
        Ok(config)
    }

    pub fn parse(text: &str) -> Result<Config, String> {
        Config::from_document(&parse_toml(text)?)
    }

    /// Reads a file. A missing file is an empty config; anything else wrong is an error naming
    /// the file.
    pub fn load(path: &Path) -> Result<Config, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => Config::parse(&text).map_err(|e| format!("{}: {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(format!("could not read {}: {e}", path.display())),
        }
    }
}

/// The folder Zacess settings live in: `$XDG_CONFIG_HOME/zacess` or `~/.config/zacess` on macOS
/// and Linux, `%APPDATA%\zacess` on Windows.
pub fn config_dir_from(var: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let var = |name: &str| var(name).filter(|v| !v.is_empty());
    let base = if cfg!(windows) {
        PathBuf::from(var("APPDATA")?)
    } else if let Some(dir) = var("XDG_CONFIG_HOME") {
        PathBuf::from(dir)
    } else {
        PathBuf::from(var("HOME")?).join(".config")
    };
    Some(base.join("zacess"))
}

/// The config file: `ZACESS_CONFIG` when set, otherwise `scenes.toml` in the config folder.
pub fn config_path_from(var: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if let Some(path) = var(CONFIG_ENV).filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(path));
    }
    config_dir_from(var).map(|dir| dir.join("scenes.toml"))
}

/// The folder user scenes are loaded from: `scenes` beside the config file.
pub fn scenes_dir_from(var: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    config_path_from(var).and_then(|p| p.parent().map(|d| d.join("scenes")))
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

pub fn config_path() -> Option<PathBuf> {
    config_path_from(env)
}

pub fn scenes_dir() -> Option<PathBuf> {
    scenes_dir_from(env)
}

/// What the flags asked for, before the variables, the file and the defaults are looked at.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Flags {
    pub theme: Option<String>,
    pub scene: Option<Vec<String>>,
    pub fail_scene: Option<Vec<String>>,
    pub sounds: Option<SoundSet>,
    pub motion: Option<Level>,
    pub reduced_motion: bool,
    pub notify: Option<bool>,
    pub no_sound: bool,
    pub volume: Option<u8>,
    pub ambient: Option<bool>,
    pub ambient_volume: Option<u8>,
}

/// Every setting after the flags, the variables, the file and the defaults are combined.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    pub theme: Option<String>,
    pub theme_from_env: bool,
    pub pass: Option<Vec<String>>,
    pub fail: Option<Vec<String>>,
    pub rotation: Rotation,
    pub interval: f64,
    pub motion: Level,
    /// Reduced motion asked for by a flag, a variable or the file. The system setting is added
    /// by the caller, since any one of them turns it on.
    pub reduced_motion: bool,
    pub captions: bool,
    pub banner: bool,
    pub sound: bool,
    pub sound_set: SoundSet,
    pub volume: u8,
    pub quiet_hours: Option<QuietHours>,
    pub events: Vec<Tone>,
    pub notify: bool,
    pub ambient: bool,
    pub ambient_volume: u8,
    pub follow_min_seconds: f64,
}

fn env_bool(value: Option<&str>) -> Option<bool> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// Combines the sources: a flag wins, then a variable, then the file, then the default. A bad
/// value in a variable is an error naming the variable.
pub fn resolve(
    flags: &Flags,
    var: impl Fn(&str) -> Option<String>,
    config: &Config,
) -> Result<Resolved, String> {
    let var = |name: &str| var(name).filter(|v| !v.trim().is_empty());

    let env_theme = var(crate::theme::ENV_VAR);
    let theme = flags
        .theme
        .clone()
        .or(env_theme.clone())
        .or(config.theme.clone());
    let theme_from_env = flags.theme.is_none() && env_theme.is_some();

    let pass = flags
        .scene
        .clone()
        .or_else(|| var(SCENE_ENV).map(|v| split_names(&v)))
        .or(config.pass.clone());
    let fail = flags
        .fail_scene
        .clone()
        .or_else(|| var(FAIL_SCENE_ENV).map(|v| split_names(&v)))
        .or(config.fail.clone());

    let motion = match (flags.motion, var(MOTION_ENV)) {
        (Some(level), _) => level,
        (None, Some(text)) => Level::parse(&text).ok_or_else(|| {
            format!("{MOTION_ENV} is \"{text}\", which is not calm, standard or lively.")
        })?,
        (None, None) => config.motion.unwrap_or_default(),
    };

    let env_reduced = var(crate::motion::ENV_VAR);
    let reduced_motion = flags.reduced_motion
        || crate::motion::env_wants_reduced(env_reduced.as_deref())
        || config.reduced_motion == Some(true);

    let env_sound = var(sound::SOUND_ENV);
    let sound_on = if flags.no_sound {
        false
    } else if let Some(on) = env_bool(env_sound.as_deref()) {
        on
    } else {
        config.sound.unwrap_or(true)
    };

    let sound_set = match (flags.sounds, var(sound::SET_ENV)) {
        (Some(set), _) => set,
        (None, Some(text)) => SoundSet::parse(&text).ok_or_else(|| {
            format!(
                "{} is \"{text}\", which is not a sound set. Run zacess-scenes list to see them.",
                sound::SET_ENV
            )
        })?,
        (None, None) => config.sound_set.unwrap_or(SoundSet::Bells),
    };

    let volume = match (flags.volume, var(sound::VOLUME_ENV)) {
        (Some(volume), _) => volume,
        (None, Some(text)) => sound::parse_volume(&text).ok_or_else(|| {
            format!(
                "{} is \"{text}\", which is not a whole number from 0 to 100.",
                sound::VOLUME_ENV
            )
        })?,
        (None, None) => config.volume.unwrap_or(sound::DEFAULT_VOLUME),
    };

    let notify = match (flags.notify, var(NOTIFY_ENV)) {
        (Some(on), _) => on,
        (None, Some(text)) => env_bool(Some(&text)).ok_or_else(|| {
            format!("{NOTIFY_ENV} is \"{text}\"; use 1 or 0, true or false, on or off.")
        })?,
        (None, None) => config.notify.unwrap_or(false),
    };

    let ambient = match (flags.ambient, var(AMBIENT_ENV)) {
        (Some(on), _) => on,
        (None, Some(text)) => env_bool(Some(&text)).ok_or_else(|| {
            format!("{AMBIENT_ENV} is \"{text}\"; use 1 or 0, true or false, on or off.")
        })?,
        (None, None) => config.ambient.unwrap_or(false),
    };

    let events = Tone::ALL
        .iter()
        .zip(&config.events)
        .filter(|(tone, on)| on.unwrap_or(tone.on_by_default()))
        .map(|(tone, _)| *tone)
        .collect();

    Ok(Resolved {
        theme,
        theme_from_env,
        pass,
        fail,
        rotation: config.rotation.unwrap_or_default(),
        interval: config.interval.unwrap_or(0.0),
        motion,
        reduced_motion,
        captions: config.captions.unwrap_or(true),
        banner: config.banner.unwrap_or(true),
        sound: sound_on,
        sound_set,
        volume,
        quiet_hours: config.quiet_hours,
        events,
        notify,
        // sound off turns the ambient sound off too, so one switch silences everything
        ambient: ambient && sound_on,
        ambient_volume: flags
            .ambient_volume
            .or(config.ambient_volume)
            .unwrap_or(AMBIENT_DEFAULT_VOLUME),
        follow_min_seconds: config.follow_min_seconds.unwrap_or(0.0),
    })
}

/// The starter file `config init` writes. Every setting is there, commented out at its default,
/// so the file changes nothing until a line is uncommented.
pub fn starter() -> String {
    let sets: Vec<&str> = SoundSet::ALL.iter().map(|s| s.name()).collect();
    format!(
        r#"# Zacess Scenes settings.
#
# Every setting below is commented out and shows its default. Remove the # in front of a line to
# change it. A flag on the command line wins over an environment variable, which wins over this
# file, which wins over the default. Run zacess-scenes list to see every scene and sound set.

[scenes]
# The Zacess theme, by id, short name or full name. Without it the theme comes from the theme
# state file, otherwise Zacess Deep Contrast Dark.
# theme = "deep-dark"

# The scenes the watch mode shows while a build passes and while it fails. Pass and general
# scenes can go in the pass list; fail scenes in the fail list. Your own Lua scenes from the
# scenes folder beside this file can go in either. Without these the lists are every pass scene
# starting with waves and every fail scene starting with storm.
# pass = ["waves", "koi-pond", "lanterns", "city-night"]
# fail = ["storm", "flood", "fog"]

# How the scene changes: fixed keeps the first scene of the list, cycle moves through the list
# and shuffle picks at random. The scene changes on each new result and, when interval is above
# 0, every that many minutes while the result stays the same. A scene run on its own with no
# name moves through the pass list the same way, so shuffle and an interval turn it into a
# slideshow.
# rotation = "fixed"
# interval = 0

# How busy and quick the scenes are: calm, standard or lively. Lively never flashes either.
# motion = "standard"

# Slower movement, fewer things moving and no lightning. The system's reduce motion setting
# turns this on too.
# reduced_motion = false

# The short line under the banner and the large ALL GOOD or BUILD FAILED banner. With the
# banner off the state is still written on one line.
# captions = true
# banner = true

[sound]
# Tones for the watch mode. The screen always shows the same state, so sound is never the only
# signal.
# enabled = true

# The sound set: {sets}.
# set = "bells"

# From 0 to 100. The + and - keys change it while the watch mode runs.
# volume = {volume}

# No sound between two times of day, such as "22:00-07:00".
# quiet_hours = ""

[sound.events]
# Which events make a sound.
# pass = true
# fail = true
# tick = true
# recovered = true
# still_failing = false
# warnings = true
# typo = true

[notifications]
# A desktop notification when the build changes between passing and failing, which a screen
# reader can announce. Off unless turned on here, with --notify or with ZACESS_NOTIFY=1.
# enabled = false

[ambient]
# A quiet background sound that suits the scene: water, rain, wind, fire or a night meadow. It
# plays under the tones and stops for a moment while one plays. a turns it on or off and [ and ]
# change its volume while a scene runs. Sound off and quiet hours silence it too.
# enabled = false
# volume = {ambient_volume}

[follow]
# zacess-scenes follow shows how each command in a shell ended. A command that finishes faster
# than this many seconds is not counted; 0 counts every command, typos included.
# min_seconds = 0
"#,
        sets = sets.join(", "),
        volume = sound::DEFAULT_VOLUME,
        ambient_volume = AMBIENT_DEFAULT_VOLUME,
    )
}

/// Writes the starter file, refusing to replace one that exists unless `force` is set.
pub fn write_starter(path: &Path, force: bool) -> Result<(), String> {
    if path.exists() && !force {
        return Err(format!(
            "{} already exists. Run zacess-scenes config init --force to replace it.",
            path.display()
        ));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    }
    std::fs::write(path, starter()).map_err(|e| format!("could not write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn the_parser_reads_every_value_kind() {
        let doc = parse_toml(
            "top = 1\n# a comment\n[a]\ns = \"x # not a comment \\\"q\\\" \\u00e9\" # comment\n\
             l = 'C:\\path'\nf = 2.5\nn = -3\nb = false\nlist = [\n  \"one\", # first\n  'two',\n]\n\
             [a.b-c]\nk = []\n",
        )
        .unwrap();
        assert_eq!(doc["top"].0, Value::Integer(1));
        assert_eq!(
            doc["a.s"].0,
            Value::String("x # not a comment \"q\" \u{e9}".into())
        );
        assert_eq!(doc["a.l"].0, Value::String("C:\\path".into()));
        assert_eq!(doc["a.f"].0, Value::Float(2.5));
        assert_eq!(doc["a.n"].0, Value::Integer(-3));
        assert_eq!(doc["a.b"].0, Value::Bool(false));
        assert_eq!(
            doc["a.list"].0,
            Value::Array(vec![
                Value::String("one".into()),
                Value::String("two".into())
            ])
        );
        assert_eq!(doc["a.b-c.k"], (Value::Array(vec![]), 14));
    }

    #[test]
    fn parser_errors_name_the_line() {
        for (text, line) in [
            ("a = 1\nb", "line 2"),
            ("[open", "line 1"),
            ("a = \"never closed", "line 1"),
            ("a = [1, 2", "line 1"),
            ("a = 1\na = 2", "line 2"),
            ("a = bare words", "line 1"),
            ("a = \"x\" y", "line 1"),
            ("a = [1 2]", "line 1"),
            ("bad key = 1", "line 1"),
            ("a = \"\\q\"", "line 1"),
            ("a =", "line 1"),
        ] {
            let error = parse_toml(text).unwrap_err();
            assert!(error.starts_with(line), "{text:?}: {error}");
        }
    }

    #[test]
    fn the_starter_parses_to_an_empty_config_and_names_every_key() {
        let config = Config::parse(&starter()).unwrap();
        assert_eq!(config, Config::default());
        let text = starter();
        for key in KEYS {
            let last = key.rsplit('.').next().unwrap();
            assert!(text.contains(&format!("# {last} = ")), "{key}");
        }
        // uncommenting every setting still parses
        let all: String = text
            .lines()
            .map(|l| {
                let t = l.trim_start_matches("# ");
                if t.contains(" = ") && !t.contains("quiet_hours") {
                    t.to_string()
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let config = Config::parse(&all).unwrap();
        assert_eq!(config.pass.as_ref().unwrap()[0], "waves");
        assert_eq!(config.events[4], Some(false));
        assert_eq!(config.rotation, Some(Rotation::Fixed));
    }

    #[test]
    fn a_full_config_is_read() {
        let config = Config::parse(
            "[scenes]\ntheme = \"cream\"\npass = [\"Koi_Pond\", \"waves\"]\nfail = \"flood\"\n\
             rotation = \"shuffle\"\ninterval = 2.5\nmotion = \"lively\"\nreduced_motion = true\n\
             captions = false\nbanner = false\n[sound]\nenabled = false\nset = \"marimba\"\n\
             volume = 20\nquiet_hours = \"22:00-07:00\"\n[sound.events]\nstill_failing = true\n\
             tick = false\n[notifications]\nenabled = true\n",
        )
        .unwrap();
        assert_eq!(config.theme.as_deref(), Some("cream"));
        assert_eq!(config.pass, Some(vec!["koi-pond".into(), "waves".into()]));
        assert_eq!(config.fail, Some(vec!["flood".into()]));
        assert_eq!(config.rotation, Some(Rotation::Shuffle));
        assert_eq!(config.interval, Some(2.5));
        assert_eq!(config.motion, Some(Level::Lively));
        assert_eq!(config.reduced_motion, Some(true));
        assert_eq!((config.captions, config.banner), (Some(false), Some(false)));
        assert_eq!(config.sound, Some(false));
        assert_eq!(config.sound_set, Some(SoundSet::Marimba));
        assert_eq!(config.volume, Some(20));
        assert_eq!(config.quiet_hours, QuietHours::parse("22:00-07:00"));
        assert_eq!(config.events[2], Some(false));
        assert_eq!(config.events[4], Some(true));
        assert_eq!(config.notify, Some(true));
    }

    #[test]
    fn bad_settings_say_what_is_wanted() {
        for (text, needle) in [
            ("[scenes]\nrotation = \"spin\"", "fixed, cycle or shuffle"),
            ("[scenes]\nmotion = \"fast\"", "calm, standard or lively"),
            ("[scenes]\ncaptions = \"yes\"", "true or false"),
            ("[sound]\nvolume = 101", "0 to 100"),
            ("[sound]\nset = \"tuba\"", "not a sound set"),
            ("[sound]\nquiet_hours = \"late\"", "22:00-07:00"),
            ("[scenes]\npass = []", "at least one"),
            ("[scenes]\ninterval = -1", "below 0"),
            ("[sound.events]\nboom = true", "not a setting"),
            ("[scenes]\nscene = \"sea\"", "not a setting"),
        ] {
            let error = Config::parse(text).unwrap_err();
            assert!(error.contains(needle), "{text:?}: {error}");
            assert!(error.starts_with("line 2"), "{error}");
        }
    }

    #[test]
    fn a_missing_file_is_an_empty_config() {
        let path = std::env::temp_dir().join("zacess-scenes-no-such-config.toml");
        assert_eq!(Config::load(&path).unwrap(), Config::default());
    }

    #[test]
    fn the_paths_follow_the_platform() {
        let var = |name: &str| match name {
            "HOME" => Some("/home/a".to_string()),
            "APPDATA" => Some("C:\\Users\\a\\AppData\\Roaming".to_string()),
            _ => None,
        };
        let path = config_path_from(var).unwrap();
        if cfg!(windows) {
            assert!(path.starts_with("C:\\Users\\a\\AppData\\Roaming"));
        } else {
            assert_eq!(path, PathBuf::from("/home/a/.config/zacess/scenes.toml"));
            let xdg = |name: &str| (name == "XDG_CONFIG_HOME").then(|| "/x".to_string());
            assert_eq!(
                config_path_from(xdg).unwrap(),
                PathBuf::from("/x/zacess/scenes.toml")
            );
            assert_eq!(
                scenes_dir_from(var).unwrap(),
                PathBuf::from("/home/a/.config/zacess/scenes")
            );
        }
        let custom = |name: &str| (name == CONFIG_ENV).then(|| "/tmp/z.toml".to_string());
        assert_eq!(
            config_path_from(custom).unwrap(),
            PathBuf::from("/tmp/z.toml")
        );
        assert_eq!(config_path_from(none), None);
    }

    #[test]
    fn a_flag_beats_a_variable_which_beats_the_file_which_beats_the_default() {
        let file = Config {
            pass: Some(vec!["tea".into()]),
            motion: Some(Level::Calm),
            sound_set: Some(SoundSet::Pad),
            volume: Some(20),
            notify: Some(true),
            sound: Some(false),
            ..Config::default()
        };
        let defaults = resolve(&Flags::default(), none, &Config::default()).unwrap();
        assert_eq!(defaults.motion, Level::Standard);
        assert_eq!(defaults.sound_set, SoundSet::Bells);
        assert_eq!(defaults.volume, sound::DEFAULT_VOLUME);
        assert!(!defaults.notify && defaults.sound && defaults.captions && defaults.banner);
        assert_eq!(defaults.pass, None);
        assert!(!defaults.events.contains(&Tone::StillFailing));

        let from_file = resolve(&Flags::default(), none, &file).unwrap();
        assert_eq!(from_file.pass, Some(vec!["tea".into()]));
        assert_eq!(from_file.motion, Level::Calm);
        assert_eq!(from_file.sound_set, SoundSet::Pad);
        assert!(from_file.notify && !from_file.sound);

        let env = |name: &str| match name {
            SCENE_ENV => Some("koi-pond, Lanterns".to_string()),
            MOTION_ENV => Some("lively".to_string()),
            "ZACESS_SOUNDS" => Some("chip".to_string()),
            "ZACESS_VOLUME" => Some("70".to_string()),
            "ZACESS_SOUND" => Some("1".to_string()),
            NOTIFY_ENV => Some("off".to_string()),
            _ => None,
        };
        let from_env = resolve(&Flags::default(), env, &file).unwrap();
        assert_eq!(
            from_env.pass,
            Some(vec!["koi-pond".into(), "lanterns".into()])
        );
        assert_eq!(from_env.motion, Level::Lively);
        assert_eq!(from_env.sound_set, SoundSet::Chip);
        assert_eq!(from_env.volume, 70);
        assert!(from_env.sound && !from_env.notify);

        let flags = Flags {
            scene: Some(vec!["orbit".into()]),
            motion: Some(Level::Standard),
            sounds: Some(SoundSet::Glass),
            volume: Some(5),
            notify: Some(true),
            no_sound: true,
            ..Flags::default()
        };
        let from_flags = resolve(&flags, env, &file).unwrap();
        assert_eq!(from_flags.pass, Some(vec!["orbit".into()]));
        assert_eq!(from_flags.motion, Level::Standard);
        assert_eq!(from_flags.sound_set, SoundSet::Glass);
        assert_eq!(from_flags.volume, 5);
        assert!(from_flags.notify && !from_flags.sound);
    }

    #[test]
    fn any_source_turns_reduced_motion_on() {
        let on = Config {
            reduced_motion: Some(true),
            ..Config::default()
        };
        assert!(
            resolve(&Flags::default(), none, &on)
                .unwrap()
                .reduced_motion
        );
        let env = |name: &str| (name == "ZACESS_REDUCED_MOTION").then(|| "1".to_string());
        assert!(
            resolve(&Flags::default(), env, &Config::default())
                .unwrap()
                .reduced_motion
        );
        assert!(
            !resolve(&Flags::default(), none, &Config::default())
                .unwrap()
                .reduced_motion
        );
    }

    #[test]
    fn bad_variables_are_errors_naming_them() {
        for (name, value) in [
            (MOTION_ENV, "fast"),
            ("ZACESS_SOUNDS", "tuba"),
            ("ZACESS_VOLUME", "loud"),
            (NOTIFY_ENV, "maybe"),
        ] {
            let var = |n: &str| (n == name).then(|| value.to_string());
            let error = resolve(&Flags::default(), var, &Config::default()).unwrap_err();
            assert!(error.contains(name), "{error}");
        }
    }

    #[test]
    fn the_starter_is_written_once_unless_forced() {
        let dir = std::env::temp_dir().join(format!("zacess-config-{}", std::process::id()));
        let path = dir.join("zacess").join("scenes.toml");
        write_starter(&path, false).unwrap();
        assert!(write_starter(&path, false).unwrap_err().contains("--force"));
        std::fs::write(&path, "changed").unwrap();
        write_starter(&path, true).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), starter());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
