pub unsafe fn memset(dest: *mut u8, value: u8, count: usize) -> *mut u8 {
    let mut i = 0;
    while i < count {
        *dest.add(i) = value;
        i += 1;
    }
    dest
}

pub unsafe fn memcpy(dest: *mut u8, src: *const u8, count: usize) -> *mut u8 {
    let mut i = 0;
    while i < count {
        *dest.add(i) = *src.add(i);
        i += 1;
    }
    dest
}

pub unsafe fn memcmp(a: *const u8, b: *const u8, count: usize) -> i32 {
    let mut i = 0;
    while i < count {
        let va = *a.add(i);
        let vb = *b.add(i);
        if va != vb {
            return if va < vb { -1 } else { 1 };
        }
        i += 1;
    }
    0
}

pub unsafe fn strlen(s: *const u8) -> usize {
    let mut i = 0;
    while *s.add(i) != 0 {
        i += 1;
    }
    i
}

pub unsafe fn strcpy(dest: *mut u8, src: *const u8) -> *mut u8 {
    let mut i = 0;
    while *src.add(i) != 0 {
        *dest.add(i) = *src.add(i);
        i += 1;
    }
    *dest.add(i) = 0;
    dest
}

pub unsafe fn strcmp(a: *const u8, b: *const u8) -> i32 {
    let mut i = 0;
    while *a.add(i) != 0 && *b.add(i) != 0 && *a.add(i) == *b.add(i) {
        i += 1;
    }
    (*a.add(i) as i32) - (*b.add(i) as i32)
}
