#![no_std]
#![no_main]

use core::arch::asm;
use core::arch::global_asm;
use core::panic::PanicInfo;

global_asm!(include_str!("boot.S"), options(raw));

mod drivers;
mod idt;
mod libk;
mod ports;

use drivers::keyboard::{self, Key};

use crate::drivers::vga::switch_terminal;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    drivers::vga::init();
    idt::init_pic();
    keyboard::init();

    println!("42");

    loop {
        keyboard::poll();
        if let Some(event) = keyboard::get_key(){

            if let Key::Function(fn_key) = event.key{
                drivers::vga::switch_terminal(fn_key);
            }
            if let Key::Char(c) = event.key{
                print!("{}", c as char);
            }
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
