use crate::drivers::keyboard::{self, Key, KeyEvent};
use crate::drivers::vga::{self, MAX_TERMINALS};
use core::arch::asm;

use crate::boot::{STACK, STACK_SIZE, Stack};
use crate::ports;
use crate::{print, println};

const MAX_LEN_SHELL: usize = 64;

pub struct Shell {
    buffer: [[u8; MAX_LEN_SHELL]; MAX_TERMINALS],
    length: [usize; MAX_TERMINALS],
    current_tty: usize,
}

impl Shell {
    pub const fn new() -> Self {
        Self {
            buffer: [[0; MAX_LEN_SHELL]; MAX_TERMINALS],
            length: [0; MAX_TERMINALS],
            current_tty: 0,
        }
    }

    pub fn init(&mut self) {
        print!("> ");
    }

    pub fn handle_key(&mut self, event: KeyEvent) {
        match event.key {
            Key::Char(b'\n') => {
                self.enter();
            }

            Key::Char(0x08) => {
                self.backspace();
            }

            Key::Char(c) => {
                self.input_char(c);
            }

            Key::Function(f) => {
                self.switch_tty(f as usize);
            }

            _ => {}
        }
    }

    fn switch_tty(&mut self, tty: usize) {
        if tty >= MAX_TERMINALS {
            return;
        }

        self.current_tty = tty;
        vga::switch_terminal(tty.try_into().unwrap());
    }

    pub fn input_char(&mut self, c: u8) {
        let tty = self.current_tty;
        let length = self.length[tty];

        if length >= MAX_LEN_SHELL {
            return;
        }

        self.buffer[tty][length] = c;
        self.length[tty] += 1;

        vga::putc(c);
    }

    pub fn backspace(&mut self) {
        let tty = self.current_tty;

        if self.length[tty] == 0 {
            return;
        }

        self.length[tty] -= 1;

        let length = self.length[tty];

        self.buffer[tty][length] = 0;

        vga::backspace();
    }

    pub fn enter(&mut self) {
        let tty = self.current_tty;
        let length = self.length[tty];

        vga::putc(b'\n');

        if let Ok(command) = core::str::from_utf8(&self.buffer[tty][..length]) {
            execute_cmd(command);
        }

        self.clear();

        print!("> ");
    }

    fn clear(&mut self) {
        let tty = self.current_tty;

        self.buffer[tty] = [0; MAX_LEN_SHELL];
        self.length[tty] = 0;
    }
}

static mut SHELL: Shell = Shell::new();

pub fn init() {
    unsafe {
        (&raw mut SHELL).as_mut().unwrap().init();
    }
}

pub fn handle_keyboard() {
    while let Some(event) = keyboard::get_key() {
        unsafe {
            (&raw mut SHELL).as_mut().unwrap().handle_key(event);
        }
    }
}

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

fn command_help_vga() {
    println!("Code Page 437 characters: ");
    println!();
    for i in 0..4u8 {
        for j in 0..64u8 {
            let c = i * 64u8 + j;
            vga::put_raw(c);
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
    println!("help        - Print this help message");
    println!("help vga    - Code Page 437 characters");
}

fn execute_cmd(command: &str) {
    match command {
        "halt" => command_halt(),
        "reboot" => command_reboot(),
        "stack" => command_stack(),
        "clear" => command_clear(),
        "shutdown" => command_shutdown(),
        "help" => command_help(),
        "help vga" => command_help_vga(),
        "" => {}

        _ => println!("Unknown command: {}", command),
    }
}

const BYTES_PER_LINE: usize = 16;
const MAX_LINES: usize = 32;

fn command_clear() {
    vga::clear();
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

    let stack_start = &STACK as *const Stack as usize;
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
