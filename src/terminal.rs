//! The only module that touches a real terminal: raw mode, the alternate screen, drawing a frame
//! and reading keys. The terminal is restored when the program ends, when it panics and on
//! Ctrl+C, which arrives as a key in raw mode.

use std::io::{self, Stdout, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crossterm::cursor;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};

use crate::colour::Rgb;
use crate::frame::Frame;

/// Whether the terminal is currently taken over, so it is restored exactly once.
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// What a key or a terminal event asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    None,
    Quit,
    Pause,
    Help,
    Resize(usize, usize),
}

/// Maps a key to an input. Releases are ignored, since Windows reports both edges.
pub fn interpret(key: KeyEvent) -> Input {
    if key.kind == KeyEventKind::Release {
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

/// The terminal size, falling back to a default when there is no terminal to ask.
pub fn size_or(default: (usize, usize)) -> (usize, usize) {
    terminal::size()
        .ok()
        .map(|(w, h)| (w as usize, h as usize))
        .filter(|&(w, h)| w > 0 && h > 0)
        .unwrap_or(default)
}

fn restore() {
    if ACTIVE.swap(false, Ordering::SeqCst) {
        let mut out = io::stdout();
        let _ = execute!(out, ResetColor, cursor::Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
        let _ = out.flush();
    }
}

/// A taken-over terminal. Dropping it gives the terminal back.
pub struct Terminal {
    out: Stdout,
    previous: Option<Frame>,
    colour: Option<Option<Rgb>>,
}

impl Terminal {
    /// Prints `intro` as a plain line, so a screen reader hears what started and how to quit,
    /// then switches to raw mode and the alternate screen with the cursor hidden.
    pub fn open(intro: &str) -> io::Result<Self> {
        let mut out = io::stdout();
        writeln!(out, "{intro}")?;
        out.flush()?;
        terminal::enable_raw_mode()?;
        if let Err(error) = execute!(
            out,
            EnterAlternateScreen,
            cursor::Hide,
            Clear(ClearType::All)
        ) {
            let _ = terminal::disable_raw_mode();
            return Err(error);
        }
        ACTIVE.store(true, Ordering::SeqCst);
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore();
            previous_hook(info);
        }));
        Ok(Self {
            out,
            previous: None,
            colour: None,
        })
    }

    pub fn size(&self) -> io::Result<(usize, usize)> {
        let (w, h) = terminal::size()?;
        Ok((w as usize, h as usize))
    }

    /// Clears the screen and forgets the last frame, so the next draw repaints everything.
    pub fn reset(&mut self) -> io::Result<()> {
        self.previous = None;
        self.colour = None;
        execute!(self.out, Clear(ClearType::All))
    }

    /// Writes the cells that changed since the last frame. With `colour` off every cell uses the
    /// terminal's own text colour.
    pub fn draw(&mut self, frame: &Frame, colour: bool) -> io::Result<()> {
        let unchanged = |x: usize, y: usize, cell| {
            self.previous
                .as_ref()
                .is_some_and(|p| p.get(x as i32, y as i32) == Some(cell))
        };
        let mut last: Option<(usize, usize)> = None;
        for (y, row) in frame.rows().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                // the last cell is skipped: printing there makes some terminals scroll
                if (x + 1 == frame.width() && y + 1 == frame.height()) || unchanged(x, y, *cell) {
                    continue;
                }
                if last != Some((x.wrapping_sub(1), y)) {
                    queue!(self.out, cursor::MoveTo(x as u16, y as u16))?;
                }
                let wanted = if colour { cell.colour } else { None };
                if self.colour != Some(wanted) {
                    match wanted {
                        Some(c) => queue!(
                            self.out,
                            SetForegroundColor(Color::Rgb {
                                r: c.r,
                                g: c.g,
                                b: c.b
                            })
                        )?,
                        None => queue!(self.out, ResetColor)?,
                    }
                    self.colour = Some(wanted);
                }
                queue!(self.out, Print(cell.ch))?;
                last = Some((x, y));
            }
        }
        queue!(self.out, cursor::MoveTo(0, 0))?;
        self.out.flush()?;
        self.previous = Some(frame.clone());
        Ok(())
    }

    /// Waits up to `timeout` for a key or a resize.
    pub fn poll(&mut self, timeout: Duration) -> io::Result<Input> {
        if !event::poll(timeout)? {
            return Ok(Input::None);
        }
        Ok(match event::read()? {
            Event::Key(key) => interpret(key),
            Event::Resize(w, h) => Input::Resize(w as usize, h as usize),
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
    fn key_releases_are_ignored() {
        let release = KeyEvent::new_with_kind(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        );
        assert_eq!(interpret(release), Input::None);
    }

    #[test]
    fn restoring_an_untouched_terminal_does_nothing() {
        assert!(!ACTIVE.load(Ordering::SeqCst));
        restore();
        assert!(!ACTIVE.load(Ordering::SeqCst));
    }
}
