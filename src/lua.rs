//! Scenes written in Lua. A `.lua` file in the user's scenes folder returns a table that names
//! the scene and gives a `frame` function, which draws with palette role names rather than
//! colours. Lua 5.4 is compiled into the program, so nothing has to be installed.
//!
//! A script runs in a sandbox: no file, process, network or module access, a cap on memory and
//! a budget of instructions for every call, so a runaway loop ends in an error rather than a
//! frozen terminal. Every frame it draws passes through the same flash meter the built-in scenes
//! are tested against; a frame that would break the limit is held back and a script that keeps
//! trying is stopped. Any error stops the script and is reported through `Scene::problem`, so the
//! caller can swap in a built-in scene.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use mlua::{
    ChunkMode, Function, HookTriggers, Lua, LuaOptions, StdLib, Table, Value, Variadic, VmState,
};

use crate::colour::Rgb;
use crate::flash::{Meter, Verdict};
use crate::frame::Frame;
use crate::rng::Rng;
use crate::scene::{Level, Moment, Motion, Scene};
use crate::scenes::{self, Kind};
use crate::theme::ThemeData;
use crate::watch::view::{error_colour, warning_colour};

/// Instructions a script may run while it is loaded and in `init`.
pub const INIT_BUDGET: i64 = 20_000_000;
/// Instructions a script may run in one call of `frame`, `resize` or a hook. A frame at 20 a
/// second with this many would still be far beyond any sensible scene.
pub const FRAME_BUDGET: i64 = 2_000_000;
/// The memory a script's Lua state may use, in bytes.
pub const MEMORY_LIMIT: usize = 32 * 1024 * 1024;
/// Frames held back by the flash meter within ten seconds before the script is stopped.
pub const MAX_HELD: usize = 10;
/// The most `put` calls one frame may make, so a script cannot grow the draw list without end.
const MAX_PUTS: usize = 20_000;
/// The longest text one `put` call draws.
const MAX_PUT_CHARS: usize = 1_000;
/// How often the budget hook runs, in instructions. A larger step costs less and still stops a
/// loop within a millisecond or so.
const HOOK_STEP: u32 = 1_000;
/// The longest caption a hook may return.
const MAX_CAPTION_CHARS: usize = 60;

/// A colour a script names. Every one maps to a theme role, so a script can never pick a colour
/// that misses the theme's contrast floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Text,
    Muted,
    Accent,
    Link,
    Error,
    Warning,
    Info,
    Bright,
    Grey,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    Syntax(usize),
}

/// Every role name a script may use, in the order the docs list them.
pub const ROLE_NAMES: [&str; 30] = [
    "text",
    "muted",
    "accent",
    "link",
    "error",
    "warning",
    "info",
    "bright",
    "grey",
    "red",
    "green",
    "yellow",
    "blue",
    "magenta",
    "cyan",
    "bright_red",
    "bright_green",
    "bright_yellow",
    "bright_blue",
    "bright_magenta",
    "bright_cyan",
    "syntax.keyword",
    "syntax.control",
    "syntax.function",
    "syntax.type",
    "syntax.string",
    "syntax.number",
    "syntax.comment",
    "syntax.property",
    "syntax.regex",
];

impl Role {
    /// The role for a name, ignoring case and allowing `gray` and `foreground` as well.
    pub fn parse(name: &str) -> Option<Role> {
        let name = name.trim().to_ascii_lowercase().replace('-', "_");
        let role = match name.as_str() {
            "text" | "foreground" => Role::Text,
            "muted" => Role::Muted,
            "accent" => Role::Accent,
            "link" => Role::Link,
            "error" => Role::Error,
            "warning" => Role::Warning,
            "info" => Role::Info,
            "bright" => Role::Bright,
            "grey" | "gray" => Role::Grey,
            "red" => Role::Red,
            "green" => Role::Green,
            "yellow" => Role::Yellow,
            "blue" => Role::Blue,
            "magenta" => Role::Magenta,
            "cyan" => Role::Cyan,
            "bright_red" => Role::BrightRed,
            "bright_green" => Role::BrightGreen,
            "bright_yellow" => Role::BrightYellow,
            "bright_blue" => Role::BrightBlue,
            "bright_magenta" => Role::BrightMagenta,
            "bright_cyan" => Role::BrightCyan,
            other => {
                let syntax = other.strip_prefix("syntax.")?;
                let index = [
                    "keyword", "control", "function", "type", "string", "number", "comment",
                    "property", "regex",
                ]
                .iter()
                .position(|n| *n == syntax)?;
                Role::Syntax(index)
            }
        };
        Some(role)
    }

    /// The role's colour in a theme. A colour that would miss the floor, which no role in the
    /// current palette does, falls back to the text colour.
    pub fn colour(self, theme: &ThemeData) -> Rgb {
        let wanted = match self {
            Role::Text => theme.foreground,
            Role::Muted => theme.muted,
            Role::Accent => theme.accent,
            Role::Link => theme.link,
            Role::Error => error_colour(theme),
            Role::Warning => warning_colour(theme),
            Role::Info => theme.info,
            Role::Bright => theme.bright(),
            Role::Grey => theme.grey(),
            Role::Red => theme.red(),
            Role::Green => theme.green(),
            Role::Yellow => theme.yellow(),
            Role::Blue => theme.blue(),
            Role::Magenta => theme.magenta(),
            Role::Cyan => theme.cyan(),
            Role::BrightRed => theme.bright_red(),
            Role::BrightGreen => theme.bright_green(),
            Role::BrightYellow => theme.bright_yellow(),
            Role::BrightBlue => theme.bright_blue(),
            Role::BrightMagenta => theme.bright_magenta(),
            Role::BrightCyan => theme.bright_cyan(),
            Role::Syntax(i) => theme.syntax[i.min(8)],
        };
        if theme.contrast_of(wanted) >= theme.floor() {
            wanted
        } else {
            theme.foreground
        }
    }
}

/// Whether a character is safe in every terminal font the program supports, the same set the
/// built-in scenes are tested against. Anything else a script draws becomes `?`.
pub fn safe_glyph(ch: char) -> bool {
    (' '..='~').contains(&ch)
        || ch == '\u{b7}'
        || ('\u{2580}'..='\u{259f}').contains(&ch)
        || "\u{2500}\u{2502}\u{250c}\u{2510}\u{2514}\u{2518}\u{251c}\u{2524}\u{252c}\u{2534}\u{253c}\u{25e3}\u{25e5}"
            .contains(ch)
}

/// One `put` call, kept as a role so the same frame can be drawn in any theme.
#[derive(Clone, Debug, PartialEq)]
struct Put {
    x: i32,
    y: i32,
    text: String,
    role: Role,
}

/// A user scene found on disk: what the listing and the config need, plus its source.
#[derive(Clone, Debug, PartialEq)]
pub struct UserScene {
    pub name: String,
    pub summary: String,
    pub kind: Kind,
    pub captions: Vec<String>,
    pub path: PathBuf,
    source: String,
}

impl UserScene {
    /// A scene from source text, for the tests and for the starter check.
    pub fn from_source(path: &Path, source: &str) -> Result<UserScene, String> {
        read_metadata(path, source)
    }
}

/// A name as the command line spells it: lower case with hyphens.
fn normalise(name: &str) -> String {
    name.trim().to_ascii_lowercase().replace(['_', ' '], "-")
}

/// The file name a script's errors are reported against.
fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "scene.lua".to_string())
}

/// The first line of an error, without mlua's own prefixes and tracebacks, so it fits the status
/// bar.
fn short_error(error: &mlua::Error) -> String {
    let mut error = error;
    while let mlua::Error::CallbackError { cause, .. } = error {
        error = cause;
    }
    let text = match error {
        mlua::Error::RuntimeError(message) | mlua::Error::SyntaxError { message, .. } => {
            message.clone()
        }
        mlua::Error::MemoryError(_) => "used more than its memory limit".to_string(),
        other => other.to_string(),
    };
    let line = text.lines().next().unwrap_or("").trim();
    let line = line.strip_prefix("runtime error: ").unwrap_or(line);
    if line.is_empty() {
        "an unknown error".to_string()
    } else {
        line.to_string()
    }
}

/// A Lua state with the sandbox in place and the shared budget counter its hook reads.
struct Sandbox {
    lua: Lua,
    remaining: Arc<AtomicI64>,
}

impl Sandbox {
    fn new() -> mlua::Result<Sandbox> {
        // only the libraries that touch nothing outside the state: no io, os, package, debug
        // or coroutine, whose threads would need hooks of their own
        let lua = Lua::new_with(
            StdLib::STRING | StdLib::TABLE | StdLib::MATH | StdLib::UTF8,
            LuaOptions::default(),
        )?;
        let globals = lua.globals();
        for name in [
            "dofile",
            "loadfile",
            "load",
            "loadstring",
            "require",
            "collectgarbage",
            "print",
            "io",
            "os",
            "package",
            "debug",
            "coroutine",
        ] {
            globals.raw_set(name, Value::Nil)?;
        }
        if let Ok(string) = globals.get::<Table>("string") {
            // a dumped function is a binary chunk; nothing can load one, but it has no use here
            string.raw_set("dump", Value::Nil)?;
        }
        lua.set_memory_limit(MEMORY_LIMIT)?;
        let remaining = Arc::new(AtomicI64::new(INIT_BUDGET / i64::from(HOOK_STEP)));
        let counter = Arc::clone(&remaining);
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(HOOK_STEP),
            move |_, _| {
                if counter.fetch_sub(1, Ordering::Relaxed) <= 0 {
                    Err(mlua::Error::runtime(
                        "ran past its instruction budget, which usually means an endless loop",
                    ))
                } else {
                    Ok(VmState::Continue)
                }
            },
        )?;
        Ok(Sandbox { lua, remaining })
    }

    /// Gives the next call a fresh budget.
    fn budget(&self, instructions: i64) {
        self.remaining
            .store(instructions / i64::from(HOOK_STEP), Ordering::Relaxed);
    }

    /// Runs a script's text and returns the table it gives back.
    fn run(&self, path: &Path, source: &str) -> mlua::Result<Table> {
        self.budget(INIT_BUDGET);
        let value: Value = self
            .lua
            .load(source)
            .set_name(format!("={}", file_label(path)))
            .set_mode(ChunkMode::Text)
            .eval()?;
        match value {
            Value::Table(table) => Ok(table),
            _ => Err(mlua::Error::runtime(
                "the script must end by returning a table, such as return { frame = ... }",
            )),
        }
    }
}

/// Reads a script's name, kind, summary and captions.
fn read_metadata(path: &Path, source: &str) -> Result<UserScene, String> {
    let label = file_label(path);
    let fail = |message: String| format!("{label}: {message}");
    let sandbox = Sandbox::new().map_err(|e| fail(short_error(&e)))?;
    let table = sandbox
        .run(path, source)
        .map_err(|e| fail(short_error(&e)))?;
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name = match table.get::<Value>("name") {
        Ok(Value::String(s)) => normalise(&s.to_string_lossy()),
        Ok(Value::Nil) => normalise(&stem),
        _ => return Err(fail("name must be a string".to_string())),
    };
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(fail(format!(
            "\"{name}\" is not a usable scene name: use letters, digits and hyphens"
        )));
    }
    if scenes::info(&name).is_some() {
        return Err(fail(format!(
            "\"{name}\" is the name of a built-in scene, so this scene needs another name"
        )));
    }
    let kind = match table.get::<Value>("kind") {
        Ok(Value::String(s)) => {
            let text = s.to_string_lossy();
            Kind::parse(&text).ok_or_else(|| {
                fail(format!(
                    "kind is \"{text}\" but must be \"pass\", \"fail\" or \"general\""
                ))
            })?
        }
        Ok(Value::Nil) => Kind::General,
        _ => return Err(fail("kind must be a string".to_string())),
    };
    let summary = match table.get::<Value>("summary") {
        Ok(Value::String(s)) => s.to_string_lossy().trim().to_string(),
        _ => format!("A scene of your own from {label}"),
    };
    let mut captions = Vec::new();
    if let Ok(Value::Table(list)) = table.get::<Value>("captions") {
        for caption in list.sequence_values::<Value>() {
            match caption {
                Ok(Value::String(s)) => {
                    let text: String = s
                        .to_string_lossy()
                        .trim()
                        .chars()
                        .take(MAX_CAPTION_CHARS)
                        .collect();
                    if !text.is_empty() {
                        captions.push(text);
                    }
                }
                _ => return Err(fail("captions must be a list of strings".to_string())),
            }
        }
    }
    match table.get::<Value>("frame") {
        Ok(Value::Function(_)) => {}
        _ => return Err(fail("the table needs a frame function".to_string())),
    }
    Ok(UserScene {
        name,
        summary,
        kind,
        captions,
        path: path.to_path_buf(),
        source: source.to_string(),
    })
}

/// Loads one script.
pub fn load_file(path: &Path) -> Result<UserScene, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("{}: could not be read: {e}", file_label(path)))?;
    read_metadata(path, &source)
}

/// Loads every `.lua` file in a folder, in name order. A folder that does not exist has no
/// scenes and no errors. A script that fails to load or reuses a name is reported and left out.
pub fn load_dir(dir: &Path) -> (Vec<UserScene>, Vec<String>) {
    let mut found = Vec::new();
    let mut errors = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (found, errors);
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "lua"))
        .collect();
    paths.sort();
    for path in paths {
        match load_file(&path) {
            Ok(scene) => {
                if found.iter().any(|s: &UserScene| s.name == scene.name) {
                    errors.push(format!(
                        "{}: another script already uses the name \"{}\"",
                        file_label(&path),
                        scene.name
                    ));
                } else {
                    found.push(scene);
                }
            }
            Err(error) => errors.push(error),
        }
    }
    (found, errors)
}

/// A user scene's name with a static lifetime, as `Scene::name` needs. Each name is kept once,
/// however many times its scene is built.
fn static_name(name: &str) -> &'static str {
    use std::sync::Mutex;
    static NAMES: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut names = NAMES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(found) = names.iter().find(|n| **n == name) {
        return found;
    }
    let leaked: &'static str = Box::leak(name.to_string().into_boxed_str());
    names.push(leaked);
    leaked
}

/// A random number from the scene's own generator, in the forms `math.random` takes.
fn random_value(rng: &mut Rng, args: &Variadic<f64>) -> mlua::Result<f64> {
    match args.len() {
        0 => Ok(f64::from(rng.unit())),
        1 | 2 => {
            let (low, high) = if args.len() == 1 {
                (1.0, args[0].floor())
            } else {
                (args[0].floor(), args[1].floor())
            };
            if high < low {
                return Err(mlua::Error::runtime("random: the interval is empty"));
            }
            let span = (high - low + 1.0).min(u64::MAX as f64) as u64;
            Ok(low + (rng.next_u64() % span.max(1)) as f64)
        }
        _ => Err(mlua::Error::runtime("random takes at most two numbers")),
    }
}

/// A scene driven by a script.
pub struct LuaScene {
    name: &'static str,
    sandbox: Sandbox,
    module: Table,
    ctx: Table,
    puts: Rc<RefCell<Vec<Put>>>,
    width: usize,
    height: usize,
    t: f64,
    /// Counts the frames the script has drawn, so the meter judges each one once.
    generation: u64,
    /// Set once the script has failed; it is never called again after that.
    problem: RefCell<Option<String>>,
    meter: RefCell<Meter>,
    /// The last frame the meter let through, drawn again while a frame is held back.
    shown: RefCell<Vec<Put>>,
    /// The generation of the frame the meter last judged.
    judged: Cell<Option<u64>>,
    scratch: RefCell<Frame>,
}

impl LuaScene {
    fn build(
        user: &UserScene,
        width: usize,
        height: usize,
        motion: Motion,
        rng: &mut Rng,
    ) -> mlua::Result<LuaScene> {
        let sandbox = Sandbox::new()?;
        let module = sandbox.run(&user.path, &user.source)?;
        let lua = &sandbox.lua;
        let puts: Rc<RefCell<Vec<Put>>> = Rc::new(RefCell::new(Vec::new()));
        let scene_rng = Rc::new(RefCell::new(Rng::new(rng.next_u64())));

        let sink = Rc::clone(&puts);
        let put = lua.create_function(
            move |_, (x, y, text, role): (f64, f64, Value, Option<String>)| {
                let role = match role {
                    None => Role::Text,
                    Some(name) => Role::parse(&name).ok_or_else(|| {
                        mlua::Error::runtime(format!(
                            "\"{name}\" is not a palette role; use one such as \"text\", \
                             \"accent\" or \"syntax.keyword\""
                        ))
                    })?,
                };
                let text = match text {
                    Value::String(s) => s.to_string_lossy(),
                    Value::Integer(i) => i.to_string(),
                    Value::Number(n) => n.to_string(),
                    _ => return Err(mlua::Error::runtime("put needs text to draw")),
                };
                let text: String = text
                    .chars()
                    .take(MAX_PUT_CHARS)
                    .map(|c| if safe_glyph(c) { c } else { '?' })
                    .collect();
                let mut list = sink.borrow_mut();
                if list.len() >= MAX_PUTS {
                    return Err(mlua::Error::runtime(format!(
                        "more than {MAX_PUTS} put calls in one frame"
                    )));
                }
                if x.is_finite() && y.is_finite() {
                    list.push(Put {
                        x: x.floor().clamp(-1e6, 1e6) as i32,
                        y: y.floor().clamp(-1e6, 1e6) as i32,
                        text,
                        role,
                    });
                }
                Ok(())
            },
        )?;
        let source = Rc::clone(&scene_rng);
        let random = lua.create_function(move |_, args: Variadic<f64>| {
            random_value(&mut source.borrow_mut(), &args)
        })?;
        let globals = lua.globals();
        globals.set("put", put.clone())?;
        // math.random is seeded from the clock in Lua 5.4; the scene's own generator keeps a
        // seeded run repeatable
        if let Ok(math) = globals.get::<Table>("math") {
            math.set("random", random.clone())?;
            math.set(
                "randomseed",
                lua.create_function(|_, _: Variadic<Value>| Ok(()))?,
            )?;
        }
        let ctx = lua.create_table()?;
        ctx.set("put", put)?;
        ctx.set("random", random)?;
        ctx.set(
            "motion",
            match motion.level {
                Level::Calm => "calm",
                Level::Standard => "standard",
                Level::Lively => "lively",
            },
        )?;
        ctx.set("reduced", motion.calm)?;
        ctx.set("speed", motion.speed())?;
        ctx.set("pace", motion.pace())?;
        ctx.set("busy", motion.busy())?;

        let mut scene = LuaScene {
            name: static_name(&user.name),
            sandbox,
            module,
            ctx,
            puts,
            width: width.max(1),
            height: height.max(1),
            t: 0.0,
            generation: 0,
            problem: RefCell::new(None),
            meter: RefCell::new(Meter::new()),
            shown: RefCell::new(Vec::new()),
            judged: Cell::new(None),
            scratch: RefCell::new(Frame::new(width, height)),
        };
        scene.update_ctx(0.0)?;
        scene.call_hook("init", INIT_BUDGET)?;
        scene.render_frame(0.0)?;
        Ok(scene)
    }

    fn update_ctx(&self, dt: f32) -> mlua::Result<()> {
        self.ctx.set("width", self.width)?;
        self.ctx.set("height", self.height)?;
        self.ctx.set("time", self.t)?;
        self.ctx.set("dt", dt)?;
        Ok(())
    }

    /// Calls an optional function of the script's table with the context and returns what it
    /// gave back.
    fn call_hook(&self, name: &str, budget: i64) -> mlua::Result<Value> {
        match self.module.get::<Value>(name)? {
            Value::Function(f) => self.call(&f, budget),
            Value::Nil => Ok(Value::Nil),
            _ => Err(mlua::Error::runtime(format!("{name} must be a function"))),
        }
    }

    fn call(&self, function: &Function, budget: i64) -> mlua::Result<Value> {
        self.sandbox.budget(budget);
        function.call::<Value>(self.ctx.clone())
    }

    /// Runs `frame` and keeps what it drew.
    fn render_frame(&mut self, dt: f32) -> mlua::Result<()> {
        self.update_ctx(dt)?;
        self.puts.borrow_mut().clear();
        self.generation += 1;
        let frame: Function = self.module.get("frame")?;
        self.call(&frame, FRAME_BUDGET)?;
        Ok(())
    }

    fn fail(&self, error: &mlua::Error) {
        let mut problem = self.problem.borrow_mut();
        if problem.is_none() {
            *problem = Some(format!("{}: {}", self.name, short_error(error)));
        }
    }

    fn stopped(&self) -> bool {
        self.problem.borrow().is_some()
    }

    fn paint(puts: &[Put], frame: &mut Frame, theme: &ThemeData) {
        for put in puts {
            let colour = Some(put.role.colour(theme));
            for (i, ch) in put.text.chars().enumerate() {
                frame.put(put.x.saturating_add(i as i32), put.y, ch, colour);
            }
        }
    }
}

impl Scene for LuaScene {
    fn name(&self) -> &'static str {
        self.name
    }

    fn resize(&mut self, width: usize, height: usize, _rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        *self.scratch.borrow_mut() = Frame::new(self.width, self.height);
        // a new size is one change of its own, not a flash
        self.meter.borrow_mut().reset();
        if self.stopped() {
            return;
        }
        let result = self
            .update_ctx(0.0)
            .and_then(|()| self.call_hook("resize", FRAME_BUDGET).map(|_| ()))
            .and_then(|()| self.render_frame(0.0));
        if let Err(error) = result {
            self.fail(&error);
        }
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.meter.borrow_mut().advance(dt);
        if self.stopped() {
            return;
        }
        self.t += f64::from(dt.max(0.0));
        if let Err(error) = self.render_frame(dt) {
            self.fail(&error);
        }
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        // each new frame from the script is judged once, however many themes it is drawn in
        if !self.stopped() && self.judged.get() != Some(self.generation) {
            self.judged.set(Some(self.generation));
            let mut scratch = self.scratch.borrow_mut();
            scratch.clear();
            Self::paint(&self.puts.borrow(), &mut scratch, theme);
            let mut meter = self.meter.borrow_mut();
            if meter.judge(&scratch, theme) == Verdict::Show {
                *self.shown.borrow_mut() = self.puts.borrow().clone();
            }
            if meter.held_recently() > MAX_HELD {
                *self.problem.borrow_mut() = Some(format!(
                    "{} was stopped because it flashed too often",
                    self.name
                ));
            }
        }
        Self::paint(&self.shown.borrow(), frame, theme);
    }

    fn problem(&self) -> Option<String> {
        self.problem.borrow().clone()
    }

    fn caption_for(&mut self, moment: Moment) -> Option<String> {
        if self.stopped() {
            return None;
        }
        let hook = match moment {
            Moment::Pass => "on_pass",
            Moment::Fail => "on_fail",
            Moment::Recovered => "on_recovered",
        };
        let result = self
            .update_ctx(0.0)
            .and_then(|()| self.call_hook(hook, FRAME_BUDGET));
        match result {
            Ok(Value::String(s)) => {
                let text: String = s
                    .to_string_lossy()
                    .trim()
                    .chars()
                    .take(MAX_CAPTION_CHARS)
                    .map(|c| if safe_glyph(c) { c } else { '?' })
                    .collect();
                (!text.is_empty()).then_some(text)
            }
            Ok(_) => None,
            Err(error) => {
                self.fail(&error);
                None
            }
        }
    }
}

/// Builds a user scene, fitted to a size. A script that fails to load or to run its first frame
/// is an error here; one that fails later reports it through `Scene::problem`.
pub fn create(
    user: &UserScene,
    width: usize,
    height: usize,
    motion: Motion,
    rng: &mut Rng,
) -> Result<Box<dyn Scene>, String> {
    LuaScene::build(user, width, height, motion, rng)
        .map(|scene| Box::new(scene) as Box<dyn Scene>)
        .map_err(|error| format!("{}: {}", user.name, short_error(&error)))
}

/// A commented starter script for `zacess-scenes scene new <name>`.
pub fn starter(name: &str) -> String {
    let name = normalise(name);
    STARTER.replace("{name}", &name)
}

const STARTER: &str = r#"-- {name}: a Zacess Scenes scene of your own.
--
-- This file returns a table. Zacess Scenes calls its frame function about 20 times a
-- second and draws whatever it puts on screen. The full reference is docs/lua.md in the
-- project: https://github.com/zacesshq/scenes/blob/main/docs/lua.md
--
-- Try it with:  zacess-scenes {name}
-- In the watch mode:  zacess-scenes watch --scene {name}

-- state lives in ordinary local variables, kept between frames
local drops = {}

return {
  -- the name used on the command line and in scenes.toml
  name = "{name}",
  -- "pass" for a passing build, "fail" for a failing one or "general" for either
  kind = "pass",
  -- one line for zacess-scenes list
  summary = "Slow drops falling past a quiet horizon",
  -- shown under ALL GOOD (or BUILD FAILED for a fail scene), one per run in turn
  captions = {
    "All good. Nothing to fix.",
    "A clean build, quietly.",
    "Everything compiles.",
  },

  -- called once before the first frame
  init = function(ctx)
    for i = 1, 12 do
      drops[i] = { x = ctx.random(0, ctx.width - 1), y = ctx.random() * ctx.height }
    end
  end,

  -- called after the terminal changes size; ctx.width and ctx.height are already new
  resize = function(ctx)
    for _, drop in ipairs(drops) do
      drop.x = drop.x % ctx.width
    end
  end,

  -- called every frame: move things on by ctx.dt seconds, then draw them
  frame = function(ctx)
    -- ctx.speed is 0.5 under reduced motion and more on the lively level
    local fall = 3 * ctx.speed
    for _, drop in ipairs(drops) do
      drop.y = drop.y + fall * ctx.dt
      if drop.y >= ctx.height - 1 then
        drop.y = 0
        drop.x = ctx.random(0, ctx.width - 1)
      end
      -- put(x, y, text, role): the role is a palette name, never a colour
      put(drop.x, math.floor(drop.y), ":", "bright_blue")
    end
    -- a horizon along the bottom row
    put(0, ctx.height - 1, string.rep("~", ctx.width), "cyan")
  end,

  -- optional: return a caption for the moment; return nothing to use the list above
  on_recovered = function(ctx)
    return "Fixed. The drops fall on."
  end,
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flash::{MAX_CHANGES_PER_SECOND, most_changes_per_second, regions};
    use crate::theme;
    use std::time::{Duration, Instant};

    const EXAMPLE: &str = include_str!("../examples/scenes/lantern.lua");

    fn scene_from(source: &str) -> Result<UserScene, String> {
        UserScene::from_source(Path::new("test.lua"), source)
    }

    fn build(source: &str) -> Result<Box<dyn Scene>, String> {
        let user = scene_from(source)?;
        create(&user, 40, 12, Motion::FULL, &mut Rng::new(1))
    }

    /// Steps a scene and draws it, returning the last frame.
    fn run(scene: &mut dyn Scene, frames: usize, width: usize, height: usize) -> Frame {
        let mut rng = Rng::new(2);
        let mut frame = Frame::new(width, height);
        for _ in 0..frames {
            scene.step(0.05, &mut rng);
            frame.clear();
            scene.draw(&mut frame, theme::default_theme());
        }
        frame
    }

    #[test]
    fn metadata_is_read_with_defaults() {
        let full = scene_from(
            r#"return { name = "My_Scene", kind = "fail", summary = " Rust on a gate ",
               captions = { "One", "Two" }, frame = function() end }"#,
        )
        .unwrap();
        assert_eq!(full.name, "my-scene");
        assert_eq!(full.kind, Kind::Fail);
        assert_eq!(full.summary, "Rust on a gate");
        assert_eq!(full.captions, vec!["One", "Two"]);
        let bare = UserScene::from_source(
            Path::new("/x/quiet_pond.lua"),
            "return { frame = function() end }",
        )
        .unwrap();
        assert_eq!(bare.name, "quiet-pond");
        assert_eq!(bare.kind, Kind::General);
        assert!(bare.summary.contains("quiet_pond.lua"));
        assert!(bare.captions.is_empty());
    }

    #[test]
    fn bad_metadata_is_refused_with_the_file_named() {
        for (source, needle) in [
            ("return 1", "returning a table"),
            ("return {}", "frame function"),
            ("return { kind = 'ok', frame = function() end }", "kind"),
            ("return { name = 'a b!', frame = function() end }", "usable"),
            ("return {", "test.lua"),
        ] {
            let error = scene_from(source).unwrap_err();
            assert!(error.contains(needle), "{source}: {error}");
            assert!(error.starts_with("test.lua"), "{error}");
        }
    }

    #[test]
    fn a_built_in_name_is_refused() {
        let error = scene_from("return { name = 'waves', frame = function() end }").unwrap_err();
        assert!(error.contains("built-in"), "{error}");
    }

    #[test]
    fn a_folder_loads_in_order_and_reports_clashes() {
        let dir = std::env::temp_dir().join(format!("zacess-lua-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("b.lua"),
            "return { name = 'one', frame = function() end }",
        )
        .unwrap();
        std::fs::write(
            dir.join("a.lua"),
            "return { name = 'one', frame = function() end }",
        )
        .unwrap();
        std::fs::write(dir.join("c.lua"), "return nil").unwrap();
        std::fs::write(dir.join("notes.txt"), "not a scene").unwrap();
        let (found, errors) = load_dir(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path.file_name().unwrap(), "a.lua");
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors[0].starts_with("b.lua") && errors[0].contains("already"));
        assert!(errors[1].starts_with("c.lua"));
        let (none, no_errors) = load_dir(Path::new("/no/such/folder/at/all"));
        assert!(none.is_empty() && no_errors.is_empty());
    }

    #[test]
    fn sandbox_escapes_are_refused() {
        for escape in [
            "io.open('/etc/passwd')",
            "os.execute('true')",
            "os.getenv('HOME')",
            "require('os')",
            "package.loadlib('x', 'y')",
            "dofile('/etc/passwd')",
            "loadfile('/etc/passwd')",
            "load('return 1')",
            "debug.getinfo(1)",
            "string.dump(function() end)",
            "collectgarbage('count')",
            "coroutine.create(function() end)",
            "print('to the terminal')",
        ] {
            let source = format!("{escape}\nreturn {{ frame = function() end }}");
            let error = scene_from(&source).unwrap_err();
            assert!(
                error.contains("nil") || error.contains("attempt"),
                "{escape} was not refused: {error}"
            );
            // the same call made from inside a frame stops the scene instead
            let source =
                format!("return {{ frame = function(ctx) if ctx.time > 0 then {escape} end end }}");
            let mut scene = build(&source).unwrap();
            run(scene.as_mut(), 2, 40, 12);
            assert!(scene.problem().is_some(), "{escape} ran in a frame");
        }
    }

    #[test]
    fn the_instruction_budget_stops_an_endless_loop() {
        let start = Instant::now();
        let error = scene_from("while true do end").unwrap_err();
        assert!(error.contains("budget"), "{error}");
        let mut scene = build(
            "return { frame = function(ctx) if ctx.time > 0 then while true do end end end }",
        )
        .unwrap();
        run(scene.as_mut(), 3, 40, 12);
        let problem = scene.problem().unwrap();
        assert!(problem.contains("budget"), "{problem}");
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "{:?}",
            start.elapsed()
        );
        // a stopped scene is not called again, so later frames cost nothing
        let later = Instant::now();
        run(scene.as_mut(), 100, 40, 12);
        assert!(later.elapsed() < Duration::from_millis(200));
    }

    #[test]
    fn the_memory_limit_holds() {
        let error = scene_from(
            "local s = string.rep('x', 64 * 1024 * 1024)\nreturn { frame = function() end }",
        )
        .unwrap_err();
        assert!(error.contains("memory"), "{error}");
    }

    #[test]
    fn an_unknown_role_stops_the_scene() {
        let mut scene = build(
            "return { frame = function(ctx) put(0, 0, 'x', ctx.time > 0 and '#ff0000' or 'text') end }",
        )
        .unwrap();
        run(scene.as_mut(), 2, 40, 12);
        let problem = scene.problem().unwrap();
        assert!(problem.contains("palette role"), "{problem}");
        assert!(build("return { frame = function() put(0, 0, 'x', 'pink') end }").is_err());
        assert_eq!(Role::parse("Syntax.Keyword"), Some(Role::Syntax(0)));
        assert_eq!(Role::parse("gray"), Some(Role::Grey));
        for name in ROLE_NAMES {
            assert!(Role::parse(name).is_some(), "{name}");
        }
    }

    #[test]
    fn every_role_keeps_the_floor_in_every_theme() {
        for theme in theme::all() {
            for name in ROLE_NAMES {
                let colour = Role::parse(name).unwrap().colour(theme);
                assert!(
                    theme.contrast_of(colour) >= theme.floor(),
                    "{name} in {}",
                    theme.name
                );
            }
        }
    }

    #[test]
    fn unsafe_characters_become_question_marks() {
        let mut scene = build(
            "return { frame = function() put(0, 0, 'a\\tb\\u{1F600}\\u{2588}', 'text') end }",
        )
        .unwrap();
        let frame = run(scene.as_mut(), 1, 10, 2);
        assert_eq!(frame.row_text(0), "a?b?\u{2588}     ");
    }

    #[test]
    fn a_flashing_script_is_damped_then_stopped() {
        let source = r#"return { frame = function(ctx)
            if math.floor(ctx.time * 20 + 0.5) % 2 == 0 then
              for y = 0, ctx.height - 1 do put(0, y, string.rep("\u{2588}", ctx.width), "text") end
            end
          end }"#;
        let mut scene = build(source).unwrap();
        let theme = theme::default_theme();
        let mut rng = Rng::new(3);
        let mut frame = Frame::new(40, 12);
        let mut shown = Vec::new();
        for _ in 0..(10 * 20) {
            scene.step(0.05, &mut rng);
            frame.clear();
            scene.draw(&mut frame, theme);
            shown.push(regions(&frame, theme));
        }
        assert!(most_changes_per_second(&shown, 20) <= MAX_CHANGES_PER_SECOND);
        let problem = scene.problem().unwrap();
        assert!(problem.contains("flashed"), "{problem}");
    }

    #[test]
    fn the_example_renders_in_every_theme_within_the_floor() {
        let user = UserScene::from_source(Path::new("lantern.lua"), EXAMPLE).unwrap();
        assert_eq!(user.name, "lantern");
        assert_eq!(user.kind, Kind::Pass);
        assert!(user.captions.len() >= 3);
        for (width, height) in [(20, 8), (80, 24)] {
            for motion in Motion::ALL {
                let mut scene = create(&user, width, height, motion, &mut Rng::new(4)).unwrap();
                let mut rng = Rng::new(5);
                let mut frame = Frame::new(width, height);
                let mut measured = Vec::new();
                for step in 0..(30 * 20) {
                    scene.step(0.05, &mut rng);
                    for theme in theme::all() {
                        frame.clear();
                        scene.draw(&mut frame, theme);
                        assert!(frame.drawn() > 0, "nothing at {width}x{height}");
                        for colour in frame.colours() {
                            assert!(theme.contrast_of(colour) >= theme.floor(), "{}", theme.name);
                        }
                        if std::ptr::eq(theme, theme::default_theme()) {
                            measured.push(regions(&frame, theme));
                        }
                    }
                    if step % 100 == 0 {
                        assert!(frame.cells().iter().all(|c| safe_glyph(c.ch)));
                    }
                }
                assert!(scene.problem().is_none(), "{:?}", scene.problem());
                assert!(most_changes_per_second(&measured, 20) <= MAX_CHANGES_PER_SECOND);
                scene.resize(1, 1, &mut rng);
                scene.step(0.05, &mut rng);
                scene.resize(width, height, &mut rng);
                scene.step(0.05, &mut rng);
                assert!(scene.problem().is_none(), "{:?}", scene.problem());
            }
        }
    }

    #[test]
    fn the_starter_loads_and_renders() {
        let source = starter("My Scene");
        assert!(source.contains("name = \"my-scene\""));
        let user = UserScene::from_source(Path::new("my-scene.lua"), &source).unwrap();
        assert_eq!(user.name, "my-scene");
        assert_eq!(user.kind, Kind::Pass);
        let mut scene = create(&user, 30, 10, Motion::CALM, &mut Rng::new(1)).unwrap();
        let frame = run(scene.as_mut(), 40, 30, 10);
        assert!(frame.drawn() > 0);
        assert!(frame.row_text(9).starts_with("~~~"));
        assert!(scene.problem().is_none());
        assert_eq!(
            scene.caption_for(Moment::Recovered).as_deref(),
            Some("Fixed. The drops fall on.")
        );
        assert_eq!(scene.caption_for(Moment::Pass), None);
    }

    #[test]
    fn the_worked_example_in_the_docs_runs() {
        let docs = include_str!("../docs/lua.md");
        let start = docs.rfind("```lua\n").expect("a lua block") + "```lua\n".len();
        let source = &docs[start..start + docs[start..].find("```").expect("its end")];
        let user = UserScene::from_source(Path::new("jar.lua"), source).unwrap();
        assert_eq!(user.name, "jar");
        let mut scene = create(&user, 40, 14, Motion::LIVELY, &mut Rng::new(1)).unwrap();
        let frame = run(scene.as_mut(), 200, 40, 14);
        assert!(scene.problem().is_none(), "{:?}", scene.problem());
        assert!(frame.row_text(13).starts_with("===="));
        assert!(frame.to_text().contains('|'));
        assert_eq!(
            scene.caption_for(Moment::Recovered).as_deref(),
            Some("Fixed. The jar glows again.")
        );
    }

    #[test]
    fn the_same_seed_draws_the_same_frame() {
        let user = UserScene::from_source(Path::new("lantern.lua"), EXAMPLE).unwrap();
        let texts: Vec<String> = (0..2)
            .map(|_| {
                let mut scene = create(&user, 60, 20, Motion::FULL, &mut Rng::new(9)).unwrap();
                run(scene.as_mut(), 80, 60, 20).to_text()
            })
            .collect();
        assert_eq!(texts[0], texts[1]);
    }

    #[test]
    fn a_runtime_error_becomes_a_problem() {
        let mut scene = build(
            "return { frame = function(ctx) if ctx.time > 0.2 then local t = nil; return t.x end put(0, 0, 'ok') end }",
        )
        .unwrap();
        let frame = run(scene.as_mut(), 10, 40, 12);
        let problem = scene.problem().unwrap();
        assert!(problem.starts_with("test: test.lua:1:"), "{problem}");
        // the last good frame stays on screen until the caller swaps the scene
        assert!(frame.row_text(0).starts_with("ok"));
        let error = build("return { frame = function() error('boom') end }").err();
        assert!(error.unwrap().contains("boom"));
    }

    #[test]
    fn the_frame_drawn_after_a_resize_is_shown() {
        let mut scene =
            build("return { frame = function(ctx) put(0, 0, ctx.width .. 'x') end }").unwrap();
        let mut rng = Rng::new(1);
        run(scene.as_mut(), 2, 40, 12);
        scene.resize(25, 5, &mut rng);
        let mut frame = Frame::new(25, 5);
        scene.draw(&mut frame, theme::default_theme());
        assert!(
            frame.row_text(0).starts_with("25x"),
            "{}",
            frame.row_text(0)
        );
    }

    #[test]
    fn hooks_return_captions() {
        let mut scene = build(
            r#"return { frame = function() put(0, 0, "x") end,
               on_pass = function(ctx) return "  Passed at " .. ctx.width .. "  " end,
               on_fail = function() return 42 end }"#,
        )
        .unwrap();
        assert_eq!(
            scene.caption_for(Moment::Pass).as_deref(),
            Some("Passed at 40")
        );
        assert_eq!(scene.caption_for(Moment::Fail), None);
        assert_eq!(scene.caption_for(Moment::Recovered), None);
        assert!(scene.problem().is_none());
    }

    #[test]
    fn random_is_seeded_and_in_range() {
        let mut rng = Rng::new(1);
        for _ in 0..1000 {
            let v = random_value(&mut rng, &Variadic::from_iter([3.0, 5.0])).unwrap();
            assert!((3.0..=5.0).contains(&v) && v.fract() == 0.0);
            let u = random_value(&mut rng, &Variadic::new()).unwrap();
            assert!((0.0..1.0).contains(&u));
        }
        assert!(random_value(&mut rng, &Variadic::from_iter([5.0, 3.0])).is_err());
    }

    #[test]
    #[ignore = "prints the example at 60 by 20 for a look"]
    fn print_the_example() {
        let user = UserScene::from_source(Path::new("lantern.lua"), EXAMPLE).unwrap();
        for (width, height) in [(60, 20), (20, 8)] {
            let mut scene = create(&user, width, height, Motion::FULL, &mut Rng::new(1)).unwrap();
            println!("{}\n", run(scene.as_mut(), 120, width, height).to_text());
        }
    }
}
