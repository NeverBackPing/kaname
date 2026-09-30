use crate::scrollback;
use crate::{drivers::cp437, scrollback::RowHandle};

pub use crate::drivers::vga::{self, Color};

#[derive(Clone)]
pub struct Terminal {
    head: Option<scrollback::RowHandle>,

    x: usize,
    y: usize,

    width: usize,
    height: usize,

    cursor_row: usize, // absolute in buffer
    cursor_col: usize,

    view_row: usize, // first visible line

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
            width,
            head: None,
            height,
            cursor_row: 0,
            cursor_col: 0,
            view_row: 0,
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
        self.view_row = self.live_view_row();
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

    fn blank_line(&mut self, y: usize) {
        for col in 0..self.width {
            vga::put_entry_at(vga::entry(b' ', self.fg, self.bg), col + self.x, y + self.y);
        }
    }

    pub fn flush(&mut self) {
        if self.dirty_first > self.dirty_last {
            self.update_cursor();
            return;
        }

        let (mut cur_handle, mut cur_row) = (Some(self.get_current_row()), self.cursor_row);
        let last = self.dirty_last.min(self.cursor_row - self.view_row);

        while cur_row > self.view_row + self.dirty_last {
            cur_row -= 1;
            if let Some(handle) = cur_handle {
                cur_handle = handle.prev();
            }
        }

        for y in last + 1..=self.dirty_last {
            self.blank_line(y);
        }

        for y in (self.dirty_first..=last).rev() {
            let row = if let Some(handle) = cur_handle {
                scrollback::get_row_data(handle)
            } else {
                None
            };
            if let Some(row) = row {
                for (col, &cell) in row.iter().enumerate().take(self.width) {
                    vga::put_entry_at(cell, col + self.x, y + self.y);
                }
            } else {
                self.blank_line(y);
            }

            if let Some(handle) = cur_handle {
                cur_handle = handle.prev();
            }
        }
        self.mark_clean();
        self.update_cursor();
    }

    pub fn update_cursor(&mut self) {
        let content_row = self.cursor_row.saturating_sub(self.view_row);
        if content_row >= self.height {
            vga::disable_cursor();
            return;
        }
        vga::enable_cursor(14, 15);
        vga::set_position(self.y + content_row, self.x + self.cursor_col);
        vga::update_cursor();
    }

    fn get_current_row(&mut self) -> RowHandle {
        if let Some(row) = self.head {
            if scrollback::get_row_data(row).is_some() {
                return row;
            }
            self.mark_all_dirty();
        }
        let row = scrollback::allocate_row(None, vga::entry(b' ', self.fg, self.bg));
        self.head = Some(row);
        row
    }

    pub fn put_raw(&mut self, c: u8, fg: Color, bg: Color) {
        if self.pending_wrap {
            self.newline();
        }
        let row = scrollback::get_row_data(self.get_current_row());
        let Some(row) = row else {
            return;
        };
        row[self.cursor_col] = vga::entry(c, fg, bg);
        self.mark_row_dirty(self.cursor_row);
        self.cursor_col += 1;
        let old_view_row = self.live_view_row();
        if self.view_row != old_view_row {
            self.mark_all_dirty();
            self.view_row = old_view_row;
        }
        if self.cursor_col >= self.width {
            self.cursor_col = self.width - 1;
            self.pending_wrap = true;
        }
    }

    pub fn backspace(&mut self) {
        if self.pending_wrap {
            self.pending_wrap = false;
        } else if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            let Some(prev) = self.head.and_then(|h| h.prev()) else {
                return;
            };
            if scrollback::get_row_data(prev).is_none() {
                return;
            }
            self.head = Some(prev);
            self.cursor_row -= 1;
            self.cursor_col = self.width - 1;
        } else {
            return;
        }

        let row = scrollback::get_row_data(self.get_current_row());
        let Some(row) = row else { return };
        row[self.cursor_col] = vga::entry(b' ', self.fg, self.bg);
        self.mark_row_dirty(self.cursor_row);
        let old_view_row = self.live_view_row();
        if old_view_row != self.view_row {
            self.view_row = old_view_row;
            self.mark_all_dirty();
        }
    }

    pub fn clear(&mut self) {
        self.head = None;
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.view_row = 0;
        self.pending_wrap = false;
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
                '\r' => {
                    self.cursor_col = 0;
                    self.pending_wrap = false;
                }
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
        let row = scrollback::allocate_row(self.head, vga::entry(b' ', self.fg, self.bg));
        let old_view_row = self.view_row;

        self.head = Some(row);
        self.cursor_row += 1;

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
