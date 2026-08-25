use core::arch::asm;

pub const KERNEL_CODE_SEG: u16 = 0x08;

#[allow(dead_code)]
pub const KERNEL_DATA_SEG: u16 = 0x10;

const GDT_ENTRY_COUNT: usize = 7;

#[repr(C, packed)]
struct GdtDescriptor {
    size: u16,
    offset: u32,
}

#[derive(Clone, Copy)]
struct AccessByte(u8);

impl AccessByte {
    const fn new(
        accessed: u8,
        read_write: u8,
        direction_conforming: u8,
        executable: u8,
        descriptor_type: u8,
        privilege: u8,
        present: u8,
    ) -> Self {
        Self(
            (accessed & 0b1)
                | (read_write & 0b1) << 1
                | (direction_conforming & 0b1) << 2
                | (executable & 0b1) << 3
                | (descriptor_type & 0b1) << 4
                | (privilege & 0b11) << 5
                | (present & 0b1) << 7,
        )
    }

    const fn bits(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy)]
struct Flags(u8);

impl Flags {
    const fn new(long_mode: u8, size: u8, granularity: u8) -> Self {
        Self((long_mode & 0b1) << 1 | (size & 0b1) << 2 | (granularity & 0b1) << 3)
    }

    const fn bits(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy)]
struct SegmentDescriptor(u64);

impl SegmentDescriptor {
    const NIL: Self = Self(0);

    const fn new(base: u32, limit: u32, access: AccessByte, flags: Flags) -> Self {
        Self(
            ((limit & 0xffff) as u64)
                | (((base & 0xffff) as u64) << 16)
                | ((((base >> 16) & 0xff) as u64) << 32)
                | ((access.bits() as u64) << 40)
                | ((((limit >> 16) & 0xf) as u64) << 48)
                | ((flags.bits() as u64) << 52)
                | ((((base >> 24) & 0xff) as u64) << 56),
        )
    }

    #[allow(dead_code)]
    fn bits(self) -> u64 {
        self.0
    }
}

static GDT_ENTRIES: [SegmentDescriptor; GDT_ENTRY_COUNT] = [
    // 0x00 - Null descriptor
    SegmentDescriptor::NIL,
    // 0x08 - Kernel Code (RWX, ring 0)
    SegmentDescriptor::new(
        0,
        0xFFFFF,
        AccessByte::new(0, 1, 0, 1, 1, 0, 1),
        Flags::new(0, 1, 1),
    ),
    // 0x10 - Kernel Data (RW, ring 0)
    SegmentDescriptor::new(
        0,
        0xFFFFF,
        AccessByte::new(0, 1, 0, 0, 1, 0, 1),
        Flags::new(0, 1, 1),
    ),
    // 0x18 - Kernel Stack (RW, expand-down, ring 0)
    SegmentDescriptor::new(
        0,
        0xFFFFF,
        AccessByte::new(0, 1, 1, 0, 1, 0, 1),
        Flags::new(0, 1, 1),
    ),
    // 0x20 - User Code (RWX, ring 3)
    SegmentDescriptor::new(
        0,
        0xFFFFF,
        AccessByte::new(0, 1, 0, 1, 1, 3, 1),
        Flags::new(0, 1, 1),
    ),
    // 0x28 - User Data (RW, ring 3)
    SegmentDescriptor::new(
        0,
        0xFFFFF,
        AccessByte::new(0, 1, 0, 0, 1, 3, 1),
        Flags::new(0, 1, 1),
    ),
    // 0x30 - User Stack (RW, expand-down, ring 3)
    SegmentDescriptor::new(
        0,
        0xFFFFF,
        AccessByte::new(0, 1, 1, 0, 1, 3, 1),
        Flags::new(0, 1, 1),
    ),
];

pub fn init() {
    unsafe {
        let descriptor = GdtDescriptor {
            size: (size_of::<[SegmentDescriptor; GDT_ENTRY_COUNT]>() - 1) as u16,
            offset: (&raw const GDT_ENTRIES) as u32,
        };

        asm!("lgdt [{ptr}]", ptr = in(reg) &descriptor);

        asm!(
            "push {sel}",
            "lea {tmp}, [2f]",
            "push {tmp}",
            "retf",
            "2:",
            sel = const KERNEL_CODE_SEG as u32,
            tmp = out(reg) _,
        );

        asm!(
            "mov ds, {sel:x}",
            "mov es, {sel:x}",
            "mov fs, {sel:x}",
            "mov gs, {sel:x}",
            "mov ss, {sel:x}",
            sel = in(reg) KERNEL_DATA_SEG as u32,
        );
    }
}
