// NebulaOS System Services
// Main module for all system services

pub mod network;  // Networking service
pub mod security; // Security service
pub mod power;    // Power management service

use spin::Mutex;

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

/// Global services manager instance
pub static SERVICES_MANAGER: Mutex<ServicesManager> = Mutex::new(ServicesManager::new());

/// Initialize all system services
pub fn init() {
    SERVICES_MANAGER.lock().init();
}