//! Terminal state management - buffer, cursor, and cell data

use crate::ansi::AnsiParser;
use crate::pixel_canvas::PixelCanvas;

/// A single cell in the terminal grid
#[derive(Clone, Debug)]
pub struct Cell {
    /// The character in this cell
    pub character: char,
    /// Foreground color (RGB)
    pub fg_color: [f32; 3],
    /// Background color (RGB)
    pub bg_color: [f32; 3],
    /// Bold attribute
    pub bold: bool,
    /// Italic attribute
    pub italic: bool,
    /// Underline attribute
    pub underline: bool,
    /// Whether this cell is part of a wide character
    pub wide: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            character: ' ',
            fg_color: [0.9, 0.9, 0.9],
            bg_color: [0.1, 0.1, 0.12],
            bold: false,
            italic: false,
            underline: false,
            wide: false,
        }
    }
}

/// Current text attributes for new characters
#[derive(Clone, Debug)]
pub struct Attributes {
    pub fg_color: [f32; 3],
    pub bg_color: [f32; 3],
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            fg_color: [0.9, 0.9, 0.9],
            bg_color: [0.1, 0.1, 0.12],
            bold: false,
            italic: false,
            underline: false,
        }
    }
}

/// Terminal cursor state
#[derive(Clone, Debug)]
pub struct Cursor {
    pub col: u16,
    pub row: u16,
    pub visible: bool,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            col: 0,
            row: 0,
            visible: true,
        }
    }
}

/// The main terminal state
pub struct Terminal {
    /// Grid dimensions
    pub cols: u16,
    pub rows: u16,
    /// The character grid (current screen)
    pub cells: Vec<Vec<Cell>>,
    /// Scrollback buffer (history)
    pub scrollback: Vec<Vec<Cell>>,
    /// Maximum scrollback lines
    pub max_scrollback: usize,
    /// Current scroll offset (0 = no scroll, positive = scrolled up)
    pub scroll_offset: usize,
    /// Cursor state
    pub cursor: Cursor,
    /// Current text attributes
    pub attributes: Attributes,
    /// ANSI escape sequence parser
    pub parser: AnsiParser,
    /// Pixel canvas for high-resolution graphics
    pub pixel_canvas: PixelCanvas,
    /// Pending PTUI messages to process
    pub pending_ptui: Vec<String>,
    /// Alternate screen buffer (for programs like vim)
    alt_cells: Option<Vec<Vec<Cell>>>,
    alt_cursor: Option<Cursor>,
}

impl Terminal {
    /// Create a new terminal with the given dimensions
    pub fn new(cols: u16, rows: u16) -> Self {
        let cells = vec![vec![Cell::default(); cols as usize]; rows as usize];
        Self {
            cols,
            rows,
            cells,
            scrollback: Vec::new(),
            max_scrollback: 10000,
            scroll_offset: 0,
            cursor: Cursor::default(),
            attributes: Attributes::default(),
            parser: AnsiParser::new(),
            pixel_canvas: PixelCanvas::new(),
            pending_ptui: Vec::new(),
            alt_cells: None,
            alt_cursor: None,
        }
    }

    /// Resize the terminal
    pub fn resize(&mut self, new_cols: u16, new_rows: u16) {
        if new_cols == self.cols && new_rows == self.rows {
            return;
        }

        // Create new grid
        let mut new_cells = vec![vec![Cell::default(); new_cols as usize]; new_rows as usize];

        // Copy existing content
        for (row_idx, row) in self.cells.iter().enumerate() {
            if row_idx >= new_rows as usize {
                break;
            }
            for (col_idx, cell) in row.iter().enumerate() {
                if col_idx >= new_cols as usize {
                    break;
                }
                new_cells[row_idx][col_idx] = cell.clone();
            }
        }

        self.cells = new_cells;
        self.cols = new_cols;
        self.rows = new_rows;

        // Adjust cursor position if needed
        if self.cursor.col >= new_cols {
            self.cursor.col = new_cols - 1;
        }
        if self.cursor.row >= new_rows {
            self.cursor.row = new_rows - 1;
        }
    }

    /// Process input bytes from the PTY
    pub fn process_input(&mut self, data: &[u8]) {
        // Take parser out temporarily to avoid borrow issues
        let mut parser = std::mem::take(&mut self.parser);
        for &byte in data {
            parser.advance(byte, self);
        }
        self.parser = parser;
    }

    /// Put a character at the cursor position and advance
    pub fn put_char(&mut self, c: char) {
        if c == '\r' {
            self.cursor.col = 0;
            return;
        }

        if c == '\n' {
            self.newline();
            return;
        }

        if c == '\x08' {
            // Backspace
            if self.cursor.col > 0 {
                self.cursor.col -= 1;
            }
            return;
        }

        if c == '\t' {
            // Tab - advance to next 8-column boundary
            let next_tab = ((self.cursor.col / 8) + 1) * 8;
            self.cursor.col = next_tab.min(self.cols - 1);
            return;
        }

        // Check if we need to wrap
        if self.cursor.col >= self.cols {
            self.newline();
            self.cursor.col = 0;
        }

        // Get character width
        let width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);

        // Set the cell
        let row = self.cursor.row as usize;
        let col = self.cursor.col as usize;

        if row < self.cells.len() && col < self.cells[row].len() {
            self.cells[row][col] = Cell {
                character: c,
                fg_color: self.attributes.fg_color,
                bg_color: self.attributes.bg_color,
                bold: self.attributes.bold,
                italic: self.attributes.italic,
                underline: self.attributes.underline,
                wide: width > 1,
            };

            // For wide characters, mark the next cell as part of it
            if width > 1 && col + 1 < self.cells[row].len() {
                self.cells[row][col + 1] = Cell {
                    character: ' ',
                    wide: true,
                    ..Cell::default()
                };
            }
        }

        // Advance cursor
        self.cursor.col += width as u16;
    }

    /// Handle newline - may trigger scrolling
    pub fn newline(&mut self) {
        if self.cursor.row + 1 >= self.rows {
            self.scroll_up(1);
        } else {
            self.cursor.row += 1;
        }
    }

    /// Scroll the screen up by n lines
    pub fn scroll_up(&mut self, n: usize) {
        for _ in 0..n {
            if !self.cells.is_empty() {
                // Move top line to scrollback
                let line = self.cells.remove(0);
                if self.scrollback.len() >= self.max_scrollback {
                    self.scrollback.remove(0);
                }
                self.scrollback.push(line);

                // Add new blank line at bottom
                self.cells.push(vec![Cell::default(); self.cols as usize]);
            }
        }
    }

    /// Clear the screen
    pub fn clear(&mut self) {
        for row in &mut self.cells {
            for cell in row {
                *cell = Cell::default();
            }
        }
        self.cursor.col = 0;
        self.cursor.row = 0;
    }

    /// Clear from cursor to end of screen
    pub fn clear_below(&mut self) {
        // Clear rest of current line
        self.clear_line_from_cursor();

        // Clear all lines below
        for row_idx in (self.cursor.row as usize + 1)..self.cells.len() {
            for cell in &mut self.cells[row_idx] {
                *cell = Cell::default();
            }
        }
    }

    /// Clear from start of screen to cursor
    pub fn clear_above(&mut self) {
        // Clear lines above cursor
        for row_idx in 0..self.cursor.row as usize {
            for cell in &mut self.cells[row_idx] {
                *cell = Cell::default();
            }
        }

        // Clear current line up to cursor
        let row = self.cursor.row as usize;
        if row < self.cells.len() {
            for col in 0..=self.cursor.col as usize {
                if col < self.cells[row].len() {
                    self.cells[row][col] = Cell::default();
                }
            }
        }
    }

    /// Clear the current line
    pub fn clear_line(&mut self) {
        let row = self.cursor.row as usize;
        if row < self.cells.len() {
            for cell in &mut self.cells[row] {
                *cell = Cell::default();
            }
        }
    }

    /// Clear from cursor to end of line
    pub fn clear_line_from_cursor(&mut self) {
        let row = self.cursor.row as usize;
        if row < self.cells.len() {
            for col in self.cursor.col as usize..self.cells[row].len() {
                self.cells[row][col] = Cell::default();
            }
        }
    }

    /// Clear from start of line to cursor
    pub fn clear_line_to_cursor(&mut self) {
        let row = self.cursor.row as usize;
        if row < self.cells.len() {
            for col in 0..=self.cursor.col as usize {
                if col < self.cells[row].len() {
                    self.cells[row][col] = Cell::default();
                }
            }
        }
    }

    /// Set cursor position (1-based, as per ANSI)
    pub fn set_cursor(&mut self, row: u16, col: u16) {
        self.cursor.row = (row.saturating_sub(1)).min(self.rows - 1);
        self.cursor.col = (col.saturating_sub(1)).min(self.cols - 1);
    }

    /// Move cursor up
    pub fn cursor_up(&mut self, n: u16) {
        self.cursor.row = self.cursor.row.saturating_sub(n);
    }

    /// Move cursor down
    pub fn cursor_down(&mut self, n: u16) {
        self.cursor.row = (self.cursor.row + n).min(self.rows - 1);
    }

    /// Move cursor forward (right)
    pub fn cursor_forward(&mut self, n: u16) {
        self.cursor.col = (self.cursor.col + n).min(self.cols - 1);
    }

    /// Move cursor backward (left)
    pub fn cursor_backward(&mut self, n: u16) {
        self.cursor.col = self.cursor.col.saturating_sub(n);
    }

    /// Switch to alternate screen buffer
    pub fn enter_alt_screen(&mut self) {
        if self.alt_cells.is_none() {
            self.alt_cells = Some(std::mem::replace(
                &mut self.cells,
                vec![vec![Cell::default(); self.cols as usize]; self.rows as usize],
            ));
            self.alt_cursor = Some(std::mem::replace(&mut self.cursor, Cursor::default()));
        }
    }

    /// Switch back from alternate screen buffer
    pub fn exit_alt_screen(&mut self) {
        if let Some(cells) = self.alt_cells.take() {
            self.cells = cells;
        }
        if let Some(cursor) = self.alt_cursor.take() {
            self.cursor = cursor;
        }
    }

    /// Delete n characters at cursor, shifting the rest left
    pub fn delete_chars(&mut self, n: u16) {
        let row = self.cursor.row as usize;
        let col = self.cursor.col as usize;
        if row < self.cells.len() {
            for i in 0..n as usize {
                if col + i < self.cells[row].len() {
                    let src = col + n as usize + i;
                    if src < self.cells[row].len() {
                        self.cells[row][col + i] = self.cells[row][src].clone();
                    } else {
                        self.cells[row][col + i] = Cell::default();
                    }
                }
            }
        }
    }

    /// Insert n blank characters at cursor, shifting rest right
    pub fn insert_chars(&mut self, n: u16) {
        let row = self.cursor.row as usize;
        let col = self.cursor.col as usize;
        if row < self.cells.len() {
            // Shift right
            for i in (col..self.cells[row].len().saturating_sub(n as usize)).rev() {
                self.cells[row][i + n as usize] = self.cells[row][i].clone();
            }
            // Clear inserted area
            for i in 0..n as usize {
                if col + i < self.cells[row].len() {
                    self.cells[row][col + i] = Cell::default();
                }
            }
        }
    }

    /// Get cell at position (for rendering)
    pub fn get_cell(&self, row: usize, col: usize) -> Option<&Cell> {
        self.cells.get(row).and_then(|r| r.get(col))
    }
}
