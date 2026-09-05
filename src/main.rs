#![no_std]
#![no_main]
#![feature(default_field_values)]
#![allow(dead_code)]

use core::arch::asm;
use core::panic::PanicInfo;

mod boot;
mod drivers;
mod gdt;
mod idt;
mod libk;
mod multiboot2;
mod pic;
mod ports;
mod shell;
mod memory;

use drivers::{
    keyboard::{self, Key},
    terminal::{self, Color},
    vga,
};

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
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn kernel_main(_magic: u32, info_raw: *const multiboot2::Info) -> ! {
    gdt::init();
    idt::init();
    terminal::init();
    memory::init_paging();
    pic::init();
    keyboard::init();

    print_boot_screen();

    shell::init();

    idt::enable_interrupts();

    log!("[boot] init complete");

    if let Some(info) = unsafe { info_raw.as_ref() } {
        for tag in info.tags() {
            if let multiboot2::Tag::Mmap(tag) = tag {
                log!("[mmap] tag detected");
                for entry in tag.entries() {
                    let type_name = match entry.type_ {
                        1 => "Available",
                        2 => "Reserved",
                        3 => "AcpiInfo",
                        4 => "HibernationReserved",
                        5 => "DefectiveRam",
                        _ => "Unknown",
                    };
                    log!(
                        "[mmap] {:#010X}-{:#010X}  {}",
                        entry.base_addr,
                        entry.base_addr + entry.length - 1,
                        type_name,
                    );
                }
            }
        }
    }

    loop {
        shell::handle_keyboard();
        if let Some(event) = keyboard::get_key() {
            if let Key::Function(fn_key) = event.key {
                terminal::switch_to(fn_key);
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
    terminal::set_color(Color::Red, Color::White);
    println!("{}", info);
    vga::disable_cursor();
    println!("System halted.");
    loop {
        unsafe {
            asm!("cli; hlt");
        }
    }
}
