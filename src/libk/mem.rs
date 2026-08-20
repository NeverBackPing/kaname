#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    for i in 0..n {
        unsafe {
            *dest.add(i) = *src.add(i);
        }
    }
    dest
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, mut n: usize) -> *mut u8 {
    if dest as *const u8 <= src {
        for i in 0..n {
            unsafe {
                *dest.add(i) = *src.add(i);
            }
        }
        return dest;
    }
    while n > 0 {
        n -= 1;
        unsafe {
            *dest.add(n) = *src.add(n);
        }
    }
    dest
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8 {
    let pattern: usize = 0x01010101 * (c as u8 as usize);
    let mut i: usize = 0;

    while (dest as usize + i) & 3 != 0 && i < n {
        unsafe {
            *dest.add(i) = c as u8;
        }
        i += 1;
    }

    while i + 4 <= n {
        unsafe {
            *((dest as usize + i) as *mut usize) = pattern;
            i += 4;
        }
    }

    while i < n {
        unsafe {
            *dest.add(i) = c as u8;
        }
        i += 1;
    }
    dest
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i: usize = 0;

    while i < n {
        unsafe {
            if *a.add(i) != *b.add(i) {
                return (*a.add(i) as i32) - (*b.add(i) as i32);
            }
            i += 1;
        }
    }
    0
}
