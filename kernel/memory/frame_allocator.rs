// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This file is part of NebulaOS.
//
// NebulaOS is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.
//
// NebulaOS is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

use crate::sync::SpinLock;

const FRAME_SIZE: usize = 4096;
const MEMORY_START: usize = 0x100000;
const MEMORY_END: usize = 0x10000000;
const TOTAL_FRAMES: usize = (MEMORY_END - MEMORY_START) / FRAME_SIZE;
const BITMAP_SIZE: usize = (TOTAL_FRAMES + 7) / 8;

struct AllocatorState {
    bitmap: [u8; BITMAP_SIZE],
    allocated_frames: usize,
}

static STATE: SpinLock<AllocatorState> = SpinLock::new(AllocatorState {
    bitmap: [0; BITMAP_SIZE],
    allocated_frames: 0,
});

pub fn init() {
    let mut state = STATE.lock();
    state.bitmap.fill(0);
    for index in 0..256 {
        set_bit(&mut state, index);
    }
    state.allocated_frames = 256;
}

fn set_bit(state: &mut AllocatorState, index: usize) {
    state.bitmap[index / 8] |= 1 << (index % 8);
}

fn clear_bit(state: &mut AllocatorState, index: usize) {
    state.bitmap[index / 8] &= !(1 << (index % 8));
}

fn get_bit(state: &AllocatorState, index: usize) -> bool {
    state.bitmap[index / 8] & (1 << (index % 8)) != 0
}

pub fn alloc_frame() -> Option<usize> {
    let mut state = STATE.lock();
    for index in 0..TOTAL_FRAMES {
        if !get_bit(&state, index) {
            set_bit(&mut state, index);
            state.allocated_frames += 1;
            return Some(MEMORY_START + index * FRAME_SIZE);
        }
    }
    None
}

pub fn alloc_frame_below(limit: usize) -> Option<usize> {
    let frame_limit = limit
        .saturating_sub(MEMORY_START)
        .min(MEMORY_END - MEMORY_START)
        / FRAME_SIZE;
    let mut state = STATE.lock();
    for index in 0..frame_limit {
        if !get_bit(&state, index) {
            set_bit(&mut state, index);
            state.allocated_frames += 1;
            return Some(MEMORY_START + index * FRAME_SIZE);
        }
    }
    None
}

pub fn alloc_contiguous_frames_below(count: usize, limit: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let frame_limit = limit
        .saturating_sub(MEMORY_START)
        .min(MEMORY_END - MEMORY_START)
        / FRAME_SIZE;
    if count > frame_limit {
        return None;
    }

    let mut state = STATE.lock();
    for start in 0..=frame_limit - count {
        if (start..start + count).all(|index| !get_bit(&state, index)) {
            for index in start..start + count {
                set_bit(&mut state, index);
            }
            state.allocated_frames += count;
            return Some(MEMORY_START + start * FRAME_SIZE);
        }
    }
    None
}

pub fn free_frame(address: usize) {
    if !(MEMORY_START..MEMORY_END).contains(&address) {
        return;
    }
    let mut state = STATE.lock();
    let index = (address - MEMORY_START) / FRAME_SIZE;
    if !get_bit(&state, index) {
        return;
    }
    clear_bit(&mut state, index);
    state.allocated_frames -= 1;
}

pub fn free_frame_count() -> usize {
    let state = STATE.lock();
    TOTAL_FRAMES - state.allocated_frames
}

pub fn total_frame_count() -> usize {
    TOTAL_FRAMES
}
