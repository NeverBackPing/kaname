#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

mod boot;
mod drivers;
mod gdt;
mod idt;
mod libk;
mod pic;
mod ports;
mod shell;

use drivers::keyboard::{self, Key};

// CP437 byte values used in this art:
//   ' ' = 0x20   '=' = 0x3D   'x' = 0x78
//   '\u{2591}' (light shade  ░) = 0xB0
//   '\u{2592}' (medium shade ▒) = 0xB1
//   '\u{2593}' (dark shade   ▓) = 0xB2
//   '\u{2588}' (full block   █) = 0xDB
// All other characters (letters, digits, punctuation) are plain ASCII
// and map 1:1 to the same byte value in CP437.
//
// Bytes >= 0x80 are not valid UTF-8 on their own, so they can't live in a
// normal &str literal -- these are byte-string literals (b"...") instead,
// i.e. plain [u8; N] arrays. Feed these directly to your VGA byte-writer.

const BOOT_SCREEN: &[&[u8]] = &[
    b"================================================================================",
    b"                       K F S   -   K E R N E L   B O O T                        ",
    b"",
    b"    \xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB                                        ",
    b"    \xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDB\xDB\xDB\xDF\xDF\xDF\xDF\xDF\xDF\xDB\xDB\xDB\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF                                        ",
    b"              \xDB\xDB\xDB      \xDB\xDB\xDB                           \xDC\xDB\xDB\xDB\xDB\xDB\xDF   \xDB\xDB\xDB\xDB\xDB\xDF\xDF\xDB\xDB\xDB\xDB\xDB\xDB",
    b"      \xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB                 \xDC\xDB\xDB\xDB\xDB\xDB\xDF     \xDB\xDB\xDB\xDF   \xDB\xDB\xDB\xDB\xDB\xDB",
    b"      \xDB\xDB\xDF     \xDB\xDB\xDB      \xDB\xDB\xDF     \xDF\xDB\xDB               \xDC\xDB\xDB\xDB\xDB\xDB\xDF       \xDB\xDF     \xDB\xDB\xDB\xDB\xDB\xDB",
    b"      \xDB\xDB      \xDB\xDB\xDB      \xDB\xDB       \xDB\xDB             \xDC\xDB\xDB\xDB\xDB\xDB\xDF               \xDB\xDB\xDB\xDB\xDB\xDB\xDF",
    b"      \xDB\xDB\xDC     \xDB\xDB\xDB     \xDC\xDB\xDB\xDC     \xDC\xDB\xDB           \xDC\xDB\xDB\xDB\xDB\xDB\xDF               \xDC\xDB\xDB\xDB\xDB\xDB\xDF  ",
    b"      \xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB         \xDC\xDB\xDB\xDB\xDB\xDB\xDF               \xDC\xDB\xDB\xDB\xDB\xDB\xDF    ",
    b"               \xDB\xDB\xDB                     x  \xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC  \xDC\xDB\xDB\xDB\xDB\xDB\xDB     \xDC",
    b"             \xDC\xDB\xDB\xDB                         \xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB  \xDB\xDB\xDB\xDB\xDB\xDB\xDB   \xDC\xDB\xDB",
    b"   \xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB     \xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB  \xDB\xDB\xDB\xDB\xDB\xDB\xDB \xDC\xDB\xDB\xDB\xDB",
    b"           \xDB\xDB\xDB\xDF          \xDC\xDB\xDB\xDB                          \xDB\xDB\xDB\xDB\xDB\xDB   \xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF",
    b"         \xDC\xDB\xDB\xDB           \xDC\xDB\xDB\xDF                           \xDB\xDB\xDB\xDB\xDB\xDB               ",
    b"        \xDC\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDC\xDC\xDC\xDC\xDC\xDB\xDB\xDB\xDF                            \xDB\xDB\xDB\xDB\xDB\xDB               ",
    b"                \xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDC\xDC\xDC\xDC                                              ",
    b"    \xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDB\xDB\xDB\xDB\xDB\xDB\xDF\xDF\xDF   \xDF\xDF\xDF\xDB\xDB\xDB\xDB\xDB\xDB\xDC\xDC                                         ",
    b"    \xDF\xDB\xDB\xDB\xDB\xDF\xDF\xDF\xDF\xDF\xDF               \xDF\xDF\xDF\xDB\xDF                                         ",
    b"",
    b"                     Copyright (C) 2026  smamalig, sjossain                     ",
    b"================================================================================",
];

fn print_line(line: &[u8]) {
    for &byte in line {
        drivers::vga::putc(byte);
    }
    drivers::vga::putc(b'\n');
}

fn print_boot_screen() {
    for line in BOOT_SCREEN {
        print_line(line);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    drivers::vga::init();
    gdt::init();
    idt::init();
    pic::init();
    keyboard::init();
    shell::init();

    print_boot_screen();

    idt::enable_interrupts();
    loop {
        shell::handle_keyboard();
        if let Some(event) = keyboard::get_key() {
            if let Key::Function(fn_key) = event.key {
                drivers::vga::switch_terminal(fn_key);
            } else if let Key::Char(c) = event.key {
                print!("{}", c as char);
            }
        }
        unsafe {
            asm!("hlt");
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    #[allow(clippy::empty_loop)]
    loop {
        unsafe {
            asm!("cli; hlt");
        }
    }
}
