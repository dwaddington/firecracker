/// Synchronization engine backend
/// IBM Corp. (c) 2022
///
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use logger::{debug, info};
use std::thread;

static INITIAL_SNAPSHOT_BUFFER_SIZE: usize = 8 * usize::pow(1024, 3); // 8GiB
static CACHE_LINE_SIZE: usize = 64;

type SignalType = u64;

/// Remote synchronization state
pub struct SyncState {
    dirty: bool,
    /// Buffer holding prior memory snapshot
    pub prior_buffer: Vec<u8>,
    thread: Option<JoinHandle<()>>,
    exit_thread: Arc<AtomicBool>,
    tx_channel: Sender<SignalType>,
    /// Buffer for XOR data
    pub xor_data: Arc<Mutex<Vec<u64>>>,
}

/// Synchronization worker thread entry point
fn thread_entry(
    exit_bool: Arc<AtomicBool>,
    rx: Receiver<SignalType>,
    xor_buffer: Arc<Mutex<Vec<u64>>>,
) {
    while !exit_bool.load(Ordering::SeqCst) {
        info!("worker thread receiving on channel......");

        match rx.recv() {
            Ok(data) => {
                let mut xor_memory = xor_buffer.lock().expect("Poison mutex");
                assert!(xor_memory.len() == 0 || xor_memory.len() % (CACHE_LINE_SIZE / 8) == 0); // check whole cache lines
                debug!(
                    "rx recv code: {} with xor buffer {}",
                    data,
                    xor_memory.len()
                );
                info!(
                    "Synchronization worker thread received XOR memory {} bytes",
                    xor_memory.len()
                );
                // now empty it!
                xor_memory.clear();
            }
            Err(err) => debug!("rx not OK {}", err),
        };
    }
    debug!("Synchronization worker exiting.");
}

impl SyncState {
    /// Instantiate new sync state
    pub fn new() -> SyncState {
        let (tx, rx): (Sender<SignalType>, Receiver<SignalType>) = mpsc::channel();
        let base = Arc::new(AtomicBool::new(false));
        let thread_ref = base.clone();
        let xorbuffer = Arc::new(Mutex::<Vec<u64>>::new(vec![]));
        let xd = xorbuffer.clone();
        SyncState {
            dirty: false,
            prior_buffer: vec![0; INITIAL_SNAPSHOT_BUFFER_SIZE],
            thread: Some(thread::spawn(move || {
                thread_entry(thread_ref, rx, xorbuffer);
            })),
            exit_thread: base,
            tx_channel: tx,
            xor_data: xd,
        }
    }

    /// Send work to the worker thread
    pub fn signal_work(&self, v: u64) {
        self.tx_channel.send(v).expect("tx_channel.send failed");
    }

    /// Return true if dirty
    pub fn is_copied(&self) -> bool {
        self.dirty
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
        self.signal_work(0);
        self.thread
            .take()
            .expect("call on non running")
            .join()
            .expect("join failed");
        debug!("worker shut down OK!!!!")
    }
}

impl Default for SyncState {
    fn default() -> SyncState {
        SyncState::new()
    }
}
