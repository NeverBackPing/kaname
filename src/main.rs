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
mod memory;
mod multiboot2;
mod pic;
mod ports;
mod shell;

use drivers::{
    keyboard::{self, Key},
    terminal::{self, Color},
    vga,
};
use idt::InterruptFrame;

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

fn page_fault_handler(frame: &InterruptFrame) {
    let fault_addr: u32;

    unsafe {
        asm!(
            "mov {0:e}, cr2",
            out(reg) fault_addr,
        );
    }

    let error = idt::PageFaultError(frame.error_code());

    let privilege = if error.user() { "User" } else { "Kernel" };

    let kind = if error.reserved_bit() {
        "Reserved Bit Violation"
    } else if error.violation() {
        "Protection Violation"
    } else {
        "Page Not Present"
    };

    let access = if error.instr_fetch() {
        "Instruction Fetch"
    } else if error.write() {
        "Write"
    } else {
        "Read"
    };

    panic!(
        "\
┌── PAGE FAULT ─── Error={:#010X} ─────────────────────────────┐
│ {} {} on {:<padding$} │
│ CR2={:#010X} EIP={:#010X} CS ={:#010X} EFLAGS={:#010X} │
│ EAX={:#010X} EBX={:#010X} ECX={:#010X}    EDX={:#010X} │
│ ESI={:#010X} EDI={:#010X} EBP={:#010X}    ESP={:#010X} │
└────────────────────────────────────────────────────────────────┘",
        frame.error_code(),
        privilege,
        kind,
        access,
        fault_addr,
        frame.eip(),
        frame.cs(),
        frame.eflags(),
        frame.eax(),
        frame.ebx(),
        frame.ecx(),
        frame.edx(),
        frame.esi(),
        frame.edi(),
        frame.ebp(),
        frame.esp(),
        padding = 57 - privilege.len() - kind.len(),
    );
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

    idt::register_handler(idt::InterruptVector::PageFault as u8, page_fault_handler);
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
    idt::disable_interrupts();
    println!("{}", info);
    vga::disable_cursor();
    println!("System halted.");
    loop {
        unsafe {
            asm!("cli; hlt");
        }
    }
}
