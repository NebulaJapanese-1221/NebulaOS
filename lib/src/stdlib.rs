pub unsafe fn atoi(s: *const u8) -> i32 {
    let mut i = 0;
    let mut sign = 1;
    let mut result = 0;
    if *s == b'-' {
        sign = -1;
        i += 1;
    }
    while *s.add(i) >= b'0' && *s.add(i) <= b'9' {
        result = result * 10 + (*s.add(i) - b'0') as i32;
        i += 1;
    }
    result * sign
}

pub unsafe fn itoa(value: i32, buffer: *mut u8) {
    let mut i = 0;
    let mut val = value;
    if val == 0 {
        *buffer = b'0';
        *buffer.add(1) = 0;
        return;
    }
    if val < 0 {
        *buffer.add(i) = b'-';
        i += 1;
        val = -val;
    }
    let mut start = i;
    while val > 0 {
        *buffer.add(i) = (val % 10) as u8 + b'0';
        i += 1;
        val /= 10;
    }
    let mut j = start;
    let mut k = i - 1;
    while j < k {
        let tmp = *buffer.add(j);
        *buffer.add(j) = *buffer.add(k);
        *buffer.add(k) = tmp;
        j += 1;
        k -= 1;
    }
    *buffer.add(i) = 0;
}

static mut RANDOM_SEED: u32 = 1;

pub unsafe fn srand(seed: u32) {
    RANDOM_SEED = seed;
}

pub unsafe fn rand() -> i32 {
    RANDOM_SEED = RANDOM_SEED.wrapping_mul(1103515245).wrapping_add(12345);
    ((RANDOM_SEED / 65536) % 2048) as i32
}
