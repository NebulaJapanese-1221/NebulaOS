// Process Management for NebulaOS
// Enhanced with threading support and process groups

use alloc::vec::Vec;
use alloc::string::String;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicUsize, Ordering};
use x86_64::structures::paging::{PageTable, Mapper, Size4KiB, FrameAllocator};
use x86_64::{VirtAddr, PhysAddr};
use crate::memory::protection::MemoryProtection;

/// Process ID generator
static NEXT_PID: AtomicUsize = AtomicUsize::new(1);

/// Process state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProcessState {
    Created,
    Running,
    Blocked,
    Sleeping(usize), // Add wakeup tick
    Zombie,
    Stopped,
}

/// Process structure
pub struct Process {
    pub pid: usize,
    pub parent_pid: usize,
    pub state: ProcessState,
    pub page_table: PageTable,
    pub kernel_stack_ptr: u64,
    pub user_stack_ptr: u64,
    pub page_directory_phys_addr: u64,
    pub user_eip: u32,
    pub name: String,
    pub threads: Vec<Thread>,
    pub children: Vec<usize>,
    pub exit_code: i32,
    pub memory_protection: MemoryProtection,
    pub process_group: usize,
    pub session: usize,
    pub signals: SignalHandler,
}

impl Process {
    /// Create a new process
    pub fn new(name: &str, parent_pid: usize) -> Self {
        let pid = NEXT_PID.fetch_add(1, Ordering::SeqCst);
        
        Process {
            pid,
            parent_pid,
            state: ProcessState::Created,
            page_table: unsafe { PageTable::new() },
            kernel_stack_ptr: 0,
            user_stack_ptr: 0,
            page_directory_phys_addr: 0,
            user_eip: 0,
            name: name.to_string(),
            threads: Vec::new(),
            children: Vec::new(),
            exit_code: 0,
            memory_protection: MemoryProtection::new(),
            process_group: pid, // New processes start their own process group
            session: pid,       // New processes start their own session
            signals: SignalHandler::new(),
        }
    }
    
    /// Create a new thread in this process
    pub fn create_thread(&mut self, entry: u32, stack_size: usize) -> Thread {
        let tid = self.threads.len();
        let thread = Thread::new(self.pid, tid, entry, stack_size);
        self.threads.push(thread.clone());
        thread
    }
    
    /// Get the main thread
    pub fn main_thread(&self) -> Option<&Thread> {
        self.threads.first()
    }
    
    /// Add a child process
    pub fn add_child(&mut self, child_pid: usize) {
        self.children.push(child_pid);
    }
    
    /// Remove a child process
    pub fn remove_child(&mut self, child_pid: usize) {
        self.children.retain(|&pid| pid != child_pid);
    }
    
    /// Set process state
    pub fn set_state(&mut self, state: ProcessState) {
        self.state = state;
    }
    
    /// Send a signal to this process
    pub fn send_signal(&mut self, signal: Signal) {
        self.signals.send(signal);
    }
    
    /// Check for pending signals
    pub fn check_signals(&mut self) -> Option<Signal> {
        self.signals.check()
    }
    
    /// Set process group
    pub fn set_process_group(&mut self, pgid: usize) {
        self.process_group = pgid;
    }
    
    /// Set session
    pub fn set_session(&mut self, sid: usize) {
        self.session = sid;
    }
}

/// Thread structure
#[derive(Debug, Clone)]
pub struct Thread {
    pub tid: usize,
    pub pid: usize,
    pub state: ProcessState,
    pub stack_ptr: u64,
    pub stack_size: usize,
    pub entry_point: u32,
    pub registers: ThreadRegisters,
    pub thread_local_storage: *mut u8,
}

impl Thread {
    /// Create a new thread
    pub fn new(pid: usize, tid: usize, entry: u32, stack_size: usize) -> Self {
        Thread {
            tid,
            pid,
            state: ProcessState::Created,
            stack_ptr: 0,
            stack_size,
            entry_point: entry,
            registers: ThreadRegisters::new(),
            thread_local_storage: core::ptr::null_mut(),
        }
    }
    
    /// Set thread state
    pub fn set_state(&mut self, state: ProcessState) {
        self.state = state;
    }
}

/// Thread registers
#[derive(Debug, Clone, Copy)]
pub struct ThreadRegisters {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
    pub esi: u32,
    pub edi: u32,
    pub ebp: u32,
    pub esp: u32,
    pub eip: u32,
    pub eflags: u32,
    pub cs: u32,
    pub ds: u32,
    pub es: u32,
    pub fs: u32,
    pub gs: u32,
}

impl ThreadRegisters {
    /// Create new registers with default values
    pub fn new() -> Self {
        ThreadRegisters {
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 0,
            esi: 0,
            edi: 0,
            ebp: 0,
            esp: 0,
            eip: 0,
            eflags: 0,
            cs: 0,
            ds: 0,
            es: 0,
            fs: 0,
            gs: 0,
        }
    }
}

/// Signal types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Signal {
    Sighup = 1,    // Hangup
    Sigint = 2,    // Interrupt
    Sigquit = 3,   // Quit
    Sigill = 4,    // Illegal instruction
    Sigtrap = 5,   // Trace/breakpoint trap
    Sigabrt = 6,   // Abort
    Sigfpe = 8,    // Floating point exception
    Sigkill = 9,   // Kill
    Sigsegv = 11,  // Segment violation
    Sigpipe = 13,  // Broken pipe
    Sigalrm = 14,  // Alarm clock
    Sigterm = 15,  // Termination
    Sigchld = 17,  // Child status change
    Sigcont = 18,  // Continue
    Sigstop = 19,  // Stop
    Sigtstp = 20,  // Keyboard stop
    Sigttin = 21,  // Background read
    Sigttou = 22,  // Background write
}

/// Signal handler
pub struct SignalHandler {
    pending: Vec<Signal>,
    handlers: BTreeMap<Signal, SignalHandlerFunc>,
}

impl SignalHandler {
    /// Create a new signal handler
    pub fn new() -> Self {
        SignalHandler {
            pending: Vec::new(),
            handlers: BTreeMap::new(),
        }
    }
    
    /// Send a signal
    pub fn send(&mut self, signal: Signal) {
        self.pending.push(signal);
    }
    
    /// Check for pending signals
    pub fn check(&mut self) -> Option<Signal> {
        self.pending.pop()
    }
    
    /// Set a signal handler
    pub fn set_handler(&mut self, signal: Signal, handler: SignalHandlerFunc) {
        self.handlers.insert(signal, handler);
    }
    
    /// Get the handler for a signal
    pub fn get_handler(&self, signal: Signal) -> Option<SignalHandlerFunc> {
        self.handlers.get(&signal).copied()
    }
}

/// Signal handler function type
pub type SignalHandlerFunc = fn(pid: usize, signal: Signal);

/// Process manager
pub struct ProcessManager {
    pub processes: BTreeMap<usize, Process>,
    pub current_pid: usize,
    pub process_groups: BTreeMap<usize, Vec<usize>>, // PGID -> [PIDs]
    pub sessions: BTreeMap<usize, Vec<usize>>,      // SID -> [PGIDs]
}

impl ProcessManager {
    /// Create a new process manager
    pub fn new() -> Self {
        ProcessManager {
            processes: BTreeMap::new(),
            current_pid: 0,
            process_groups: BTreeMap::new(),
            sessions: BTreeMap::new(),
        }
    }
    
    /// Create a new process
    pub fn create_process(&mut self, name: &str) -> usize {
        let parent_pid = self.current_pid;
        let mut process = Process::new(name, parent_pid);
        let pid = process.pid;
        
        // Add to process groups and sessions
        self.process_groups.entry(process.process_group)
            .or_insert_with(Vec::new)
            .push(pid);
        self.sessions.entry(process.session)
            .or_insert_with(Vec::new)
            .push(process.process_group);
        
        // Add to parent's children
        if let Some(parent) = self.processes.get_mut(&parent_pid) {
            parent.add_child(pid);
        }
        
        self.processes.insert(pid, process);
        pid
    }
    
    /// Destroy a process
    pub fn destroy_process(&mut self, pid: usize) -> Option<Process> {
        if let Some(mut process) = self.processes.remove(&pid) {
            // Remove from parent's children
            if let Some(parent) = self.processes.get_mut(&process.parent_pid) {
                parent.remove_child(pid);
            }
            
            // Remove from process group
            if let Some(pg) = self.process_groups.get_mut(&process.process_group) {
                pg.retain(|&p| p != pid);
            }
            
            // Remove from session if process group is empty
            if let Some(pg) = self.process_groups.get(&process.process_group) {
                if pg.is_empty() {
                    self.process_groups.remove(&process.process_group);
                    if let Some(s) = self.sessions.get_mut(&process.session) {
                        s.retain(|&p| p != process.process_group);
                    }
                }
            }
            
            // Clean up children (reparent to init)
            for &child_pid in &process.children {
                if let Some(child) = self.processes.get_mut(&child_pid) {
                    child.parent_pid = 1; // Reparent to init
                }
            }
            
            Some(process)
        } else {
            None
        }
    }
    
    /// Set the current process
    pub fn set_current(&mut self, pid: usize) {
        self.current_pid = pid;
    }
    
    /// Get the current process
    pub fn current(&self) -> Option<&Process> {
        self.processes.get(&self.current_pid)
    }
    
    /// Get process by ID
    pub fn get(&self, pid: usize) -> Option<&Process> {
        self.processes.get(&pid)
    }
    
    /// Get process by ID (mutable)
    pub fn get_mut(&mut self, pid: usize) -> Option<&mut Process> {
        self.processes.get_mut(&pid)
    }
    
    /// Send signal to a process
    pub fn send_signal(&mut self, pid: usize, signal: Signal) {
        if let Some(process) = self.processes.get_mut(&pid) {
            process.send_signal(signal);
        }
    }
    
    /// Send signal to a process group
    pub fn send_signal_to_group(&mut self, pgid: usize, signal: Signal) {
        if let Some(pids) = self.process_groups.get(&pgid) {
            for &pid in pids {
                self.send_signal(pid, signal);
            }
        }
    }
    
    /// Create a new process group
    pub fn create_process_group(&mut self, leader_pid: usize) -> usize {
        let pgid = leader_pid;
        self.process_groups.insert(pgid, vec![leader_pid]);
        
        if let Some(process) = self.processes.get_mut(&leader_pid) {
            process.set_process_group(pgid);
        }
        
        pgid
    }
    
    /// Create a new session
    pub fn create_session(&mut self, leader_pid: usize) -> usize {
        let sid = leader_pid;
        let pgid = self.create_process_group(leader_pid);
        self.sessions.insert(sid, vec![pgid]);
        
        if let Some(process) = self.processes.get_mut(&leader_pid) {
            process.set_session(sid);
        }
        
        sid
    }
}

