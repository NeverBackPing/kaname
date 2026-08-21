use core::cell::UnsafeCell;
use core::fmt;
use core::ptr::{read_volatile, write_volatile};

use crate::ports;

const VGA_BUFFER: *mut u16 = 0xb8000 as *mut u16;
const WIDTH: usize = 80;
const HEIGHT: usize = 25;

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

fn entry(c: u8, fg: Color, bg: Color) -> u16 {
    let attr = (fg as u8) | ((bg as u8) << 4);
    (c as u16) | ((attr as u16) << 8)
}

pub fn init() -> () {
    let terminal = TERMINAL.0.get();
    unsafe {
        (*terminal).clear_screen(Color::Black);
        (*terminal).enable_cursor(14, 15);
        (*terminal).update_cursor();
    };
}

pub struct Writer {
    row: usize,
    col: usize,
    fg: Color,
    bg: Color,
}

impl Writer {
    const fn new() -> Self {
        Writer {
            row: 0,
            col: 0,
            fg: Color::LightGreen,
            bg: Color::Black,
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
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                unsafe {
                    write_volatile(VGA_BUFFER.add(y * WIDTH + x), entry(b' ', self.fg, self.bg));
                }
            }
        }
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
        let pos: u16 = (self.row * WIDTH + self.col) as u16;
        ports::outb(0x3D4, 0x0F);
        ports::outb(0x3D5, pos as u8);
        ports::outb(0x3D4, 0x0E);
        ports::outb(0x3D5, (pos >> 8) as u8);
    }

    fn scroll(&mut self) {
        for row in 1..HEIGHT {
            for col in 0..WIDTH {
                let cell = unsafe { read_volatile(VGA_BUFFER.add(row * WIDTH + col)) };
                unsafe { write_volatile(VGA_BUFFER.add((row - 1) * WIDTH + col), cell) };
            }
        }
        for col in 0..WIDTH {
            self.put_at(HEIGHT - 1, col, b' ');
        }
        self.row = HEIGHT - 1;
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
            b'\n' => self.newline(),
            byte => {
                if self.col >= WIDTH {
                    self.newline()
                }
                self.put_at(self.row, self.col, byte);
                self.col += 1;
                self.update_cursor();
            }
        }
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

static TERMINAL: Terminal = Terminal(UnsafeCell::new(Writer::new()));

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use fmt::Write;
    unsafe { (*TERMINAL.0.get()).write_fmt(args).unwrap() };
}

#[macro_export]
macro_rules! println {
    () => ($crate::drivers::vga::_print(format_args!("\n")));
    ($($arg:tt)*) => ($crate::drivers::vga::_print(format_args!("{}\n", format_args!($($arg)*))));
}
