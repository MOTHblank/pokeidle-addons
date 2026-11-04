//! Bounded, process-wide executors for Kick's blocking channel and HLS work.
//! A task occupies a pool thread only while it is actually running. Stopped
//! queued tasks are discarded, and idle workers retire without blocking the UI.
//! The two pools retain enough capacity for four accounts with 12 streams each.
use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const MAX_CHANNEL_WORKERS: usize = 48;
const MAX_HLS_WORKERS: usize = 48;
const MAX_PENDING: usize = 96;
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const WORKER_STACK: usize = 768 * 1024;

type Work = Box<dyn FnOnce() + Send + 'static>;

struct Task {
    stop: Arc<AtomicBool>,
    work: Work,
}
#[derive(Default)]
struct Queue {
    pending: VecDeque<Task>,
    workers: usize,
    idle: usize,
}
#[derive(Default)]
struct Shared {
    queue: Mutex<Queue>,
    available: Condvar,
}
struct Pool {
    shared: Arc<Shared>,
    limit: usize,
    name: &'static str,
}

/// Read only when the Diagnostics page explicitly requests a snapshot.
/// This does not initialize either pool or install ongoing counters.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PoolUsage {
    pub(crate) workers: usize,
    pub(crate) idle: usize,
    pub(crate) pending: usize,
}
impl Pool {
    fn usage(&self) -> Option<PoolUsage> {
        self.shared.queue.lock().ok().map(|queue| PoolUsage {
            workers: queue.workers, idle: queue.idle, pending: queue.pending.len(),
        })
    }
}
impl Pool {
    fn new(name: &'static str, limit: usize) -> Self {
        Self { shared: Arc::new(Shared::default()), limit, name }
    }
    fn discard_canceled(&self) {
        if let Ok(mut queue) = self.shared.queue.lock() {
            queue.pending.retain(|task| !task.stop.load(Ordering::Acquire));
        }
    }
    fn submit(&self, stop: Arc<AtomicBool>, work: Work) -> Result<(), String> {
        if stop.load(Ordering::Acquire) { return Ok(()); }
        let mut queue = self.shared.queue.lock()
            .map_err(|_| "Kick worker queue unavailable".to_owned())?;
        // Repeated stop/reconnect cycles must not accumulate stale sessions
        // or retain their credentials in the pending queue.
        queue.pending.retain(|task| !task.stop.load(Ordering::Acquire));
        if queue.pending.len() >= MAX_PENDING {
            return Err("Kick workers are busy; pending queue is full".into());
        }
        queue.pending.push_back(Task { stop, work });
        if queue.idle > 0 {
            self.shared.available.notify_one();
        }
        // One idle worker can handle one queued job, not an entire burst.
        // Spawn for any backlog beyond the workers that are already idle.
        if queue.workers < self.limit && queue.pending.len() > queue.idle {
            queue.workers += 1;
            let shared = Arc::clone(&self.shared);
            match thread::Builder::new().name(self.name.into())
                .stack_size(WORKER_STACK)
                .spawn(move || worker_loop(shared))
            {
                Ok(handle) => retire(handle),
                Err(_) => {
                    queue.workers -= 1;
                    if queue.workers == 0 {
                        queue.pending.pop_back();
                        return Err("Could not start a Kick pooled worker".into());
                    }
                    // An existing worker will drain the queue on completion.
                }
            }
        }
        Ok(())
    }
}
fn worker_loop(shared: Arc<Shared>) {
    loop {
        let task = {
            let Ok(mut queue) = shared.queue.lock() else { return };
            loop {
                if let Some(task) = queue.pending.pop_front() { break task; }
                queue.idle += 1;
                let (next, result) = match shared.available.wait_timeout(queue, IDLE_TIMEOUT) {
                    Ok(value) => value,
                    Err(poisoned) => poisoned.into_inner(),
                };
                queue = next;
                queue.idle -= 1;
                if result.timed_out() && queue.pending.is_empty() {
                    queue.workers -= 1;
                    return;
                }
            }
        };
        if !task.stop.load(Ordering::Acquire) {
            // A single failed channel must never permanently consume a slot.
            let _ = catch_unwind(AssertUnwindSafe(task.work));
        }
    }
}

/// Join completed threads on one process-wide reaper, never on a UI frame.
/// In-progress tasks remain cancelable; their pool slots free on return.
pub(crate) fn retire(handle: JoinHandle<()>) {
    static REAPER: OnceLock<Option<mpsc::Sender<JoinHandle<()>>>> = OnceLock::new();
    let sender = REAPER.get_or_init(|| {
        let (sender, receiver) = mpsc::channel::<JoinHandle<()>>();
        thread::Builder::new().name("moth-kick-reaper".into())
            .stack_size(256 * 1024)
            .spawn(move || {
                let mut handles: Vec<JoinHandle<()>> = Vec::new();
                loop {
                    match receiver.recv_timeout(Duration::from_millis(250)) {
                        Ok(handle) => handles.push(handle),
                        Err(mpsc::RecvTimeoutError::Timeout) => {},
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                    while let Ok(handle) = receiver.try_recv() {
                        handles.push(handle);
                    }
                    let mut index = 0;
                    while index < handles.len() {
                        if handles[index].is_finished() {
                            let _ = handles.swap_remove(index).join();
                        } else {
                            index += 1;
                        }
                    }
                }
            }).ok().map(|_| sender)
    });
    if let Some(sender) = sender {
        // A failed send only detaches a handle; it never waits for the thread.
        let _ = sender.send(handle);
    }
}

static CHANNEL_POOL: OnceLock<Pool> = OnceLock::new();
static HLS_POOL: OnceLock<Pool> = OnceLock::new();

fn channel_pool() -> &'static Pool {
    CHANNEL_POOL.get_or_init(|| Pool::new("moth-kick-channel", MAX_CHANNEL_WORKERS))
}
fn hls_pool() -> &'static Pool {
    HLS_POOL.get_or_init(|| Pool::new("moth-kick-hls", MAX_HLS_WORKERS))
}

/// No new pools/threads are created by opening diagnostics.
pub(crate) fn diagnostics_usage() -> (Option<PoolUsage>, Option<PoolUsage>) {
    (CHANNEL_POOL.get().and_then(Pool::usage), HLS_POOL.get().and_then(Pool::usage))
}
pub(crate) fn submit_channel(stop: Arc<AtomicBool>,
                             work: impl FnOnce() + Send + 'static) -> Result<(), String> {
    channel_pool().submit(stop, Box::new(work))
}
pub(crate) fn submit_hls(stop: Arc<AtomicBool>,
                         work: impl FnOnce() + Send + 'static) -> Result<(), String> {
    hls_pool().submit(stop, Box::new(work))
}
pub(crate) fn discard_canceled_channels() { channel_pool().discard_canceled(); }
pub(crate) fn discard_canceled_hls() { hls_pool().discard_canceled(); }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_work_is_bounded_and_canceled_work_is_not_executed() {
        let pool = Pool::new("moth-kick-pool-test", 1);
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        pool.submit(Arc::new(AtomicBool::new(false)), Box::new(move || {
            let _ = started_tx.send(());
            let _ = release_rx.recv();
        })).unwrap();
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();

        let stale = Arc::new(AtomicBool::new(false));
        let (stale_tx, stale_rx) = mpsc::channel();
        for _ in 0..MAX_PENDING {
            let stale_tx = stale_tx.clone();
            pool.submit(Arc::clone(&stale), Box::new(move || {
                let _ = stale_tx.send(());
            })).unwrap();
        }
        assert!(pool.submit(Arc::new(AtomicBool::new(false)),
            Box::new(|| {})).is_err(), "all queued slots must be bounded");
        stale.store(true, Ordering::Release);
        let (fresh_tx, fresh_rx) = mpsc::channel();
        pool.submit(Arc::new(AtomicBool::new(false)), Box::new(move || {
            let _ = fresh_tx.send(());
        })).unwrap();
        release_tx.send(()).unwrap();
        fresh_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(stale_rx.try_recv().is_err());
    }

    #[test]
    fn active_channel_jobs_run_in_parallel() {
        let pool = Pool::new("moth-kick-pool-parallel-test", 2);
        let (started_tx, started_rx) = mpsc::channel();
        let release = Arc::new(AtomicBool::new(false));
        for _ in 0..2 {
            let started_tx = started_tx.clone();
            let release = Arc::clone(&release);
            pool.submit(Arc::new(AtomicBool::new(false)), Box::new(move || {
                let _ = started_tx.send(());
                while !release.load(Ordering::Acquire) {
                    thread::sleep(Duration::from_millis(5));
                }
            })).unwrap();
        }
        for _ in 0..2 {
            started_rx.recv_timeout(Duration::from_secs(2))
                .expect("pool must run both blocking jobs concurrently");
        }
        release.store(true, Ordering::Release);
    }
}
