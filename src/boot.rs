use core::arch::naked_asm;

const MAGIC: u32 = 0x1BADB002; // Multiboot magic
const ALIGN: u32 = 1 << 0; // Align loaded modules on page boundaries
const MEMINFO: u32 = 1 << 1; // Request memory information from GRUB
const FLAGS: u32 = ALIGN | MEMINFO;
const CHECKSUM: u32 = (0u32).wrapping_sub(MAGIC.wrapping_add(FLAGS));

#[repr(C)]
struct MultibootHeader {
    magic: u32,
    flags: u32,
    checksum: u32,
}

#[unsafe(link_section = ".multiboot")]
#[used]
static MULTIBOOT_HEADER: MultibootHeader = MultibootHeader {
    magic: MAGIC,
    flags: FLAGS,
    checksum: CHECKSUM,
};

const STACK_SIZE: usize = 16 * 1024;

#[repr(align(16))]
struct Stack(#[allow(dead_code)] [u8; STACK_SIZE]);

#[unsafe(link_section = ".bss")]
static STACK: Stack = Stack([0; STACK_SIZE]);

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub extern "C" fn _start() -> ! {
    naked_asm!(
        "lea esp, [{stack} + {stack_size}]",
        "call {entry}",
        "2:",
        "hlt",
        "jmp 2b",
        stack = sym STACK,
        stack_size = const STACK_SIZE,
        entry = sym crate::kernel_main,
    )
}
