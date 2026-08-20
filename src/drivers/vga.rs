use core::cell::UnsafeCell;
use core::fmt;
use core::ptr::{read_volatile, write_volatile};

const VGA_BUFFER: *mut u16 = 0xb8000 as *mut u16;
const WIDTH: usize = 80;
const HEIGHT: usize = 25;

#[repr(u8)]
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum Color {
    Black = 0, Blue = 1, Green = 2, Cyan = 3, Red = 4, Magenta = 5,
    Brown = 6, LightGrey = 7, DarkGrey = 8, LightBlue = 9, LightGreen = 10,
    LightCyan = 11, LightRed = 12, LightMagenta = 13, Yellow = 14, White = 15,
}

fn entry(c: u8, fg: Color, bg: Color) -> u16 {
    let attr = (fg as u8) | ((bg as u8) << 4);
    (c as u16) | ((attr as u16) << 8)
}

pub struct Writer {
    row: usize,
    col: usize,
    fg: Color,
    bg: Color,
}

impl Writer {
    const fn new() -> Self {
        Writer { row: 0, col: 0, fg: Color::LightGreen, bg: Color::Black }
    }

    fn put_at(&self, row: usize, col: usize, c: u8) {
        unsafe {
            write_volatile(VGA_BUFFER.add(row * WIDTH + col), entry(c, self.fg, self.bg));
        }
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
        if self.row + 1 >= HEIGHT { self.scroll() } else { self.row += 1 }
    }

    pub fn write_byte(&mut self, b: u8) {
        match b {
            b'\n' => self.newline(),
            byte => {
                if self.col >= WIDTH { self.newline() }
                self.put_at(self.row, self.col, byte);
                self.col += 1;
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
