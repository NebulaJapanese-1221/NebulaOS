use crate::common::stdint::*;
use crate::common::process::{self, Process, PROC_READY};

pub const SCHEDULER_TIME_SLICE: u32 = 10;

static mut SCHEDULER_QUEUE: [u32; crate::common::process::MAX_PROCESSES] = [0; crate::common::process::MAX_PROCESSES];
static mut QUEUE_HEAD: usize = 0;
static mut QUEUE_TAIL: usize = 0;
static mut QUEUE_COUNT: usize = 0;
static mut CURRENT_SCHEDULED_PID: u32 = 0;
static mut TICK_COUNTER: u32 = 0;

pub unsafe fn scheduler_init() {
    for i in 0..crate::common::process::MAX_PROCESSES {
        SCHEDULER_QUEUE[i] = 0;
    }
    QUEUE_HEAD = 0;
    QUEUE_TAIL = 0;
    QUEUE_COUNT = 0;
    CURRENT_SCHEDULED_PID = 0;
    TICK_COUNTER = 0;
}

pub unsafe fn scheduler_add(pid: u32) {
    if QUEUE_COUNT >= crate::common::process::MAX_PROCESSES {
        return;
    }
    SCHEDULER_QUEUE[QUEUE_TAIL] = pid;
    QUEUE_TAIL = (QUEUE_TAIL + 1) % crate::common::process::MAX_PROCESSES;
    QUEUE_COUNT += 1;
}

pub unsafe fn scheduler_remove(pid: u32) {
    for i in 0..QUEUE_COUNT {
        let idx = (QUEUE_HEAD + i) % crate::common::process::MAX_PROCESSES;
        if SCHEDULER_QUEUE[idx] == pid {
            for j in i..QUEUE_COUNT - 1 {
                let cur = (QUEUE_HEAD + j) % crate::common::process::MAX_PROCESSES;
                let nxt = (QUEUE_HEAD + j + 1) % crate::common::process::MAX_PROCESSES;
                SCHEDULER_QUEUE[cur] = SCHEDULER_QUEUE[nxt];
            }
            QUEUE_TAIL = (QUEUE_TAIL - 1 + crate::common::process::MAX_PROCESSES) % crate::common::process::MAX_PROCESSES;
            QUEUE_COUNT -= 1;
            return;
        }
    }
}

pub unsafe fn scheduler_tick() {
    if QUEUE_COUNT == 0 {
        return;
    }
    TICK_COUNTER += 1;
    if TICK_COUNTER < SCHEDULER_TIME_SLICE {
        return;
    }
    TICK_COUNTER = 0;
    let old_pid = SCHEDULER_QUEUE[QUEUE_HEAD];
    QUEUE_HEAD = (QUEUE_HEAD + 1) % crate::common::process::MAX_PROCESSES;
    QUEUE_TAIL = (QUEUE_TAIL + 1) % crate::common::process::MAX_PROCESSES;
    SCHEDULER_QUEUE[QUEUE_TAIL] = old_pid;
    CURRENT_SCHEDULED_PID = SCHEDULER_QUEUE[QUEUE_HEAD];
    process::process_set_current(CURRENT_SCHEDULED_PID);
    if let Some(cur) = (CURRENT_SCHEDULED_PID as usize).checked_sub(1).and_then(|idx| PROCESS_TABLE.get_mut(idx)) {
        cur.state = PROC_RUNNING;
    }
}
