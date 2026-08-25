#![no_std]
#![no_main]

use core::arch::asm;
use core::arch::global_asm;
use core::panic::PanicInfo;

global_asm!(include_str!("boot.S"), options(raw));

mod drivers;
mod idt;
mod libk;
mod pic;
mod ports;

use drivers::keyboard::{self, Key};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    drivers::vga::init();
    idt::init();
    pic::init();
    keyboard::init();

    println!("42");

    idt::enable_interrupts();
    loop {
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
