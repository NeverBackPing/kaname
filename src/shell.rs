use crate::drivers::keyboard::{self, Key, KeyEvent};
use crate::drivers::vga::{self, *};
use core::arch::asm;

use crate::ports::{inb, outb};
use crate::println;

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

        vga::putc(b'>');
        vga::putc(b' ');
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

#[allow(dead_code)]
pub fn command_halt() {
    println!("System halted.");

    vga::disable_cursor();

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
