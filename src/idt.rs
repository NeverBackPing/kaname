use crate::gdt::KERNEL_CODE_SEG;
use core::arch::{asm, naked_asm};
use core::mem::size_of;

#[repr(u8)]
pub enum InterruptVector {
    DivideError,
    Debug,
    Nmi,
    Breakpoint,
    Overflow,
    BoundRangeExceeded,
    InvalidOpcode,
    DeviceNotAvailable,
    DoubleFault,
    CoprocessorSegmentOverrun, // obsolete
    InvalidTss,
    SegmentNotPresent,
    StackSegmentFault,
    GeneralProtection,
    PageFault,
    // 15 reserved
    X87FloatingPoint = 16,
    AlignmentCheck,
    MachineCheck,
    SimdFloatingPoint,
    VirtualizationException,
    ControlProtectionException,

    HypervisorInjection = 28,
    VmmCommunication,
    SecurityException,
}

const IDT_SIZE: usize = 256;

pub struct PageFaultError(pub u32);

impl PageFaultError {
    pub fn violation(&self) -> bool {
        self.0 & 1 << 0 != 0
    }

    pub fn write(&self) -> bool {
        self.0 & 1 << 1 != 0
    }

    pub fn user(&self) -> bool {
        self.0 & 1 << 2 != 0
    }

    pub fn reserved_bit(&self) -> bool {
        self.0 & 1 << 3 != 0
    }

    pub fn instr_fetch(&self) -> bool {
        self.0 & 1 << 4 != 0
    }
}

#[repr(u8)]
#[derive(Clone, Copy)]
enum GateType {
    TaskGate = 0x5,
    Interrupt16 = 0x6,
    Trap16 = 0x7,
    Interrupt32 = 0xE,
    Trap32 = 0xF,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    reserved0: u8,
    type_attr: u8, // bits 0-3: gate_type, bit 4: 0, bits 5-6: ring, bit 7: present
    offset_high: u16,
}

impl IdtEntry {
    const NULL: IdtEntry = IdtEntry {
        offset_low: 0,
        selector: 0,
        reserved0: 0,
        type_attr: 0,
        offset_high: 0,
    };

    fn new(offset: u32, selector: u16, ring: u8, gate_type: GateType) -> Self {
        let type_attr = (gate_type as u8) | ((ring & 0b11) << 5) | (1 << 7);
        IdtEntry {
            offset_low: offset as u16,
            selector,
            reserved0: 0,
            type_attr,
            offset_high: (offset >> 16) as u16,
        }
    }
}

#[repr(C, packed)]
struct IdtDescriptor {
    size: u16,
    offset: u32,
}

#[repr(C)]
pub struct InterruptFrame {
    edi: u32,
    esi: u32,
    ebp: u32,
    esp: u32,
    ebx: u32,
    edx: u32,
    ecx: u32,
    eax: u32,
    vector: u32,
    error_code: u32,
    eip: u32,
    cs: u32,
    eflags: u32,
}

impl InterruptFrame {
    pub fn edi(&self) -> u32 {
        self.edi
    }

    pub fn esi(&self) -> u32 {
        self.esi
    }

    pub fn ebp(&self) -> u32 {
        self.ebp
    }

    pub fn esp(&self) -> u32 {
        self.esp
    }

    pub fn ebx(&self) -> u32 {
        self.ebx
    }

    pub fn edx(&self) -> u32 {
        self.edx
    }

    pub fn ecx(&self) -> u32 {
        self.ecx
    }

    pub fn eax(&self) -> u32 {
        self.eax
    }

    pub fn vector(&self) -> u32 {
        self.vector
    }

    pub fn error_code(&self) -> u32 {
        self.error_code
    }

    pub fn eip(&self) -> u32 {
        self.eip
    }

    pub fn cs(&self) -> u32 {
        self.cs
    }

    pub fn eflags(&self) -> u32 {
        self.eflags
    }
}

pub type InterruptHandler = fn(&InterruptFrame);

fn has_error_code(vector: u8) -> bool {
    use InterruptVector::*;
    const DOUBLE_FAULT: u8 = DoubleFault as u8;
    const INVALID_TSS: u8 = InvalidTss as u8;
    const SEGMENT_NOT_PRESENT: u8 = SegmentNotPresent as u8;
    const STACK_SEGMENT_FAULT: u8 = StackSegmentFault as u8;
    const GENERAL_PROTECTION: u8 = GeneralProtection as u8;
    const PAGE_FAULT: u8 = PageFault as u8;
    const ALIGNMENT_CHECK: u8 = AlignmentCheck as u8;
    const CONTROL_PROTECTION_EXCEPTION: u8 = ControlProtectionException as u8;

    matches!(
        vector,
        DOUBLE_FAULT
            | INVALID_TSS
            | SEGMENT_NOT_PRESENT
            | STACK_SEGMENT_FAULT
            | GENERAL_PROTECTION
            | PAGE_FAULT
            | ALIGNMENT_CHECK
            | CONTROL_PROTECTION_EXCEPTION
    )
}

static mut HANDLERS: [Option<InterruptHandler>; IDT_SIZE] = [None; IDT_SIZE];
static mut IDT_ENTRIES: [IdtEntry; IDT_SIZE] = [IdtEntry::NULL; IDT_SIZE];

#[unsafe(naked)]
extern "C" fn stub_with_err<const V: u8>() {
    naked_asm!(
        "push {vector}",
        "jmp isr_common_stub",
        vector = const V
    );
}

#[unsafe(naked)]
extern "C" fn stub_no_err<const V: u8>() {
    naked_asm!(
        "push 0",
        "push {vector}",
        "jmp isr_common_stub",
        vector = const V
    );
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
extern "C" fn isr_common_stub() {
    naked_asm!(
        // Save all general-purpose registers
        "pusha",
        // Pass stack pointer (pointing to InterruptFrame) as argument
        "mov eax, esp",
        "push eax",
        "call interrupt_dispatch",
        "add esp, 4",
        // Restore registers, clean up vector and error code, return from interrupt
        "popa",
        "add esp, 8",
        "iretd",
    );
}

#[unsafe(no_mangle)]
extern "C" fn interrupt_dispatch(frame: *mut InterruptFrame) {
    unsafe {
        let frame = &*frame;
        let vector = frame.vector() as usize;

        if vector < IDT_SIZE
            && let Some(handler) = HANDLERS[vector]
        {
            handler(frame);
        }
    }
}

pub fn register_handler(vector: u8, handler: InterruptHandler) {
    unsafe {
        HANDLERS[vector as usize] = Some(handler);
    }
}

use seq_macro::seq;

pub fn init() {
    unsafe {
        seq!(N in 0..256 {
            let addr = if has_error_code(N) {
                stub_with_err::<N> as *const() as u32
            } else {
                stub_no_err::<N> as *const() as u32
            };
            IDT_ENTRIES[N] = IdtEntry::new(addr, KERNEL_CODE_SEG, 0, GateType::Interrupt32);
        });

        let descriptor = IdtDescriptor {
            size: (size_of::<[IdtEntry; IDT_SIZE]>() - 1) as u16,
            offset: (&raw const IDT_ENTRIES) as u32,
        };

        asm!("lidt [{ptr}]", ptr = in(reg) &descriptor);
    }
}

pub fn enable_interrupts() {
    unsafe {
        asm!("sti");
    }
}

pub fn disable_interrupts() {
    unsafe {
        asm!("cli");
    }
}
