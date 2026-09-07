use crate::println;
use core::arch::asm;

const PAGE_SIZE: usize = 0x1000;
const ENTRIES: usize = 1024;
const MAX_PT: usize = 5;

const PT_KERNEL_MEMORY: usize = 4;
const KERNEL_VIRT: u32 = 0xE000_0000;
const KERNEL_PHYS: u32 = 0x0010_0000;

#[derive(Clone, Copy)]
pub struct PageDirectoryEntry(u32);

impl PageDirectoryEntry {
    pub const fn new() -> Self {
        Self(0)
    }

    pub fn set_address(&mut self, address: u32) {
        self.0 = (self.0 & 0xFFF) | (address & 0xFFFF_F000);
    }

    pub fn set_present(&mut self) {
        self.0 |= 1 << 0;
    }

    pub fn set_writable(&mut self) {
        self.0 |= 1 << 1;
    }
}

#[derive(Clone, Copy)]
pub struct PageTableEntry(u32);

#[allow(dead_code)]
impl PageTableEntry {
    pub const fn new() -> Self {
        Self(0)
    }

    pub fn value(&self) -> u32 {
        self.0
    }

    pub fn set_address(&mut self, address: u32) {
        self.0 = (self.0 & 0xFFF) | (address & 0xFFFF_F000);
    }

    pub fn set_present(&mut self) {
        self.0 |= 1 << 0;
    }

    pub fn set_writable(&mut self) {
        self.0 |= 1 << 1;
    }

    pub fn set_user(&mut self) {
        self.0 |= 1 << 2;
    }
}

#[repr(C, align(4096))]
pub struct PageDirectory {
    pub entries: [PageDirectoryEntry; ENTRIES],
}

impl PageDirectory {
    pub const fn new() -> Self {
        Self {
            entries: [PageDirectoryEntry::new(); ENTRIES],
        }
    }
}

#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; ENTRIES],
}

impl PageTable {
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::new(); ENTRIES],
        }
    }
}

#[allow(dead_code)]
static mut PAGE_DIRECTORY: PageDirectory = PageDirectory::new();

static mut PAGE_TABLES: [PageTable; MAX_PT] = [
    PageTable::new(),
    PageTable::new(),
    PageTable::new(),
    PageTable::new(),
    PageTable::new(),
];

pub fn init_paging() {
    unsafe {
        let page_tables_ptr = &raw mut PAGE_TABLES;

        let tables_slice =
            core::slice::from_raw_parts_mut(page_tables_ptr as *mut PageTable, MAX_PT);

        for (table_index, table) in tables_slice.iter_mut().take(4).enumerate() {
            let table_address = table as *const PageTable as u32;

            PAGE_DIRECTORY.entries[table_index].set_address(table_address);

            PAGE_DIRECTORY.entries[table_index].set_present();

            PAGE_DIRECTORY.entries[table_index].set_writable();

            for (page_index, entry) in table.entries.iter_mut().enumerate() {
                let address = (table_index * ENTRIES * PAGE_SIZE + page_index * PAGE_SIZE) as u32;

                entry.set_address(address);

                entry.set_present();

                entry.set_writable();
            }
        }

        // High-half mapping: 0xE0000000 → physical kernel

        const KERNEL_PD_INDEX: usize = (KERNEL_VIRT >> 22) as usize;

        //  Kernel PT[4].
        let kernel_table = &mut tables_slice[PT_KERNEL_MEMORY];

        let kernel_table_address = kernel_table as *const PageTable as u32;

        PAGE_DIRECTORY.entries[KERNEL_PD_INDEX].set_address(kernel_table_address);

        PAGE_DIRECTORY.entries[KERNEL_PD_INDEX].set_present();

        PAGE_DIRECTORY.entries[KERNEL_PD_INDEX].set_writable();

        for (page_index, entry) in kernel_table.entries.iter_mut().enumerate() {
            let physical_address = KERNEL_PHYS + (page_index * PAGE_SIZE) as u32;

            entry.set_address(physical_address);

            entry.set_present();

            entry.set_writable();
        }

        // Load CR3
        let page_directory_address = &raw const PAGE_DIRECTORY as u32;

        asm!(
            "mov cr3, {0}",
            in(reg) page_directory_address,
            options(nostack, preserves_flags)
        );

        // Enable paging

        let mut cr0: u32;

        asm!(
            "mov {0}, cr0",
            out(reg) cr0,
            options(nostack, preserves_flags)
        );

        cr0 |= 1 << 31;

        asm!(
            "mov cr0, {0}",
            in(reg) cr0,
            options(nostack, preserves_flags)
        );
    }
}

#[allow(unused)]
#[allow(dead_code)]
pub fn test_high_half() {
    const TEST_PHYS: u32 = 0x0100_0000;
    const TEST_VIRT: u32 = 0xE000_0000;

    unsafe {
        *(TEST_PHYS as *mut u32) = 0x1234_5678;

        let value = *(TEST_VIRT as *const u32);

        println!("Physical : {:#010X}", TEST_PHYS);
        println!("Virtual  : {:#010X}", TEST_VIRT);
        println!("Value    : {:#010X}", value);

        if value == 0x1234_5678 {
            println!("HIGH HALF MAPPING: OK");
        } else {
            println!("HIGH HALF MAPPING: ERROR");
        }
    }
}

#[allow(dead_code)]
pub fn print_page_tables() {
    unsafe {
        println!("--------- PAGE TABLES ---------");

        let page_tables_ptr = &raw const PAGE_TABLES;

        let tables_slice = core::slice::from_raw_parts(page_tables_ptr as *const PageTable, MAX_PT);

        for (table_index, table) in tables_slice.iter().enumerate() {
            let mut first: Option<(usize, u32)> = None;
            let mut last: Option<(usize, u32)> = None;
            let mut count = 0;

            for (page_index, entry) in table.entries.iter().enumerate() {
                let entry = entry.value();

                if entry & 1 == 0 {
                    continue;
                }

                count += 1;

                if first.is_none() {
                    first = Some((page_index, entry));
                }

                last = Some((page_index, entry));
            }

            if count == 0 {
                println!("PT[{}]: empty", table_index);
                continue;
            }

            println!("PT[{}]: {} pages", table_index, count);

            if let Some((page_index, entry)) = first {
                let virtual_address = if table_index < 4 {
                    (table_index * ENTRIES * PAGE_SIZE + page_index * PAGE_SIZE) as u32
                } else {
                    KERNEL_VIRT + (page_index * PAGE_SIZE) as u32
                };

                let pd_index = (virtual_address >> 22) & 0x3FF;

                let pt_index = (virtual_address >> 12) & 0x3FF;

                let physical_address = entry & 0xFFFF_F000;

                println!(
                    "V:{:#010X} -> PD[{}] -> PT[{}] -> P:{:#010X}",
                    virtual_address, pd_index, pt_index, physical_address
                );
            }

            if let Some((page_index, entry)) = last {
                let virtual_address = if table_index < 4 {
                    (table_index * ENTRIES * PAGE_SIZE + page_index * PAGE_SIZE) as u32
                } else {
                    KERNEL_VIRT + (page_index * PAGE_SIZE) as u32
                };

                let pd_index = (virtual_address >> 22) & 0x3FF;

                let pt_index = (virtual_address >> 12) & 0x3FF;

                let physical_address = entry & 0xFFFF_F000;

                println!(
                    "V:{:#010X} -> PD[{}] -> PT[{}] -> P:{:#010X}",
                    virtual_address, pd_index, pt_index, physical_address
                );
            }

            if table_index + 1 != MAX_PT {
                println!();
            }
        }

        println!("---------------------------------");
    }
}
