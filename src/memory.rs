use core::arch::asm;
use crate::{print, println};

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
        // --------------------------------------------------
        // Identity mapping : first 16 MiB
        // --------------------------------------------------

        for table_index in 0..MAX_PT {
            let table_address =
                &raw const PAGE_TABLES[table_index] as u32;

            PAGE_DIRECTORY.entries[table_index]
                .set_address(table_address);

            PAGE_DIRECTORY.entries[table_index]
                .set_present();

            PAGE_DIRECTORY.entries[table_index]
                .set_writable();

            for page_index in 0..ENTRIES {
                let address =
                    (table_index * ENTRIES * PAGE_SIZE
                    + page_index * PAGE_SIZE) as u32;

                PAGE_TABLES[table_index].entries[page_index]
                    .set_address(address);

                PAGE_TABLES[table_index].entries[page_index]
                    .set_present();

                PAGE_TABLES[table_index].entries[page_index]
                    .set_writable();
            }
        }

        // High-half mapping: 0xE0000000 → physical kernel

        const KERNEL_PD_INDEX: usize =
            (KERNEL_VIRT >> 22) as usize;

        // On utilise PAGE_TABLES[3] pour le kernel.
        let kernel_table_address =
            &raw const PAGE_TABLES[PT_KERNEL_MEMORY] as u32;

        PAGE_DIRECTORY.entries[KERNEL_PD_INDEX]
            .set_address(kernel_table_address);

        PAGE_DIRECTORY.entries[KERNEL_PD_INDEX]
            .set_present();

        PAGE_DIRECTORY.entries[KERNEL_PD_INDEX]
            .set_writable();

        for page_index in 0..ENTRIES {
            let physical_address =
                KERNEL_PHYS + (page_index * PAGE_SIZE) as u32;

            PAGE_TABLES[PT_KERNEL_MEMORY].entries[page_index]
                .set_address(physical_address);

            PAGE_TABLES[PT_KERNEL_MEMORY].entries[page_index]
                .set_present();

            PAGE_TABLES[PT_KERNEL_MEMORY].entries[page_index]
                .set_writable();
        }

        // Load CR3
        let page_directory_address =
            &raw const PAGE_DIRECTORY as u32;

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

pub fn test_high_half() {
    const TEST_PHYS: u32 = 0x0100_0000;
    const TEST_VIRT: u32 = 0xE000_0000;
    
    // Écrit une valeur dans la mémoire physique
    //*(TEST_PHYS as *mut u32) = 0x1234_5678;
    
    
    unsafe {
        // Lit la même mémoire via l'adresse virtuelle
        //let value = *(TEST_VIRT as *const u32);
        
        println!("Physical : {:#010X}", TEST_PHYS);
        println!("Virtual  : {:#010X}", TEST_VIRT);
        //println!("Value    : {:#010X}", value);
        
        /*if value == 0x1234_5678 {
            println!("HIGH HALF MAPPING: OK");
        } 
        else {
            println!("HIGH HALF MAPPING: ERROR");
        }
        */
    }
}

pub fn print_page_tables() {
    unsafe {

        for table_index in 0..MAX_PT {
            println!();
            println!("--- PAGE TABLE {} ---", table_index);

            for page_index in 0..ENTRIES {
                let entry = PAGE_TABLES[table_index].entries[page_index];

                let physical = entry.value();

                if physical & 1 == 0 {
                    continue;
                }

                let virtual_address =
                    if table_index == 4 {
                        // High-half
                        0xE000_0000
                            + (page_index * PAGE_SIZE) as u32
                    } else {
                        // Identity mapping
                        (table_index * ENTRIES * PAGE_SIZE
                            + page_index * PAGE_SIZE) as u32
                    };

                let physical_address =
                    physical & 0xFFFF_F000;

                println!(
                    "{:#010X} -> {:#010X}",
                    virtual_address,
                    physical_address
                );
            }
        }
    }
}


