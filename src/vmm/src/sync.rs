// Copyright 2022 IBM Corporation. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Support for snapshot synchronization
use crate::memory_snapshot::SnapshotMemory;
use crate::vmm_config::snapshot::SnapshotType;
use snapshot::Snapshot;
use core::slice::SlicePattern;
use std::fs::OpenOptions;
use std::io::Write;
use std::net::TcpStream;
use std::io::Cursor;

use versionize::VersionMap;
use vm_memory::{GuestAddress, MemoryRegionAddress};

use crate::persist::CreateSnapshotError;
use crate::{mem_size_mib, MicrovmState, Vmm};
use std::path::Path; //, MemoryBackingFile};

use logger::{debug, info};
use bytemuck::cast_slice;

/// Synchronize snapshot state
pub fn snapshot_state_to_sync(
    microvm_state: &MicrovmState,
    snapshot_path: &Path,
    snapshot_data_version: u16,
    version_map: VersionMap,
) -> std::result::Result<(), CreateSnapshotError> {
    use self::CreateSnapshotError::*;

    debug!("snapshot state to sync");

    let mut snapshot_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(snapshot_path)
        .map_err(|e| SnapshotBackingFile("open", e))?;

    let mut snapshot = Snapshot::new(version_map, snapshot_data_version);

    snapshot
        .save(&mut snapshot_file, microvm_state)
        .map_err(SerializeMicrovmState)?;

    snapshot_file
        .flush()
        .map_err(|e| SnapshotBackingFile("flush", e))?;

    snapshot_file
        .sync_all()
        .map_err(|e| SnapshotBackingFile("sync_all", e))
}

/// Perform XOR across two memories
fn do_xor(p: &[u64], q: &Vec<u8>) {

    let slice_q = bytemuck::cast_slice::<u8,u64>(q.as_slice());
    // assert!(slice_q.len() == p.len());
    debug!("p={} q={}", p.len(), slice_q.len());
    
    for i in 0..p.len() {
       let _ = p[i] ^ slice_q[i];
    }
    debug!("xor complete!");
}

fn print_type_of<T>(_: T) {
    debug!("{}", std::any::type_name::<T>())
}

/// Synchronize snapshot memory
pub fn snapshot_memory_to_sync(
    vmm: &mut Vmm,
    url: &str,
    snapshot_type: &SnapshotType,
) -> std::result::Result<(), CreateSnapshotError> {
    use self::CreateSnapshotError::*;

    if vmm.sync_state.dirty == true {
        debug!("snapshot_memory_to_sync: dirty exists!");

        let memory_regions = vmm.guest_memory().describe();
        for region in memory_regions.regions {
            debug!("memory region -> addr:{:#X} size:{}MiB", region.base_address, region.size / (1024*1024));
            let ga = GuestAddress(region.base_address);

            //            let new_version = unsafe { Vec::from_raw_parts(region.base_address as *mut u64, region.size / 8, region.size / 8) };
            let new_version = unsafe { std::slice::from_raw_parts(region.base_address as *const u64, region.size / 8) };
            
            do_xor(new_version, &vmm.sync_state.buffer);
        }
        
    }
    else {
        debug!("snapshot_memory_to_sync: skipping, no existing copy");
    }
    
    vmm.update_sync_state();
    
//     let mut stream = TcpStream::connect(url).expect("unable to connect to remote server");
//     // let mut file = OpenOptions::new()
//     //     .write(true)
//     //     .create(true)
//     //     .truncate(true)
//     //     .open(mem_file_path)
//     //     .map_err(|e| MemoryBackingFile("open", e))?;
//     //    file.set_len((mem_size_mib * 1024 * 1024) as u64)
//   //      .map_err(|e| MemoryBackingFile("set_length", e))?;

    
//     // Set the length of the file to the full size of the memory area.
//     let mem_size_mib = mem_size_mib(vmm.guest_memory());
//     debug!("snapshot_memory_to_sync: size MiB = {}", &mem_size_mib);

//     assert!(snapshot_type == &SnapshotType::Sync);

//     

//     // seccomp needs to be set to allow us to allocate new memory
//     let buffer_memory = Vec::with_capacity((mem_size_mib * 1024 * 1024) as usize);
//     let mut buffer = Cursor::new(buffer_memory);
    

//     // send dirty pages
//     let dirty_bitmap = vmm.get_dirty_bitmap().map_err(DirtyBitmap)?;
//     vmm.guest_memory()
//         .dump_dirty(&mut buffer, &dirty_bitmap)
//         .map_err(Memory);

//     stream.write(buffer.get_ref());

// //    vmm.guest_memory().dump(&mut dump_copy).map_err(Memory)?;
//     info!("snapshot_memory_to_sync: copied memory to new region");
    
//     for region in memory_regions.regions {
//         debug!("memory region -> addr:{:#X} size:{}MiB", region.base_address, region.size / (1024*1024));
//         let ga = GuestAddress(region.base_address);
//         let raw_slice = unsafe { std::slice::from_raw_parts(region.base_address as *const u8, region.size); };
//     }


  
    
    //    debug!("memory state -> {:?}", memory_state);
    // diff
    //let dirty_bitmap = vmm.get_dirty_bitmap().map_err(DirtyBitmap)?;
    //        vmm.guest_memory()
    //            .dump_dirty(&mut file, &dirty_bitmap)
    //            .map_err(Memory)

    // full
    

    //     fn dump<T: std::io::Write>(&self, writer: &mut T) -> std::result::Result<(), Error> {
    //     self.iter()
    //         .try_for_each(|region| {
    //             region.write_all_to(MemoryRegionAddress(0), writer, region.len() as usize)
    //         })
    //         .map_err(Error::WriteMemory)
    // }


    // file.flush().map_err(|e| MemoryBackingFile("flush", e))?;
    // file.sync_all()
    //     .map_err(|e| MemoryBackingFile("sync_all", e))
    Ok(())
}
