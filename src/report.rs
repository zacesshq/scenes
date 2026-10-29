//! The report bacon exports after each run of a job, reduced to what the watch view shows: the
//! counts, whether the run failed and each error, warning or failing test with its location.

use crate::json::{self, Value};

/// What kind of problem an item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Error,
    Warning,
    /// A failing test.
    Test,
}

/// One problem in the report: its first line and, when bacon found one, where it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub kind: Kind,
    pub message: String,
    pub location: Option<String>,
}

/// The parts of a bacon report the view uses.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub errors: u64,
    pub warnings: u64,
    pub test_fails: u64,
    /// The exit code of the job's command when it failed.
    pub exit_code: Option<i64>,
    pub items: Vec<Item>,
}

impl Report {
    /// Whether the run failed: an error, a failing test or a command that exited with an error
    /// code, as Clippy does when warnings are denied.
    pub fn failed(&self) -> bool {
        self.errors > 0 || self.test_fails > 0 || self.exit_code.is_some_and(|code| code != 0)
    }

    /// The items of one kind, in report order.
    pub fn of_kind(&self, kind: Kind) -> impl Iterator<Item = &Item> {
        self.items.iter().filter(move |item| item.kind == kind)
    }
}

/// The plain text of one report line: its styled pieces joined, with any escape sequence left in
/// the text removed so it can never reach the terminal.
fn line_text(line: &Value) -> String {
    let mut text = String::new();
    let pieces = line
        .get("content")
        .and_then(|c| c.get("strings"))
        .and_then(Value::as_array)
        .unwrap_or_default();
    for piece in pieces {
        if let Some(raw) = piece.get("raw").and_then(Value::as_str) {
            text.push_str(raw);
        }
    }
    strip_controls(&text)
}

/// Removes escape sequences and other control characters, keeping tabs as spaces.
pub fn strip_controls(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\u{1b}' => {
                // a CSI sequence runs to its final byte in @ to ~
                if chars.peek() == Some(&'[') {
                    chars.next();
                    for next in chars.by_ref() {
                        if ('@'..='~').contains(&next) {
                            break;
                        }
                    }
                }
            }
            '\t' => out.push(' '),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// The kind a title line starts, from its `line_type`, such as `{"Title": "Error"}`.
fn title_kind(line_type: &Value) -> Option<Kind> {
    match line_type.get("Title")?.as_str()? {
        "Error" => Some(Kind::Error),
        "Warning" => Some(Kind::Warning),
        "TestFail" | "TestOutput" => Some(Kind::Test),
        _ => None,
    }
}

/// A location line's text without the arrow rustc puts before it.
fn tidy_location(text: &str) -> String {
    let text = text.trim();
    text.strip_prefix("-->")
        .map(str::trim_start)
        .unwrap_or(text)
        .to_string()
}

/// Reads a report from the JSON bacon writes with its `json-report` export.
pub fn parse(text: &str) -> Result<Report, String> {
    let root = json::parse(text)?;
    if !matches!(root, Value::Object(_)) {
        return Err("the report is not a JSON object".to_string());
    }
    let count = |name: &str| {
        root.get("stats")
            .and_then(|s| s.get(name))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    let exit_code = root
        .get("error_code")
        .and_then(Value::as_f64)
        .filter(|n| n.fract() == 0.0)
        .map(|n| n as i64);
    let mut items: Vec<Item> = Vec::new();
    // set while the lines after a title belong to it, so its location can be attached
    let mut open = false;
    for line in root
        .get("lines")
        .and_then(Value::as_array)
        .unwrap_or_default()
    {
        let line_type = line.get("line_type").unwrap_or(&Value::Null);
        if let Some(kind) = title_kind(line_type) {
            items.push(Item {
                kind,
                message: line_text(line).trim().to_string(),
                location: None,
            });
            open = true;
        } else if line_type.as_str() == Some("Location") {
            if let Some(item) = items.last_mut().filter(|_| open) {
                if item.location.is_none() {
                    item.location = Some(tidy_location(&line_text(line)));
                }
            }
        } else if line_type.get("Title").is_some() || line_type.as_str() == Some("SectionEnd") {
            open = false;
        }
    }
    Ok(Report {
        errors: count("errors"),
        warnings: count("warnings"),
        test_fails: count("test_fails"),
        exit_code,
        items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Report {
        let text = match name {
            "check-clean" => include_str!("../tests/fixtures/bacon/check-clean.json"),
            "check-warning" => include_str!("../tests/fixtures/bacon/check-warning.json"),
            "check-error" => include_str!("../tests/fixtures/bacon/check-error.json"),
            "clippy-warnings" => include_str!("../tests/fixtures/bacon/clippy-warnings.json"),
            "test-failing" => include_str!("../tests/fixtures/bacon/test-failing.json"),
            "test-passing" => include_str!("../tests/fixtures/bacon/test-passing.json"),
            _ => panic!("no fixture {name}"),
        };
        parse(text).unwrap_or_else(|error| panic!("{name}: {error}"))
    }

    #[test]
    fn a_clean_check_passes_with_nothing_to_show() {
        let report = fixture("check-clean");
        assert!(!report.failed());
        assert_eq!(
            (report.errors, report.warnings, report.test_fails),
            (0, 0, 0)
        );
        assert_eq!(report.exit_code, None);
        assert!(report.items.is_empty());
    }

    #[test]
    fn warnings_alone_still_pass() {
        let report = fixture("check-warning");
        assert!(!report.failed());
        assert_eq!(report.warnings, 1);
        assert_eq!(
            report.items,
            vec![Item {
                kind: Kind::Warning,
                message: "warning: unused variable: `unused`".to_string(),
                location: Some("src/main.rs:2:9".to_string()),
            }]
        );
    }

    #[test]
    fn errors_fail_with_each_message_and_location() {
        let report = fixture("check-error");
        assert!(report.failed());
        assert_eq!(report.errors, 2);
        assert_eq!(report.exit_code, Some(101));
        let errors: Vec<&Item> = report.of_kind(Kind::Error).collect();
        assert_eq!(errors.len(), 2);
        assert_eq!(
            errors[0].message,
            "error[E0425]: cannot find value `missing` in this scope"
        );
        assert_eq!(errors[0].location.as_deref(), Some("src/main.rs:4:26"));
        assert_eq!(errors[1].message, "error[E0308]: mismatched types");
        assert_eq!(errors[1].location.as_deref(), Some("src/main.rs:3:22"));
        assert!(report.items.iter().all(|i| !i.message.contains('\u{1b}')));
    }

    #[test]
    fn clippy_warnings_are_warnings() {
        let report = fixture("clippy-warnings");
        assert!(!report.failed());
        assert_eq!(report.warnings, 2);
        assert_eq!(report.of_kind(Kind::Warning).count(), 2);
        assert_eq!(report.of_kind(Kind::Error).count(), 0);
        assert!(
            report
                .items
                .iter()
                .all(|i| i.location.as_deref().is_some_and(|l| l.starts_with("src/")))
        );
    }

    #[test]
    fn a_failing_test_fails_and_names_the_test() {
        let report = fixture("test-failing");
        assert!(report.failed());
        assert_eq!(report.test_fails, 1);
        let tests: Vec<&Item> = report.of_kind(Kind::Test).collect();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].message, "failed: tests::adds_up");
        assert!(
            tests[0]
                .location
                .as_deref()
                .is_some_and(|l| l.contains("src/main.rs:9:9"))
        );
    }

    #[test]
    fn passing_tests_pass() {
        let report = fixture("test-passing");
        assert!(!report.failed());
        assert!(report.items.is_empty());
    }

    #[test]
    fn a_denied_warning_exit_code_fails_the_run() {
        let report =
            parse(r#"{"error_code": 101, "stats": {"warnings": 1}, "lines": []}"#).unwrap();
        assert!(report.failed());
        let report = parse(r#"{"error_code": null, "lines": []}"#).unwrap();
        assert!(!report.failed());
    }

    #[test]
    fn broken_reports_are_errors() {
        assert!(parse("").is_err());
        assert!(parse("[1, 2]").is_err());
        assert!(parse(r#"{"stats": "#).is_err());
    }

    #[test]
    fn escape_sequences_never_reach_the_text() {
        assert_eq!(
            strip_controls("\u{1b}[1m\u{1b}[92mok\u{1b}[0m\tdone\u{7}"),
            "ok done"
        );
    }
}
