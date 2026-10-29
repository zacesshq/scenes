//! Running bacon as a separate program and reading what it reports. bacon is started headless
//! with its JSON report export turned on through `--config-toml`, so the project's own
//! `bacon.toml` is read as usual and never written to.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant, SystemTime};

use crate::report::{self, Report};

/// The command that installs bacon, shown when it is missing.
pub const INSTALL: &str = "cargo install --locked bacon";

/// How long after a report output lines are still taken as the end of that run rather than the
/// start of the next. The reader thread can hand over the last lines a moment after the file
/// lands.
const SETTLE: Duration = Duration::from_millis(400);

/// Something that happened since the last look.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// bacon started a run: the job printed its first line.
    Started,
    /// A run finished and its report was read.
    Finished(Report),
    /// bacon stopped on its own, with the last line it printed.
    Exited(String),
}

/// A TOML basic string, so any path can be put in the config.
pub fn toml_string(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The config passed to bacon: the JSON report written after every run to `report`.
pub fn config_toml(report: &Path) -> String {
    format!(
        "[exports.json-report]\nauto = true\npath = {}\n",
        toml_string(&report.display().to_string())
    )
}

/// A running bacon.
pub struct Bacon {
    child: Child,
    report: PathBuf,
    /// When and how big the report last was, so a rewrite is noticed.
    seen: Option<(SystemTime, u64)>,
    lines: Receiver<String>,
    last_line: String,
    last_report: Option<Instant>,
    running: bool,
}

impl Bacon {
    /// Starts `bacon --headless -j <job>` in `project`, writing its report to `report`. A stale
    /// report from an earlier run is removed first.
    pub fn start(
        program: &Path,
        project: &Path,
        job: &str,
        report: &Path,
    ) -> std::io::Result<Self> {
        if let Some(dir) = report.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let _ = std::fs::remove_file(report);
        let mut child = Command::new(program)
            .args([
                "--headless",
                "-j",
                job,
                "--config-toml",
                &config_toml(report),
            ])
            .current_dir(project)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let (send, lines) = mpsc::channel();
        // the job's progress lines arrive on standard error; standard output carries bacon's
        // own summary and test output, read only so the pipe never fills and stalls bacon
        if let Some(stderr) = child.stderr.take() {
            std::thread::spawn(move || {
                for line in BufReader::new(stderr).lines() {
                    let Ok(line) = line else { break };
                    if send.send(line).is_err() {
                        break;
                    }
                }
            });
        }
        if let Some(mut stdout) = child.stdout.take() {
            std::thread::spawn(move || {
                let mut sink = [0_u8; 4096];
                while matches!(stdout.read(&mut sink), Ok(n) if n > 0) {}
            });
        }
        Ok(Self {
            child,
            report: report.to_path_buf(),
            seen: None,
            lines,
            last_line: String::new(),
            last_report: None,
            running: false,
        })
    }

    /// Looks for new output, a new report or bacon having stopped.
    pub fn poll(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(line) = self.lines.try_recv() {
            let text = report::strip_controls(&line);
            if !text.trim().is_empty() {
                self.last_line = text.trim().to_string();
            }
            let settled = self.last_report.is_none_or(|at| at.elapsed() >= SETTLE);
            if !self.running && settled {
                self.running = true;
                events.push(Event::Started);
            }
        }
        if let Ok(meta) = std::fs::metadata(&self.report) {
            let stamp = (
                meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                meta.len(),
            );
            if self.seen != Some(stamp) && meta.len() > 0 {
                // a report caught half written fails to parse and is read again next time
                if let Ok(report) = std::fs::read_to_string(&self.report)
                    .map_err(|e| e.to_string())
                    .and_then(|text| report::parse(&text))
                {
                    self.seen = Some(stamp);
                    self.running = false;
                    self.last_report = Some(Instant::now());
                    events.push(Event::Finished(report));
                }
            }
        }
        if let Ok(Some(_)) = self.child.try_wait() {
            events.push(Event::Exited(std::mem::take(&mut self.last_line)));
        }
        events
    }

    /// Stops bacon and removes its report.
    pub fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.report);
    }
}

impl Drop for Bacon {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_turns_the_json_report_on_at_the_path() {
        let toml = config_toml(Path::new("/tmp/zacess scenes/report.json"));
        assert_eq!(
            toml,
            "[exports.json-report]\nauto = true\npath = \"/tmp/zacess scenes/report.json\"\n"
        );
    }

    #[test]
    fn toml_strings_escape_quotes_backslashes_and_controls() {
        assert_eq!(
            toml_string(r#"C:\Users\a "b"\r.json"#),
            r#""C:\\Users\\a \"b\"\\r.json""#
        );
        assert_eq!(toml_string("a\nb"), "\"a\\u000Ab\"");
    }
}
