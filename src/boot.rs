use core::arch::naked_asm;

const MAGIC: u32 = 0xE85250D6; // Multiboot2 magic
const ARCH_I386: u32 = 0;

const FLAGS_NONE: u16 = 0;
const FLAG_OPTIONAL: u16 = 1;

const TYPE_END_TAG: u16 = 0;
const TYPE_INFO_REQ: u16 = 1;

#[repr(C, packed)]
struct InformationRequestTag {
    type_: u16,
    flags: u16,
    size: u32,
}

#[repr(C, packed)]
struct EndTag {
    type_: u16,
    flags: u16,
    size: u32,
}

#[repr(C, packed)]
struct Multiboot2Header {
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,
    info_req: InformationRequestTag,
    end: EndTag,
}

#[unsafe(link_section = ".multiboot")]
#[used]
static MULTIBOOT_HEADER: Multiboot2Header = Multiboot2Header {
    magic: MAGIC,
    architecture: ARCH_I386,
    header_length: size_of::<Multiboot2Header>() as u32,
    checksum: 0u32.wrapping_sub(
        MAGIC.wrapping_add(ARCH_I386.wrapping_add(size_of::<Multiboot2Header>() as u32)),
    ),
    info_req: InformationRequestTag {
        type_: TYPE_INFO_REQ,
        flags: FLAGS_NONE,
        size: size_of::<InformationRequestTag>() as u32,
    },
    end: EndTag {
        type_: TYPE_END_TAG,
        flags: FLAGS_NONE,
        size: size_of::<EndTag>() as u32,
    },
};

pub const STACK_SIZE: usize = 16 * 1024;

#[repr(align(16))]
pub struct Stack([u8; STACK_SIZE]);

#[unsafe(link_section = ".bss")]
pub static STACK: Stack = Stack([0; STACK_SIZE]);

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub extern "C" fn _start() -> ! {
    naked_asm!(
        "lea esp, [{stack} + {stack_size}]",
        "xor ebp, ebp",
        // reset EFLAGS
        "push 0",
        "popf",
        // Push multiboot information structure pointer
        "push ebx",
        // Push magic value
        "push eax",
        "call {entry}",
        "2:",
        "hlt",
        "jmp 2b",
        stack = sym STACK,
        stack_size = const STACK_SIZE,
        entry = sym crate::kernel_main,
    )
}
