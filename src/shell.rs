use crate::drivers::vga;
use crate::terminal::Color;
use core::arch::asm;

use crate::boot::{STACK, STACK_SIZE};
use crate::{compositor, ports};
use crate::{print, println};

fn command_halt() {
    println!("System halted.");

    vga::disable_cursor();

    unsafe {
        loop {
            asm!("cli; hlt");
        }
    }
}

fn command_reboot() {
    let mut good: u8 = 0x02;

    while (good & 0x02) == 0x02 {
        good = ports::inb(0x64);
    }

    ports::outb(0x64, 0xFE);

    unsafe {
        loop {
            asm!("cli; hlt");
        }
    }
}

// Since an ACPI parser is not in the scope of this project, we just call the emulator-specific
// shutdown methods
fn command_shutdown() {
    // QEMU 2.0+
    ports::outw(0x604, 0x2000);
    // Bochs and older QEMU
    ports::outw(0xB004, 0x2000);
    // Virtualbox
    ports::outw(0x4004, 0x3400);
    // Cloud Hypervisor
    ports::outw(0x600, 0x34);
    // Did not work, halt
    panic!("Shutdown failed");
}

fn command_panic() {
    panic!("Manual kernel panic");
}

fn command_help_memory() {
    if !compositor::get_terminfo().is_full_width() {
        println!("Cannot display: please open a full-width terminal");
        return;
    }

    println!("                          [Figure: Page Translation]                          ");
    println!("                                                                              ");
    println!("             ╓31           24╥23           16╥15            8╥7             0╖");
    println!("   linear  → ╟─┬─┬─┬─┬─┬─┬─┬─╫─┬─┬─┬─┬─┬─┬─┬─╫─┬─┬─┬─┬─┬─┬─┬─╫─┬─┬─┬─┬─┬─┬─┬─╢");
    println!("   address   ╟─┴─┴─┴─┴─┴─┴─┴─╨─┴─┼─┴─┴─┴─┴─┴─╨─┴─┴─┴─┼─┴─┴─┴─╨─┴─┴─┴─┴─┴─┴─┴─╢");
    println!("             │  Directory index  │    Table index    │        Offset         │");
    println!("             └┬──────────────────┴┬──────────────────┴───────┬───────────────┘");
    println!("              │                   │                          │                ");
    println!(" CR3 ──────┬───→╔Page═Dir═════╗ ┌──→╔Page═Table═══╗          │ ╔RAM══════════╗");
    println!(" physical  │  │ ║1024 entries ║ │ │ ║1024 entries ║          │ ║     ...     ║");
    println!("           │  │ ╠═════════════╣ │ │ ╠═════════════╣          │ ║     ...     ║");
    println!("           │  │ ║     ...     ║ │ │ ║     ...     ║          │ ╟─────────────╢");
    println!("           │  │ ║     ...     ║ │ │ ║     ...     ║ ┌─────────→║Page Frame   ║");
    println!("           │  │ ╟─────────────╢ │ │ ║     ...     ║ │        └→║(4096 bytes) ║");
    println!("           │  └→║ entry (PDE) ╟─┘ │ ║     ...     ║ │          ╟─────────────╢");
    println!("           │    ╟─────────────╢   │ ╟─────────────╢ │   ┌─────→║Page Frame   ║");
    println!("           │    ║     ...     ║   │ ║ other entry ╟─│───┘      ╟─────────────╢");
    println!("         linear ║     ...     ║   │ ╟─────────────╢ │          ║     ...     ║");
    println!("           │    ║     ...     ║   └→║ entry (PTE) ╟─┘          ║     ...     ║");
    println!("           │    ╟─────────────╢     ╟─────────────╢            ║     ...     ║");
    println!("           │    ║  PDE[1023]  ║     ║     ...     ║            ║     ...     ║");
    println!("           │    ║  recursive  ║     ║     ...     ║            ║     ...     ║");
    println!("           └────╢   mapping   ║     ║     ...     ║            ║     ...     ║");
    println!("                ╚═════════════╝     ╚═════════════╝            ╚═════════════╝");
}

const PALETTE: [Color; 16] = [
    Color::Black,
    Color::Blue,
    Color::Green,
    Color::Cyan,
    Color::Red,
    Color::Magenta,
    Color::Brown,
    Color::LightGrey,
    Color::DarkGrey,
    Color::LightBlue,
    Color::LightGreen,
    Color::LightCyan,
    Color::LightRed,
    Color::LightMagenta,
    Color::Yellow,
    Color::White,
];

const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";

fn command_help_vga() {
    if !compositor::get_terminfo().is_full_width() {
        println!("Cannot display: please open a full-width terminal");
        return;
    }

    for (bg_idx, &bg) in PALETTE.iter().enumerate() {
        for (fg_idx, &fg) in PALETTE.iter().enumerate() {
            compositor::put_raw(b' ', fg, bg);
            compositor::put_raw(HEX_DIGITS[fg_idx], fg, bg);
            compositor::put_raw(HEX_DIGITS[bg_idx], fg, bg);
            compositor::put_raw(b' ', fg, bg);
            compositor::put_raw(b' ', fg, bg);
        }
        println!();
    }
    println!("Code Page 437 character table");
    for i in 0..4u8 {
        for j in 0..64u8 {
            let c = i * 64u8 + j;
            compositor::put_raw(c, Color::Black, Color::White);
        }
        println!();
    }
    println!();
}

fn command_help() {
    println!("halt        - Halt the machine");
    println!("reboot      - Reboot the machine");
    println!("stack       - Print kernel stack");
    println!("clear       - Clear screen");
    println!("shutdown    - Shutdown system");
    println!("panic       - Trigger a manual kernel panic");
    println!("help        - Print this help message");
    println!("help memory - Linear pointer breakdown");
    println!("help vga    - Code Page 437 characters and color matrix");
}

pub fn execute_cmd(command: &str) {
    match command {
        "halt" => command_halt(),
        "reboot" => command_reboot(),
        "stack" => command_stack(),
        "clear" => command_clear(),
        "shutdown" => command_shutdown(),
        "panic" => command_panic(),
        "help" => command_help(),
        "help memory" => command_help_memory(),
        "help vga" => command_help_vga(),
        "terminfo" => command_terminfo(),
        "" => {}
        _ => {
            // terminal::set_color(Color::Red, Color::White);
            println!("Unknown command: {}", command);
            // terminal::set_color(Color::Black, Color::White);
        }
    }
}

const BYTES_PER_LINE: usize = 16;
const MAX_LINES: usize = 32;

fn command_terminfo() {
    println!("{}", compositor::get_terminfo());
}

fn command_clear() {
    compositor::clear();
}

fn command_stack() {
    let esp: usize;

    unsafe {
        core::arch::asm!(
            "mov {}, esp",
            out(reg) esp,
            options(nomem, nostack, preserves_flags)
        );
    }

    let stack_start = &raw const STACK as usize;
    let stack_end = stack_start + STACK_SIZE;

    let start = esp & !(BYTES_PER_LINE - 1);

    println!(
        "ESP={:#010x}  stack=[{:#010x}, {:#010x})",
        esp, stack_start, stack_end
    );

    let mut addr = start;

    for _ in 0..MAX_LINES {
        if addr >= stack_end {
            break;
        }

        if addr < stack_start {
            addr += BYTES_PER_LINE;
            continue;
        }

        print!("{:08x}  ", addr);

        for i in 0..BYTES_PER_LINE {
            if i == 8 {
                print!(" ");
            }

            let current = addr + i;

            if current < stack_end {
                let byte = unsafe { *(current as *const u8) };
                print!("{:02x} ", byte);
            } else {
                print!("   ");
            }
        }

        print!(" ");

        for i in 0..BYTES_PER_LINE {
            let current = addr + i;

            if current < stack_end {
                let byte = unsafe { *(current as *const u8) };

                if byte.is_ascii_graphic() || byte == b' ' {
                    print!("{}", byte as char);
                } else {
                    print!(".");
                }
            } else {
                print!(" ");
            }
        }

        println!();

        addr += BYTES_PER_LINE;
    }
}
