// 1MiB of scrollback data = 6241 lines
// generation tracking allows for up to 409M lines before wraparound

const MAX_COLS: usize = 80;

struct RowBlock {
    next: Option<RowHandle>,
    generation: u16,
    cells: [u16; MAX_COLS],
}

impl RowBlock {
    pub const fn new() -> Self {
        Self {
            generation: 0,
            next: None,
            cells: [0; MAX_COLS],
        }
    }
}

#[derive(Clone, Copy)]
pub struct RowHandle {
    id: u16,
    generation: u16,
}

impl RowHandle {
    pub fn prev(self) -> Option<RowHandle> {
        let pool = get_pool();
        let row_entry = &pool.data[self.id as usize];
        if self.generation != row_entry.generation {
            None
        } else {
            row_entry.next
        }
    }
}

const ROW_COUNT: usize = 0x10_0000 / size_of::<RowBlock>();

struct Pool {
    next_free: u16,
    data: [RowBlock; ROW_COUNT],
}

static mut POOL: Pool = Pool {
    data: [const { RowBlock::new() }; ROW_COUNT],
    next_free: 0,
};

fn get_pool() -> &'static mut Pool {
    unsafe { (&raw mut POOL).as_mut().unwrap() }
}

pub fn allocate_row(current_row: Option<RowHandle>, blank: u16) -> RowHandle {
    let pool = get_pool();
    let handle_id = pool.next_free;
    let generation = &mut pool.data[handle_id as usize].generation;
    *generation = generation.wrapping_add(1);
    pool.next_free = (pool.next_free + 1) % ROW_COUNT as u16;
    pool.data[handle_id as usize].next = current_row;
    pool.data[handle_id as usize].cells = [blank; MAX_COLS];
    RowHandle {
        id: handle_id,
        generation: pool.data[handle_id as usize].generation,
    }
}

pub fn get_row_data(row: RowHandle) -> Option<&'static mut [u16; MAX_COLS]> {
    let pool = get_pool();
    let row_entry = &mut pool.data[row.id as usize];
    if row_entry.generation != row.generation {
        None
    } else {
        Some(&mut row_entry.cells)
    }
}
