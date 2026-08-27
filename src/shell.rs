use core::arch::asm;
use crate::drivers::vga;
use crate::drivers::keyboard::{self, Key, KeyEvent};

use crate::ports::{inb, outb};
use crate::println;

const MAX_COMMAND: usize = 64;

pub struct Shell {
    buffer: [u8; MAX_COMMAND],
    length: usize,
}

impl Shell {
    pub const fn new() -> Self {
        Self {
            buffer: [0; MAX_COMMAND],
            length: 0,
        }
    }

    pub fn init(&mut self) {
        vga::putc(b'>');
        vga::putc(b' ');
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

            Key::Function(0) => {
                vga::switch_terminal(0);
            }

            Key::Function(1) => {
                vga::switch_terminal(1);
            }

            Key::Function(2) => {
                vga::switch_terminal(2);
            }

            Key::Function(3) => {
                vga::switch_terminal(3);
            }

            Key::Function(4) => {
                vga::switch_terminal(4);
            }

            Key::Function(5) => {
                vga::switch_terminal(5);
            }

            _ => {}
        }
    }

    pub fn input_char(&mut self, c: u8) {
        if self.length >= MAX_COMMAND {
            return;
        }

        self.buffer[self.length] = c;
        self.length += 1;

        vga::putc(c);
    }

    pub fn backspace(&mut self) {
        if self.length == 0 {
            return;
        }

        self.length -= 1;
        self.buffer[self.length] = 0;

        vga::backspace();
    }

    pub fn enter(&mut self) {
        vga::putc(b'\n');

        if let Ok(command) = core::str::from_utf8(&self.buffer[..self.length]) {
            execute_cmd(command);
        }

        self.clear();

        vga::putc(b'>');
        vga::putc(b' ');
    }

    fn clear(&mut self) {
        self.buffer = [0; MAX_COMMAND];
        self.length = 0;
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
            (&raw mut SHELL)
                .as_mut()
                .unwrap()
                .handle_key(event);
        }
    }
}

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


pub fn execute_cmd(command: &str) {
    match command {
        "halt" => command_halt(),
        "reboot" => command_reboot(),
        "stack" => {}
        "" => {}

        _ => println!("Unknown command: {}", command),
    }
}