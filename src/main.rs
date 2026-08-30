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

use drivers::{keyboard::{self, Key}, vga};

const BOOT_SCREEN: &[&str] = &[
    "================================================================================",
    "                       K F S   -   K E R N E L   B O O T",
    "",
    "    ████████████████████████████████",
    "    ▀▀▀▀▀▀▀▀▀▀███▀▀▀▀▀▀███▀▀▀▀▀▀▀▀▀▀",
    "              ███      ███                           ▄█████▀   █████▀▀██████",
    "      ████████████████████████████                 ▄█████▀     ███▀   ██████",
    "      ██▀     ███      ██▀     ▀██               ▄█████▀       █▀     ██████",
    "      ██      ███      ██       ██             ▄█████▀               ▄█████▀",
    "      ██▄     ███     ▄██▄     ▄██           ▄█████▀               ▄█████▀",
    "      ████████████████████████████         ▄█████▀               ▄█████▀",
    "               ███                     x  ███████▄▄▄▄▄▄▄▄▄▄▄▄  ▄██████     ▄",
    "             ▄███                         ███████████████████  ███████   ▄██",
    "   ██████████████████████████████████     ███████████████████  ███████ ▄████",
    "           ███▀          ▄███                          ██████  ▀▀▀▀▀▀▀▀▀▀▀▀▀",
    "         ▄███           ▄██▀                           ██████",
    "        ▄█████████▄▄▄▄▄███▀                            ██████",
    "                ██████████▄▄▄▄",
    "    ▄▄▄▄▄▄▄▄██████▀▀▀   ▀▀▀██████▄▄",
    "    ▀████▀▀▀▀▀▀               ▀▀▀█▀",
    "",
    "                     Copyright (C) 2026  smamalig, sjossain",
    "================================================================================",
];

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_boot_screen() {
    for line in BOOT_SCREEN {
        print_line(line);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    vga::init();
    gdt::init();
    idt::init();
    pic::init();
    keyboard::init();

    print_boot_screen();

    shell::init();

    idt::enable_interrupts();
    loop {
        shell::handle_keyboard();
        if let Some(event) = keyboard::get_key() {
            if let Key::Function(fn_key) = event.key {
                vga::switch_terminal(fn_key);
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
fn panic(info: &PanicInfo) -> ! {
    vga::set_color(vga::Color::Red);
    println!("{}", info);
    vga::disable_cursor();
    println!("System halted.");
    loop {
        unsafe {
            asm!("cli; hlt");
        }
    }
}
