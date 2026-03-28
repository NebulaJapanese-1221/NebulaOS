use crate::kernel::io;
use alloc::vec::Vec;
use nebulafs::vdev::BlockDevice;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

pub const SECTOR_SIZE: usize = 512;
pub const ATA_TIMEOUT_MS: usize = 3000;

pub static PRIMARY_WAITING_TASK: AtomicUsize = AtomicUsize::new(usize::MAX);
pub static SECONDARY_WAITING_TASK: AtomicUsize = AtomicUsize::new(usize::MAX);

static PRIMARY_BUS_LOCK: Mutex<()> = Mutex::new(());
static SECONDARY_BUS_LOCK: Mutex<()> = Mutex::new(());

#[repr(u16)]
enum Command {
    Read = 0x20,
    Write = 0x30,
    Identify = 0xEC,
    CacheFlush = 0xE7,
}

#[derive(Debug)]
pub struct AtaDrive {
    port_base: u16,
    is_master: bool,
}

impl AtaDrive {
    /// Creates a new ATA drive handle.
    /// `primary`: true for 0x1F0 (Primary Bus), false for 0x170 (Secondary Bus).
    /// `master`: true for Master drive, false for Slave.
    pub fn new(primary: bool, master: bool) -> Self {
        Self {
            port_base: if primary { 0x1F0 } else { 0x170 },
            is_master: master,
        }
    }

    /// Acquires the lock for the underlying ATA bus (Primary or Secondary).
    fn lock_bus(&self) -> spin::MutexGuard<'_, ()> {
        if self.port_base == 0x1F0 {
            PRIMARY_BUS_LOCK.lock()
        } else {
            SECONDARY_BUS_LOCK.lock()
        }
    }

    /// Reads `sectors` count of sectors starting at `lba`.
    pub fn read_sectors(&self, lba: u32, sectors: u8) -> Vec<u8> {
        let _bus_lock = self.lock_bus();

        for attempt in 1..=3 {
            let mut data = Vec::with_capacity(sectors as usize * SECTOR_SIZE);
            let mut success = true;

            unsafe {
                io::outb(self.port_base + 6, 0xE0 | ((self.is_master as u8) << 4) | ((lba >> 24) as u8 & 0x0F));
                io::outb(self.port_base + 1, 0x00);
                io::outb(self.port_base + 2, sectors);
                io::outb(self.port_base + 3, lba as u8);
                io::outb(self.port_base + 4, (lba >> 8) as u8);
                io::outb(self.port_base + 5, (lba >> 16) as u8);
                io::outb(self.port_base + 7, Command::Read as u8);
                
                for _ in 0..sectors {
                    if !self.wait_for_interrupt() {
                        success = false;
                        break;
                    }
                    
                    // Read 256 words (512 bytes)
                    for _ in 0..256 {
                        let word = io::inw(self.port_base);
                        data.push((word & 0xFF) as u8);
                        data.push((word >> 8) as u8);
                    }
                }
            }

            if success {
                return data;
            }

            crate::serial_println!("[ATA] Read failed at LBA {} (Attempt {}/3), retrying...", lba, attempt);
        }
        
        Vec::new() // Failed after 3 retries
    }
    
    /// Writes `data` to sectors starting at `lba`.
    /// Data length must be a multiple of 512.
    pub fn write_sectors(&self, lba: u32, data: &[u8]) {
        if data.len() % SECTOR_SIZE != 0 {
            return; 
        }
        let _bus_lock = self.lock_bus();

        let sectors = (data.len() / SECTOR_SIZE) as u8;
        
        for attempt in 1..=3 {
            let mut success = true;
            unsafe {
                io::outb(self.port_base + 6, 0xE0 | ((self.is_master as u8) << 4) | ((lba >> 24) as u8 & 0x0F));
                io::outb(self.port_base + 1, 0x00);
                io::outb(self.port_base + 2, sectors);
                io::outb(self.port_base + 3, lba as u8);
                io::outb(self.port_base + 4, (lba >> 8) as u8);
                io::outb(self.port_base + 5, (lba >> 16) as u8);
                io::outb(self.port_base + 7, Command::Write as u8);
                
                for i in 0..sectors {
                    if !self.wait_for_interrupt() {
                        success = false;
                        break;
                    }
                    
                    for j in 0..256 {
                        let offset = (i as usize * SECTOR_SIZE) + (j * 2);
                        // Little endian word
                        let word = (data[offset] as u16) | ((data[offset + 1] as u16) << 8);
                        io::outw(self.port_base, word);
                    }
                }
                
                if success {
                    // Wait for last write to complete before flushing
                    self.wait_busy();
                    
                    // Flush Cache
                    io::outb(self.port_base + 7, Command::CacheFlush as u8);
                    self.wait_busy();
                    return;
                }
            }

            crate::serial_println!("[ATA] Write failed at LBA {} (Attempt {}/3), retrying...", lba, attempt);
        }
    }

    /// Blocks the current task and waits for the ATA interrupt to fire.
    /// Returns true if the interrupt occurred, false if it timed out.
    unsafe fn wait_for_interrupt(&self) -> bool {
        let tid = crate::kernel::process::SCHEDULER.lock().get_current_task_id();
        
        if self.port_base == 0x1F0 {
            PRIMARY_WAITING_TASK.store(tid, Ordering::SeqCst);
        } else {
            SECONDARY_WAITING_TASK.store(tid, Ordering::SeqCst);
        }

        // Use Sleeping state to handle timeout efficiently. 
        // The scheduler will automatically wake us up after ATA_TIMEOUT_MS if no IRQ occurs.
        crate::kernel::process::SCHEDULER.lock().sleep_current_task(ATA_TIMEOUT_MS);

        // Perform an eager yield. This task will stop executing immediately and give up the CPU.
        // It will resume here once the interrupt handler calls unblock_task or the timeout expires.
        self.yield_task();

        // Check if we were resumed because the IRQ occurred (success) or because of a timeout.
        if self.port_base == 0x1F0 {
            PRIMARY_WAITING_TASK.swap(usize::MAX, Ordering::SeqCst) == usize::MAX
        } else {
            SECONDARY_WAITING_TASK.swap(usize::MAX, Ordering::SeqCst) == usize::MAX
        }
    }

    /// Waits for the drive to be ready (BSY=0, DRQ=1) for data transfer.
    unsafe fn poll(&self) {
        // Delay (400ns)
        for _ in 0..4 { io::inb(self.port_base + 7); }
        
        loop {
            let status = io::inb(self.port_base + 7);
            // BSY=0 and DRQ=1
            if (status & 0x80) == 0 && (status & 0x08) != 0 { break; }
            if (status & 0x01) != 0 { break; } // Error

            // Yield to other tasks while waiting for the drive to become ready
            self.yield_task();
        }
    }
    
    /// Waits for the drive to not be busy (BSY=0).
    unsafe fn wait_busy(&self) {
        for _ in 0..4 { io::inb(self.port_base + 7); }
        loop {
            let status = io::inb(self.port_base + 7);
            if (status & 0x80) == 0 { break; }
            if (status & 0x01) != 0 { break; }

            // Yield to other tasks while the drive is busy
            self.yield_task();
        }
    }

    /// Helper to voluntarily yield the current task's time slice.
    /// It manually constructs an interrupt frame to safely call the scheduler.
    #[inline(always)]
    unsafe fn yield_task(&self) {
        core::arch::asm!(
            "pushfd", "push cs", "lea eax, [1f]", "push eax",
            "push 0", "push ds", "push es", "push fs", "push gs", "pusha",
            "mov eax, esp", "push eax",
            "call {yield_now}",
            "add esp, 4", "mov esp, eax",
            "popa", "pop gs", "pop fs", "pop es", "pop ds", "add esp, 4", "iretd",
            "1:",
            yield_now = sym crate::kernel::process::yield_now,
            out("eax") _,
        );
    }
}

/// The ATA Interrupt Handler for IRQ 14 and 15.
pub extern "x86-interrupt" fn interrupt_handler(_frame: &mut crate::kernel::interrupts::InterruptStackFrame) {
    unsafe {
        // Acknowledge the interrupt by reading the Status Register
        io::inb(0x1F7); // Primary
        io::inb(0x177); // Secondary

        let p_task = PRIMARY_WAITING_TASK.swap(usize::MAX, Ordering::SeqCst);
        if p_task != usize::MAX {
            crate::kernel::process::SCHEDULER.lock().unblock_task(p_task);
        }

        let s_task = SECONDARY_WAITING_TASK.swap(usize::MAX, Ordering::SeqCst);
        if s_task != usize::MAX {
            crate::kernel::process::SCHEDULER.lock().unblock_task(s_task);
        }

        // Send EOI to PICs
        io::outb(0x20, 0x20); // Master
        io::outb(0xA0, 0x20); // Slave
    }
}

impl BlockDevice for AtaDrive {
    fn read(&self, offset: u64, size: usize) -> Vec<u8> {
        let lba = (offset / SECTOR_SIZE as u64) as u32;
        let sector_count = ((size + SECTOR_SIZE - 1) / SECTOR_SIZE) as u8;
        let mut data = self.read_sectors(lba, sector_count);
        
        // Trim to exact requested size if necessary
        if data.len() > size {
            data.truncate(size);
        }
        data
    }

    fn write(&self, offset: u64, data: &[u8]) {
        let lba = (offset / SECTOR_SIZE as u64) as u32;
        // Pad data to sector boundary if needed (naive implementation)
        let mut padded = Vec::from(data);
        while padded.len() % SECTOR_SIZE != 0 {
            padded.push(0);
        }
        self.write_sectors(lba, padded.as_slice());
    }
}