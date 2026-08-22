use crate::ports::{inb, outb};

//IO PIC 8259
const MASTER_CMD: u16 = 0x20;
const SLAVE_CMD: u16 = 0xA0;
const MASTER_DATA: u16 = 0x21;
const SLAVE_DATA: u16 = 0xA1;

const MASTER_PIC: u8 = 0x20; // 32
const SLAVE_PIC: u8 = 0x28; // 40

// Wait for give time
#[inline(always)]
fn io_wait() {
    outb(0x80, 0);
}

pub fn init_pic() {
    // Init
    outb(MASTER_CMD, 0x11);
    io_wait();
    outb(SLAVE_CMD, 0x11);
    io_wait();

    // Mapping
    outb(MASTER_DATA, MASTER_PIC);
    io_wait();
    outb(SLAVE_DATA, SLAVE_PIC);
    io_wait();

    // Link Master/Slave

    // Mastering IRQ2
    outb(MASTER_DATA, 0x04);
    io_wait();
    // connect IRQ2
    outb(SLAVE_DATA, 0x02);
    io_wait();

    // def the mode
    outb(MASTER_DATA, 0x01);
    io_wait();
    outb(SLAVE_DATA, 0x01);
    io_wait();
}

#[allow(dead_code)]
pub fn set_mask(mut line: u8) {
    let port: u16 = if line < 8 { MASTER_DATA } else { SLAVE_DATA };
    if line >= 8 {
        line -= 8;
    }
    let value = inb(port) | (1 << line);

    outb(port, value);
}

#[allow(dead_code)]
pub fn clear_mask(mut line: u8) {
    let port: u16 = if line < 8 { MASTER_DATA } else { SLAVE_DATA };
    if line >= 8 {
        line -= 8;
    }

    let value = inb(port) | (1 << line);
    outb(port, value);
}
