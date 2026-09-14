use core::arch::asm;

#[inline(always)]
pub fn outb(port: u16, val: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") val,
        );
    }
}

#[inline(always)]
pub fn inb(port: u16) -> u8 {
    let val: u8;
    unsafe {
        asm!(
            "in al, dx",
            in("dx") port,
            out("al") val,
        );
    }
    val
}

#[inline(always)]
pub fn outw(port: u16, val: u16) {
    unsafe {
        asm!(
            "out dx, ax",
            in("dx") port,
            in("ax") val,
        );
    }
}

#[inline(always)]
pub fn io_wait() {
    outb(0x80, 0);
}
