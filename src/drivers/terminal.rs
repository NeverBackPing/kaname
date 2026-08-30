use core::cell::UnsafeCell;
use core::fmt;
use core::sync::atomic::{AtomicU8, Ordering};

pub use crate::drivers::vga::{self, Color};

pub const COLS: usize = vga::WIDTH;
pub const ROWS: usize = vga::HEIGHT;
pub const SCROLLBACK: usize = 200;
const BUF_SIZE: usize = COLS * SCROLLBACK;

pub const MAX_TTYS: usize = 7;
pub const LOG_TTY: usize = 6;

static ACTIVE_TTY: AtomicU8 = AtomicU8::new(0);

struct Terminal {
    buffer: [u16; BUF_SIZE],

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
    pub const fn new() -> Self {
        Self {
            buffer: [vga::entry(b' ', Color::Black, Color::White); BUF_SIZE],
            cursor_row: 0,
            cursor_col: 0,
            view_row: 0,
            total_rows: 0,
            pending_wrap: false,
            fg: Color::Black,
            bg: Color::White,
            dirty_last: ROWS - 1,
            dirty_first: 0,
        }
    }

    fn mark_all_dirty(&mut self) {
        self.dirty_first = 0;
        self.dirty_last = ROWS - 1;
    }

    fn mark_clean(&mut self) {
        self.dirty_first = ROWS;
        self.dirty_last = 0;
    }

    fn mark_row_dirty(&mut self, abs_row: usize) {
        if abs_row < self.view_row {
            return;
        }
        let vis = abs_row - self.view_row;
        if vis >= ROWS {
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
        let active = ACTIVE_TTY.load(Ordering::Relaxed) as usize;
        if !core::ptr::eq(self, TTYS[active].0.get()) {
            return;
        }

        if self.dirty_first <= self.dirty_last {
            for vis in self.dirty_first..self.dirty_last + 1 {
                let src_row = self.view_row + vis;
                if src_row < SCROLLBACK {
                    for col in 0..COLS {
                        vga::put_entry_at(self.buffer[src_row * COLS + col], col, vis);
                    }
                } else {
                    for col in 0..COLS {
                        vga::put_entry_at(vga::entry(b' ', self.fg, self.bg), col, vis);
                    }
                }
            }
            self.mark_clean();
        }
        let content_row = self.cursor_row.saturating_sub(self.view_row);
        vga::set_position(content_row, self.cursor_col);
        vga::update_cursor();
    }

    pub fn put_raw(&mut self, c: u8, fg: Color, bg: Color) {
        if self.pending_wrap {
            self.newline();
        }
        self.buffer[self.cursor_row * COLS + self.cursor_col] = vga::entry(c, fg, bg);
        self.mark_row_dirty(self.cursor_row);
        self.cursor_col += 1;
        if self.cursor_col >= COLS {
            self.cursor_col = COLS - 1;
            self.pending_wrap = true;
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = COLS - 1;
        }
        self.pending_wrap = false;
        self.buffer[self.cursor_row * COLS + self.cursor_col] = vga::entry(b' ', self.fg, self.bg);
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

    fn live_view_row(&self) -> usize {
        self.cursor_row.saturating_sub(ROWS - 1)
    }

    fn advance_row(&mut self) {
        let old_view_row = self.view_row;
        self.cursor_row += 1;

        if self.cursor_row >= SCROLLBACK {
            // Shift scrollback buffer
            for i in 0..(SCROLLBACK - 1) * COLS {
                self.buffer[i] = self.buffer[i + COLS];
            }
            let blank = vga::entry(b' ', self.fg, self.bg);
            let last_start = (SCROLLBACK - 1) * COLS;
            for i in 0..COLS {
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
        if self.view_row >= ROWS {
            self.view_row -= ROWS;
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
        self.view_row += ROWS;
        if self.view_row > live_view {
            self.view_row = live_view;
        }
        self.mark_all_dirty();
        self.flush();
    }
}

impl fmt::Write for Terminal {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            match c {
                '\n' => self.newline(),
                '\r' => self.cursor_col = 0,
                '\t' => {
                    let next = (self.cursor_col + 8) & !7usize;
                    self.cursor_col = if next >= COLS { COLS - 1 } else { next };
                }
                _ => self.put_raw(super::cp437::encode(c), self.fg, self.bg),
            }
        }
        self.flush();

        Ok(())
    }
}

pub struct Tty(UnsafeCell<Terminal>);

unsafe impl Sync for Tty {}

static TTYS: [Tty; MAX_TTYS] = [
    Tty(UnsafeCell::new(Terminal::new())),
    Tty(UnsafeCell::new(Terminal::new())),
    Tty(UnsafeCell::new(Terminal::new())),
    Tty(UnsafeCell::new(Terminal::new())),
    Tty(UnsafeCell::new(Terminal::new())),
    Tty(UnsafeCell::new(Terminal::new())),
    Tty(UnsafeCell::new(Terminal::new())),
];

fn get_active_terminal() -> &'static mut Terminal {
    let active = ACTIVE_TTY.load(Ordering::Relaxed) as usize;

    unsafe { &mut *TTYS[active].0.get() }
}

pub fn switch_to(new_id: u8) {
    if new_id >= MAX_TTYS as u8 {
        return;
    }
    let prev_id = ACTIVE_TTY.load(Ordering::Relaxed);

    if prev_id == new_id {
        return;
    }
    ACTIVE_TTY.store(new_id, Ordering::Relaxed);
    let term = get_active_terminal();
    term.mark_all_dirty();
    term.flush();
}

pub fn scroll_up() {
    get_active_terminal().scroll_up();
}

pub fn scroll_down() {
    get_active_terminal().scroll_down();
}

pub fn backspace() {
    let term = get_active_terminal();
    term.backspace();
    term.flush();
}

pub fn put_raw(c: u8, fg: Color, bg: Color) {
    let term = get_active_terminal();
    term.put_raw(c, fg, bg);
    term.flush();
}

pub fn clear() {
    get_active_terminal().clear();
}

pub fn set_color(fg: Color, bg: Color) {
    let term = get_active_terminal();
    term.fg = fg;
    term.bg = bg;
}

pub fn init() {
    vga::init();
    vga::disable_blink();

    ACTIVE_TTY.store(0, Ordering::Relaxed);
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use fmt::Write;

    get_active_terminal().write_fmt(args).unwrap();
}

#[doc(hidden)]
pub fn _log(args: fmt::Arguments) {
    use fmt::Write;

    let term = unsafe { &mut *TTYS[LOG_TTY].0.get() };
    term.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::drivers::terminal::_print(
            format_args!("\n"),
        )
    };

    ($($arg:tt)*) => {
        $crate::drivers::terminal::_print(
            format_args!("{}\n", format_args!($($arg)*)),
        )
    };
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::drivers::terminal::_print(
            format_args!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::drivers::terminal::_log(
            format_args!($($arg)*),
        )
    };
}
