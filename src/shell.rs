use core::arch::asm;

use crate::ports::{inb, outb};
use crate::println;

#[allow(dead_code)]
pub fn command_halt() {
    println!("System halted.");
    unsafe {
        loop {
            asm!("cli; hlt");
        }
    }
}

#[allow(dead_code)]
pub fn command_reboot() {
    let mut good: u8 = 0x02;
    while (good & 0x02) == 0x02 {
        good = inb(0x64);
    }
    outb(0x64, 0xFE);
    unsafe {
        loop {
            asm!("cli; hlt");
        }
    }
}
