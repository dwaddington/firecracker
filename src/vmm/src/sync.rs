// Copyright 2022 IBM Corporation. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Support for snapshot synchronization
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use crate::memory_snapshot::SnapshotMemory;
use crate::vmm_config::snapshot::SnapshotType;
use snapshot::Snapshot;
use versionize::VersionMap;

use std::path::Path;
use crate::{MicrovmState, mem_size_mib, Vmm};
use crate::persist::{CreateSnapshotError}; //, MemoryBackingFile};

use logger::{debug};

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

/// Synchronize snapshot memory
pub fn snapshot_memory_to_sync(
    vmm: &Vmm,
    mem_file_path: &Path,
    snapshot_type: &SnapshotType,
) -> std::result::Result<(), CreateSnapshotError> {
    use self::CreateSnapshotError::*;

    debug!("snapshot memory to sync");

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(mem_file_path)
        .map_err(|e| MemoryBackingFile("open", e))?;

    // Set the length of the file to the full size of the memory area.
    let mem_size_mib = mem_size_mib(vmm.guest_memory());
    file.set_len((mem_size_mib * 1024 * 1024) as u64)
        .map_err(|e| MemoryBackingFile("set_length", e))?;

    match snapshot_type {
        SnapshotType::Diff => {
            let dirty_bitmap = vmm.get_dirty_bitmap().map_err(DirtyBitmap)?;
            vmm.guest_memory()
                .dump_dirty(&mut file, &dirty_bitmap)
                .map_err(Memory)
        }
        SnapshotType::Full => vmm.guest_memory().dump(&mut file).map_err(Memory),
        SnapshotType::Sync => return Ok(()),
    }?;
    file.flush().map_err(|e| MemoryBackingFile("flush", e))?;
    file.sync_all()
        .map_err(|e| MemoryBackingFile("sync_all", e))
}

