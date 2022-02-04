// Copyright 2022 IBM Corporation. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Front-end synchronization
use std::fs::OpenOptions;
use std::io::Result;
use std::io::Write;
use std::time::Instant;

/// Support for snapshot synchronization
use crate::memory_snapshot;
use crate::memory_snapshot::SnapshotMemory;
use crate::persist::CreateSnapshotError;
use crate::sync_backend::SyncWork;
use crate::vmm_config::snapshot::SyncSnapshotParams;
use crate::{MicrovmState, Vmm};

use snapshot::Snapshot;
use versionize::VersionMap;
use vm_memory::{Bitmap, Bytes, GuestMemory, GuestMemoryRegion, MemoryRegionAddress};

use std::path::Path; //, MemoryBackingFile};
use utils::get_page_size;

use bytemuck;
use logger::debug;

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
fn do_xor(p: &[u64], p_base: usize, q: &Vec<u8>) {
    let slice_q = bytemuck::cast_slice::<u8, u64>(q.as_slice());
    assert!(slice_q.len() >= p.len());

    debug!("p base :{:#X}", p_base);
    let scaled_base = p_base / 8;

    debug!("slice_q len   :{:#X}", slice_q.len());
    debug!("p scaled base :{:#X}", scaled_base);
    debug!("p len         :{:#X}", p.len());

    let time_start = Instant::now();
    for i in 0..p.len() {
        let _ = p[i] ^ slice_q[scaled_base + i];
    }
    debug!(
        "Complete memory: XOR time={}ms",
        time_start.elapsed().as_millis()
    );
}

#[allow(dead_code)]
fn print_type_of<T>(_: T) {
    debug!("{}", std::any::type_name::<T>())
}

#[allow(dead_code)]
/// Perform a full memory snapshot
fn full_memory_snapshot(vmm: &mut Vmm) -> std::result::Result<(), CreateSnapshotError> {
    if vmm.sync_engine.is_copied() {
        debug!("snapshot_memory_to_sync: dirty exists!");

        let memory_regions = vmm.guest_memory().describe();

        for region in memory_regions.regions {
            debug!(
                "memory region -> offset:{:#X} size:{}MiB",
                region.offset,
                region.size / (1024 * 1024)
            );

            let new_version =
                unsafe { std::slice::from_raw_parts(region.offset as *const u64, region.size / 8) };

            do_xor(new_version, region.offset as usize, &vmm.sync_engine.buffer);
        }
    } else {
        debug!("snapshot_memory_to_sync: skipping, no existing copy");
    }

    vmm.copy_all_guest_memory();
    Ok(())
}

struct RegionProcessor<'a> {
    prior_full_snapshot: &'a mut Vec<u8>,
    offset: usize,
}

impl<'a> RegionProcessor<'a> {
    fn new(full_memory_snapshot: &'a mut Vec<u8>) -> RegionProcessor<'a> {
        RegionProcessor {
            prior_full_snapshot: full_memory_snapshot,
            offset: 0,
        }
    }
}

impl<'w> std::io::Write for RegionProcessor<'w> {
    /// Write applied to dirty pages (in batch)
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let offset64 = self.offset / 8;
        let full_prior =
            bytemuck::cast_slice_mut::<u8, u64>(self.prior_full_snapshot.as_mut_slice());
        let new_slice = bytemuck::cast_slice::<u8, u64>(buf);
        let prior_slice = &mut full_prior[offset64..offset64 + new_slice.len()];

        // 64-bit iterator
        for i in 0..new_slice.len() {
            // pretend to xor
            let _ = new_slice[i] ^ prior_slice[i];
            // do update
            prior_slice[i] = new_slice[i];
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Perform a dirty-page based snapshot
fn dirtypage_memory_snapshot(vmm: &mut Vmm) -> std::result::Result<(), memory_snapshot::Error> {
    // we need a full base copy to start with
    if vmm.sync_engine.is_copied() == false {
        let time_start = Instant::now();
        vmm.copy_all_guest_memory();
        debug!(
            "Completed full copy: time={}ms",
            time_start.elapsed().as_millis()
        );
        return Ok(());
    }

    {
        let dirty_bitmap = vmm.get_dirty_bitmap().expect("get dirty bitmap failed");

        let page_size = get_page_size().map_err(memory_snapshot::Error::PageSize)?;

        let time_start = Instant::now();
        let mut page_count: usize = 0;

        // we need to make sure we have a full prior copy of memory
        if vmm.sync_engine.is_copied() == true {
            let mut writer = RegionProcessor::new(&mut vmm.sync_engine.buffer);
            vmm.guest_memory
                .iter()
                .enumerate()
                .try_for_each(|(slot, region)| {
                    //                    debug!("XXX: slot={} region.size={:?}", slot, region.size());
                    let kvm_bitmap = dirty_bitmap.get(&slot).unwrap();
                    let mut dirty_batch_start: u64 = 0;
                    let mut write_size = 0;

                    for (i, v) in kvm_bitmap.iter().enumerate() {
                        for j in 0..64 {
                            let is_kvm_page_dirty = ((v >> j) & 1u64) != 0u64;
                            let page_offset = ((i * 64) + j) * page_size;
                            let is_firecracker_page_dirty = region.bitmap().dirty_at(page_offset);
                            if is_kvm_page_dirty || is_firecracker_page_dirty {
                                // We are at the start of a new batch of dirty pages.
                                if write_size == 0 {
                                    dirty_batch_start = page_offset as u64;
                                    //debug!("XXX: dirty page {}", dirty_batch_start);
                                }
                                write_size += page_size;
                            } else if write_size > 0 {
                                writer.offset = dirty_batch_start as usize;
                                // We are at the end of a batch of dirty pages.
                                region
                                    .write_all_to(
                                        MemoryRegionAddress(dirty_batch_start),
                                        &mut writer,
                                        write_size,
                                    )
                                    .expect("write_all_to region failed");

                                page_count += write_size / page_size;

                                write_size = 0;
                            }
                        }
                    }

                    Ok(())
                })?;
        }

        debug!(
            "Completed memory XORs and update-copy on dirty pages: time={}ms page-count={}",
            time_start.elapsed().as_millis(),
            page_count
        );
    }

    Ok(())
}

/// Synchronize snapshot memory
pub fn sync_snapshot_memory(
    vmm: &mut Vmm,
    params: &SyncSnapshotParams,
) -> std::result::Result<(), CreateSnapshotError> {
    //    full_memory_snapshot(vmm);
    dirtypage_memory_snapshot(vmm).expect("dirtypage memory snapshot failed");
    vmm.sync_engine.send_work(SyncWork {
        buffer: vec![1, 2, 3],
    });

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
