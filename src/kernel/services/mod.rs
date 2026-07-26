// NebulaOS System Services
// Main module for all system services

pub mod network;  // Networking service
pub mod security; // Security service
pub mod power;    // Power management service

use crate::sync::Spinlock;
use core::sync::atomic::{AtomicBool, Ordering};

/// System services manager
pub struct ServicesManager {
    initialized: bool,
}

impl ServicesManager {
    pub fn new() -> Self {
        ServicesManager {
            initialized: false,
        }
    }
    
    pub fn init(&mut self) {
        if self.initialized {
            return;
        }
        
        // Initialize all services
        network::init();
        security::init();
        power::init();
        
        self.initialized = true;
    }
    
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

/// Global services manager instance - lazily initialized
static SERVICES_MANAGER_INITIALIZED: AtomicBool = AtomicBool::new(false);
static mut SERVICES_MANAGER_DATA: Option<Spinlock<ServicesManager>> = None;

fn get_services_manager() -> &'static Spinlock<ServicesManager> {
    if !SERVICES_MANAGER_INITIALIZED.load(Ordering::SeqCst) {
        unsafe {
            SERVICES_MANAGER_DATA = Some(Spinlock::new(ServicesManager::new()));
            SERVICES_MANAGER_INITIALIZED.store(true, Ordering::SeqCst);
        }
    }
    unsafe { SERVICES_MANAGER_DATA.as_ref().unwrap() }
}

/// Initialize all system services
pub fn init() {
    get_services_manager().lock().init();
}

