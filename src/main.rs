#![no_std]
#![no_main]

use core::arch::asm;
use core::arch::global_asm;
use core::panic::PanicInfo;

global_asm!(include_str!("boot.S"), options(raw));

mod drivers;
mod libk;
mod ports;
mod idt;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    drivers::vga::init();
    println!("42");
    idt::init_pic();

    #[allow(clippy::empty_loop)]
    loop {
        unsafe {
            asm!("cli; hlt");
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
