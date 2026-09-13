// This module is the raw VGA driver
// virtual terminal management is in terminal.rs

use crate::ports;
use core::ptr::write_volatile;

pub const WIDTH: usize = 80;
pub const HEIGHT: usize = 25;
const VGA_MEMORY: *mut u16 = 0xB8000 as *mut u16;

#[repr(u8)]
#[derive(Clone, Copy)]
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

pub const fn entry(c: u8, fg: Color, bg: Color) -> u16 {
    let attr = (fg as u8) | ((bg as u8) << 4);
    (c as u16) | ((attr as u16) << 8)
}

static mut ROW: usize = 0;
static mut COL: usize = 0;
static mut FG: Color = Color::Black;
static mut BG: Color = Color::White;

#[inline]
pub fn put_entry_at(entry: u16, x: usize, y: usize) {
    unsafe {
        write_volatile(VGA_MEMORY.add(y * WIDTH + x), entry);
    }
}

fn clear_line(line: usize) {
    for x in 0..WIDTH {
        unsafe {
            put_entry_at(entry(b' ', FG, BG), x, line);
        }
    }
}

pub fn init() {
    unsafe {
        ROW = 0;
        COL = 0;
        BG = Color::White;
        FG = Color::Black;
        clear_screen(FG, BG);
        enable_cursor(14, 15);
        update_cursor();
    }
}

pub fn set_color(fg: Color, bg: Color) {
    unsafe {
        FG = fg;
        BG = bg;
    }
}

pub fn set_position(row: usize, col: usize) {
    unsafe {
        ROW = row;
        COL = col;
    }
}

pub fn clear_screen(fg: Color, bg: Color) {
    set_color(fg, bg);
    for y in 0..HEIGHT {
        clear_line(y);
    }
}

pub fn enable_cursor(start: u8, end: u8) {
    ports::outb(0x3D4, 0x0A);
    ports::outb(0x3D5, (ports::inb(0x3D5) & 0xC0) | start);
    ports::outb(0x3D4, 0x0B);
    ports::outb(0x3D5, (ports::inb(0x3D5) & 0xE0) | end);
}

pub fn disable_cursor() {
    ports::outb(0x3D4, 0x0A);
    ports::outb(0x3D5, 0x20);
}

pub fn update_cursor() {
    let pos: u16 = unsafe { ROW * WIDTH + COL } as u16;
    ports::outb(0x3D4, 0x0F);
    ports::outb(0x3D5, pos as u8);
    ports::outb(0x3D4, 0x0E);
    ports::outb(0x3D5, (pos >> 8) as u8);
}

// TODO document function
pub fn disable_blink() {
    // Set 0x3C0 to the index state
    ports::inb(0x3DA);
    // Write index 0x10
    ports::outb(0x3C0, 0x10);
    let mode = ports::inb(0x3C1);

    ports::inb(0x3DA);
    ports::outb(0x3C0, 0x10);
    ports::outb(0x3C0, mode & !0x08);

    ports::inb(0x3DA);
    ports::outb(0x3C0, 0x20);
}
