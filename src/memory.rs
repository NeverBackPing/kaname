#[repr(C, align(4096))]
pub struct PageDirectory {
    pub entries: [u32; 1024],
}

#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [u32; 1024],
}

static mut PAGE_DIRECTORY: PageDirectory = PageDirectory {
    entries: [0; 1024],
};

static mut PAGE_TABLE: PageTable = PageTable {
    entries: [0; 1024],
};

pub fn init_paging() {
    unsafe {
        let page_directory = &raw mut PAGE_DIRECTORY;
        let page_table = &raw mut PAGE_TABLE;

        // Page Directory
        for i in 0..1024 {
            (*page_directory).entries[i] = 0x00000002;
        }

        // Identity map first 4 MiB
        for i in 0..1024 {
            (*page_table).entries[i] = (i as u32 * 0x1000) | 0x3;
        }

        // PDE[0] -> PAGE_TABLE
        (*page_directory).entries[0] =
            (page_table as *const PageTable as u32) | 0x3;
    }
}
