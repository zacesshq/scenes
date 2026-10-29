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
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::terminal::{
    self, BeginSynchronizedUpdate, Clear, ClearType, EndSynchronizedUpdate, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use crossterm::{execute, queue};

use crate::colour::{ColourDepth, Rgb, nearest_256};
use crate::frame::Frame;

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
        _ => Input::None,
    }
}

/// Maps a key in the watch mode: `1` to `4`, `r`, `p` and `s` on top of the keys every mode has.
pub fn interpret_watch(key: KeyEvent) -> Input {
    if key.kind != KeyEventKind::Press || key.modifiers.contains(KeyModifiers::CONTROL) {
        return interpret(key);
    }
    match key.code {
        KeyCode::Char(c @ ('1'..='4' | 'r' | 'R' | 'p' | 'P' | 's' | 'S')) => {
            Input::Key(c.to_ascii_lowercase())
        }
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

/// How colours reach the terminal.
#[derive(Clone, Copy, Debug)]
pub struct Paint {
    pub depth: ColourDepth,
    /// The theme background and floor, so a colour mapped to the 256 colour table still keeps
    /// the floor.
    pub background: Rgb,
    pub floor: f64,
}

/// A taken-over terminal. Dropping it gives the terminal back.
pub struct Terminal {
    /// One frame's output, written in a single call so a slow terminal never shows half a frame.
    buffer: Vec<u8>,
    previous: Option<Frame>,
    colour: Option<Option<Rgb>>,
    paint: Option<Paint>,
    mapped: HashMap<Rgb, u8>,
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
        for signal in [
            signal_hook::consts::SIGTERM,
            signal_hook::consts::SIGHUP,
            signal_hook::consts::SIGQUIT,
            signal_hook::consts::SIGINT,
        ] {
            signal_hook::flag::register(signal, Arc::clone(&stop))?;
        }
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
            buffer: Vec::with_capacity(64 * 1024),
            previous: None,
            colour: None,
            paint: None,
            mapped: HashMap::new(),
            pending: None,
            stop,
            keys: interpret,
        })
    }

    /// Sets how colours are sent. Without a call every colour is sent as 24 bit colour.
    pub fn set_paint(&mut self, paint: Paint) {
        self.paint = Some(paint);
        self.mapped.clear();
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
        self.previous = None;
        self.colour = None;
        execute!(io::stdout(), Clear(ClearType::All))
    }

    fn set_colour(&mut self, wanted: Option<Rgb>) -> io::Result<()> {
        match wanted {
            None => queue!(self.buffer, ResetColor),
            Some(c) => {
                let colour = match self.paint {
                    Some(paint) if paint.depth == ColourDepth::Ansi256 => {
                        let index = *self
                            .mapped
                            .entry(c)
                            .or_insert_with(|| nearest_256(c, paint.background, paint.floor));
                        Color::AnsiValue(index)
                    }
                    _ => Color::Rgb {
                        r: c.r,
                        g: c.g,
                        b: c.b,
                    },
                };
                queue!(self.buffer, SetForegroundColor(colour))
            }
        }
    }

    /// Writes the cells that changed since the last frame and returns whether any did. With
    /// `colour` off every cell uses the terminal's own text colour.
    pub fn draw(&mut self, frame: &Frame, colour: bool) -> io::Result<bool> {
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
                let wanted = if colour { cell.colour } else { None };
                if self.colour != Some(wanted) {
                    self.set_colour(wanted)?;
                    self.colour = Some(wanted);
                }
                queue!(self.buffer, Print(cell.ch))?;
                changed = true;
                // block and shade characters are ambiguous width, two columns in some
                // terminals, so the cursor is only trusted to advance by one after ASCII
                cursor_at = cell.ch.is_ascii().then_some((x + 1, y));
            }
        }
        self.previous = Some(frame.clone());
        if !changed {
            return Ok(false);
        }
        queue!(self.buffer, cursor::MoveTo(0, 0), EndSynchronizedUpdate)?;
        let mut out = io::stdout().lock();
        out.write_all(&self.buffer)?;
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

    #[test]
    fn restoring_an_untouched_terminal_does_nothing() {
        assert!(!ACTIVE.load(Ordering::SeqCst));
        restore();
        assert!(!ACTIVE.load(Ordering::SeqCst));
    }
}
