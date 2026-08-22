const DATA_PORT: u16 = 0x60;
const STATUS_PORT: u16 = 0x64;
const PIC1_CMD: u16 = 0x20;
const PIC_EOI: u8 = 0x20;
const KEYBOARD_IRQ: u8 = 1;

use crate::idt;
use crate::ports;
use core::sync::atomic::{AtomicBool, AtomicU16, Ordering};

const SHIFT_LEFT: u16 = 1 << 0;
const SHIFT_RIGHT: u16 = 1 << 1;
const GUI_LEFT: u16 = 1 << 2;
const GUI_RIGHT: u16 = 1 << 3;
const CTRL_LEFT: u16 = 1 << 4;
const CTRL_RIGHT: u16 = 1 << 5;
const ALT_LEFT: u16 = 1 << 6;
const ALT_RIGHT: u16 = 1 << 7;
const CAPS_LOCK: u16 = 1 << 8;

static MODS: AtomicU16 = AtomicU16::new(0);

#[allow(dead_code)]
#[derive(Default, Clone, Copy)]
pub struct KeyModifiers {
    shift: bool,
    ctrl: bool,
    alt: bool,
    gui: bool,
    caps_lock: bool,
}

#[derive(PartialEq, Default, Clone, Copy)]
pub enum Key {
    #[default]
    None,
    Char(u8),
    Function(u8),
    Nav(NavKey),
    Media(MediaKey),
}

#[repr(u8)]
#[derive(PartialEq, Clone, Copy)]
pub enum NavKey {
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    Insert,
    Delete,
    PageUp,
    PageDown,
}

#[repr(u8)]
#[derive(PartialEq, Clone, Copy)]
pub enum MediaKey {
    PrevTrack,
    NextTrack,
    Mute,
    Calculator,
    Play,
    Stop,
    VolumeDown,
    VolumeUp,
    WwwHome,
    WwwSearch,
    WwwFavorites,
    WwwRefresh,
    WwwStop,
    WwwForward,
    WwwBack,
    MyComputer,
    Email,
    MediaSelect,
}

#[allow(dead_code)]
#[derive(Default, Clone, Copy)]
pub struct KeyEvent {
    pub key: Key,
    pub mods: KeyModifiers,
}

impl KeyEvent {
    pub const fn new() -> Self {
        Self {
            key: Key::None,
            mods: KeyModifiers {
                shift: false,
                ctrl: false,
                alt: false,
                gui: false,
                caps_lock: false,
            },
        }
    }
}

pub struct RingBuffer {
    head: u8,
    tail: u8,
    buffer: [KeyEvent; 256],
}

impl RingBuffer {
    pub fn push(&mut self, value: KeyEvent) {
        let next = self.head.wrapping_add(1);
        if next == self.tail {
            return;
        }
        self.buffer[self.head as usize] = value;
        self.head = next;
    }

    pub fn pop(&mut self) -> Option<KeyEvent> {
        if self.head == self.tail {
            return None;
        }
        let char = self.buffer[self.tail as usize];
        self.tail = self.tail.wrapping_add(1);
        Some(char)
    }
}

static mut RB: RingBuffer = RingBuffer {
    buffer: [KeyEvent::new(); 256],
    head: 0,
    tail: 0,
};

pub fn get_key() -> Option<KeyEvent> {
    unsafe { (&raw mut RB).as_mut().unwrap().pop() }
}

pub fn init() {
    idt::clear_mask(KEYBOARD_IRQ);
}

pub fn poll() {
    if ports::inb(STATUS_PORT) & 1 != 0 {
        keyboard_handler();
    }
}

const SCANCODE_NORMAL: [u8; 128] = const {
    let mut table = [0u8; 128];

    table[0x02] = b'1';
    table[0x03] = b'2';
    table[0x04] = b'3';
    table[0x05] = b'4';
    table[0x06] = b'5';
    table[0x07] = b'6';
    table[0x08] = b'7';
    table[0x09] = b'8';
    table[0x0A] = b'9';
    table[0x0B] = b'0';
    table[0x0C] = b'-';
    table[0x0D] = b'=';
    table[0x0E] = 0x08;
    table[0x0F] = b'\t';

    table[0x10] = b'q';
    table[0x11] = b'w';
    table[0x12] = b'e';
    table[0x13] = b'r';
    table[0x14] = b't';
    table[0x15] = b'y';
    table[0x16] = b'u';
    table[0x17] = b'i';
    table[0x18] = b'o';
    table[0x19] = b'p';
    table[0x1A] = b'[';
    table[0x1B] = b']';
    table[0x1C] = b'\n';

    table[0x1E] = b'a';
    table[0x1F] = b's';
    table[0x20] = b'd';
    table[0x21] = b'f';
    table[0x22] = b'g';
    table[0x23] = b'h';
    table[0x24] = b'j';
    table[0x25] = b'k';
    table[0x26] = b'l';
    table[0x27] = b';';
    table[0x28] = b'\'';
    table[0x29] = b'`';
    table[0x2B] = b'\\';

    table[0x2C] = b'z';
    table[0x2D] = b'x';
    table[0x2E] = b'c';
    table[0x2F] = b'v';
    table[0x30] = b'b';
    table[0x31] = b'n';
    table[0x32] = b'm';
    table[0x33] = b',';
    table[0x34] = b'.';
    table[0x35] = b'/';
    table[0x39] = b' ';

    table
};

const SCANCODE_SHIFTED: [u8; 128] = const {
    let mut table = [0u8; 128];

    table[0x02] = b'!';
    table[0x03] = b'@';
    table[0x04] = b'#';
    table[0x05] = b'$';
    table[0x06] = b'%';
    table[0x07] = b'^';
    table[0x08] = b'&';
    table[0x09] = b'*';
    table[0x0A] = b'(';
    table[0x0B] = b')';
    table[0x0C] = b'_';
    table[0x0D] = b'+';
    table[0x0E] = 0x08;
    table[0x0F] = b'\t';

    table[0x10] = b'Q';
    table[0x11] = b'W';
    table[0x12] = b'E';
    table[0x13] = b'R';
    table[0x14] = b'T';
    table[0x15] = b'Y';
    table[0x16] = b'U';
    table[0x17] = b'I';
    table[0x18] = b'O';
    table[0x19] = b'P';
    table[0x1A] = b'{';
    table[0x1B] = b'}';
    table[0x1C] = b'\n';

    table[0x1E] = b'A';
    table[0x1F] = b'S';
    table[0x20] = b'D';
    table[0x21] = b'F';
    table[0x22] = b'G';
    table[0x23] = b'H';
    table[0x24] = b'J';
    table[0x25] = b'K';
    table[0x26] = b'L';
    table[0x27] = b':';
    table[0x28] = b'"';
    table[0x29] = b'~';
    table[0x2B] = b'|';

    table[0x2C] = b'Z';
    table[0x2D] = b'X';
    table[0x2E] = b'C';
    table[0x2F] = b'V';
    table[0x30] = b'B';
    table[0x31] = b'N';
    table[0x32] = b'M';
    table[0x33] = b'<';
    table[0x34] = b'>';
    table[0x35] = b'?';
    table[0x39] = b' ';

    table
};

static EXT: AtomicBool = AtomicBool::new(false);

fn press_modifier(mask: u16) {
    MODS.fetch_or(mask, Ordering::Relaxed);
}

fn release_modifier(mask: u16) {
    MODS.fetch_and(!mask, Ordering::Relaxed);
}

fn toggle_modifier(mask: u16) {
    MODS.fetch_xor(mask, Ordering::Relaxed);
}

fn snapshot_mods() -> KeyModifiers {
    let mods = MODS.load(Ordering::Relaxed);
    KeyModifiers {
        shift: (mods & SHIFT_LEFT == SHIFT_LEFT) || (mods & SHIFT_RIGHT == SHIFT_RIGHT),
        ctrl: (mods & CTRL_LEFT == CTRL_LEFT) || (mods & CTRL_RIGHT == CTRL_RIGHT),
        alt: (mods & ALT_LEFT == ALT_LEFT) || (mods & ALT_RIGHT == ALT_RIGHT),
        gui: (mods & GUI_LEFT == GUI_LEFT) || (mods & GUI_RIGHT == GUI_RIGHT),
        caps_lock: mods & CAPS_LOCK == CAPS_LOCK,
    }
}

fn keyboard_handler() {
    let scancode = ports::inb(DATA_PORT);

    // no data
    if scancode == 0 {
        return;
    }

    if scancode == 0xE0 {
        EXT.store(true, Ordering::Relaxed);
        eoi();
        return;
    }

    if EXT.load(Ordering::Relaxed) {
        EXT.store(false, Ordering::Relaxed);

        if scancode & 0x80 != 0 {
            let released = scancode & 0x7F;
            match released {
                0x1D => release_modifier(CTRL_RIGHT),
                0x38 => release_modifier(ALT_RIGHT),
                0x5B => release_modifier(GUI_LEFT),
                0x5C => release_modifier(GUI_RIGHT),
                _ => {}
            }
            eoi();
            return;
        }

        match scancode {
            0x1D => {
                press_modifier(CTRL_RIGHT);
                eoi();
                return;
            }
            0x38 => {
                press_modifier(ALT_RIGHT);
                eoi();
                return;
            }
            0x5B => {
                press_modifier(GUI_LEFT);
                eoi();
                return;
            }
            0x5C => {
                press_modifier(GUI_RIGHT);
                eoi();
                return;
            }
            _ => {}
        }
        let val: Key = match scancode {
            0x47 => Key::Nav(NavKey::Home),
            0x48 => Key::Nav(NavKey::ArrowUp),
            0x4B => Key::Nav(NavKey::ArrowLeft),
            0x4D => Key::Nav(NavKey::ArrowRight),
            0x4F => Key::Nav(NavKey::End),
            0x50 => Key::Nav(NavKey::ArrowDown),
            0x51 => Key::Nav(NavKey::PageDown),
            0x52 => Key::Nav(NavKey::Insert),
            0x53 => Key::Nav(NavKey::Delete),
            0x49 => Key::Nav(NavKey::PageUp),
            0x10 => Key::Media(MediaKey::PrevTrack),
            0x19 => Key::Media(MediaKey::NextTrack),
            0x20 => Key::Media(MediaKey::Mute),
            0x21 => Key::Media(MediaKey::Calculator),
            0x22 => Key::Media(MediaKey::Play),
            0x24 => Key::Media(MediaKey::Stop),
            0x2E => Key::Media(MediaKey::VolumeDown),
            0x30 => Key::Media(MediaKey::VolumeUp),
            0x32 => Key::Media(MediaKey::WwwHome),
            0x65 => Key::Media(MediaKey::WwwSearch),
            0x66 => Key::Media(MediaKey::WwwFavorites),
            0x67 => Key::Media(MediaKey::WwwRefresh),
            0x68 => Key::Media(MediaKey::WwwStop),
            0x69 => Key::Media(MediaKey::WwwForward),
            0x6A => Key::Media(MediaKey::WwwBack),
            0x6B => Key::Media(MediaKey::MyComputer),
            0x6C => Key::Media(MediaKey::Email),
            0x6D => Key::Media(MediaKey::MediaSelect),
            _ => Key::None,
        };
        if val != Key::None {
            unsafe {
                (&raw mut RB).as_mut().unwrap().push(KeyEvent {
                    key: val,
                    mods: snapshot_mods(),
                });
            }
        }
        eoi();
        return;
    }

    if scancode & 0x80 != 0 {
        let released = scancode & 0x7F;
        match released {
            0x2A => release_modifier(SHIFT_LEFT),
            0x36 => release_modifier(SHIFT_RIGHT),
            0x1D => release_modifier(CTRL_LEFT),
            0x38 => release_modifier(ALT_LEFT),
            _ => {}
        }
        eoi();
        return;
    }

    match scancode {
        0x2A => {
            press_modifier(SHIFT_LEFT);
            eoi();
            return;
        }
        0x36 => {
            press_modifier(SHIFT_RIGHT);
            eoi();
            return;
        }
        0x1D => {
            press_modifier(CTRL_LEFT);
            eoi();
            return;
        }
        0x38 => {
            press_modifier(ALT_LEFT);
            eoi();
            return;
        }
        0x3A => {
            toggle_modifier(CAPS_LOCK);
            eoi();
            return;
        }
        0x3B..=0x44 => {
            unsafe {
                (&raw mut RB).as_mut().unwrap().push(KeyEvent {
                    key: Key::Function(scancode - 0x3B),
                    mods: snapshot_mods(),
                });
            }
            eoi();
            return;
        }
        0x57 | 0x58 => {
            unsafe {
                (&raw mut RB).as_mut().unwrap().push(KeyEvent {
                    key: Key::Function(scancode - 0x57 + 10),
                    mods: snapshot_mods(),
                });
            }
            eoi();
            return;
        }
        _ => {}
    }

    let mods = MODS.load(Ordering::Relaxed);
    let shift_active = (mods & SHIFT_LEFT == SHIFT_LEFT) || (mods & SHIFT_RIGHT == SHIFT_RIGHT);
    let is_letter = (0x10..=0x19).contains(&scancode)
        || (0x1E..=0x26).contains(&scancode)
        || (0x2C..=0x32).contains(&scancode);
    let effective_shift = if is_letter {
        shift_active != (mods & CAPS_LOCK == CAPS_LOCK)
    } else {
        shift_active
    };
    let c = if effective_shift {
        SCANCODE_SHIFTED[scancode as usize]
    } else {
        SCANCODE_NORMAL[scancode as usize]
    };
    if c != 0 {
        unsafe {
            (&raw mut RB).as_mut().unwrap().push(KeyEvent {
                key: Key::Char(c),
                mods: snapshot_mods(),
            });
        }
    }
}

// end of interrupt
fn eoi() {
    ports::outb(PIC1_CMD, PIC_EOI);
}
