//! What the watch mode draws over its scene: the large banner, the caption or the list of
//! problems, the key hint and the status bar. Everything is drawn into a frame, so it is tested
//! without a terminal.

use crate::colour::Rgb;
use crate::frame::Frame;
use crate::glyphs;
use crate::report::{Kind, Report};
use crate::theme::ThemeData;

/// Where the build stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// No run has finished yet.
    Waiting,
    Passing,
    Failing,
    /// bacon stopped on its own.
    Stopped,
}

/// Everything the overlay needs for one frame.
#[derive(Clone, Debug)]
pub struct View<'a> {
    pub project: &'a str,
    pub job: &'a str,
    pub phase: Phase,
    /// A run is under way, after an earlier result or before the first.
    pub running: bool,
    pub report: Option<&'a Report>,
    /// Seconds since the last result.
    pub ago: Option<u64>,
    pub caption: &'a str,
    /// Why bacon stopped, when it did.
    pub message: Option<&'a str>,
    /// Short notes for the status bar, such as `sound off`.
    pub notes: Vec<&'a str>,
    /// Seconds since the key hint was asked for, `None` once it has gone.
    pub hint_age: Option<f32>,
}

/// The rows of the banner font, five high. `#` is a filled cell.
fn glyph(ch: char) -> &'static [&'static str; 5] {
    match ch {
        'A' => &[".##.", "#..#", "####", "#..#", "#..#"],
        'B' => &["###.", "#..#", "###.", "#..#", "###."],
        'D' => &["###.", "#..#", "#..#", "#..#", "###."],
        'E' => &["####", "#...", "###.", "#...", "####"],
        'F' => &["####", "#...", "###.", "#...", "#..."],
        'G' => &[".###", "#...", "#.##", "#..#", ".###"],
        'I' => &["###", ".#.", ".#.", ".#.", "###"],
        'L' => &["#...", "#...", "#...", "#...", "####"],
        'O' => &[".##.", "#..#", "#..#", "#..#", ".##."],
        'U' => &["#..#", "#..#", "#..#", "#..#", ".##."],
        // the check mark and the cross that open each banner
        '+' => &["......#", ".....#.", "#...#..", ".#.#...", "..#...."],
        'x' => &["#...#", ".#.#.", "..#..", ".#.#.", "#...#"],
        _ => &["..", "..", "..", "..", ".."],
    }
}

/// The banner text as five rows of `#` and spaces, one column between letters.
pub fn banner_rows(text: &str) -> [String; 5] {
    let mut rows: [String; 5] = Default::default();
    for (i, ch) in text.chars().enumerate() {
        for (row, line) in rows.iter_mut().zip(glyph(ch)) {
            if i > 0 {
                row.push(' ');
            }
            row.extend(line.chars().map(|c| if c == '#' { '#' } else { ' ' }));
        }
    }
    rows
}

/// The banner's text with its leading mark: `+` for the check and `x` for the cross.
fn banner_text(phase: Phase) -> Option<&'static str> {
    match phase {
        Phase::Passing => Some("+ ALL GOOD"),
        Phase::Failing => Some("x BUILD FAILED"),
        _ => None,
    }
}

/// A colour if it keeps the theme's floor, otherwise the fallback, which always does.
fn keeping(theme: &ThemeData, wanted: Rgb, fallback: Rgb) -> Rgb {
    if theme.contrast_of(wanted) >= theme.floor() {
        wanted
    } else {
        fallback
    }
}

/// The theme's error colour. Its red stands in where the error colour is below the floor.
pub fn error_colour(theme: &ThemeData) -> Rgb {
    keeping(theme, theme.error, theme.red())
}

/// The theme's warning colour. Its yellow stands in where the warning colour is below the floor.
pub fn warning_colour(theme: &ThemeData) -> Rgb {
    keeping(theme, theme.warning, theme.yellow())
}

/// The colour of a passing build.
pub fn success_colour(theme: &ThemeData) -> Rgb {
    theme.green()
}

fn check_mark() -> char {
    // the square root sign is in the Linux and classic Windows console fonts; the check mark
    // is not
    if glyphs::basic() {
        '\u{221a}'
    } else {
        '\u{2713}'
    }
}

fn cross() -> char {
    if glyphs::basic() { 'x' } else { '\u{2717}' }
}

fn separator() -> &'static str {
    " \u{2502} "
}

fn ellipsis() -> &'static str {
    if glyphs::basic() { "..." } else { "\u{2026}" }
}

fn plural(count: u64, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// A duration in seconds as a short age: `4s ago`, `3m ago` or `2h ago`.
pub fn age_text(seconds: u64) -> String {
    match seconds {
        0..60 => format!("{seconds}s ago"),
        60..3600 => format!("{}m ago", seconds / 60),
        _ => format!("{}h ago", seconds / 3600),
    }
}

/// A run of text in one colour.
pub type Span = (String, Rgb);

/// The status bar as coloured runs, fitted to `width`. When it is too long the notes go first,
/// then the age and then the job. Whatever is still too long is cut short.
pub fn status_spans(view: &View, theme: &ThemeData, width: usize) -> Vec<Span> {
    let muted = theme.muted;
    let mut state: Vec<Span> = Vec::new();
    let warnings = view.report.map_or(0, |r| r.warnings);
    let warning_span = || {
        let colour = if warnings > 0 {
            warning_colour(theme)
        } else {
            muted
        };
        (
            format!(" \u{b7} {}", plural(warnings, "warning", "warnings")),
            colour,
        )
    };
    match view.phase {
        Phase::Passing => {
            state.push((format!("{} passing", check_mark()), success_colour(theme)));
            state.push(warning_span());
        }
        Phase::Failing => {
            state.push((format!("{} failing", cross()), error_colour(theme)));
            if let Some(report) = view.report {
                if report.errors > 0 {
                    state.push((
                        format!(" \u{b7} {}", plural(report.errors, "error", "errors")),
                        error_colour(theme),
                    ));
                }
                if report.test_fails > 0 {
                    state.push((
                        format!(
                            " \u{b7} {}",
                            plural(report.test_fails, "failed test", "failed tests")
                        ),
                        error_colour(theme),
                    ));
                }
            }
            state.push(warning_span());
        }
        Phase::Waiting => state.push(("running".to_string(), theme.foreground)),
        Phase::Stopped => state.push(("bacon stopped".to_string(), error_colour(theme))),
    }
    if view.running && view.phase != Phase::Waiting {
        state.push((" \u{b7} running".to_string(), theme.foreground));
    }

    let ago = view.ago.map(age_text);
    let build = |with_job: bool, with_ago: bool, notes: usize| {
        let mut spans: Vec<Span> = vec![(view.project.to_string(), theme.bright())];
        if with_job {
            spans.push((separator().to_string(), muted));
            spans.push((view.job.to_string(), theme.foreground));
        }
        spans.push((separator().to_string(), muted));
        spans.extend(state.iter().cloned());
        if let Some(ago) = ago.as_ref().filter(|_| with_ago) {
            spans.push((separator().to_string(), muted));
            spans.push((ago.clone(), muted));
        }
        for note in view.notes.iter().take(notes) {
            spans.push((separator().to_string(), muted));
            spans.push((note.to_string(), muted));
        }
        spans
    };
    let length = |spans: &[Span]| spans.iter().map(|(t, _)| t.chars().count()).sum::<usize>();
    let notes = view.notes.len();
    let tries = (0..=notes)
        .rev()
        .map(|n| (true, true, n))
        .chain([(true, false, 0), (false, false, 0)]);
    let mut chosen = Vec::new();
    for (with_job, with_ago, n) in tries {
        chosen = build(with_job, with_ago, n);
        if length(&chosen) <= width {
            return chosen;
        }
    }
    // still too long: cut the runs at the width
    let mut left = width;
    let mut cut = Vec::new();
    for (text, colour) in chosen {
        if left == 0 {
            break;
        }
        let taken: String = text.chars().take(left).collect();
        left -= taken.chars().count();
        cut.push((taken, colour));
    }
    cut
}

/// The status bar as plain text.
pub fn status_text(view: &View, theme: &ThemeData, width: usize) -> String {
    status_spans(view, theme, width)
        .into_iter()
        .map(|(text, _)| text)
        .collect()
}

/// The key hint for a width.
pub fn hint_text(width: usize) -> &'static str {
    if width >= 78 {
        "q quits, 1 to 4 switch the job, r reruns, p pauses, s turns sound on or off"
    } else if width >= 44 {
        "q quit  1-4 job  r rerun  p pause  s sound"
    } else {
        "q quit  r rerun  s sound"
    }
}

/// Cuts text to a width, ending with an ellipsis when anything was cut.
fn fit(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let mark = ellipsis();
    let keep = width.saturating_sub(mark.chars().count());
    let mut out: String = text.chars().take(keep).collect();
    out.push_str(mark);
    out.chars().take(width).collect()
}

/// One line of the panel under the banner.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub text: String,
    pub colour: Rgb,
}

/// The lines under the banner: the caption and any warnings while passing, the errors and
/// failing tests first and then the warnings while failing. Each problem is its message and,
/// indented below it, its location. Lines past `max` give way to a count of what is left.
pub fn detail_lines(view: &View, theme: &ThemeData, max: usize) -> Vec<Line> {
    let mut lines = Vec::new();
    let line = |text: String, colour: Rgb| Line { text, colour };
    match view.phase {
        Phase::Waiting => {
            lines.push(line(
                format!("Running the {} job with bacon", view.job),
                theme.foreground,
            ));
            lines.push(line(
                "The scene changes when the first run finishes.".to_string(),
                theme.muted,
            ));
            return lines;
        }
        Phase::Stopped => {
            lines.push(line("bacon stopped.".to_string(), error_colour(theme)));
            if let Some(message) = view.message.filter(|m| !m.is_empty()) {
                lines.push(line(message.to_string(), theme.foreground));
            }
            lines.push(line(
                "Press r to start it again or q to quit.".to_string(),
                theme.muted,
            ));
            return lines;
        }
        Phase::Passing => {
            lines.push(line(view.caption.to_string(), theme.foreground));
        }
        Phase::Failing => {}
    }
    let Some(report) = view.report else {
        return lines;
    };
    let mut items: Vec<_> = report
        .of_kind(Kind::Error)
        .chain(report.of_kind(Kind::Test))
        .collect();
    items.extend(report.of_kind(Kind::Warning));
    if !lines.is_empty() && !items.is_empty() {
        lines.push(line(String::new(), theme.muted));
    }
    let mut shown = 0;
    for item in &items {
        let needed = 1 + usize::from(item.location.is_some());
        // a line is kept back for the count of the rest unless this is the last item
        let reserve = usize::from(shown + 1 < items.len());
        if lines.len() + needed + reserve > max {
            break;
        }
        let (mark, colour) = match item.kind {
            Kind::Error | Kind::Test => (cross(), error_colour(theme)),
            Kind::Warning => ('!', warning_colour(theme)),
        };
        lines.push(line(format!("{mark} {}", item.message), colour));
        if let Some(location) = &item.location {
            lines.push(line(format!("  {location}"), theme.muted));
        }
        shown += 1;
    }
    if shown < items.len() {
        lines.push(line(
            format!("and {} more", items.len() - shown),
            theme.muted,
        ));
    }
    lines
}

/// Draws the overlay over a frame the scene has already drawn into.
pub fn draw(frame: &mut Frame, theme: &ThemeData, view: &View) {
    let width = frame.width();
    let height = frame.height();
    let status_row = height as i32 - 1;

    // the status bar
    frame.fill_row(status_row, ' ', None);
    let mut x = 0;
    for (text, colour) in status_spans(view, theme, width) {
        frame.put_str(x, status_row, &text, Some(colour));
        x += text.chars().count() as i32;
    }

    // the key hint, shown then muted then gone like the scenes' help line, so it never flashes
    let mut bottom = status_row;
    let hint_colour = match view.hint_age {
        Some(age) if age < crate::app::HELP_SHOWN => Some(theme.foreground),
        Some(age) if age < crate::app::HELP_SHOWN + crate::app::HELP_FADING => Some(theme.muted),
        _ => None,
    };
    if let Some(colour) = hint_colour.filter(|_| height >= 4) {
        bottom -= 1;
        frame.fill_row(bottom, ' ', None);
        frame.put_str(0, bottom, &fit(hint_text(width), width), Some(colour));
    }

    // the panel: banner, then the details, centred in the rows above
    let room = bottom.max(0) as usize;
    let banner = banner_text(view.phase);
    let rows = banner.map(banner_rows);
    let big = rows
        .as_ref()
        .is_some_and(|r| r[0].chars().count() + 4 <= width && room >= 12);
    let banner_height = match (banner, big) {
        (None, _) => 0,
        (Some(_), true) => 6,
        (Some(_), false) => 2,
    };
    let inner = width.saturating_sub(4).max(1);
    let details = detail_lines(view, theme, room.saturating_sub(banner_height + 2));
    let content_height = banner_height + details.len();
    if content_height == 0 || room == 0 {
        return;
    }
    let content_width = details
        .iter()
        .map(|l| l.text.chars().count())
        .chain(rows.iter().filter(|_| big).map(|r| r[0].chars().count()))
        .chain(banner.filter(|_| !big).map(|b| b.chars().count()))
        .max()
        .unwrap_or(0)
        .min(inner);
    let panel_width = (content_width + 4).min(width);
    let panel_height = (content_height + 2).min(room);
    let left = (width - panel_width) / 2;
    let top = (room - panel_height) / 2;
    // the panel is cleared to the background so the scene never runs through the text
    for y in top..top + panel_height {
        for x in left..left + panel_width {
            frame.put(x as i32, y as i32, ' ', None);
        }
    }
    let text_left = (left + 2) as i32;
    let mut y = (top + 1) as i32;
    let banner_colour = if view.phase == Phase::Passing {
        success_colour(theme)
    } else {
        error_colour(theme)
    };
    if let Some(text) = banner {
        if big {
            let rows = rows.as_ref().expect("a big banner has rows");
            let offset = (content_width.saturating_sub(rows[0].chars().count()) / 2) as i32;
            for row in rows {
                for (dx, ch) in row.chars().enumerate() {
                    if ch == '#' {
                        frame.put(
                            text_left + offset + dx as i32,
                            y,
                            '\u{2588}',
                            Some(banner_colour),
                        );
                    }
                }
                y += 1;
            }
        } else {
            let mark = if view.phase == Phase::Passing {
                check_mark()
            } else {
                cross()
            };
            let line = format!("{mark}{}", &text[1..]);
            frame.put_str(text_left, y, &fit(&line, inner), Some(banner_colour));
            y += 1;
        }
        y += 1;
    }
    for line in &details {
        if y >= (top + panel_height) as i32 {
            break;
        }
        frame.put_str(text_left, y, &fit(&line.text, inner), Some(line.colour));
        y += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{self, Item};
    use crate::theme;

    fn passing() -> Report {
        Report {
            warnings: 2,
            items: vec![
                Item {
                    kind: Kind::Warning,
                    message: "warning: unused variable: `x`".into(),
                    location: Some("src/main.rs:2:9".into()),
                },
                Item {
                    kind: Kind::Warning,
                    message: "warning: function `f` is never used".into(),
                    location: Some("src/lib.rs:7:4".into()),
                },
            ],
            ..Report::default()
        }
    }

    fn failing() -> Report {
        report::parse(include_str!("../../tests/fixtures/bacon/check-error.json")).unwrap()
    }

    fn view<'a>(phase: Phase, report: Option<&'a Report>) -> View<'a> {
        View {
            project: "sample",
            job: "check",
            phase,
            running: false,
            report,
            ago: Some(12),
            caption: "Everything builds.",
            message: None,
            notes: Vec::new(),
            hint_age: None,
        }
    }

    #[test]
    fn the_status_bar_reads_project_job_state_and_age() {
        let theme = theme::default_theme();
        let report = passing();
        let text = status_text(&view(Phase::Passing, Some(&report)), theme, 120);
        assert_eq!(
            text,
            "sample \u{2502} check \u{2502} \u{2713} passing \u{b7} 2 warnings \u{2502} 12s ago"
        );
        let clean = Report::default();
        let text = status_text(&view(Phase::Passing, Some(&clean)), theme, 120);
        assert!(
            text.contains("\u{2713} passing \u{b7} 0 warnings"),
            "{text}"
        );
        let one = Report {
            warnings: 1,
            ..Report::default()
        };
        let text = status_text(&view(Phase::Passing, Some(&one)), theme, 120);
        assert!(text.contains("1 warning \u{2502}"), "{text}");
    }

    #[test]
    fn the_status_bar_says_failing_in_the_error_colour() {
        let theme = theme::default_theme();
        let report = failing();
        let spans = status_spans(&view(Phase::Failing, Some(&report)), theme, 120);
        let text: String = spans.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(
            text,
            "sample \u{2502} check \u{2502} \u{2717} failing \u{b7} 2 errors \u{b7} 0 warnings \u{2502} 12s ago"
        );
        let failing_span = spans.iter().find(|(t, _)| t.contains("failing")).unwrap();
        assert_eq!(failing_span.1, error_colour(theme));
    }

    #[test]
    fn the_status_bar_shows_running_notes_and_failed_tests() {
        let theme = theme::default_theme();
        let report =
            report::parse(include_str!("../../tests/fixtures/bacon/test-failing.json")).unwrap();
        let mut v = view(Phase::Failing, Some(&report));
        v.job = "test";
        v.running = true;
        v.notes = vec!["no sound player"];
        let text = status_text(&v, theme, 200);
        assert!(text.contains("1 failed test"), "{text}");
        assert!(text.contains("\u{b7} running"), "{text}");
        assert!(text.ends_with("\u{2502} no sound player"), "{text}");
        let waiting = status_text(&view(Phase::Waiting, None), theme, 120);
        assert_eq!(
            waiting,
            "sample \u{2502} check \u{2502} running \u{2502} 12s ago"
        );
    }

    #[test]
    fn the_status_bar_fits_every_width() {
        let theme = theme::default_theme();
        let report = failing();
        let mut v = view(Phase::Failing, Some(&report));
        v.notes = vec!["sound off", "paused"];
        for width in 1..=120 {
            let text = status_text(&v, theme, width);
            assert!(text.chars().count() <= width, "{width}: {text}");
        }
        // the state outlasts the notes, the age and the job
        let narrow = status_text(&v, theme, 40);
        assert!(narrow.contains("failing"), "{narrow}");
        assert!(!narrow.contains("paused"), "{narrow}");
    }

    #[test]
    fn ages_read_in_seconds_minutes_and_hours() {
        assert_eq!(age_text(0), "0s ago");
        assert_eq!(age_text(59), "59s ago");
        assert_eq!(age_text(60), "1m ago");
        assert_eq!(age_text(3599), "59m ago");
        assert_eq!(age_text(7200), "2h ago");
    }

    #[test]
    fn the_banners_are_five_rows_of_even_width() {
        for text in ["+ ALL GOOD", "x BUILD FAILED"] {
            let rows = banner_rows(text);
            let width = rows[0].chars().count();
            assert!(rows.iter().all(|r| r.chars().count() == width), "{text}");
            assert!(width + 4 <= 80, "{text} is {width} wide");
        }
    }

    #[test]
    fn passing_shows_the_banner_caption_and_warnings() {
        let theme = theme::default_theme();
        let report = passing();
        let mut frame = Frame::new(80, 24);
        draw(&mut frame, theme, &view(Phase::Passing, Some(&report)));
        let text = frame.to_text();
        assert!(text.contains('\u{2588}'));
        assert!(text.contains("Everything builds."));
        assert!(text.contains("! warning: unused variable: `x`"));
        assert!(text.contains("  src/main.rs:2:9"));
        assert!(frame.row_text(23).starts_with("sample"));
        let drawn = frame.colours();
        assert!(drawn.contains(&success_colour(theme)));
        assert!(drawn.contains(&warning_colour(theme)));
        assert!(!drawn.contains(&error_colour(theme)));
    }

    #[test]
    fn failing_lists_the_errors_before_the_warnings() {
        let theme = theme::default_theme();
        let mut report = failing();
        report.items.extend(passing().items);
        let mut frame = Frame::new(100, 30);
        draw(&mut frame, theme, &view(Phase::Failing, Some(&report)));
        let text = frame.to_text();
        let error = text.find("cannot find value `missing`").unwrap();
        let warning = text.find("! warning").unwrap();
        assert!(error < warning);
        assert!(text.contains("src/main.rs:4:26"));
        assert!(frame.colours().contains(&error_colour(theme)));
    }

    #[test]
    fn a_long_list_ends_with_a_count_of_the_rest() {
        let theme = theme::default_theme();
        let mut report = Report::default();
        for i in 0..30 {
            report.items.push(Item {
                kind: Kind::Error,
                message: format!("error: number {i}"),
                location: Some(format!("src/main.rs:{i}:1")),
            });
        }
        let lines = detail_lines(&view(Phase::Failing, Some(&report)), theme, 9);
        assert_eq!(lines.len(), 9);
        assert_eq!(lines.last().unwrap().text, "and 26 more");
    }

    #[test]
    fn a_small_terminal_gets_a_one_line_banner() {
        let theme = theme::default_theme();
        let report = failing();
        let mut frame = Frame::new(40, 10);
        draw(&mut frame, theme, &view(Phase::Failing, Some(&report)));
        let text = frame.to_text();
        assert!(text.contains("\u{2717} BUILD FAILED"), "{text}");
        assert!(!text.contains('\u{2588}'));
        for (w, h) in [(1, 1), (5, 2), (20, 8), (30, 3)] {
            let mut tiny = Frame::new(w, h);
            draw(&mut tiny, theme, &view(Phase::Failing, Some(&report)));
        }
    }

    #[test]
    fn waiting_and_stopped_say_what_is_happening() {
        let theme = theme::default_theme();
        let mut frame = Frame::new(80, 24);
        draw(&mut frame, theme, &view(Phase::Waiting, None));
        assert!(frame.to_text().contains("Running the check job with bacon"));
        let mut stopped = view(Phase::Stopped, None);
        stopped.message = Some("error: no such job");
        frame.clear();
        draw(&mut frame, theme, &stopped);
        let text = frame.to_text();
        assert!(text.contains("bacon stopped."));
        assert!(text.contains("error: no such job"));
        assert!(text.contains("Press r"));
    }

    #[test]
    fn the_hint_shows_then_dims_then_goes() {
        let theme = theme::default_theme();
        let mut v = view(Phase::Waiting, None);
        let mut frame = Frame::new(80, 24);
        v.hint_age = Some(0.0);
        draw(&mut frame, theme, &v);
        assert!(frame.row_text(22).starts_with("q quits"));
        assert_eq!(frame.get(0, 22).unwrap().colour, Some(theme.foreground));
        v.hint_age = Some(crate::app::HELP_SHOWN + 0.5);
        frame.clear();
        draw(&mut frame, theme, &v);
        assert_eq!(frame.get(0, 22).unwrap().colour, Some(theme.muted));
        v.hint_age = None;
        frame.clear();
        draw(&mut frame, theme, &v);
        assert!(!frame.row_text(22).starts_with("q quits"));
        for width in [20, 44, 78, 120] {
            assert!(hint_text(width).chars().count() <= width.max(24));
        }
    }

    #[test]
    fn every_overlay_colour_keeps_the_floor_in_every_theme() {
        let pass = passing();
        let mut fail = failing();
        fail.items.extend(passing().items);
        for theme in theme::all() {
            assert!(theme.contrast_of(error_colour(theme)) >= theme.floor());
            assert!(theme.contrast_of(warning_colour(theme)) >= theme.floor());
            assert_ne!(error_colour(theme), warning_colour(theme), "{}", theme.name);
            assert_ne!(error_colour(theme), success_colour(theme), "{}", theme.name);
            for (phase, report) in [
                (Phase::Passing, Some(&pass)),
                (Phase::Failing, Some(&fail)),
                (Phase::Waiting, None),
                (Phase::Stopped, None),
            ] {
                let mut v = view(phase, report);
                v.hint_age = Some(crate::app::HELP_SHOWN + 0.5);
                v.notes = vec!["sound off"];
                let mut frame = Frame::new(100, 30);
                draw(&mut frame, theme, &v);
                for colour in frame.colours() {
                    let ratio = theme.contrast_of(colour);
                    assert!(
                        ratio >= theme.floor(),
                        "{} {} at {ratio:.2}:1 in {:?}",
                        theme.name,
                        colour.hex(),
                        phase
                    );
                }
            }
        }
    }
}
