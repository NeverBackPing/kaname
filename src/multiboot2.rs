pub const BOOTLOADER_MAGIC: usize = 0x36D76289;

pub const TAG_TYPE_END: u32 = 0;
pub const TAG_TYPE_CMDLINE: u32 = 1;
pub const TAG_TYPE_BOOT_LOADER_NAME: u32 = 2;
pub const TAG_TYPE_MODULE: u32 = 3;
pub const TAG_TYPE_BASIC_MEMINFO: u32 = 4;
pub const TAG_TYPE_BOOTDEV: u32 = 5;
pub const TAG_TYPE_MMAP: u32 = 6;
pub const TAG_TYPE_VBE: u32 = 7;
pub const TAG_TYPE_FRAMEBUFFER: u32 = 8;
pub const TAG_TYPE_ELF_SECTIONS: u32 = 9;
pub const TAG_TYPE_APM: u32 = 10;
pub const TAG_TYPE_EFI32_SYSTABLE: u32 = 11;
pub const TAG_TYPE_EFI64_SYSTABLE: u32 = 12;
pub const TAG_TYPE_SMBIOS: u32 = 13;
pub const TAG_TYPE_ACPI_RSDP_V1: u32 = 14;
pub const TAG_TYPE_ACPI_RSDP_V2: u32 = 15;
pub const TAG_TYPE_NETWORK: u32 = 16;
pub const TAG_TYPE_EFI_MMAP: u32 = 17;
pub const TAG_TYPE_EFI_BOOT_NOT_TERMINATED: u32 = 18;
pub const TAG_TYPE_EFI32_IMAGE_HANDLE: u32 = 19;
pub const TAG_TYPE_EFI64_IMAGE_HANDLE: u32 = 20;

#[repr(C)]
pub struct Info {
    total_size: u32,
    reserved: u32,
}

impl Info {
    pub fn tags(&self) -> TagIter<'_> {
        // Tags start after Info in memory
        let start = self as *const Info as usize + size_of::<Info>();
        let end = self as *const Info as usize + self.total_size as usize;

        TagIter {
            addr: start,
            end,
            _marker: core::marker::PhantomData,
        }
    }
}

pub enum Tag<'a> {
    End,
    Mmap(&'a MmapTag),
    Unknown,
}

impl<'a> Tag<'_> {
    unsafe fn from_raw(raw: *const RawTag) -> Tag<'a> {
        unsafe {
            match (*raw).type_ {
                TAG_TYPE_END => Tag::End,
                TAG_TYPE_MMAP => {
                    let mmap = (raw as *const MmapTag).as_ref();
                    Tag::Mmap(mmap.unwrap())
                }
                _ => Tag::Unknown,
            }
        }
    }
}

pub struct TagIter<'a> {
    addr: usize,
    end: usize,
    _marker: core::marker::PhantomData<&'a Info>,
}

impl<'a> Iterator for TagIter<'a> {
    type Item = Tag<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // No more tags to parse
        if self.addr + size_of::<RawTag>() > self.end {
            return None;
        }

        let tag = unsafe { &*(self.addr as *const RawTag) };

        // Reject degenerate headers
        if (tag.size as usize) < size_of::<RawTag>() || (tag.size as usize) > self.end - self.addr {
            return None;
        }

        let current = self.addr;

        // Advance iterator and align to an 8-byte boundary
        self.addr += (tag.size as usize + 7) & !7;

        Some(unsafe { Tag::from_raw(current as *const RawTag) })
    }
}

#[repr(C, packed)]
pub struct RawTag {
    type_: u32,
    size: u32,
}

#[repr(u32)]
#[derive(Clone)]
pub enum MmapEntryType {
    Available = 1,
    Reserved = 2,
    AcpiInfo = 3,
    HibernationReserved = 4,
    DefectiveRam = 5,
}

#[repr(C)]
pub struct MmapEntry {
    pub base_addr: u64,
    pub length: u64,
    pub type_: u32,
    reserved: u32,
}

#[repr(C, packed)]
pub struct MmapTag {
    base: RawTag,
    entry_size: u32,
    entry_version: u32,
}

impl MmapTag {
    pub fn entries(&self) -> MmapEntryIter {
        let base = self as *const MmapTag as usize;

        MmapEntryIter {
            addr: base + 16,
            end: base + self.base.size as usize,
            entry_size: self.entry_size as usize,
        }
    }
}

pub struct MmapEntryIter {
    addr: usize,
    end: usize,
    entry_size: usize,
}

impl Iterator for MmapEntryIter {
    type Item = MmapEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.addr + self.entry_size > self.end {
            return None;
        }

        let entry = unsafe { core::ptr::read_unaligned(self.addr as *const MmapEntry) };

        self.addr += self.entry_size;

        Some(entry)
    }
}
