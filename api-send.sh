#!/bin/bash
#
# see https://github.com/firecracker-microvm/firecracker/blob/main/src/api_server/swagger/firecracker.yaml
#
if [ "$1" == "info" ] ; then
    curl --unix-socket /tmp/firecracker.socket -i \
         -X GET "http://localhost/" \
         -H  "accept: application/json" \
         -H  "Content-Type: application/json"
fi


if [ "$1" == "ballon" ] ; then
    curl --unix-socket /tmp/firecracker.socket -i \
         -X GET "http://localhost/balloon" \
         -H  "accept: application/json" \
         -H  "Content-Type: application/json"
fi



if [ "$1" == "config" ] ; then
    curl --unix-socket /tmp/firecracker.socket -i \
         -X GET "http://localhost/machine-config" \
         -H  "accept: application/json" \
         -H  "Content-Type: application/json" -o tmp
    cat tmp
fi


if [ "$1" == "config" ] ; then
    curl --unix-socket /tmp/firecracker.socket -i \
         -X GET "http://localhost/machine-config" \
         -H  "accept: application/json" \
         -H  "Content-Type: application/json" -o tmp
    cat tmp
fi

function vm_pause {
        curl --unix-socket /tmp/firecracker.socket -i \
         -X PATCH "http://localhost/vm" \
         -H  "accept: application/json" \
         -H  "Content-Type: application/json" \
         -d '{
            "state": "Paused"
             }'
}

function vm_resume {
        curl --unix-socket /tmp/firecracker.socket -i \
         -X PATCH 'http://localhost/vm' \
         -H 'Accept: application/json' \
         -H 'Content-Type: application/json' \
         -d '{
            "state": "Resumed"
    }'
}

function vm_snapshot_create {
    curl --unix-socket /tmp/firecracker.socket -i \
    -X PUT 'http://localhost/snapshot/create' \
    -H  'Accept: application/json' \
    -H  'Content-Type: application/json' \
    -d '{
            "snapshot_type": "Full",
            "snapshot_path": "./vm_state.snap",
            "mem_file_path": "./memory.snap"
    }'
}

function vm_snapshot_diff_create {
    curl --unix-socket /tmp/firecracker.socket -i \
    -X PUT 'http://localhost/snapshot/create' \
    -H  'Accept: application/json' \
    -H  'Content-Type: application/json' \
    -d '{
            "snapshot_type": "Diff",
            "snapshot_path": "./vm_state.snapdiff",
            "mem_file_path": "./memory.snapdiff",
            "version": "0.23.0"
    }'
}

function vm_snapshot_sync_create {
    curl --unix-socket /tmp/firecracker.socket -i \
    -X PUT 'http://localhost/snapshot/create' \
    -H  'Accept: application/json' \
    -H  'Content-Type: application/json' \
    -d '{
            "snapshot_type": "Sync",
            "snapshot_path": "./vm_state.sync",
            "mem_file_path": "./memory.sync"
    }'
}

function vm_sync_snapshot {
    curl --unix-socket /tmp/firecracker.socket -i \
    -X PUT 'http://localhost/snapshot/sync' \
    -H  'Accept: application/json' \
    -H  'Content-Type: application/json' \
    -d '{
            "server_url": "127.0.0.1:2222"
    }'
}


function vm_snapshot_load {
    curl --unix-socket /tmp/firecracker.socket -i \
    -X PUT 'http://localhost/snapshot/load' \
    -H  'Accept: application/json' \
    -H  'Content-Type: application/json' \
    -d '{
            "snapshot_path": "./snapshot_state.snap",
            "mem_file_path": "./snapshot_mem.snap",
            "enable_diff_snapshots": true,
            "resume_vm": false
    }'
}

function vm_enable_dirty_tracking {
    curl --unix-socket /tmp/firecracker.socket -i  \
    -X PUT 'http://localhost/machine-config' \
    -H 'Accept: application/json'            \
    -H 'Content-Type: application/json'      \
    -d '{
            "vcpu_count": 2,
            "mem_size_mib": 1024,
            "ht_enabled": false,
            "track_dirty_pages": true
    }'
}

if [ "$1" == "pause" ] ; then
    vm_pause
fi

if [ "$1" == "resume" ] ; then
    vm_resume
fi

if [ "$1" == "snap" ] ; then
    vm_pause
    vm_snapshot_create
    vm_resume
fi

if [ "$1" == "snap_diff" ] ; then
    vm_pause
    vm_snapshot_diff_create
    vm_resume
fi

if [ "$1" == "sync" ] ; then
    vm_pause
    vm_snapshot_sync_create
    vm_resume
fi

if [ "$1" == "syncbatch" ] ; then
    for i in {1..120}
    do
        vm_pause
        vm_snapshot_sync_create
        vm_resume
        sleep 1
    done
fi

if [ "$1" == "syncsnapshot" ] ; then
    vm_sync_snapshot
fi
    

# rm /tmp/firecracker.socket ; ./firecracker/firecracker --api-sock /tmp/firecracker.socket
if [ "$1" == "loadsnap" ] ; then
    vm_snapshot_load
    vm_resume
fi
