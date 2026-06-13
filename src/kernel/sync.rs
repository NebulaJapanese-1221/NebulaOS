use core::sync::atomic::{AtomicBool, Ordering};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::arch::asm;

pub struct Spinlock<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for Spinlock<T> {}

pub struct SpinlockGuard<'a, T> {
    lock: &'a AtomicBool,
    data: &'a mut T,
    interrupts_enabled: bool,
}

impl<T> Spinlock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> SpinlockGuard<'_, T> {
        // Save current EFLAGS and disable interrupts
        let eflags: u32;
        unsafe {
            asm!("pushfd", "pop {0}", "cli", out(reg) eflags);
        }
        // Bit 9 of EFLAGS is the Interrupt Flag (IF)
        let interrupts_enabled = (eflags & 0x200) != 0;

        // Loop until the lock is acquired
        while self.lock.compare_exchange(
            false,
            true,
            Ordering::Acquire,
            Ordering::Relaxed
        ).is_err() {
            // Spin-loop hint for the CPU to save power/cycles
            core::hint::spin_loop();
        }

        SpinlockGuard {
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() },
            interrupts_enabled,
        }
    }

    /// Forcibly releases the lock.
    /// 
    /// # Safety
    /// This should only be used in emergency situations like a kernel panic.
    pub unsafe fn force_unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }
}

impl<'a, T> Deref for SpinlockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { self.data }
}

impl<'a, T> DerefMut for SpinlockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T { self.data }
}

impl<'a, T> Drop for SpinlockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
        
        // Re-enable interrupts only if they were enabled before we locked
        if self.interrupts_enabled {
            unsafe { asm!("sti"); }
        }
    }
}