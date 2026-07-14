// Security Service for NebulaOS
// User authentication and authorization

use alloc::string::String;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU32, Ordering};

/// User structure
#[derive(Debug, Clone)]
pub struct User {
    pub uid: u32,
    pub username: String,
    pub password_hash: String, // In real implementation, use proper password hashing
    pub gid: u32,              // Primary group ID
    pub groups: Vec<u32>,      // Additional group IDs
    pub home_dir: String,
    pub shell: String,
}

impl User {
    pub fn new(username: &str, password: &str) -> Self {
        static NEXT_UID: AtomicU32 = AtomicU32::new(1000);
        let uid = NEXT_UID.fetch_add(1, Ordering::SeqCst);
        
        User {
            uid,
            username: username.to_string(),
            password_hash: password.to_string(), // TODO: Proper hashing
            gid: uid, // Default to user's own group
            groups: Vec::new(),
            home_dir: format!("/home/{}", username),
            shell: String::from("/bin/sh"),
        }
    }
    
    pub fn authenticate(&self, password: &str) -> bool {
        // In a real implementation, we would verify the password hash
        self.password_hash == password
    }
    
    pub fn add_group(&mut self, gid: u32) {
        if !self.groups.contains(&gid) {
            self.groups.push(gid);
        }
    }
    
    pub fn remove_group(&mut self, gid: u32) {
        self.groups.retain(|&g| g != gid);
    }
}

/// Group structure
#[derive(Debug, Clone)]
pub struct Group {
    pub gid: u32,
    pub name: String,
    pub members: Vec<u32>, // User IDs
}

impl Group {
    pub fn new(name: &str) -> Self {
        static NEXT_GID: AtomicU32 = AtomicU32::new(1000);
        let gid = NEXT_GID.fetch_add(1, Ordering::SeqCst);
        
        Group {
            gid,
            name: name.to_string(),
            members: Vec::new(),
        }
    }
    
    pub fn add_member(&mut self, uid: u32) {
        if !self.members.contains(&uid) {
            self.members.push(uid);
        }
    }
    
    pub fn remove_member(&mut self, uid: u32) {
        self.members.retain(|&u| u != uid);
    }
}

/// Security service
pub struct SecurityService {
    users: BTreeMap<u32, User>,
    groups: BTreeMap<u32, Group>,
    current_user: Option<u32>,
}

impl SecurityService {
    pub fn new() -> Self {
        let mut service = SecurityService {
            users: BTreeMap::new(),
            groups: BTreeMap::new(),
            current_user: None,
        };
        
        // Add root user
        let mut root = User::new("root", "root");
        root.uid = 0;
        root.gid = 0;
        root.home_dir = String::from("/root");
        service.users.insert(0, root);
        
        // Add root group
        let mut root_group = Group::new("root");
        root_group.gid = 0;
        root_group.add_member(0);
        service.groups.insert(0, root_group);
        
        service
    }
    
    pub fn create_user(&mut self, username: &str, password: &str) -> Result<u32, &'static str> {
        if self.users.values().any(|u| u.username == username) {
            return Err("User already exists");
        }
        
        let user = User::new(username, password);
        let uid = user.uid;
        self.users.insert(uid, user);
        Ok(uid)
    }
    
    pub fn delete_user(&mut self, uid: u32) -> Result<(), &'static str> {
        if uid == 0 {
            return Err("Cannot delete root user");
        }
        
        if let Some(user) = self.users.remove(&uid) {
            // Remove user from all groups
            for group in self.groups.values_mut() {
                group.remove_member(uid);
            }
            
            // If this is the current user, log out
            if self.current_user == Some(uid) {
                self.current_user = None;
            }
            
            Ok(())
        } else {
            Err("User not found")
        }
    }
    
    pub fn create_group(&mut self, name: &str) -> Result<u32, &'static str> {
        if self.groups.values().any(|g| g.name == name) {
            return Err("Group already exists");
        }
        
        let group = Group::new(name);
        let gid = group.gid;
        self.groups.insert(gid, group);
        Ok(gid)
    }
    
    pub fn delete_group(&mut self, gid: u32) -> Result<(), &'static str> {
        if gid == 0 {
            return Err("Cannot delete root group");
        }
        
        if let Some(group) = self.groups.remove(&gid) {
            // Remove group from all users
            for user in self.users.values_mut() {
                user.remove_group(gid);
            }
            
            Ok(())
        } else {
            Err("Group not found")
        }
    }
    
    pub fn authenticate(&mut self, username: &str, password: &str) -> Result<(), &'static str> {
        if let Some(user) = self.users.values().find(|u| u.username == username) {
            if user.authenticate(password) {
                self.current_user = Some(user.uid);
                Ok(())
            } else {
                Err("Invalid password")
            }
        } else {
            Err("User not found")
        }
    }
    
    pub fn logout(&mut self) {
        self.current_user = None;
    }
    
    pub fn current_user(&self) -> Option<&User> {
        self.current_user.and_then(|uid| self.users.get(&uid))
    }
    
    pub fn get_user(&self, uid: u32) -> Option<&User> {
        self.users.get(&uid)
    }
    
    pub fn get_group(&self, gid: u32) -> Option<&Group> {
        self.groups.get(&gid)
    }
    
    pub fn check_permission(&self, uid: u32, permission: Permission) -> bool {
        // In a real implementation, we would check user and group permissions
        // For now, root can do anything
        if let Some(user) = self.users.get(&uid) {
            if user.uid == 0 {
                return true; // Root has all permissions
            }
        }
        false
    }
}

/// Permission types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Permission {
    Read,
    Write,
    Execute,
    Admin,
}

/// Global security service instance
pub static SECURITY_SERVICE: spin::Mutex<SecurityService> = spin::Mutex::new(SecurityService::new());

/// Initialize the security service
pub fn init() {
    // Security service is initialized automatically
}