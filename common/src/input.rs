use core::arch::asm;

const QUEUE_CAPACITY: usize = 64;

#[derive(Clone, Copy)]
pub enum InputEvent {
    Empty,
    KeyDown { scancode: u8, modifiers: u8 },
    MouseMotion { dx: i16, dy: i16, buttons: u8 },
}

static mut QUEUE: [InputEvent; QUEUE_CAPACITY] = [InputEvent::Empty; QUEUE_CAPACITY];
static mut HEAD: usize = 0;
static mut TAIL: usize = 0;
static mut COUNT: usize = 0;

#[cfg(all(not(test), target_arch = "x86_64"))]
unsafe fn disable_interrupts() -> usize {
    let flags: usize;
    asm!("pushfq", "pop {}", "cli", out(reg) flags);
    flags
}

#[cfg(all(not(test), target_arch = "x86"))]
unsafe fn disable_interrupts() -> usize {
    let flags: usize;
    asm!("pushfd", "pop {}", "cli", out(reg) flags);
    flags
}

#[cfg(test)]
unsafe fn disable_interrupts() -> usize {
    0
}

#[cfg(all(not(test), target_arch = "x86_64"))]
unsafe fn restore_interrupts(flags: usize) {
    asm!("push {}", "popfq", in(reg) flags);
}

#[cfg(all(not(test), target_arch = "x86"))]
unsafe fn restore_interrupts(flags: usize) {
    asm!("push {}", "popfd", in(reg) flags);
}

#[cfg(test)]
unsafe fn restore_interrupts(_flags: usize) {}

pub unsafe fn push(event: InputEvent) {
    let flags = disable_interrupts();
    if COUNT < QUEUE_CAPACITY {
        QUEUE[TAIL] = event;
        TAIL = (TAIL + 1) % QUEUE_CAPACITY;
        COUNT += 1;
    }
    restore_interrupts(flags);
}

pub unsafe fn pop() -> Option<InputEvent> {
    let flags = disable_interrupts();
    let event = if COUNT == 0 {
        None
    } else {
        let event = QUEUE[HEAD];
        QUEUE[HEAD] = InputEvent::Empty;
        HEAD = (HEAD + 1) % QUEUE_CAPACITY;
        COUNT -= 1;
        Some(event)
    };
    restore_interrupts(flags);
    event
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_preserves_order_and_drops_when_full() {
        unsafe {
            HEAD = 0;
            TAIL = 0;
            COUNT = 0;
            for key in 0..QUEUE_CAPACITY as u8 {
                push(InputEvent::KeyDown { scancode: key, modifiers: 0 });
            }
            push(InputEvent::KeyDown { scancode: 255, modifiers: 0 });
            for key in 0..QUEUE_CAPACITY as u8 {
                match pop() {
                    Some(InputEvent::KeyDown { scancode, .. }) => assert_eq!(scancode, key),
                    _ => panic!("unexpected input event"),
                }
            }
            assert!(pop().is_none());
        }
    }
}