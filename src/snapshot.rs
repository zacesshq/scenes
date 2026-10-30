//! Snapshots: a scene drawn with the real renderer and no terminal, written as a self-contained
//! SVG still in a terminal window frame or as JSON frames a web page can play. Both merge runs of
//! one colour so the files stay small. The frames go through the same flash meter as a user
//! scene, so a page playing them can never flash.

use crate::catalogue::Catalogue;
use crate::cli::SnapshotState;
use crate::colour::Rgb;
use crate::flash::{Meter, Verdict};
use crate::frame::{Cell, Frame};
use crate::report::{Item, Kind, Report};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::{self, Phase, View};

/// Steps simulated per second while running a scene up to the moment of a still.
const STEP_RATE: f32 = 20.0;
/// The width of a cell in the SVG, in pixels, for a 14 pixel monospace font.
const CELL_WIDTH: f32 = 8.4;
/// The height of a row in the SVG.
const ROW_HEIGHT: f32 = 17.0;
const FONT_SIZE: f32 = 14.0;
/// The padding inside the window and the height of its title bar.
const PAD: f32 = 14.0;
const TITLE_BAR: f32 = 30.0;

/// A run of cells in one colour on one row.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub x: usize,
    pub y: usize,
    pub text: String,
    pub colour: Rgb,
}

/// The runs of one frame. Spaces join the run they sit in, since they draw nothing. A run
/// that is only spaces is left out.
pub fn runs(frame: &Frame, theme: &ThemeData) -> Vec<Run> {
    text_runs(frame, theme, false)
}

/// The runs to draw as text: like `runs`, but block glyphs are left to `block_rects`.
fn text_runs(frame: &Frame, theme: &ThemeData, skip_blocks: bool) -> Vec<Run> {
    let mut out = Vec::new();
    for (y, row) in frame.rows().enumerate() {
        let mut current: Option<Run> = None;
        let mut pending_spaces = 0;
        for (x, cell) in row.iter().enumerate() {
            if skip_blocks && block_shape(cell.ch).is_some() {
                // a block is drawn as a rectangle, so the text on each side becomes its own run;
                // joined across it, some renderers squeeze the gap and stretch the run over it
                out.extend(current.take());
                pending_spaces = 0;
                continue;
            }
            if cell.ch == ' ' {
                if current.is_some() {
                    pending_spaces += 1;
                }
                continue;
            }
            if skip_blocks && pending_spaces > 1 {
                // the same for a wide gap: renderers that collapse spaces would stretch the run
                out.extend(current.take());
                pending_spaces = 0;
            }
            let colour = cell.colour.unwrap_or(theme.foreground);
            match current.as_mut() {
                Some(run) if run.colour == colour => {
                    run.text.extend(std::iter::repeat_n(' ', pending_spaces));
                    run.text.push(cell.ch);
                }
                _ => {
                    if let Some(run) = current.take() {
                        out.push(run);
                    }
                    current = Some(Run {
                        x,
                        y,
                        text: cell.ch.to_string(),
                        colour,
                    });
                }
            }
            pending_spaces = 0;
        }
        out.extend(current);
    }
    out
}

/// The parts of a cell a block glyph covers as (left, top, right, bottom) fractions plus the
/// opacity of the shade glyphs. Fonts draw these with gaps between cells, so a still draws them
/// as rectangles to show the solid shapes a terminal shows.
/// Part of a cell as (left, top, right, bottom) fractions.
type Part = (f32, f32, f32, f32);

fn block_shape(ch: char) -> Option<(&'static [Part], f32)> {
    const FULL: &[Part] = &[(0.0, 0.0, 1.0, 1.0)];
    const TL: Part = (0.0, 0.0, 0.5, 0.5);
    const TR: Part = (0.5, 0.0, 1.0, 0.5);
    const BL: Part = (0.0, 0.5, 0.5, 1.0);
    const BR: Part = (0.5, 0.5, 1.0, 1.0);
    let shape: (&'static [Part], f32) = match ch {
        '\u{2588}' => (FULL, 1.0),
        '\u{2580}' => (&[(0.0, 0.0, 1.0, 0.5)], 1.0),
        '\u{2584}' => (&[(0.0, 0.5, 1.0, 1.0)], 1.0),
        '\u{258c}' => (&[(0.0, 0.0, 0.5, 1.0)], 1.0),
        '\u{2590}' => (&[(0.5, 0.0, 1.0, 1.0)], 1.0),
        '\u{2581}' => (&[(0.0, 0.875, 1.0, 1.0)], 1.0),
        '\u{2582}' => (&[(0.0, 0.75, 1.0, 1.0)], 1.0),
        '\u{2583}' => (&[(0.0, 0.625, 1.0, 1.0)], 1.0),
        '\u{2585}' => (&[(0.0, 0.375, 1.0, 1.0)], 1.0),
        '\u{2586}' => (&[(0.0, 0.25, 1.0, 1.0)], 1.0),
        '\u{2587}' => (&[(0.0, 0.125, 1.0, 1.0)], 1.0),
        '\u{2589}' => (&[(0.0, 0.0, 0.875, 1.0)], 1.0),
        '\u{258a}' => (&[(0.0, 0.0, 0.75, 1.0)], 1.0),
        '\u{258b}' => (&[(0.0, 0.0, 0.625, 1.0)], 1.0),
        '\u{258d}' => (&[(0.0, 0.0, 0.375, 1.0)], 1.0),
        '\u{258e}' => (&[(0.0, 0.0, 0.25, 1.0)], 1.0),
        '\u{258f}' => (&[(0.0, 0.0, 0.125, 1.0)], 1.0),
        '\u{2594}' => (&[(0.0, 0.0, 1.0, 0.125)], 1.0),
        '\u{2595}' => (&[(0.875, 0.0, 1.0, 1.0)], 1.0),
        '\u{2591}' => (FULL, 0.25),
        '\u{2592}' => (FULL, 0.5),
        '\u{2593}' => (FULL, 0.75),
        '\u{2596}' => (&[BL], 1.0),
        '\u{2597}' => (&[BR], 1.0),
        '\u{2598}' => (&[TL], 1.0),
        '\u{259d}' => (&[TR], 1.0),
        '\u{2599}' => (&[TL, BL, BR], 1.0),
        '\u{259a}' => (&[TL, BR], 1.0),
        '\u{259b}' => (&[TL, TR, BL], 1.0),
        '\u{259c}' => (&[TL, TR, BR], 1.0),
        '\u{259e}' => (&[TR, BL], 1.0),
        '\u{259f}' => (&[TR, BL, BR], 1.0),
        _ => return None,
    };
    Some(shape)
}

/// The SVG rectangles for every block glyph in a frame. Neighbours on a row with the same glyph
/// and colour share one rectangle, which keeps the files small.
fn block_rects(frame: &Frame, theme: &ThemeData, left: f32, top: f32) -> String {
    let mut out = String::new();
    for (y, row) in frame.rows().enumerate() {
        let mut x = 0;
        while x < row.len() {
            let cell = row[x];
            let Some((parts, opacity)) = block_shape(cell.ch) else {
                x += 1;
                continue;
            };
            let colour = cell_colour(cell, theme);
            let mut end = x + 1;
            while end < row.len()
                && row[end].ch == cell.ch
                && cell_colour(row[end], theme) == colour
            {
                end += 1;
            }
            let count = (end - x) as f32;
            let fill_opacity = if opacity < 1.0 {
                format!(" fill-opacity=\"{}\"", round(opacity))
            } else {
                String::new()
            };
            for &(l, t, r, b) in parts {
                // a part that spans the cell's whole width can stretch over the run; any other
                // part repeats in every cell of it
                let repeats: Vec<(f32, f32)> = if l == 0.0 && r == 1.0 {
                    vec![(x as f32, count)]
                } else {
                    (x..end).map(|i| (i as f32 + l, r - l)).collect()
                };
                for (start, span) in repeats {
                    out.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"{fill_opacity}/>\n",
                        round(left + start * CELL_WIDTH),
                        round(top + (y as f32 + t) * ROW_HEIGHT),
                        round(span * CELL_WIDTH),
                        round((b - t) * ROW_HEIGHT),
                        colour.hex()
                    ));
                }
            }
            x = end;
        }
    }
    out
}

fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

fn json_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// The SVG of a frame in a terminal window frame with a title, described for screen readers by
/// `label`.
pub fn svg(frame: &Frame, theme: &ThemeData, title: &str, label: &str) -> String {
    let inner_width = frame.width() as f32 * CELL_WIDTH;
    let inner_height = frame.height() as f32 * ROW_HEIGHT;
    let width = inner_width + 2.0 * PAD;
    let height = inner_height + 2.0 * PAD + TITLE_BAR;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" \
viewBox=\"0 0 {width} {height}\" role=\"img\" aria-label=\"{label}\">\n<title>{label}</title>\n",
        label = xml_escape(label),
    ));
    // the window: the theme's background with an edge in its muted colour
    out.push_str(&format!(
        "<rect x=\"0.5\" y=\"0.5\" width=\"{}\" height=\"{}\" rx=\"10\" fill=\"{}\" stroke=\"{}\"/>\n",
        width - 1.0,
        height - 1.0,
        theme.background.hex(),
        theme.muted.hex()
    ));
    out.push_str(&format!(
        "<line x1=\"0.5\" y1=\"{TITLE_BAR}\" x2=\"{}\" y2=\"{TITLE_BAR}\" stroke=\"{}\"/>\n",
        width - 0.5,
        theme.muted.hex()
    ));
    for (i, colour) in [theme.red(), theme.yellow(), theme.green()]
        .iter()
        .enumerate()
    {
        out.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\"/>\n",
            PAD + 4.0 + i as f32 * 16.0,
            TITLE_BAR / 2.0,
            colour.hex()
        ));
    }
    let font = "ui-monospace,SFMono-Regular,Menlo,Consolas,'DejaVu Sans Mono',monospace";
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" fill=\"{}\" font-family=\"{font}\" font-size=\"13\" \
text-anchor=\"middle\">{}</text>\n",
        width / 2.0,
        TITLE_BAR / 2.0 + 4.5,
        theme.foreground.hex(),
        xml_escape(title)
    ));
    out.push_str(&block_rects(frame, theme, PAD, TITLE_BAR + PAD));
    out.push_str(&format!(
        "<g font-family=\"{font}\" font-size=\"{FONT_SIZE}\" xml:space=\"preserve\">\n"
    ));
    for run in text_runs(frame, theme, true) {
        let x = PAD + run.x as f32 * CELL_WIDTH;
        let y = TITLE_BAR + PAD + run.y as f32 * ROW_HEIGHT + FONT_SIZE - 1.0;
        let length = run.text.chars().count() as f32 * CELL_WIDTH;
        // the length is fixed per run, so a font whose block characters are wider or narrower
        // than its letters still lines every run up with the grid
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" fill=\"{}\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\">{}</text>\n",
            round(x),
            round(y),
            run.colour.hex(),
            round(length),
            xml_escape(&run.text)
        ));
    }
    out.push_str("</g>\n</svg>\n");
    out
}

/// A number with at most two decimals and no trailing zeros.
fn round(value: f32) -> String {
    let text = format!("{value:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Frames as compact JSON: the size, the rate, the background and each frame's runs.
pub fn frames_json(frames: &[Frame], fps: u32, theme: &ThemeData) -> String {
    let (width, height) = frames.first().map_or((0, 0), |f| (f.width(), f.height()));
    let mut out = format!(
        "{{\"width\":{width},\"height\":{height},\"fps\":{fps},\"background\":\"{}\",\"frames\":[",
        theme.background.hex()
    );
    for (i, frame) in frames.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('[');
        for (j, run) in runs(frame, theme).iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            out.push_str(&format!(
                "{{\"x\":{},\"y\":{},\"text\":\"{}\",\"colour\":\"{}\"}}",
                run.x,
                run.y,
                json_escape(&run.text),
                run.colour.hex()
            ));
        }
        out.push(']');
    }
    out.push_str("]}\n");
    out
}

/// What a snapshot draws.
pub struct Shot<'a> {
    pub catalogue: &'a Catalogue,
    pub scene: &'a str,
    pub theme: &'a ThemeData,
    pub size: (usize, usize),
    pub seed: u64,
    pub motion: Motion,
    pub state: Option<SnapshotState>,
}

impl Shot<'_> {
    fn build(&self, rng: &mut Rng) -> Result<Box<dyn Scene>, String> {
        self.catalogue
            .create(self.scene, self.size.0, self.size.1, self.motion, rng)
    }

    /// The sample report a watch mode shot shows.
    fn report(&self) -> Option<Report> {
        match self.state? {
            SnapshotState::Pass => Some(Report::default()),
            SnapshotState::Fail => Some(Report {
                errors: 2,
                warnings: 1,
                exit_code: Some(101),
                items: vec![
                    Item {
                        kind: Kind::Error,
                        message: "error[E0308]: mismatched types".into(),
                        location: Some("src/main.rs:14:22".into()),
                    },
                    Item {
                        kind: Kind::Error,
                        message: "error[E0425]: cannot find value `count` in this scope".into(),
                        location: Some("src/lib.rs:31:9".into()),
                    },
                    Item {
                        kind: Kind::Warning,
                        message: "warning: unused variable: `total`".into(),
                        location: Some("src/lib.rs:27:9".into()),
                    },
                ],
                ..Report::default()
            }),
        }
    }

    /// Draws one frame of the scene, with the watch mode's banner, caption and status bar on
    /// top when a state is asked for.
    fn draw(&self, scene: &dyn Scene, report: Option<&Report>, frame: &mut Frame) {
        frame.clear();
        scene.draw(frame, self.theme);
        if let Some(state) = self.state {
            let caption = self
                .catalogue
                .find(self.scene)
                .and_then(|e| e.captions.first().cloned());
            let view = View {
                project: "my-project",
                job: "check",
                mode: crate::watch::Mode::Bacon,
                typo: false,
                phase: match state {
                    SnapshotState::Pass => Phase::Passing,
                    SnapshotState::Fail => Phase::Failing,
                },
                running: false,
                report,
                ago: Some(12),
                caption: caption.as_deref(),
                banner: true,
                recovered: false,
                alert: None,
                message: None,
                notes: Vec::new(),
                hint_age: None,
            };
            view::draw(frame, self.theme, &view);
        }
    }

    /// The still `time` seconds into the scene.
    pub fn still(&self, time: f32) -> Result<Frame, String> {
        let mut rng = Rng::new(self.seed);
        let mut scene = self.build(&mut rng)?;
        let steps = (time * STEP_RATE).round() as usize;
        for _ in 0..steps {
            scene.step(1.0 / STEP_RATE, &mut rng);
        }
        if let Some(problem) = scene.problem() {
            return Err(problem);
        }
        let report = self.report();
        let mut frame = Frame::new(self.size.0, self.size.1);
        self.draw(scene.as_ref(), report.as_ref(), &mut frame);
        Ok(frame)
    }

    /// `count` frames at `fps`, starting `time` seconds in. A frame that would change a large
    /// area too often is replaced by the one before it, as on screen.
    pub fn frames(&self, time: f32, count: u32, fps: u32) -> Result<Vec<Frame>, String> {
        let mut rng = Rng::new(self.seed);
        let mut scene = self.build(&mut rng)?;
        for _ in 0..(time * STEP_RATE).round() as usize {
            scene.step(1.0 / STEP_RATE, &mut rng);
        }
        let report = self.report();
        let dt = 1.0 / fps.max(1) as f32;
        let mut meter = Meter::new();
        let mut frames: Vec<Frame> = Vec::with_capacity(count as usize);
        let mut frame = Frame::new(self.size.0, self.size.1);
        for _ in 0..count {
            scene.step(dt, &mut rng);
            meter.advance(dt);
            self.draw(scene.as_ref(), report.as_ref(), &mut frame);
            match (meter.judge(&frame, self.theme), frames.last()) {
                (Verdict::Hold, Some(last)) => frames.push(last.clone()),
                _ => frames.push(frame.clone()),
            }
        }
        if let Some(problem) = scene.problem() {
            return Err(problem);
        }
        Ok(frames)
    }
}

/// The words a screen reader hears for a still.
pub fn label(
    scene: &str,
    summary: &str,
    theme: &ThemeData,
    state: Option<SnapshotState>,
) -> String {
    let state = match state {
        Some(SnapshotState::Pass) => {
            " The watch mode is showing ALL GOOD with a check mark and a status bar reading passing."
        }
        Some(SnapshotState::Fail) => {
            " The watch mode is showing BUILD FAILED with a cross, two errors with their locations and a status bar reading failing."
        }
        None => "",
    };
    let summary = summary.trim_end_matches('.');
    format!(
        "The {scene} scene from Zacess Scenes in {}: {summary}.{state}",
        theme.name
    )
}

/// Every colour a cell can hold in a theme, for checking a snapshot.
pub fn cell_colour(cell: Cell, theme: &ThemeData) -> Rgb {
    cell.colour.unwrap_or(theme.foreground)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flash::{MAX_CHANGES_PER_SECOND, most_changes_per_second, regions};
    use crate::theme;

    fn shot<'a>(catalogue: &'a Catalogue, scene: &'a str, theme: &'a ThemeData) -> Shot<'a> {
        Shot {
            catalogue,
            scene,
            theme,
            size: (60, 20),
            seed: 1,
            motion: Motion::FULL,
            state: None,
        }
    }

    /// A small check that a document is well formed XML: every tag closes in order, every
    /// attribute is quoted and no bare ampersand or angle bracket appears in text.
    fn well_formed(xml: &str) -> Result<(), String> {
        let mut stack: Vec<String> = Vec::new();
        let mut rest = xml;
        while let Some(open) = rest.find('<') {
            let text = &rest[..open];
            if text.contains('>') {
                return Err(format!("a bare > in {text:?}"));
            }
            for (i, _) in text.match_indices('&') {
                let entity = &text[i..];
                if !["&amp;", "&lt;", "&gt;", "&quot;", "&apos;"]
                    .iter()
                    .any(|e| entity.starts_with(e))
                {
                    return Err(format!("a bare & in {text:?}"));
                }
            }
            let close = rest[open..].find('>').ok_or("an unclosed tag")? + open;
            let tag = &rest[open + 1..close];
            if let Some(name) = tag.strip_prefix('/') {
                let top = stack.pop().ok_or("a closing tag with nothing open")?;
                if top != name.trim() {
                    return Err(format!("</{name}> closes <{top}>"));
                }
            } else {
                let self_closing = tag.ends_with('/');
                let body = tag.trim_end_matches('/');
                let name = body.split_whitespace().next().ok_or("an empty tag")?;
                if body.matches('"').count() % 2 != 0 {
                    return Err(format!("unbalanced quotes in <{body}>"));
                }
                if !self_closing {
                    stack.push(name.to_string());
                }
            }
            rest = &rest[close + 1..];
        }
        if stack.is_empty() {
            Ok(())
        } else {
            Err(format!("{stack:?} never closed"))
        }
    }

    #[test]
    fn runs_merge_one_colour_and_skip_blank_space() {
        let theme = theme::default_theme();
        let mut frame = Frame::new(12, 2);
        frame.put_str(1, 0, "ab", Some(theme.cyan()));
        frame.put_str(5, 0, "cd", Some(theme.cyan()));
        frame.put_str(7, 0, "e", Some(theme.red()));
        let runs = runs(&frame, theme);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text, "ab  cd");
        assert_eq!((runs[0].x, runs[0].y), (1, 0));
        assert_eq!(runs[1].text, "e");
    }

    #[test]
    fn every_still_is_well_formed_and_uses_only_palette_colours() {
        let catalogue = Catalogue::built_in();
        for theme in theme::all() {
            for entry in catalogue.entries.iter().step_by(4) {
                let shot = shot(&catalogue, &entry.name, theme);
                let frame = shot.still(3.0).unwrap();
                let label = label(&entry.name, &entry.summary, theme, None);
                let svg = svg(
                    &frame,
                    theme,
                    &format!("{} \u{b7} {}", entry.name, theme.name),
                    &label,
                );
                well_formed(&svg).unwrap_or_else(|e| panic!("{}: {e}", entry.name));
                assert!(svg.contains("role=\"img\""));
                assert!(svg.contains("aria-label=\"The "));
                let allowed = {
                    let mut c = theme.drawable();
                    c.push(theme.background);
                    c
                };
                for cell in frame.cells() {
                    let colour = cell_colour(*cell, theme);
                    assert!(
                        allowed.contains(&colour),
                        "{} draws {}",
                        entry.name,
                        colour.hex()
                    );
                }
            }
        }
    }

    #[test]
    fn a_still_escapes_text_and_finds_a_bad_document() {
        let theme = theme::default_theme();
        let mut frame = Frame::new(10, 1);
        frame.put_str(0, 0, "<&>\"'", Some(theme.cyan()));
        let svg = svg(&frame, theme, "a & b", "x < y");
        well_formed(&svg).unwrap();
        assert!(svg.contains("&lt;&amp;&gt;&quot;&apos;"));
        assert!(well_formed("<svg><g></svg>").is_err());
        assert!(well_formed("<svg>a & b</svg>").is_err());
    }

    #[test]
    fn the_same_seed_gives_the_same_output() {
        let catalogue = Catalogue::built_in();
        let theme = theme::default_theme();
        let a = shot(&catalogue, "koi-pond", theme).still(2.0).unwrap();
        let b = shot(&catalogue, "koi-pond", theme).still(2.0).unwrap();
        assert_eq!(svg(&a, theme, "t", "l"), svg(&b, theme, "t", "l"));
        let fa = shot(&catalogue, "fog", theme).frames(1.0, 12, 12).unwrap();
        let fb = shot(&catalogue, "fog", theme).frames(1.0, 12, 12).unwrap();
        assert_eq!(frames_json(&fa, 12, theme), frames_json(&fb, 12, theme));
    }

    #[test]
    fn watch_states_show_the_banner_caption_and_status() {
        let catalogue = Catalogue::built_in();
        let theme = theme::default_theme();
        let mut pass = shot(&catalogue, "waves", theme);
        pass.size = (80, 24);
        pass.state = Some(SnapshotState::Pass);
        let text = pass.still(2.0).unwrap().to_text();
        assert!(
            text.contains('\u{2588}') && text.contains("passing"),
            "{text}"
        );
        assert!(text.contains(crate::scenes::info("waves").unwrap().captions[0]));
        let mut fail = shot(&catalogue, "storm", theme);
        fail.size = (80, 24);
        fail.state = Some(SnapshotState::Fail);
        let text = fail.still(2.0).unwrap().to_text();
        assert!(text.contains("failing") && text.contains("E0308"), "{text}");
    }

    #[test]
    fn frames_are_compact_json_and_never_flash() {
        let catalogue = Catalogue::built_in();
        let theme = theme::default_theme();
        let frames = shot(&catalogue, "storm", theme)
            .frames(0.0, 72, 12)
            .unwrap();
        let json = frames_json(&frames, 12, theme);
        assert!(json.starts_with("{\"width\":60,\"height\":20,\"fps\":12,\"background\":\"#"));
        assert!(crate::json::parse(&json).is_ok());
        let measured: Vec<Vec<f32>> = frames.iter().map(|f| regions(f, theme)).collect();
        assert!(most_changes_per_second(&measured, 12) <= MAX_CHANGES_PER_SECOND);
    }

    #[test]
    fn a_lua_scene_can_be_snapshotted() {
        let dir = std::env::temp_dir().join(format!("zacess-snapshot-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("lantern.lua"),
            include_str!("../examples/scenes/lantern.lua"),
        )
        .unwrap();
        let catalogue = Catalogue::load(Some(&dir));
        let name = catalogue
            .entries
            .iter()
            .find(|e| e.user.is_some())
            .unwrap()
            .name
            .clone();
        let theme = theme::default_theme();
        let frame = shot(&catalogue, &name, theme).still(2.0).unwrap();
        assert!(frame.drawn() > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
