//! The only module that touches a real terminal: raw mode, the alternate screen, drawing a frame
//! and reading keys. The terminal is restored when the program ends, when it panics and on
//! Ctrl+C, which arrives as a key in raw mode. On Unix it is also restored on SIGTERM, SIGHUP,
//! SIGQUIT and SIGINT.

use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crossterm::cursor;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{
    self, BeginSynchronizedUpdate, Clear, ClearType, EndSynchronizedUpdate, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use crossterm::{execute, queue};

use crate::colour::{ColourDepth, Rgb, nearest_256};
use crate::frame::{Cell, Frame};

/// Whether the terminal is currently taken over, so it is restored exactly once.
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// The size used when the terminal reports none, as some pseudo terminals do.
pub const FALLBACK_SIZE: (usize, usize) = (80, 24);

/// The line printed once the terminal is given back, on every way out.
pub const STOPPED: &str = "Zacess Scenes stopped.";

/// What a key or a terminal event asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    None,
    Quit,
    Pause,
    Help,
    /// A key with a meaning only in one mode, such as the job keys of the watch mode.
    Key(char),
    Resize(usize, usize),
}

/// Maps a key to an input. Releases are ignored, since Windows reports both edges. So are repeats,
/// so holding a key never toggles anything over and over.
pub fn interpret(key: KeyEvent) -> Input {
    if key.kind != KeyEventKind::Press {
        return Input::None;
    }
    match key.code {
        KeyCode::Char('c') | KeyCode::Char('C')
            if key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            Input::Quit
        }
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Input::Quit,
        KeyCode::Char(' ') => Input::Pause,
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => Input::Help,
        // n moves on a scene; N and b go back, so the shifted key reads as the reverse
        KeyCode::Char('n') => Input::Key('n'),
        KeyCode::Char('N') | KeyCode::Char('b') | KeyCode::Char('B') => Input::Key('b'),
        // the ambient sound: a turns it on or off and [ and ] change its volume in every mode
        KeyCode::Char('a') | KeyCode::Char('A') => Input::Key('a'),
        KeyCode::Char(c @ ('[' | ']')) => Input::Key(c),
        // a scene on its own has no tones, so m, + and - work the ambient sound there too
        KeyCode::Char('m') | KeyCode::Char('M') => Input::Key('m'),
        KeyCode::Char('+' | '=') => Input::Key('+'),
        KeyCode::Char('-' | '_') => Input::Key('-'),
        _ => Input::None,
    }
}

/// Maps a key in the watch mode: `1` to `4`, `r`, `p`, `s`, `m`, `+` and `-` on top of the keys
/// every mode has. `=` and `_` count as `+` and `-`, the same keys without Shift.
pub fn interpret_watch(key: KeyEvent) -> Input {
    if key.kind != KeyEventKind::Press || key.modifiers.contains(KeyModifiers::CONTROL) {
        return interpret(key);
    }
    match key.code {
        KeyCode::Char(c @ ('1'..='4' | 'r' | 'R' | 'p' | 'P' | 's' | 'S' | 'm' | 'M')) => {
            Input::Key(c.to_ascii_lowercase())
        }
        KeyCode::Char('+' | '=') => Input::Key('+'),
        KeyCode::Char('-' | '_') => Input::Key('-'),
        _ => interpret(key),
    }
}

/// A reported size with a zero in it replaced by the default.
fn usable(size: (u16, u16), default: (usize, usize)) -> (usize, usize) {
    let (w, h) = (size.0 as usize, size.1 as usize);
    if w > 0 && h > 0 { (w, h) } else { default }
}

/// The terminal size, falling back to a default when there is no terminal to ask or it reports
/// a zero.
pub fn size_or(default: (usize, usize)) -> (usize, usize) {
    terminal::size().map_or(default, |size| usable(size, default))
}

/// Whether the font in use probably lacks the geometric shapes beyond the block and shade
/// characters: the Linux console and the classic Windows console, which have no font fallback.
pub fn limited_font(var: impl Fn(&str) -> Option<String>) -> bool {
    let set = |name: &str| var(name).is_some_and(|v| !v.is_empty());
    if var("TERM").as_deref() == Some("linux") {
        return true;
    }
    cfg!(windows) && !set("WT_SESSION") && !set("TERM_PROGRAM")
}

fn restore() {
    if ACTIVE.swap(false, Ordering::SeqCst) {
        let mut out = io::stdout();
        let _ = execute!(
            out,
            EndSynchronizedUpdate,
            ResetColor,
            cursor::Show,
            LeaveAlternateScreen
        );
        let _ = terminal::disable_raw_mode();
        let _ = writeln!(out, "{STOPPED}");
        let _ = out.flush();
    }
}

/// How long the run loop has to end on its own after a termination signal before the program
/// gives the terminal back and ends without it.
#[cfg(unix)]
pub const STOP_GRACE: Duration = Duration::from_secs(1);

/// Listens for termination signals on a thread of its own. A signal sets `stop`, so the run loop
/// ends and the terminal is restored as usual. When the terminal itself has gone, as when a tmux
/// pane or an SSH session closes, crossterm's read loop never returns to the run loop and the
/// program would spin on the closed terminal for ever; so if the loop has not ended within
/// `STOP_GRACE`, the terminal is restored as far as it can be and the program ends.
#[cfg(unix)]
fn watch_signals(stop: Arc<AtomicBool>) -> io::Result<()> {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
    let mut signals = signal_hook::iterator::Signals::new([SIGTERM, SIGHUP, SIGQUIT, SIGINT])?;
    std::thread::spawn(move || {
        if let Some(signal) = signals.forever().next() {
            stop.store(true, Ordering::SeqCst);
            std::thread::sleep(STOP_GRACE);
            if ACTIVE.load(Ordering::SeqCst) {
                restore();
                std::process::exit(128 + signal);
            }
        }
    });
    Ok(())
}

/// How colours reach the terminal.
#[derive(Clone, Copy, Debug)]
pub struct Paint {
    pub depth: ColourDepth,
    /// The theme background and floor, so a colour mapped to the 256 colour table still keeps
    /// the floor.
    pub background: Rgb,
    pub floor: f64,
}

/// Turns frames into the bytes that draw them: only the cells that changed since the last frame,
/// and a colour escape only when the foreground or background actually changes. It needs no
/// terminal, so its output can be tested.
#[derive(Debug, Default)]
pub struct Encoder {
    /// One frame's output, written in a single call so a slow terminal never shows half a frame.
    buffer: Vec<u8>,
    previous: Option<Frame>,
    /// The foreground and background the terminal is known to be on, `None` when unknown.
    fg: Option<Option<Rgb>>,
    bg: Option<Option<Rgb>>,
    paint: Option<Paint>,
    mapped: HashMap<Rgb, u8>,
    /// Pixel colours mapped to the 256 colour table, which keep no text floor.
    mapped_pixels: HashMap<Rgb, u8>,
}

impl Encoder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(64 * 1024),
            ..Self::default()
        }
    }

    /// Sets how colours are sent. Without a call every colour is sent as 24 bit colour.
    pub fn set_paint(&mut self, paint: Paint) {
        self.paint = Some(paint);
        self.mapped.clear();
        self.mapped_pixels.clear();
    }

    /// Forgets the last frame and the colours, so the next frame repaints everything.
    pub fn forget(&mut self) {
        self.previous = None;
        self.fg = None;
        self.bg = None;
    }

    /// The bytes of the last encoded frame.
    pub fn bytes(&self) -> &[u8] {
        &self.buffer
    }

    fn depth_256(&self) -> Option<Paint> {
        self.paint.filter(|p| p.depth == ColourDepth::Ansi256)
    }

    /// A colour as the terminal gets it. Text keeps the floor in the 256 colour table; pixel
    /// imagery takes the nearest entry, since it carries no text and keeps no floor.
    fn terminal_colour(&mut self, c: Rgb, pixel: bool) -> Color {
        match self.depth_256() {
            Some(paint) => {
                let index = if pixel {
                    *self
                        .mapped_pixels
                        .entry(c)
                        .or_insert_with(|| pixel_256(c, paint.background))
                } else {
                    *self
                        .mapped
                        .entry(c)
                        .or_insert_with(|| nearest_256(c, paint.background, paint.floor))
                };
                Color::AnsiValue(index)
            }
            None => Color::Rgb {
                r: c.r,
                g: c.g,
                b: c.b,
            },
        }
    }

    /// Brings the terminal to a foreground and background. `fg` of `None` inside `Some` means
    /// the default; a plain `None` means any foreground will do, as for a space. A reset is the
    /// only way back to a default; it clears both, so it comes first.
    fn set_colours(
        &mut self,
        fg: Option<Option<Rgb>>,
        bg: Option<Rgb>,
        pixel: bool,
    ) -> io::Result<()> {
        let fg_needs_reset = fg == Some(None) && self.fg != Some(None);
        let bg_needs_reset = bg.is_none() && self.bg != Some(None);
        if fg_needs_reset || bg_needs_reset {
            queue!(self.buffer, ResetColor)?;
            self.fg = Some(None);
            self.bg = Some(None);
        }
        if let Some(wanted) = fg.filter(|&w| self.fg != Some(w)) {
            if let Some(c) = wanted {
                let colour = self.terminal_colour(c, pixel);
                queue!(self.buffer, SetForegroundColor(colour))?;
            }
            self.fg = Some(wanted);
        }
        if self.bg != Some(bg) {
            if let Some(c) = bg {
                let colour = self.terminal_colour(c, true);
                queue!(self.buffer, SetBackgroundColor(colour))?;
            }
            self.bg = Some(bg);
        }
        Ok(())
    }

    /// What one cell is printed as: its character, the foreground it needs (`None` for any) and
    /// its background. With colour off a pixel cell keeps only the outline of its shapes.
    fn plan(&self, cell: Cell, colour: bool) -> (char, Option<Option<Rgb>>, Option<Rgb>) {
        if !colour {
            if let Some((top, bottom)) = cell.pixel_pair() {
                let background = self.paint.map_or(Rgb::new(0, 0, 0), |p| p.background);
                let ch = crate::pixels::plain_glyph(top, bottom, background);
                return (ch, Some(None), None);
            }
            return (cell.ch, Some(None), None);
        }
        if cell.is_pixel() && self.depth_256().is_some() {
            // two pixels that land on one table entry print as a plain space of it
            if let Some((top, bottom)) = cell.pixel_pair() {
                let background = self.paint.map_or(Rgb::new(0, 0, 0), |p| p.background);
                if pixel_256(top, background) == pixel_256(bottom, background) {
                    return (' ', None, Some(bottom));
                }
            }
        }
        if cell.ch == ' ' && cell.bg.is_some() {
            return (' ', None, cell.bg);
        }
        (cell.ch, Some(cell.colour), cell.bg)
    }

    /// Encodes the cells that changed since the last frame and returns whether any did. With
    /// `colour` off every cell uses the terminal's own text colour.
    pub fn encode(&mut self, frame: &Frame, colour: bool) -> io::Result<bool> {
        self.buffer.clear();
        queue!(self.buffer, BeginSynchronizedUpdate)?;
        let mut changed = false;
        // where the cursor is known to be after the last print, if anywhere
        let mut cursor_at: Option<(usize, usize)> = None;
        for (y, row) in frame.rows().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                // the last cell is skipped: printing there makes some terminals scroll
                if x + 1 == frame.width() && y + 1 == frame.height() {
                    continue;
                }
                let unchanged = self
                    .previous
                    .as_ref()
                    .is_some_and(|p| p.get(x as i32, y as i32) == Some(*cell));
                if unchanged {
                    continue;
                }
                if cursor_at != Some((x, y)) {
                    queue!(self.buffer, cursor::MoveTo(x as u16, y as u16))?;
                }
                let (ch, fg, bg) = self.plan(*cell, colour);
                self.set_colours(fg, bg, cell.is_pixel())?;
                queue!(self.buffer, Print(ch))?;
                changed = true;
                // block and shade characters are ambiguous width, two columns in some
                // terminals, so the cursor is only trusted to advance by one after ASCII
                cursor_at = ch.is_ascii().then_some((x + 1, y));
            }
        }
        self.previous = Some(frame.clone());
        if !changed {
            return Ok(false);
        }
        // the background goes back to the terminal's own, so nothing outside a frame, such as
        // the line printed after quitting, ever picks up a scene's colour
        if self.bg != Some(None) {
            queue!(self.buffer, ResetColor)?;
            self.fg = Some(None);
            self.bg = Some(None);
        }
        queue!(self.buffer, cursor::MoveTo(0, 0), EndSynchronizedUpdate)?;
        Ok(true)
    }
}

/// The 256 colour table entry for a pixel: simply the nearest, since imagery keeps no floor.
pub fn pixel_256(colour: Rgb, background: Rgb) -> u8 {
    nearest_256(colour, background, 1.0)
}

/// A taken-over terminal. Dropping it gives the terminal back.
pub struct Terminal {
    encoder: Encoder,
    /// An input read while draining queued resizes, handed out by the next poll.
    pending: Option<Input>,
    /// Set by a termination signal, so the run loop ends and the terminal is restored.
    stop: Arc<AtomicBool>,
    /// How keys are read, which differs between the scenes and the watch mode.
    keys: fn(KeyEvent) -> Input,
}

impl Terminal {
    /// Prints `intro` as a plain line, so a screen reader hears what started and how to quit,
    /// then switches to raw mode and the alternate screen with the cursor hidden.
    pub fn open(intro: &str) -> io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        #[cfg(unix)]
        watch_signals(Arc::clone(&stop))?;
        let mut out = io::stdout();
        writeln!(out, "{intro}")?;
        out.flush()?;
        terminal::enable_raw_mode()?;
        ACTIVE.store(true, Ordering::SeqCst);
        if let Err(error) = execute!(
            out,
            EnterAlternateScreen,
            cursor::Hide,
            Clear(ClearType::All)
        ) {
            restore();
            return Err(error);
        }
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore();
            previous_hook(info);
        }));
        Ok(Self {
            encoder: Encoder::new(),
            pending: None,
            stop,
            keys: interpret,
        })
    }

    /// Sets how colours are sent. Without a call every colour is sent as 24 bit colour.
    pub fn set_paint(&mut self, paint: Paint) {
        self.encoder.set_paint(paint);
    }

    /// Sets how keys are read.
    pub fn set_keys(&mut self, keys: fn(KeyEvent) -> Input) {
        self.keys = keys;
    }

    /// The terminal size, with the fallback when it reports a zero.
    pub fn size(&self) -> io::Result<(usize, usize)> {
        Ok(usable(terminal::size()?, FALLBACK_SIZE))
    }

    /// Clears the screen and forgets the last frame, so the next draw repaints everything.
    pub fn reset(&mut self) -> io::Result<()> {
        self.encoder.forget();
        execute!(io::stdout(), ResetColor, Clear(ClearType::All))
    }

    /// Writes the cells that changed since the last frame and returns whether any did. With
    /// `colour` off every cell uses the terminal's own text colour.
    pub fn draw(&mut self, frame: &Frame, colour: bool) -> io::Result<bool> {
        if !self.encoder.encode(frame, colour)? {
            return Ok(false);
        }
        let mut out = io::stdout().lock();
        out.write_all(self.encoder.bytes())?;
        out.flush()?;
        Ok(true)
    }

    /// Whether a termination signal has asked the program to stop.
    pub fn stopping(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    /// Waits up to `timeout` for a key or a resize. A burst of resizes, as a window drag sends,
    /// comes back as one resize to the size the terminal has after the burst.
    pub fn poll(&mut self, timeout: Duration) -> io::Result<Input> {
        if self.stopping() {
            return Ok(Input::Quit);
        }
        if let Some(input) = self.pending.take() {
            return Ok(input);
        }
        if !event::poll(timeout)? {
            return Ok(if self.stopping() {
                Input::Quit
            } else {
                Input::None
            });
        }
        Ok(match event::read()? {
            Event::Key(key) => (self.keys)(key),
            Event::Resize(..) => {
                while event::poll(Duration::ZERO)? {
                    match event::read()? {
                        Event::Resize(..) => {}
                        Event::Key(key) => {
                            self.pending = Some((self.keys)(key));
                            break;
                        }
                        _ => {}
                    }
                }
                let (w, h) = self.size()?;
                Input::Resize(w, h)
            }
            _ => Input::None,
        })
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn the_quit_keys_quit() {
        assert_eq!(
            interpret(key(KeyCode::Char('q'), KeyModifiers::NONE)),
            Input::Quit
        );
        assert_eq!(
            interpret(key(KeyCode::Char('Q'), KeyModifiers::SHIFT)),
            Input::Quit
        );
        assert_eq!(
            interpret(key(KeyCode::Esc, KeyModifiers::NONE)),
            Input::Quit
        );
        assert_eq!(
            interpret(key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Input::Quit
        );
        assert_eq!(
            interpret(key(KeyCode::Char('c'), KeyModifiers::NONE)),
            Input::None
        );
    }

    #[test]
    fn space_pauses_and_h_shows_the_help() {
        assert_eq!(
            interpret(key(KeyCode::Char(' '), KeyModifiers::NONE)),
            Input::Pause
        );
        assert_eq!(
            interpret(key(KeyCode::Char('h'), KeyModifiers::NONE)),
            Input::Help
        );
        assert_eq!(
            interpret(key(KeyCode::Char('?'), KeyModifiers::SHIFT)),
            Input::Help
        );
        assert_eq!(
            interpret(key(KeyCode::Char('x'), KeyModifiers::NONE)),
            Input::None
        );
    }

    #[test]
    fn the_watch_keys_add_to_the_common_ones() {
        for (c, wanted) in [
            ('1', '1'),
            ('4', '4'),
            ('r', 'r'),
            ('R', 'r'),
            ('p', 'p'),
            ('s', 's'),
        ] {
            assert_eq!(
                interpret_watch(key(KeyCode::Char(c), KeyModifiers::NONE)),
                Input::Key(wanted)
            );
        }
        assert_eq!(
            interpret_watch(key(KeyCode::Char('5'), KeyModifiers::NONE)),
            Input::None
        );
        assert_eq!(
            interpret_watch(key(KeyCode::Char('q'), KeyModifiers::NONE)),
            Input::Quit
        );
        assert_eq!(
            interpret_watch(key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Input::Quit
        );
        let repeat =
            KeyEvent::new_with_kind(KeyCode::Char('r'), KeyModifiers::NONE, KeyEventKind::Repeat);
        assert_eq!(interpret_watch(repeat), Input::None);
        // the scenes keep their own keys only
        assert_eq!(
            interpret(key(KeyCode::Char('r'), KeyModifiers::NONE)),
            Input::None
        );
        for (c, wanted) in [
            ('m', 'm'),
            ('M', 'm'),
            ('+', '+'),
            ('=', '+'),
            ('-', '-'),
            ('_', '-'),
        ] {
            assert_eq!(
                interpret_watch(key(KeyCode::Char(c), KeyModifiers::NONE)),
                Input::Key(wanted),
                "{c}"
            );
        }
        for (c, wanted) in [('n', 'n'), ('N', 'b'), ('b', 'b'), ('B', 'b')] {
            assert_eq!(
                interpret(key(KeyCode::Char(c), KeyModifiers::NONE)),
                Input::Key(wanted)
            );
            assert_eq!(
                interpret_watch(key(KeyCode::Char(c), KeyModifiers::SHIFT)),
                Input::Key(wanted)
            );
        }
    }

    #[test]
    fn key_releases_are_ignored() {
        let release = KeyEvent::new_with_kind(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        );
        assert_eq!(interpret(release), Input::None);
    }

    #[test]
    fn key_repeats_are_ignored() {
        let repeat =
            KeyEvent::new_with_kind(KeyCode::Char(' '), KeyModifiers::NONE, KeyEventKind::Repeat);
        assert_eq!(interpret(repeat), Input::None);
    }

    #[test]
    fn a_zero_size_falls_back() {
        assert_eq!(usable((0, 0), FALLBACK_SIZE), FALLBACK_SIZE);
        assert_eq!(usable((40, 0), FALLBACK_SIZE), FALLBACK_SIZE);
        assert_eq!(usable((40, 12), FALLBACK_SIZE), (40, 12));
    }

    #[test]
    fn the_linux_console_counts_as_a_limited_font() {
        let term = |value: &'static str| move |name: &str| (name == "TERM").then(|| value.into());
        assert!(limited_font(term("linux")));
        let windows_terminal = |name: &str| (name == "WT_SESSION").then(|| "1".to_string());
        assert!(!limited_font(windows_terminal));
        if !cfg!(windows) {
            assert!(!limited_font(term("xterm-256color")));
        }
    }

    fn encoded(encoder: &mut Encoder, frame: &Frame, colour: bool) -> String {
        encoder.encode(frame, colour).unwrap();
        String::from_utf8(encoder.bytes().to_vec()).unwrap()
    }

    fn pixel_row(colours: &[(Rgb, Rgb)]) -> Frame {
        let mut frame = Frame::new(colours.len() + 1, 2);
        for (x, &(top, bottom)) in colours.iter().enumerate() {
            frame.put_cell(x as i32, 0, Cell::pixels(top, bottom));
        }
        frame
    }

    #[test]
    fn pixel_cells_send_a_background_only_when_it_changes() {
        let (a, b) = (Rgb::new(1, 2, 3), Rgb::new(40, 50, 60));
        let frame = pixel_row(&[(a, b), (a, b), (a, b), (b, b)]);
        let mut encoder = Encoder::new();
        let out = encoded(&mut encoder, &frame, true);
        assert_eq!(out.matches("\u{1b}[48;2;40;50;60m").count(), 1, "{out:?}");
        assert_eq!(out.matches("\u{1b}[38;2;1;2;3m").count(), 1, "{out:?}");
        assert_eq!(out.matches('\u{2580}').count(), 3);
        // the fourth cell is one colour, so it is a space on the same background, with no escape
        assert!(out.contains("\u{1b}[1;4H \u{1b}[0m"), "{out:?}");
        // the background is handed back before anything else is printed
        let last_bg = out.rfind("[48;").unwrap();
        assert!(out[last_bg..].contains("\u{1b}[0m"), "{out:?}");
        // nothing changed, nothing is sent
        assert!(!encoder.encode(&frame, true).unwrap());
    }

    #[test]
    fn text_after_a_pixel_cell_resets_the_background() {
        let (a, b) = (Rgb::new(1, 2, 3), Rgb::new(40, 50, 60));
        let mut frame = pixel_row(&[(a, b)]);
        frame.put(1, 0, 'x', None);
        let out = encoded(&mut Encoder::new(), &frame, true);
        let pixel = out.find('\u{2580}').unwrap();
        let reset = out[pixel..].find("\u{1b}[0m").map(|i| i + pixel).unwrap();
        assert!(reset < out.find('x').unwrap(), "{out:?}");
    }

    #[test]
    fn text_only_frames_never_send_a_background() {
        let mut frame = Frame::new(10, 2);
        frame.put_str(0, 0, "abc", Some(Rgb::new(255, 255, 0)));
        frame.put_str(4, 0, "de", None);
        let out = encoded(&mut Encoder::new(), &frame, true);
        assert!(!out.contains("[48;"), "{out:?}");
        assert_eq!(out.matches("38;2;255;255;0m").count(), 1);
    }

    #[test]
    fn the_256_colour_table_takes_the_nearest_entry_for_pixels() {
        let theme = crate::theme::default_theme();
        let mut encoder = Encoder::new();
        encoder.set_paint(Paint {
            depth: ColourDepth::Ansi256,
            background: theme.background,
            floor: theme.floor(),
        });
        let dark = Rgb::new(0, 0, 40);
        let frame = pixel_row(&[(theme.yellow(), dark), (dark, Rgb::new(0, 0, 41))]);
        let out = encoded(&mut encoder, &frame, true);
        assert!(!out.contains("38;2;") && !out.contains("48;2;"), "{out:?}");
        let index = pixel_256(dark, theme.background);
        assert!(out.contains(&format!("48;5;{index}m")), "{out:?}");
        // the second cell's two pixels share an entry, so it is a plain space
        assert_eq!(out.matches('\u{2580}').count(), 1, "{out:?}");
    }

    #[test]
    fn without_colour_pixels_keep_only_their_shapes() {
        let theme = crate::theme::default_theme();
        let mut encoder = Encoder::new();
        encoder.set_paint(Paint {
            depth: ColourDepth::TrueColour,
            background: theme.background,
            floor: theme.floor(),
        });
        let ground = Rgb::new(0, 0, 40);
        let frame = pixel_row(&[
            (ground, ground),
            (theme.yellow(), ground),
            (ground, theme.red()),
        ]);
        let out = encoded(&mut encoder, &frame, false);
        assert!(!out.contains("38;") && !out.contains("48;"), "{out:?}");
        let blank = out.find("[0m ").unwrap();
        let upper = out.find('\u{2580}').unwrap();
        let lower = out.find('\u{2584}').unwrap();
        assert!(blank < upper && upper < lower, "{out:?}");
    }

    #[test]
    fn restoring_an_untouched_terminal_does_nothing() {
        assert!(!ACTIVE.load(Ordering::SeqCst));
        restore();
        assert!(!ACTIVE.load(Ordering::SeqCst));
    }
}
