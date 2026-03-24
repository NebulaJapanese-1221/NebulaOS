use alloc::vec::Vec;
use alloc::string::String;
use crate::spa::Spa;
use crate::vdev::Vdev;
use crate::zap;
use crate::dmu::ObjsetPhys;
use core::mem::size_of;

pub struct NebulaFileSystem {
    pub spa: Spa,
}

impl NebulaFileSystem {
    /// Attempts to mount the filesystem from the given root VDEV.
    pub fn mount(root_vdev: Vdev) -> Option<Self> {
        let spa = Spa::find(root_vdev)?;
        Some(Self { spa })
    }

    /// Lists the contents of the root directory.
    pub fn list_root(&self) -> Vec<String> {
        let mut files = Vec::new();
        let root_bp = self.spa.uberblock.rootbp;
        // Read the Object Set (FileSystem) pointed to by RootBP
        let os_data = self.spa.root_vdev.read_block(root_bp.offset, root_bp.asize as usize);
        
        if os_data.len() >= size_of::<ObjsetPhys>() {
            let os = unsafe { &*(os_data.as_ptr() as *const ObjsetPhys) };
            
            // Assume Root Directory is Object 2 (Master Node logic simplified)
            if let Some(root_dnode) = os.get_dnode(&self.spa.root_vdev, 2) {
                if let Some(dir_data) = root_dnode.read_data(&self.spa.root_vdev, 0, root_dnode.datablksz as usize) {
                    let entries = zap::parse_directory(dir_data.as_slice());
                    for entry in entries {
                        let mut name = entry.name;
                        if entry.type_ == 4 { name.push('/'); } 
                        files.push(name);
                    }
                }
            }
        }
        
        if files.is_empty() {
             files.push(String::from("<Empty Pool>"));
        }
        files
    }
}