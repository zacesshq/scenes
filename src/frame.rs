//! The frame buffer every scene draws into. It renders to a string, so the whole program can run
//! without a terminal.

use crate::colour::Rgb;

/// The upper half block. A pixel cell draws it with the top pixel as the foreground and the bottom
/// pixel as the background.
pub const HALF_BLOCK: char = '\u{2580}';

/// One character cell. `None` for the colour means the terminal's default foreground and `None`
/// for the background means the terminal's own background, which every text-only scene keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub colour: Option<Rgb>,
    pub bg: Option<Rgb>,
}

impl Cell {
    pub const BLANK: Cell = Cell {
        ch: ' ',
        colour: None,
        bg: None,
    };

    /// A cell of two stacked pixels, as the pixel canvas writes it. Two equal pixels become a
    /// space on that background, which needs no foreground at all.
    pub fn pixels(top: Rgb, bottom: Rgb) -> Cell {
        if top == bottom {
            Cell {
                ch: ' ',
                colour: None,
                bg: Some(bottom),
            }
        } else {
            Cell {
                ch: HALF_BLOCK,
                colour: Some(top),
                bg: Some(bottom),
            }
        }
    }

    /// Whether the cell is imagery from the pixel canvas rather than text. Its colours carry no
    /// meaning, so they are held to the background imagery rules instead of the text floor.
    pub fn is_pixel(&self) -> bool {
        self.bg.is_some() && (self.ch == HALF_BLOCK || self.ch == ' ')
    }

    /// The top and bottom pixel of a pixel cell.
    pub fn pixel_pair(&self) -> Option<(Rgb, Rgb)> {
        let bg = self.bg?;
        match self.ch {
            ' ' => Some((bg, bg)),
            HALF_BLOCK => Some((self.colour.unwrap_or(bg), bg)),
            _ => None,
        }
    }
}

/// A grid of cells the size of the terminal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl Frame {
    /// A blank frame. A zero size becomes one cell so drawing never divides by zero.
    pub fn new(width: usize, height: usize) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        Self {
            width,
            height,
            cells: vec![Cell::BLANK; width * height],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn clear(&mut self) {
        self.cells.fill(Cell::BLANK);
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 {
            return None;
        }
        let (x, y) = (x as usize, y as usize);
        (x < self.width && y < self.height).then_some(y * self.width + x)
    }

    /// Draws one character. Anything outside the frame is dropped, so a sprite can sit half off
    /// the edge while it enters or leaves.
    pub fn put(&mut self, x: i32, y: i32, ch: char, colour: Option<Rgb>) {
        if let Some(i) = self.index(x, y) {
            self.cells[i] = Cell {
                ch,
                colour,
                bg: None,
            };
        }
    }

    /// Writes a whole cell, background included.
    pub fn put_cell(&mut self, x: i32, y: i32, cell: Cell) {
        if let Some(i) = self.index(x, y) {
            self.cells[i] = cell;
        }
    }

    /// Draws a string from `x`, spaces included.
    pub fn put_str(&mut self, x: i32, y: i32, text: &str, colour: Option<Rgb>) {
        for (i, ch) in text.chars().enumerate() {
            self.put(x + i as i32, y, ch, colour);
        }
    }

    /// Draws a multi-line sprite with its spaces left transparent, so what is behind shows through.
    pub fn put_sprite(&mut self, x: i32, y: i32, rows: &[&str], colour: Option<Rgb>) {
        for (dy, row) in rows.iter().enumerate() {
            for (dx, ch) in row.chars().enumerate() {
                if ch != ' ' {
                    self.put(x + dx as i32, y + dy as i32, ch, colour);
                }
            }
        }
    }

    pub fn fill_row(&mut self, y: i32, ch: char, colour: Option<Rgb>) {
        for x in 0..self.width as i32 {
            self.put(x, y, ch, colour);
        }
    }

    pub fn get(&self, x: i32, y: i32) -> Option<Cell> {
        self.index(x, y).map(|i| self.cells[i])
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    pub fn rows(&self) -> impl Iterator<Item = &[Cell]> {
        self.cells.chunks(self.width)
    }

    /// The characters of one row (an empty string outside the frame).
    pub fn row_text(&self, y: usize) -> String {
        self.rows()
            .nth(y)
            .map(|row| row.iter().map(|c| c.ch).collect())
            .unwrap_or_default()
    }

    /// The frame as plain text, one line per row, every row the full width.
    pub fn to_text(&self) -> String {
        let mut out = String::with_capacity((self.width + 1) * self.height);
        for (i, row) in self.rows().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.extend(row.iter().map(|c| c.ch));
        }
        out
    }

    /// Every text colour drawn in the frame, sorted and without repeats. Pixel cells are left out:
    /// they are background imagery, not text, so the text floor does not apply to them.
    pub fn colours(&self) -> Vec<Rgb> {
        let mut used: Vec<Rgb> = self
            .cells
            .iter()
            .filter(|c| !c.is_pixel())
            .filter_map(|c| c.colour)
            .collect();
        used.sort_unstable();
        used.dedup();
        used
    }

    /// How many cells hold something other than a blank space.
    pub fn drawn(&self) -> usize {
        self.cells
            .iter()
            .filter(|c| c.ch != ' ' || c.bg.is_some())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_frame_is_blank_and_the_right_size() {
        let frame = Frame::new(20, 8);
        assert_eq!(frame.width(), 20);
        assert_eq!(frame.height(), 8);
        assert_eq!(frame.drawn(), 0);
        assert_eq!(frame.to_text().lines().count(), 8);
        assert!(frame.to_text().lines().all(|l| l.chars().count() == 20));
    }

    #[test]
    fn a_zero_size_becomes_one_cell() {
        let frame = Frame::new(0, 0);
        assert_eq!((frame.width(), frame.height()), (1, 1));
    }

    #[test]
    fn drawing_outside_the_frame_is_dropped() {
        let mut frame = Frame::new(5, 2);
        frame.put(-1, 0, 'x', None);
        frame.put(5, 0, 'x', None);
        frame.put(0, 2, 'x', None);
        frame.put_str(3, 1, "abcd", None);
        assert_eq!(frame.row_text(1), "   ab");
        assert_eq!(frame.drawn(), 2);
    }

    #[test]
    fn sprites_keep_spaces_transparent() {
        let mut frame = Frame::new(6, 2);
        frame.fill_row(0, '.', None);
        frame.put_sprite(1, 0, &["a b", " c "], Some(Rgb::new(1, 2, 3)));
        assert_eq!(frame.row_text(0), ".a.b..");
        assert_eq!(frame.row_text(1), "  c   ");
        assert_eq!(frame.colours(), vec![Rgb::new(1, 2, 3)]);
    }

    #[test]
    fn put_clears_a_background_and_pixel_cells_stay_out_of_the_text_colours() {
        let mut frame = Frame::new(3, 1);
        let (a, b) = (Rgb::new(1, 2, 3), Rgb::new(4, 5, 6));
        frame.put_cell(0, 0, Cell::pixels(a, b));
        frame.put_cell(1, 0, Cell::pixels(a, a));
        frame.put_cell(2, 0, Cell::pixels(a, b));
        frame.put(2, 0, 'x', Some(b));
        assert_eq!(frame.get(0, 0).unwrap().ch, HALF_BLOCK);
        assert_eq!(frame.get(1, 0).unwrap().ch, ' ');
        assert_eq!(frame.get(1, 0).unwrap().pixel_pair(), Some((a, a)));
        assert_eq!(frame.get(2, 0).unwrap().bg, None);
        assert_eq!(frame.colours(), vec![b]);
        assert_eq!(frame.drawn(), 3);
    }

    #[test]
    fn colours_are_sorted_and_unique() {
        let mut frame = Frame::new(3, 1);
        frame.put(0, 0, 'a', Some(Rgb::new(9, 9, 9)));
        frame.put(1, 0, 'b', Some(Rgb::new(1, 1, 1)));
        frame.put(2, 0, 'c', Some(Rgb::new(9, 9, 9)));
        assert_eq!(frame.colours(), vec![Rgb::new(1, 1, 1), Rgb::new(9, 9, 9)]);
    }
}
