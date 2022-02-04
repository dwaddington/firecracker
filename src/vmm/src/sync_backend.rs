use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;

use logger::{debug, info};
use std::{thread, time};

/// Remote synchronization state
pub struct SyncState {
    dirty: bool,
    /// Buffer holding prior memory snapshot
    pub buffer: Vec<u8>,
    thread: Option<JoinHandle<()>>,
    exit_thread: Arc<AtomicBool>,
    tx_channel: Sender<Arc<SyncWork>>,
}

/// Task unit for worker thread
pub struct SyncWork {
    /// Work buffer
    pub buffer: Vec<u8>,
}

static INITIAL_SNAPSHOT_BUFFER_SIZE: usize = 8 * usize::pow(1024, 3); // 8GiB

fn thread_entry(exit_bool: Arc<AtomicBool>, rx: Receiver<Arc<SyncWork>>) {
    while exit_bool.load(Ordering::SeqCst) == false {
        info!("worker thread!!");
        thread::sleep(time::Duration::from_secs(1));
        info!("receiving on channel");

        match rx.recv() {
            Ok(data) => {
                info!("recevied work!!!! {}", data.buffer.len());
            }
            Err(err) => debug!("rx not OK {}", err),
        };
    }
}

impl SyncState {
    /// Instantiate new sync state
    pub fn new() -> SyncState {
        let (tx, rx): (Sender<Arc<SyncWork>>, Receiver<Arc<SyncWork>>) = mpsc::channel();
        let base = Arc::new(AtomicBool::new(false));
        let thread_ref = base.clone();
        SyncState {
            dirty: false,
            buffer: vec![0; INITIAL_SNAPSHOT_BUFFER_SIZE],
            thread: Some(thread::spawn(move || {
                thread_entry(thread_ref, rx);
            })),
            exit_thread: base.clone(),
            tx_channel: tx.clone(),
        }
    }

    /// Send work to the worker thread
    pub fn send_work(&self, work: SyncWork) {
        self.tx_channel.send(Arc::new(work)).expect("channel send");
    }

    /// Return true if dirty
    pub fn is_copied(&self) -> bool {
        let r = self.dirty.clone();
        return r;
    }

    /// Set dirty bit
    pub fn set_dirty(&mut self) {
        self.dirty = true;
    }

    /// Shutdown worker thread
    pub fn shutdown_worker(&mut self) {
        debug!("shutting down worker");
        // signal exit
        self.exit_thread.store(true, Ordering::SeqCst);
        // unblock rx on channel
        self.send_work(SyncWork { buffer: vec![0] });
        self.thread
            .take()
            .expect("call on non running")
            .join()
            .expect("join failed");
        debug!("worker shut down OK!!!!")
    }
}
