use core::cell::UnsafeCell;
use core::fmt;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicU8, Ordering};

use crate::ports;

const VGA_BUFFER: *mut u16 = 0xb8000 as *mut u16;
const WIDTH: usize = 80;
const HEIGHT: usize = 25;

pub const MAX_TERMINALS: usize = 6;

static ACTIVE_TERMINAL: AtomicU8 = AtomicU8::new(0);

#[repr(u8)]
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGrey = 7,
    DarkGrey = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    LightMagenta = 13,
    Yellow = 14,
    White = 15,
}

const fn entry(c: u8, fg: Color, bg: Color) -> u16 {
    let attr = (fg as u8) | ((bg as u8) << 4);
    (c as u16) | ((attr as u16) << 8)
}

pub struct InfoTty {
    name: u8,
    history: [[u16; WIDTH]; HEIGHT],
}

pub struct Writer {
    row: usize,
    col: usize,
    fg: Color,
    bg: Color,
    pub id: InfoTty,
}

impl Writer {
    const fn new() -> Self {
        Self {
            row: 0,
            col: 0,
            fg: Color::LightGreen,
            bg: Color::Black,
            id: InfoTty {
                name: 0,
                history: [[
                    entry(b' ', Color::LightGreen, Color::Black);
                    WIDTH
                ]; HEIGHT],
            },
        }
    }

    fn put_at(&mut self, row: usize, col: usize, c: u8) {
        unsafe {
            write_volatile(
                VGA_BUFFER.add(row * WIDTH + col),
                entry(c, self.fg, self.bg),
            );
        }

        self.update_cursor();
    }

    pub fn clear_screen(&mut self, color: Color) {
        self.bg = color;
        self.row = 0;
        self.col = 0;

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                unsafe {
                    write_volatile(
                        VGA_BUFFER.add(y * WIDTH + x),
                        entry(b' ', self.fg, self.bg),
                    );
                }
            }
        }

        self.update_cursor();
    }

    pub fn enable_cursor(&mut self, start: u8, end: u8) {
        ports::outb(0x3D4, 0x0A);
        ports::outb(0x3D5, (ports::inb(0x3D5) & 0xC0) | start);

        ports::outb(0x3D4, 0x0B);
        ports::outb(0x3D5, (ports::inb(0x3D5) & 0xE0) | end);
    }

    #[allow(dead_code)]
    pub fn disable_cursor(&mut self) {
        ports::outb(0x3D4, 0x0A);
        ports::outb(0x3D5, 0x20);
    }

    pub fn update_cursor(&mut self) {
        let pos = (self.row * WIDTH + self.col) as u16;

        ports::outb(0x3D4, 0x0F);
        ports::outb(0x3D5, pos as u8);

        ports::outb(0x3D4, 0x0E);
        ports::outb(0x3D5, (pos >> 8) as u8);
    }

    fn scroll(&mut self) {
        for row in 1..HEIGHT {
            for col in 0..WIDTH {
                let cell =
                    unsafe { read_volatile(VGA_BUFFER.add(row * WIDTH + col)) };

                unsafe {
                    write_volatile(
                        VGA_BUFFER.add((row - 1) * WIDTH + col),
                        cell,
                    );
                }
            }
        }

        for col in 0..WIDTH {
            unsafe {
                write_volatile(
                    VGA_BUFFER.add((HEIGHT - 1) * WIDTH + col),
                    entry(b' ', self.fg, self.bg),
                );
            }
        }

        self.row = HEIGHT - 1;
        self.col = 0;

        self.update_cursor();
    }

    fn newline(&mut self) {
        self.col = 0;

        if self.row + 1 >= HEIGHT {
            self.scroll();
        } else {
            self.row += 1;
        }

        self.update_cursor();
    }

    pub fn write_byte(&mut self, b: u8) {
        match b {
            b'\n' => {
                self.newline();
            }

            0x08 => {
                self.backspace();
            }

            byte => {
                if self.col >= WIDTH {
                    self.newline();
                }

                self.put_at(self.row, self.col, byte);
                self.col += 1;

                self.update_cursor();
            }
        }
    }

    pub fn backspace(&mut self) {
        if self.col > 0 {
            self.col -= 1;
        } else if self.row > 0 {
            self.row -= 1;
            self.col = WIDTH - 1;
        } else {
            return;
        }

        unsafe {
            write_volatile(
                VGA_BUFFER.add(self.row * WIDTH + self.col),
                entry(b' ', self.fg, self.bg),
            );
        }

        self.update_cursor();
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            self.write_byte(b);
        }

        Ok(())
    }
}

struct Terminal(UnsafeCell<Writer>);

unsafe impl Sync for Terminal {}

static TERMINAL: [Terminal; MAX_TERMINALS] = [
    Terminal(UnsafeCell::new(Writer::new())),
    Terminal(UnsafeCell::new(Writer::new())),
    Terminal(UnsafeCell::new(Writer::new())),
    Terminal(UnsafeCell::new(Writer::new())),
    Terminal(UnsafeCell::new(Writer::new())),
    Terminal(UnsafeCell::new(Writer::new())),
];

pub fn save_tty(terminal: &mut Writer) {
    for row in 0..HEIGHT {
        for col in 0..WIDTH {
            terminal.id.history[row][col] =
                unsafe { read_volatile(VGA_BUFFER.add(row * WIDTH + col)) };
        }
    }
}

pub fn restore_tty(terminal: &Writer) {
    for row in 0..HEIGHT {
        for col in 0..WIDTH {
            unsafe {
                write_volatile(
                    VGA_BUFFER.add(row * WIDTH + col),
                    terminal.id.history[row][col],
                );
            }
        }
    }
}

pub fn switch_terminal(tty_id: u8) {
    if tty_id >= MAX_TERMINALS as u8 {
        return;
    }

    let current_id = ACTIVE_TERMINAL.load(Ordering::Relaxed);

    if current_id == tty_id {
        return;
    }

    let previous_terminal =
        TERMINAL[current_id as usize].0.get();

    unsafe {
        save_tty(&mut *previous_terminal);
    }

    ACTIVE_TERMINAL.store(tty_id, Ordering::Relaxed);

    let new_terminal =
        TERMINAL[tty_id as usize].0.get();

    unsafe {
        restore_tty(&*new_terminal);
        (*new_terminal).update_cursor();
    }
}

pub fn init() {
    for (n, tty) in (0_u8..).zip(TERMINAL.iter().take(MAX_TERMINALS)) {
        let terminal = tty.0.get();

        unsafe {
            (*terminal).clear_screen(Color::Black);
            (*terminal).enable_cursor(14, 15);
            (*terminal).id.name = n;
        }
    }

    ACTIVE_TERMINAL.store(0, Ordering::Relaxed);

    let terminal = TERMINAL[0].0.get();

    unsafe {
        (*terminal).update_cursor();
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use fmt::Write;

    let active =
        ACTIVE_TERMINAL.load(Ordering::Relaxed) as usize;

    unsafe {
        (*TERMINAL[active].0.get())
            .write_fmt(args)
            .unwrap();
    }
}

pub fn putc(c: u8) {
    let active =
        ACTIVE_TERMINAL.load(Ordering::Relaxed) as usize;

    unsafe {
        (*TERMINAL[active].0.get()).write_byte(c);
    }
}

pub fn backspace() {
    let active =
        ACTIVE_TERMINAL.load(Ordering::Relaxed) as usize;

    unsafe {
        (*TERMINAL[active].0.get()).backspace();
    }
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::drivers::vga::_print(
            format_args!("\n")
        )
    };

    ($($arg:tt)*) => {
        $crate::drivers::vga::_print(
            format_args!("{}\n", format_args!($($arg)*))
        )
    };
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::drivers::vga::_print(
            format_args!($($arg)*)
        )
    };
}