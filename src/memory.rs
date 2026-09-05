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

#[warn(clippy::needless_range_loop)]
static mut PAGE_TABLES: [PageTable; MAX_PT] = [
    PageTable::new(),
    PageTable::new(),
    PageTable::new(),
    PageTable::new(),
];

#[warn(clippy::needless_range_loop)]
pub fn init_paging() {
    unsafe {
        let page_tables_ptr = &raw mut PAGE_TABLES;

        let tables_slice = core::slice::from_raw_parts_mut(page_tables_ptr as *mut PageTable, 1024);

        for (table_index, item) in tables_slice.iter_mut().take(MAX_PT).enumerate() {
            let table_address = (item as *const PageTable) as u32;

            // PD → PT
            item.entries[table_index].set_address(table_address);
            item.entries[table_index].set_present();
            item.entries[table_index].set_writable();

            // PT → physical pages
            for (page_index, entry) in item.entries.iter_mut().take(ENTRIES).enumerate() {
                let address = (table_index * ENTRIES * PAGE_SIZE + page_index * PAGE_SIZE) as u32;

                entry.set_address(address);
                entry.set_present();
                entry.set_writable();
            }
        }
    }
}
