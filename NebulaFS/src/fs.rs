use alloc::vec::Vec;
use alloc::string::String;
use crate::spa::Spa;
use crate::vdev::Vdev;
use crate::zap;
use crate::dmu::{ObjsetPhys, DnodePhys, ObjectType};
use crate::spa::BlockPointer;

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
        
        if os_data.len() >= core::mem::size_of::<ObjsetPhys>() {
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

    /// Formats the VDEV with a fresh NebulaFS structure.
    /// Warning: Destroys all data on the VDEV.
    pub fn format(root_vdev: Vdev, name: &str) -> Option<Self> {
        let mut spa = Spa::create(name, root_vdev.clone());

        // --- Layout Strategy (Simplified) ---
        // LBA 256 (128KB): Uberblock
        // LBA 512 (256KB): ObjsetPhys (The Filesystem Object Set)
        // LBA 520 (260KB): Dnode Array (Contains Root Dnode)
        // LBA 528 (264KB): Root Directory Data (ZAP)

        let offset_uberblock = 128 * 1024;
        let offset_objset = 256 * 1024;
        let offset_dnodes = 260 * 1024;
        let offset_root_dir_data = 264 * 1024;

        // 1. Create Root Directory Data (Empty ZAP)
        let dir_data = alloc::vec![0u8; 512]; 
        root_vdev.write_block(offset_root_dir_data, dir_data.as_slice());

        // 2. Create Root Directory Dnode (Object ID 2)
        let mut root_bp = BlockPointer::new();
        root_bp.offset = offset_root_dir_data;
        root_bp.asize = 512;
        root_bp.fill_count = 1;

        let mut root_dnode = DnodePhys {
            object_type: ObjectType::Directory as u8,
            indirection_levels: 0,
            nblkptr: 1,
            datablksz: 512,
            bonus_type: 0,
            blkptr: [BlockPointer::new(); 3],
            bonus: [0; 64],
        };
        root_dnode.blkptr[0] = root_bp;

        // Create a Dnode block containing [Unused, MasterNode, RootDir]
        // We just zero out the first two and place RootDir at index 2.
        // DnodePhys size is ~455 bytes, so index 2 is at offset 455*2 = 910?
        // ZFS aligns dnodes to 512 bytes usually. Let's assume 512 byte stride.
        let mut dnode_block = alloc::vec![0u8; 512 * 3];
        unsafe {
            let ptr = dnode_block.as_mut_ptr().add(512 * 2) as *mut DnodePhys;
            ptr.write(root_dnode);
        }
        root_vdev.write_block(offset_dnodes, dnode_block.as_slice());

        // 3. Create ObjsetPhys (The Filesystem)
        // Its metadnode points to the Dnode Array we just wrote.
        let mut dnode_arr_bp = BlockPointer::new();
        dnode_arr_bp.offset = offset_dnodes;
        dnode_arr_bp.asize = (512 * 3) as u32;
        dnode_arr_bp.fill_count = 1;

        let mut objset = ObjsetPhys {
            metadnode: root_dnode.clone(), // Re-use struct structure, but update pointers
            zil_header: root_dnode.clone(), // Dummy
            type_: 2, // DMU_OST_ZFS
        };
        objset.metadnode.blkptr[0] = dnode_arr_bp;
        objset.metadnode.nblkptr = 1;
        objset.metadnode.datablksz = 512; // Metadata block size
        
        let objset_slice = unsafe { core::slice::from_raw_parts(&objset as *const _ as *const u8, core::mem::size_of::<ObjsetPhys>()) };
        root_vdev.write_block(offset_objset, objset_slice);

        // 4. Update Uberblock to point to Objset
        let mut os_bp = BlockPointer::new();
        os_bp.offset = offset_objset;
        os_bp.asize = core::mem::size_of::<ObjsetPhys>() as u32; // Physical size
        os_bp.fill_count = 1;

        spa.uberblock.rootbp = os_bp;
        spa.uberblock.txg = 1;
        
        // 5. Sync (Writes Uberblock to disk)
        spa.sync();

        Some(Self { spa })
    }
}