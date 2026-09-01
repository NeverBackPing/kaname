const PAGE_SIZE: usize = 0x1000;
const ENTRIES: usize = 1024;
const MAX_PT: usize = 4;

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
        self.0 |= 1 << 1;
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
];

pub fn init_paging() {
    unsafe {
        // Create the Page Directory
        for table_index in 0..MAX_PT {
            // Address of the PT
            let table_address = &raw const PAGE_TABLES[table_index] as u32;

            // PD → PT
            PAGE_DIRECTORY.entries[table_index].set_address(table_address);

            PAGE_DIRECTORY.entries[table_index].set_present();

            PAGE_DIRECTORY.entries[table_index].set_writable();

            // Create the PT
            for page_index in 0..ENTRIES {
                // Physical address of the page
                let address = (table_index * ENTRIES * PAGE_SIZE + page_index * PAGE_SIZE) as u32;

                // PT → physical page
                PAGE_TABLES[table_index].entries[page_index].set_address(address);

                PAGE_TABLES[table_index].entries[page_index].set_present();

                PAGE_TABLES[table_index].entries[page_index].set_writable();
            }
        }
    }
}
