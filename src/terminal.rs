use crate::drivers::cp437;

pub use crate::drivers::vga::{self, Color};

pub const SCROLLBACK: usize = 25;
const MAX_COLS: usize = 80;
const BUF_SIZE: usize = MAX_COLS * SCROLLBACK;

#[derive(Clone)]
pub struct Terminal {
    buffer: [u16; BUF_SIZE],

    x: usize,
    y: usize,

    width: usize,
    height: usize,

    cursor_row: usize, // absolute in buffer
    cursor_col: usize,

    view_row: usize, // first visible line
    total_rows: usize,

    dirty_first: usize,
    dirty_last: usize,

    fg: Color,
    bg: Color,

    pending_wrap: bool,
}

impl Terminal {
    pub const fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Self {
            x,
            y,
            buffer: [vga::entry(b' ', Color::Black, Color::White); BUF_SIZE],
            width,
            height,
            cursor_row: 0,
            cursor_col: 0,
            view_row: 0,
            total_rows: 0,
            pending_wrap: false,
            fg: Color::Black,
            bg: Color::White,
            // everything marked dirty
            dirty_last: height - 1,
            dirty_first: 0,
        }
    }

    pub fn resize(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
        self.mark_all_dirty();
    }

    pub fn mark_all_dirty(&mut self) {
        self.dirty_first = 0;
        self.dirty_last = self.height - 1;
    }

    fn mark_clean(&mut self) {
        self.dirty_first = self.height;
        self.dirty_last = 0;
    }

    fn mark_row_dirty(&mut self, abs_row: usize) {
        if abs_row < self.view_row {
            return;
        }
        let vis = abs_row - self.view_row;
        if vis >= self.height {
            return;
        }
        if self.dirty_first > self.dirty_last {
            self.dirty_first = vis;
            self.dirty_last = vis;
        } else {
            if vis < self.dirty_first {
                self.dirty_first = vis;
            }
            if vis > self.dirty_last {
                self.dirty_last = vis;
            }
        }
    }

    pub fn flush(&mut self) {
        if self.dirty_first <= self.dirty_last {
            for vis in self.dirty_first..self.dirty_last + 1 {
                let src_row = self.view_row + vis;
                if src_row < SCROLLBACK {
                    for col in 0..self.width {
                        vga::put_entry_at(
                            self.buffer[src_row * MAX_COLS + col],
                            col + self.x,
                            vis + self.y,
                        );
                    }
                } else {
                    for col in 0..self.width {
                        vga::put_entry_at(
                            vga::entry(b' ', self.fg, self.bg),
                            col + self.x,
                            vis + self.y,
                        );
                    }
                }
            }
            self.mark_clean();
        }
        self.update_cursor();
    }

    pub fn update_cursor(&self) {
        let content_row = self.cursor_row.saturating_sub(self.view_row);
        vga::set_position(self.y + content_row, self.x + self.cursor_col);
        vga::update_cursor();
    }

    pub fn put_raw(&mut self, c: u8, fg: Color, bg: Color) {
        if self.pending_wrap {
            self.newline();
        }
        self.buffer[self.cursor_row * MAX_COLS + self.cursor_col] = vga::entry(c, fg, bg);
        self.mark_row_dirty(self.cursor_row);
        self.cursor_col += 1;
        if self.cursor_col >= self.width {
            self.cursor_col = self.width - 1;
            self.pending_wrap = true;
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.width - 1;
        }
        self.pending_wrap = false;
        self.buffer[self.cursor_row * MAX_COLS + self.cursor_col] =
            vga::entry(b' ', self.fg, self.bg);
        self.mark_row_dirty(self.cursor_row);
    }

    pub fn clear(&mut self) {
        let blank = vga::entry(b' ', self.fg, self.bg);
        for cell in self.buffer.iter_mut() {
            *cell = blank;
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.view_row = 0;
        self.total_rows = 0;
        self.mark_all_dirty();
        self.flush();
    }

    fn newline(&mut self) {
        self.pending_wrap = false;
        self.cursor_col = 0;
        self.advance_row();
    }

    pub fn write(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                '\n' => self.newline(),
                '\r' => self.cursor_col = 0,
                '\t' => {
                    let next = (self.cursor_col + 8) & !7usize;
                    self.cursor_col = if next >= self.width {
                        self.width - 1
                    } else {
                        next
                    };
                }
                _ => self.put_raw(cp437::encode(c), self.fg, self.bg),
            }
        }
    }

    fn live_view_row(&self) -> usize {
        self.cursor_row.saturating_sub(self.height - 1)
    }

    fn advance_row(&mut self) {
        let old_view_row = self.view_row;
        self.cursor_row += 1;

        if self.cursor_row >= SCROLLBACK {
            // Shift scrollback buffer
            for i in 0..(SCROLLBACK - 1) * MAX_COLS {
                self.buffer[i] = self.buffer[i + MAX_COLS];
            }
            let blank = vga::entry(b' ', self.fg, self.bg);
            let last_start = (SCROLLBACK - 1) * MAX_COLS;
            for i in 0..MAX_COLS {
                self.buffer[last_start + i] = blank;
            }
            self.cursor_row = SCROLLBACK - 1;
            self.mark_all_dirty();
        }

        if self.cursor_row >= self.total_rows {
            self.total_rows = self.cursor_row + 1;
        }

        self.view_row = self.live_view_row();
        if self.view_row != old_view_row {
            self.mark_all_dirty();
        } else {
            self.mark_row_dirty(self.cursor_row);
        }
    }

    pub fn scroll_up(&mut self) {
        if self.view_row == 0 {
            return;
        }
        if self.view_row >= self.height {
            self.view_row -= self.height;
        } else {
            self.view_row = 0;
        }
        self.mark_all_dirty();
        self.flush();
    }

    pub fn scroll_down(&mut self) {
        let live_view = self.live_view_row();
        if self.view_row >= live_view {
            return;
        }
        self.view_row += self.height;
        if self.view_row > live_view {
            self.view_row = live_view;
        }
        self.mark_all_dirty();
        self.flush();
    }
}

pub fn init() {
    vga::disable_blink();
    vga::init();
}
