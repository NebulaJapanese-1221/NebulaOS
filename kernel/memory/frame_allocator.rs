// Memory management - frame allocator

const FRAME_SIZE: usize = 4096;
const MEMORY_START: usize = 0x100000; // 1MB
const MEMORY_END: usize = 0x10000000; // 256MB
const TOTAL_FRAMES: usize = (MEMORY_END - MEMORY_START) / FRAME_SIZE;
const BITMAP_SIZE: usize = (TOTAL_FRAMES + 7) / 8;

static mut BITMAP: [u8; BITMAP_SIZE] = [0; BITMAP_SIZE];
static mut ALLOCATED_FRAMES: usize = 0;

pub fn init() {
    unsafe {
        for i in 0..=255 {
            set_bit(i);
        }
        ALLOCATED_FRAMES = 256;
    }
}

fn set_bit(index: usize) {
    unsafe {
        BITMAP[index / 8] |= 1 << (index % 8);
    }
}

fn clear_bit(index: usize) {
    unsafe {
        BITMAP[index / 8] &= !(1 << (index % 8));
    }
}

fn get_bit(index: usize) -> bool {
    unsafe { BITMAP[index / 8] & (1 << (index % 8)) != 0 }
}

pub fn alloc_frame() -> Option<usize> {
    unsafe {
        for i in 0..TOTAL_FRAMES {
            if !get_bit(i) {
                set_bit(i);
                ALLOCATED_FRAMES += 1;
                return Some(MEMORY_START + i * FRAME_SIZE);
            }
        }
        None
    }
}

pub fn alloc_frame_below(limit: usize) -> Option<usize> {
    unsafe {
        let frame_limit = limit
            .saturating_sub(MEMORY_START)
            .min(MEMORY_END - MEMORY_START)
            / FRAME_SIZE;
        for i in 0..frame_limit {
            if !get_bit(i) {
                set_bit(i);
                ALLOCATED_FRAMES += 1;
                return Some(MEMORY_START + i * FRAME_SIZE);
            }
        }
        None
    }
}

pub fn alloc_contiguous_frames_below(count: usize, limit: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }

    unsafe {
        let frame_limit = limit
            .saturating_sub(MEMORY_START)
            .min(MEMORY_END - MEMORY_START)
            / FRAME_SIZE;
        if count > frame_limit {
            return None;
        }

        for start in 0..=frame_limit - count {
            if (start..start + count).all(|index| !get_bit(index)) {
                for index in start..start + count {
                    set_bit(index);
                }
                ALLOCATED_FRAMES += count;
                return Some(MEMORY_START + start * FRAME_SIZE);
            }
        }
    }
    None
}

pub fn free_frame(addr: usize) {
    unsafe {
        if addr < MEMORY_START || addr >= MEMORY_END {
            return;
        }
        let index = (addr - MEMORY_START) / FRAME_SIZE;
        if !get_bit(index) {
            return;
        }
        clear_bit(index);
        ALLOCATED_FRAMES -= 1;
    }
}

pub fn free_frame_count() -> usize {
    unsafe { TOTAL_FRAMES - ALLOCATED_FRAMES }
}

pub fn total_frame_count() -> usize {
    TOTAL_FRAMES
}
