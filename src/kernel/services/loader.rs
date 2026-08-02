// Application Loader Service for NebulaOS
// Manages app registration, ELF loading, and app lifecycle

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

use core::sync::atomic::{AtomicBool, Ordering};
use crate::elf_loader::{self, LoadedProgram, verify_elf};
use crate::serial_println;
use crate::sync::Spinlock;

/// App manifest — metadata for a registered app
#[derive(Debug, Clone)]
pub struct AppManifest {
    pub app_id: u32,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub entry_point: Option<usize>,
    pub is_builtin: bool,
    pub elf_data: Option<Vec<u8>>,
}

/// App state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppState {
    Registered,
    Loaded,
    Running,
    Crashed,
}

/// Running app instance
#[derive(Debug)]
pub struct AppInstance {
    pub app_id: u32,
    pub pid: Option<usize>,
    pub state: AppState,
    pub program: Option<LoadedProgram>,
}

/// Application Loader Service
pub struct AppLoaderService {
    apps: BTreeMap<u32, AppManifest>,
    instances: BTreeMap<u32, AppInstance>,
    next_app_id: u32,
    next_instance_id: u32,
}

impl AppLoaderService {
    pub fn new() -> Self {
        AppLoaderService {
            apps: BTreeMap::new(),
            instances: BTreeMap::new(),
            next_app_id: 1,
            next_instance_id: 1,
        }
    }

    /// Register a built-in app from embedded ELF data
    pub fn register_builtin(&mut self, name: &str, version: &str, author: &str, description: &str, elf_data: &[u8]) -> Result<u32, &'static str> {
        // Verify the ELF is valid
        let _ = verify_elf(elf_data)?;
        
        let app_id = self.next_app_id;
        self.next_app_id += 1;

        let manifest = AppManifest {
            app_id,
            name: name.to_string(),
            version: version.to_string(),
            author: author.to_string(),
            description: description.to_string(),
            entry_point: None,
            is_builtin: true,
            elf_data: Some(elf_data.to_vec()),
        };

        self.apps.insert(app_id, manifest);
        serial_println!("AppLoader: Registered built-in app '{}' (id={})", name, app_id);
        Ok(app_id)
    }

    /// Register an external app from a file path (loaded from SD card or other storage)
    pub fn register_external(&mut self, name: &str, version: &str, author: &str, description: &str, elf_data: &[u8]) -> Result<u32, &'static str> {
        // Verify the ELF is valid
        let _ = verify_elf(elf_data)?;

        // Check if app with same name already exists
        if self.apps.values().any(|a| a.name == name) {
            return Err("App with this name already exists");
        }

        let app_id = self.next_app_id;
        self.next_app_id += 1;

        let manifest = AppManifest {
            app_id,
            name: name.to_string(),
            version: version.to_string(),
            author: author.to_string(),
            description: description.to_string(),
            entry_point: None,
            is_builtin: false,
            elf_data: Some(elf_data.to_vec()),
        };

        self.apps.insert(app_id, manifest);
        serial_println!("AppLoader: Registered external app '{}' (id={})", name, app_id);
        Ok(app_id)
    }

    /// Load an app into memory (parse ELF, allocate segments)
    pub fn load_app(&mut self, app_id: u32) -> Result<usize, &'static str> {
        let manifest = self.apps.get(&app_id).ok_or("App not found")?;

        let elf_data = manifest.elf_data.as_ref().ok_or("No ELF data for app")?;
        let program = elf_loader::load_elf(elf_data)?;
        let entry = elf_loader::load_elf_to_memory(&program)?;

        // Avoid double borrow by cloning the app name before mutable borrow below.
        let app_name = manifest.name.clone();

        // Store the entry point
        if let Some(manifest) = self.apps.get_mut(&app_id) {
            manifest.entry_point = Some(entry);
        }

        // Create an instance
        let instance_id = self.next_instance_id;
        self.next_instance_id += 1;

        self.instances.insert(instance_id, AppInstance {
            app_id,
            pid: Some(entry), // In real impl, this would be the spawned process PID
            state: AppState::Loaded,
            program: Some(program),
        });

        serial_println!("AppLoader: Loaded app '{}' at entry 0x{:x}", app_name, entry);
        Ok(instance_id as usize)
    }

    /// Launch an app (load if not loaded, then spawn as process)
    pub fn launch_app(&mut self, app_id: u32) -> Result<u32, &'static str> {
        let manifest = self.apps.get(&app_id).ok_or("App not found")?;

        // If not loaded yet, load it
        if manifest.entry_point.is_none() {
            self.load_app(app_id)?;
        }

        let manifest = self.apps.get(&app_id).ok_or("App not found")?;
        let entry = manifest.entry_point.ok_or("Entry point not set")?;

        // Spawn the app as a user process via the scheduler
        let pid = {
            let mut sched = crate::scheduler::get_scheduler().lock();
            sched.spawn_user_process(entry, 4096 * 4, 4096)
        };

        serial_println!("AppLoader: Launched app '{}' as PID {}", manifest.name, pid);
        Ok(pid as u32)
    }

    /// List all registered apps
    pub fn list_apps(&self) -> Vec<AppManifest> {
        self.apps.values().cloned().collect()
    }

    /// Get app manifest by ID
    pub fn get_app(&self, app_id: u32) -> Option<&AppManifest> {
        self.apps.get(&app_id)
    }

    /// Find app by name
    pub fn find_app_by_name(&self, name: &str) -> Option<&AppManifest> {
        self.apps.values().find(|a| a.name == name)
    }

    /// Unregister an app
    pub fn unregister_app(&mut self, app_id: u32) -> Result<(), &'static str> {
        if !self.apps.contains_key(&app_id) {
            return Err("App not found");
        }
        self.apps.remove(&app_id);
        self.instances.retain(|_, inst| inst.app_id != app_id);
        serial_println!("AppLoader: Unregistered app id={}", app_id);
        Ok(())
    }

    /// Initialize built-in apps
    pub fn init_builtin_apps(&mut self) {
        // The built-in apps will have their ELF binaries embedded.
        // For now, we register them with placeholder info since we're in a
        // no_std environment without separate ELF compilation for each app.
        // In production, each app would be compiled to a separate .elf file
        // and embedded as a byte array.

        self.register_builtin(
            "File Manager",
            "1.0.0",
            "NebulaOS Team",
            "Browse and manage files on the system",
            &[0x7f, b'E', b'L', b'F', 1, 1, 1, 0], // Minimal valid ELF header
        ).ok();

        self.register_builtin(
            "Terminal",
            "1.0.0",
            "NebulaOS Team",
            "Command-line interface for NebulaOS",
            &[0x7f, b'E', b'L', b'F', 1, 1, 1, 0],
        ).ok();

        self.register_builtin(
            "Calculator",
            "1.0.0",
            "NebulaOS Team",
            "Basic arithmetic calculator",
            &[0x7f, b'E', b'L', b'F', 1, 1, 1, 0],
        ).ok();

        self.register_builtin(
            "Text Editor",
            "1.0.0",
            "NebulaOS Team",
            "Edit text files",
            &[0x7f, b'E', b'L', b'F', 1, 1, 1, 0],
        ).ok();

        self.register_builtin(
            "Web Browser",
            "1.0.0",
            "NebulaOS Team",
            "Browse the web",
            &[0x7f, b'E', b'L', b'F', 1, 1, 1, 0],
        ).ok();

        self.register_builtin(
            "Image Viewer",
            "1.0.0",
            "NebulaOS Team",
            "View images",
            &[0x7f, b'E', b'L', b'F', 1, 1, 1, 0],
        ).ok();

        self.register_builtin(
            "System Monitor",
            "1.0.0",
            "NebulaOS Team",
            "Monitor system performance",
            &[0x7f, b'E', b'L', b'F', 1, 1, 1, 0],
        ).ok();

        self.register_builtin(
            "System Settings",
            "1.0.0",
            "NebulaOS Team",
            "Configure system settings",
            &[0x7f, b'E', b'L', b'F', 1, 1, 1, 0],
        ).ok();

        self.register_builtin(
            "App Loader",
            "1.0.0",
            "NebulaOS Team",
            "Browse and launch external applications",
            &[0x7f, b'E', b'L', b'F', 1, 1, 1, 0],
        ).ok();
    }
}

/// Global AppLoaderService instance
static LOADER_INITIALIZED: AtomicBool = AtomicBool::new(false);
static mut LOADER_SERVICE_DATA: Option<Spinlock<AppLoaderService>> = None;

pub fn get_loader_service() -> &'static Spinlock<AppLoaderService> {
    if !LOADER_INITIALIZED.load(Ordering::SeqCst) {
        unsafe {
            LOADER_SERVICE_DATA = Some(Spinlock::new(AppLoaderService::new()));
            LOADER_INITIALIZED.store(true, Ordering::SeqCst);
        }
    }
    unsafe { LOADER_SERVICE_DATA.as_ref().unwrap() }
}

/// Initialize the app loader service
pub fn init() {
    serial_println!("AppLoader: Initializing...");
    let mut loader = get_loader_service().lock();
    loader.init_builtin_apps();
    serial_println!("AppLoader: Initialization complete ({} apps registered)", loader.apps.len());
}

