//! Watching a project in any language: the project's own check or test command runs once at
//! the start and again after files change. Its exit code decides pass or fail. The output is
//! kept only to show the lines that look like errors, so nothing about a language's tools is
//! assumed beyond its usual command.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant, UNIX_EPOCH};

use crate::json;
use crate::report::{self, Item, Kind, Report};

use super::bacon::Event;

/// How often the files are looked at for a change.
const SCAN_EVERY: Duration = Duration::from_millis(700);
/// How long the files have to stay still before a run starts, so a save that writes several
/// files starts one run rather than several.
const SETTLE: Duration = Duration::from_millis(300);
/// The most files looked at, so a huge folder never makes a scan slow.
const MAX_FILES: usize = 20_000;
/// How deep the scan goes below the project folder.
const MAX_DEPTH: usize = 12;
/// The most output lines kept from one run.
const KEEP_LINES: usize = 400;
/// The most error lines shown from one run.
const SHOWN_ITEMS: usize = 8;

/// Folders that hold build output, dependencies or tool state rather than the project's own
/// files. A change in one is the command's own doing, so watching them would rerun it forever.
const SKIPPED: [&str; 34] = [
    ".git",
    ".hg",
    ".svn",
    "target",
    "node_modules",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".turbo",
    ".cache",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".venv",
    "venv",
    ".gradle",
    "bin",
    "obj",
    ".build",
    "_build",
    "deps",
    "coverage",
    ".nyc_output",
    ".dart_tool",
    ".angular",
    ".parcel-cache",
    ".idea",
    ".vscode",
    "DerivedData",
    "tmp",
];

/// Files that tools write as caches or logs in the project's own folders.
fn skipped_file(name: &str) -> bool {
    name.ends_with(".log")
        || name.ends_with("cache")
        || name.ends_with(".tsbuildinfo")
        || name.ends_with(".pyc")
        || name.ends_with(".swp")
        || name == ".DS_Store"
}

/// A project found by its marker file and the command that checks it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Detected {
    /// The folder holding the marker file.
    pub dir: PathBuf,
    /// The language or tool, for the intro and the help.
    pub kind: &'static str,
    /// The command run in `dir`, through the system shell.
    pub command: String,
}

/// The npm script `npm init` writes, which fails on purpose and so says nothing about the
/// project.
const NPM_PLACEHOLDER: &str = "no test specified";

/// The package manager a Node project uses, from its lockfile.
fn node_runner(dir: &Path) -> &'static str {
    if dir.join("pnpm-lock.yaml").is_file() {
        "pnpm"
    } else if dir.join("yarn.lock").is_file() {
        "yarn"
    } else if dir.join("bun.lockb").is_file() || dir.join("bun.lock").is_file() {
        "bun"
    } else {
        "npm"
    }
}

/// The command for a Node project: its `check` script, then a real `test` script, then `build`,
/// then `lint`. `None` when it has none of them.
pub fn node_command(manifest: &str, runner: &str) -> Option<String> {
    let value = json::parse(manifest).ok()?;
    let scripts = value.get("scripts")?;
    for name in ["check", "test", "build", "lint"] {
        let Some(script) = scripts.get(name).and_then(json::Value::as_str) else {
            continue;
        };
        if name == "test" && script.contains(NPM_PLACEHOLDER) {
            continue;
        }
        return Some(if runner == "npm" && name == "test" {
            "npm test".to_string()
        } else {
            format!("{runner} run {name}")
        });
    }
    None
}

/// Whether a folder holds Python tests pytest would find.
fn has_python_tests(dir: &Path) -> bool {
    if dir.join("tests").is_dir() || dir.join("test").is_dir() {
        return true;
    }
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.ends_with(".py") && (name.starts_with("test_") || name.ends_with("_test.py"))
        })
    })
}

/// Whether a folder holds a file with one of the extensions.
fn has_extension(dir: &Path, extensions: &[&str]) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry
                .path()
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| extensions.contains(&e))
        })
    })
}

/// The command for one folder, when it is the root of a project this knows.
fn detect_here(dir: &Path) -> Option<(&'static str, String)> {
    let python = if cfg!(windows) { "python" } else { "python3" };
    if dir.join("Cargo.toml").is_file() {
        return Some(("Rust", "cargo check".to_string()));
    }
    if dir.join("package.json").is_file() {
        let manifest = std::fs::read_to_string(dir.join("package.json")).unwrap_or_default();
        // a package.json with no usable script, such as one kept only for a formatter, says
        // nothing about the project, so the other markers in the folder are looked at
        if let Some(command) = node_command(&manifest, node_runner(dir)) {
            return Some(("JavaScript", command));
        }
    }
    if dir.join("go.mod").is_file() {
        return Some(("Go", "go build ./... && go vet ./...".to_string()));
    }
    if [
        "pyproject.toml",
        "setup.py",
        "setup.cfg",
        "requirements.txt",
    ]
    .iter()
    .any(|f| dir.join(f).is_file())
    {
        let command = if has_python_tests(dir) {
            format!("{python} -m pytest -q")
        } else {
            // with no tests every file is still compiled, which catches a syntax error
            format!("{python} -m compileall -q .")
        };
        return Some(("Python", command));
    }
    if dir.join("pom.xml").is_file() {
        return Some(("Java", "mvn -q compile".to_string()));
    }
    if dir.join("build.gradle").is_file() || dir.join("build.gradle.kts").is_file() {
        let wrapper = if cfg!(windows) {
            "gradlew.bat"
        } else {
            "./gradlew"
        };
        let command = if dir.join(wrapper.trim_start_matches("./")).is_file() {
            format!("{wrapper} -q build")
        } else {
            "gradle -q build".to_string()
        };
        return Some(("Gradle", command));
    }
    if has_extension(dir, &["sln", "csproj", "fsproj"]) {
        return Some((".NET", "dotnet build --nologo -v q".to_string()));
    }
    if dir.join("Package.swift").is_file() {
        return Some(("Swift", "swift build".to_string()));
    }
    if dir.join("mix.exs").is_file() {
        return Some(("Elixir", "mix compile".to_string()));
    }
    if dir.join("composer.json").is_file() {
        let manifest = std::fs::read_to_string(dir.join("composer.json")).unwrap_or_default();
        let has_test = json::parse(&manifest)
            .ok()
            .and_then(|v| v.get("scripts").and_then(|s| s.get("test")).cloned())
            .is_some();
        if has_test {
            return Some(("PHP", "composer test".to_string()));
        }
    }
    if dir.join("Gemfile").is_file() && dir.join("Rakefile").is_file() {
        return Some(("Ruby", "bundle exec rake".to_string()));
    }
    if dir.join("Makefile").is_file() || dir.join("makefile").is_file() {
        return Some(("Make", "make".to_string()));
    }
    if dir.join("CMakeLists.txt").is_file() && dir.join("build").is_dir() {
        return Some(("CMake", "cmake --build build".to_string()));
    }
    None
}

/// The nearest project at or above `start` and its command.
pub fn detect(start: &Path) -> Option<Detected> {
    start.ancestors().find_map(|dir| {
        detect_here(dir).map(|(kind, command)| Detected {
            dir: dir.to_path_buf(),
            kind,
            command,
        })
    })
}

/// The message for a folder with no project this knows.
pub fn not_detected(dir: &Path) -> String {
    format!(
        "no project was found in {} or any folder above it. The watch mode knows Rust, \
         JavaScript and TypeScript, Python, Go, Java, Gradle, .NET, Swift, Elixir, PHP, Ruby, \
         Make and CMake projects. For anything else name the command, such as:\n\n    \
         zacess-scenes watch --cmd \"javac Main.java\"\n",
        dir.display()
    )
}

/// A cheap fingerprint of the files under `dir`: how many there are and their sizes and change
/// times folded together. Any save, new file or deletion changes it.
pub fn fingerprint(dir: &Path) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |value: u64| {
        hash ^= value;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    };
    let mut count = 0_usize;
    let mut stack = vec![(dir.to_path_buf(), 0_usize)];
    while let Some((folder, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            if count >= MAX_FILES {
                return hash;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                if depth < MAX_DEPTH && !SKIPPED.contains(&name.as_ref()) {
                    stack.push((entry.path(), depth + 1));
                }
                continue;
            }
            if skipped_file(&name) {
                continue;
            }
            count += 1;
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let stamp = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_nanos() as u64);
            for byte in name.bytes() {
                mix(u64::from(byte));
            }
            mix(meta.len());
            mix(stamp);
        }
    }
    mix(count as u64);
    hash
}

/// Whether a line of output reads as an error, in the words the common tools use. Python's
/// `Traceback` heading is left out, since the line that names the exception says more.
pub fn looks_like_error(line: &str) -> bool {
    let first = line.split_whitespace().next().unwrap_or("");
    let name = first.trim_end_matches(':');
    // an exception named on its own, as Python, Java and .NET end a stack trace
    if name.len() > 5
        && (name.ends_with("Error") || name.ends_with("Exception"))
        && name.chars().next().is_some_and(char::is_alphabetic)
    {
        return true;
    }
    let lower = line.to_ascii_lowercase();
    if lower.trim_start().starts_with("traceback") {
        return false;
    }
    let starts = |p: &str| lower.trim_start().starts_with(p);
    starts("error")
        || starts("fail")
        || starts("fatal")
        || starts("syntaxerror")
        || starts("e   ")
        || lower.contains(" error ")
        || lower.contains(" error:")
        || lower.contains("error[")
        || lower.contains("exception")
        || lower.contains("panicked at")
        || lower.contains("not ok ")
        || lower.contains("✗")
        || lower.contains("✕")
}

/// A place in a file at the start of a line, such as `src/main.py:12:5`, split from the rest.
fn split_location(line: &str) -> (String, Option<String>) {
    let trimmed = line.trim();
    let first = trimmed.split_whitespace().next().unwrap_or("");
    let candidate = first.trim_end_matches(':');
    let mut parts = candidate.split(':');
    let file = parts.next().unwrap_or("");
    let has_line = parts.next().is_some_and(|n| n.parse::<u32>().is_ok());
    if has_line && file.contains('.') && !file.contains("//") {
        let rest = trimmed[first.len()..].trim().to_string();
        let message = if rest.is_empty() {
            trimmed.to_string()
        } else {
            rest
        };
        return (message, Some(candidate.to_string()));
    }
    (trimmed.to_string(), None)
}

/// The report for a finished run: its exit code and the lines that look like errors. When none
/// do, the last line of output stands in, so a failure always says something.
pub fn report_for(code: i64, lines: &[String]) -> Report {
    let failed = code != 0;
    let mut items: Vec<Item> = Vec::new();
    if failed {
        for line in lines.iter().filter(|l| looks_like_error(l)) {
            let (message, location) = split_location(line);
            if items.iter().any(|i| i.message == message) {
                continue;
            }
            items.push(Item {
                kind: Kind::Error,
                message,
                location,
            });
            if items.len() == SHOWN_ITEMS {
                break;
            }
        }
        if items.is_empty() {
            let last = lines
                .iter()
                .rev()
                .find(|l| !l.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| format!("the command exited with code {code}"));
            items.push(Item {
                kind: Kind::Error,
                message: last.trim().to_string(),
                location: None,
            });
        }
    }
    Report {
        errors: items.len() as u64,
        warnings: 0,
        test_fails: 0,
        exit_code: failed.then_some(code),
        items,
    }
}

/// The command run through the system shell, in its own process group on Unix so stopping it
/// also stops whatever it started.
fn shell(command: &str, dir: &Path) -> Command {
    let mut cmd = if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", command]);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", command]);
        cmd
    };
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    cmd.current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // tools that colour their output when they think a person is reading would put escape
        // sequences in the lines shown
        .env("NO_COLOR", "1")
        .env("CLICOLOR", "0")
        .env("FORCE_COLOR", "0");
    cmd
}

/// Stops a run and everything it started.
fn kill_tree(child: &mut Child) {
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &format!("-{}", child.id())])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &child.id().to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn forward(stream: impl std::io::Read + Send + 'static, send: Sender<String>) {
    std::thread::spawn(move || {
        for line in BufReader::new(stream).lines() {
            let Ok(line) = line else { break };
            if send.send(line).is_err() {
                break;
            }
        }
    });
}

/// A project being watched with its own command.
pub struct Runner {
    dir: PathBuf,
    command: String,
    child: Option<Child>,
    lines: Option<Receiver<String>>,
    output: Vec<String>,
    /// The fingerprint the last run started from.
    ran_from: u64,
    /// A change seen and when, waiting for the files to settle.
    changed: Option<(u64, Instant)>,
    last_scan: Instant,
    /// A run asked for that has not started yet.
    pending: bool,
    /// The run under way was started because files changed during the one before.
    follow_up: bool,
}

impl Runner {
    /// Starts watching: the first run starts straight away.
    pub fn start(dir: &Path, command: &str) -> std::io::Result<Self> {
        let mut runner = Self {
            dir: dir.to_path_buf(),
            command: command.to_string(),
            child: None,
            lines: None,
            output: Vec::new(),
            ran_from: 0,
            changed: None,
            last_scan: Instant::now(),
            pending: true,
            follow_up: false,
        };
        // the first run is started by the first poll, which also reports it as started
        runner.ran_from = fingerprint(dir);
        Ok(runner)
    }

    fn launch(&mut self) -> std::io::Result<()> {
        if let Some(mut child) = self.child.take() {
            kill_tree(&mut child);
        }
        self.ran_from = fingerprint(&self.dir);
        self.changed = None;
        self.output.clear();
        let mut child = shell(&self.command, &self.dir).spawn()?;
        let (send, lines) = mpsc::channel();
        if let Some(out) = child.stdout.take() {
            forward(out, send.clone());
        }
        if let Some(err) = child.stderr.take() {
            forward(err, send);
        }
        self.child = Some(child);
        self.lines = Some(lines);
        Ok(())
    }

    /// Runs the command again now, stopping a run under way.
    pub fn rerun(&mut self) {
        self.pending = true;
        self.follow_up = false;
        self.changed = None;
        self.last_scan = Instant::now() - SCAN_EVERY;
    }

    /// Looks for output, a finished run or a change that should start one.
    pub fn poll(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        if self.pending {
            self.pending = false;
            match self.launch() {
                Ok(()) => events.push(Event::Started),
                Err(error) => {
                    events.push(Event::Exited(format!(
                        "could not run {}: {error}",
                        self.command
                    )));
                    return events;
                }
            }
        }
        if let Some(lines) = &self.lines {
            while let Ok(line) = lines.try_recv() {
                let text = report::strip_controls(&line);
                if self.output.len() == KEEP_LINES {
                    self.output.remove(0);
                }
                self.output.push(text);
            }
        }
        if let Some(child) = self.child.as_mut() {
            if let Ok(Some(status)) = child.try_wait() {
                // the readers can still hold the last lines a moment after the exit
                if let Some(lines) = &self.lines {
                    let until = Instant::now() + Duration::from_millis(150);
                    while Instant::now() < until {
                        match lines.recv_timeout(Duration::from_millis(30)) {
                            Ok(line) => {
                                if self.output.len() == KEEP_LINES {
                                    self.output.remove(0);
                                }
                                self.output.push(report::strip_controls(&line));
                            }
                            Err(mpsc::RecvTimeoutError::Timeout) => continue,
                            Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        }
                    }
                }
                let code = status.code().map_or(1, i64::from);
                self.child = None;
                self.lines = None;
                events.push(Event::Finished(report_for(code, &self.output)));
                // files that changed during a run were saved by the person or written by the
                // command itself. One more run covers a save; when that run changes files too
                // they are the command's own, so they become the new starting point rather than
                // a reason to run forever
                let now = fingerprint(&self.dir);
                if now != self.ran_from && !self.follow_up {
                    self.follow_up = true;
                    self.pending = true;
                } else {
                    self.follow_up = false;
                    self.ran_from = now;
                }
            }
            return events;
        }
        if self.last_scan.elapsed() < SCAN_EVERY {
            return events;
        }
        self.last_scan = Instant::now();
        let now = fingerprint(&self.dir);
        match self.changed {
            _ if now == self.ran_from => self.changed = None,
            Some((seen, at)) if seen == now && at.elapsed() >= SETTLE => {
                self.pending = true;
                self.follow_up = false;
            }
            Some((seen, _)) if seen == now => {}
            _ => self.changed = Some((now, Instant::now())),
        }
        events
    }

    /// Stops a run under way.
    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            kill_tree(&mut child);
        }
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Seconds since the Unix epoch, for tests that need a file's time to move.
#[cfg(test)]
fn touch(path: &Path) {
    let file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .unwrap();
    let later = std::time::SystemTime::now() + Duration::from_secs(2);
    file.set_modified(later).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "zacess-scenes-runner-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn node_projects_pick_a_real_script() {
        let check = r#"{"scripts": {"test": "vitest", "check": "tsc --noEmit"}}"#;
        assert_eq!(node_command(check, "npm").as_deref(), Some("npm run check"));
        let test = r#"{"scripts": {"test": "jest"}}"#;
        assert_eq!(node_command(test, "npm").as_deref(), Some("npm test"));
        assert_eq!(node_command(test, "pnpm").as_deref(), Some("pnpm run test"));
        let placeholder =
            r#"{"scripts": {"test": "echo \"Error: no test specified\" && exit 1", "build": "x"}}"#;
        assert_eq!(
            node_command(placeholder, "npm").as_deref(),
            Some("npm run build")
        );
        assert_eq!(node_command(r#"{"name": "x"}"#, "npm"), None);
    }

    #[test]
    fn each_language_is_found_from_a_folder_inside_it() {
        let cases: [(&str, &str, &str); 6] = [
            ("go.mod", "module x\n", "go build"),
            ("pyproject.toml", "[project]\n", "compileall"),
            ("pom.xml", "<project/>", "mvn"),
            ("Makefile", "all:\n", "make"),
            ("Cargo.toml", "[package]\nname = \"x\"\n", "cargo check"),
            (
                "package.json",
                r#"{"scripts": {"test": "node --test"}}"#,
                "npm test",
            ),
        ];
        for (file, text, wanted) in cases {
            let dir = scratch(&file.replace('.', "-"));
            std::fs::write(dir.join(file), text).unwrap();
            let inner = dir.join("src").join("deep");
            std::fs::create_dir_all(&inner).unwrap();
            let found = detect(&inner).unwrap_or_else(|| panic!("{file} was not found"));
            assert_eq!(found.dir, dir, "{file}");
            assert!(found.command.contains(wanted), "{file}: {}", found.command);
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn python_with_tests_runs_pytest() {
        let dir = scratch("pytest");
        std::fs::write(dir.join("requirements.txt"), "").unwrap();
        std::fs::write(dir.join("test_maths.py"), "def test_x(): pass\n").unwrap();
        assert!(detect(&dir).unwrap().command.ends_with("-m pytest -q"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_fingerprint_moves_with_a_save_and_ignores_build_output() {
        let dir = scratch("fingerprint");
        std::fs::write(dir.join("main.py"), "print(1)\n").unwrap();
        std::fs::create_dir_all(dir.join("node_modules").join("x")).unwrap();
        let before = fingerprint(&dir);
        assert_eq!(before, fingerprint(&dir), "a scan with no change is stable");
        std::fs::write(dir.join("node_modules").join("x").join("a.js"), "1").unwrap();
        assert_eq!(before, fingerprint(&dir), "dependencies are not watched");
        touch(&dir.join("main.py"));
        assert_ne!(before, fingerprint(&dir), "a save is seen");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failure_shows_its_error_lines_and_where_they_are() {
        let lines: Vec<String> = [
            "collecting ...",
            "src/app.py:12: error: Name \"x\" is not defined",
            "FAILED tests/test_app.py::test_add - assert 3 == 4",
            "1 failed, 2 passed",
        ]
        .map(String::from)
        .to_vec();
        let report = report_for(1, &lines);
        assert!(report.failed());
        assert_eq!(report.items.len(), 2);
        assert_eq!(report.items[0].location.as_deref(), Some("src/app.py:12"));
        assert!(report.items[0].message.starts_with("error: Name"));
        assert!(report.items[1].message.starts_with("FAILED"));
    }

    #[test]
    fn a_python_failure_names_the_exception_not_the_traceback() {
        let lines: Vec<String> = [
            "Traceback (most recent call last):",
            "  File \"<string>\", line 1, in <module>",
            "AssertionError",
        ]
        .map(String::from)
        .to_vec();
        let report = report_for(1, &lines);
        assert_eq!(report.items.len(), 1);
        assert_eq!(report.items[0].message, "AssertionError");
        assert!(looks_like_error("ValueError: bad input"));
        assert!(looks_like_error("java.lang.NullPointerException"));
    }

    #[test]
    fn a_silent_failure_still_says_something_and_a_pass_says_nothing() {
        let report = report_for(2, &["".to_string(), "make: *** [all] 2".to_string()]);
        assert_eq!(report.items[0].message, "make: *** [all] 2");
        assert_eq!(
            report_for(3, &[]).items[0].message,
            "the command exited with code 3"
        );
        let pass = report_for(0, &["error in a log line is fine".to_string()]);
        assert!(!pass.failed());
        assert!(pass.items.is_empty());
    }

    #[test]
    fn a_command_that_writes_files_runs_once_more_then_settles() {
        if cfg!(windows) {
            return;
        }
        let dir = scratch("writes");
        std::fs::write(dir.join("main.txt"), "x").unwrap();
        let mut runner = Runner::start(&dir, "date +%s%N > stamp.txt").unwrap();
        let mut runs = 0;
        let until = Instant::now() + Duration::from_secs(6);
        while Instant::now() < until {
            for event in runner.poll() {
                if matches!(event, Event::Started) {
                    runs += 1;
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(
            runs, 2,
            "the first run, then one more for its own change, then none"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_package_json_without_scripts_lets_the_other_markers_decide() {
        let dir = scratch("prettier-only");
        std::fs::write(dir.join("package.json"), r#"{"devDependencies": {}}"#).unwrap();
        std::fs::write(dir.join("go.mod"), "module x\n").unwrap();
        assert!(detect(&dir).unwrap().command.starts_with("go build"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn caches_and_build_folders_are_not_watched() {
        let dir = scratch("caches");
        std::fs::write(dir.join("main.swift"), "").unwrap();
        let before = fingerprint(&dir);
        for folder in [".build", "_build", "coverage"] {
            std::fs::create_dir_all(dir.join(folder)).unwrap();
            std::fs::write(dir.join(folder).join("x"), "1").unwrap();
        }
        std::fs::write(dir.join(".eslintcache"), "{}").unwrap();
        std::fs::write(dir.join("debug.log"), "x").unwrap();
        assert_eq!(before, fingerprint(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_run_passes_fails_and_reruns_on_a_change() {
        let dir = scratch("run");
        std::fs::write(dir.join("state"), "pass").unwrap();
        let command = if cfg!(windows) {
            "findstr pass state"
        } else {
            "grep -q pass state || { echo 'error: state says fail'; exit 1; }"
        };
        let mut runner = Runner::start(&dir, command).unwrap();
        let wait_for = |runner: &mut Runner| -> Report {
            let until = Instant::now() + Duration::from_secs(10);
            loop {
                for event in runner.poll() {
                    if let Event::Finished(report) = event {
                        return report;
                    }
                }
                assert!(Instant::now() < until, "no result in time");
                std::thread::sleep(Duration::from_millis(50));
            }
        };
        assert!(!wait_for(&mut runner).failed());
        std::fs::write(dir.join("state"), "fail").unwrap();
        touch(&dir.join("state"));
        let failed = wait_for(&mut runner);
        assert!(failed.failed());
        if !cfg!(windows) {
            assert_eq!(failed.items[0].message, "error: state says fail");
        }
        runner.rerun();
        assert!(wait_for(&mut runner).failed(), "a rerun runs again");
        drop(runner);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
