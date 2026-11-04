//! One GUI controller per desktop session (Windows) / data root (Linux).
//!
//! Acquire this *before* TLS, updater, profiles and game workers are created.
//! A second process only signals the original and exits. The blocking OS
//! listener sleeps in the kernel while idle; there is no timer or UI polling.
use eframe::egui;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};

#[derive(Default)]
pub struct RestoreSignal {
    pending: AtomicBool,
    context: Mutex<Option<egui::Context>>,
}

impl RestoreSignal {
    pub fn attach_context(&self, context: &egui::Context) {
        if let Ok(mut slot) = self.context.lock() {
            *slot = Some(context.clone());
        }
        if self.pending.load(Ordering::Acquire) {
            context.request_repaint();
        }
    }

    fn request(&self) {
        // Keep pending requests even if the native window is not ready yet.
        self.pending.store(true, Ordering::Release);
        if let Ok(context) = self.context.lock() {
            if let Some(context) = context.as_ref() {
                context.request_repaint();
            }
        }
    }

    pub fn take(&self) -> bool {
        self.pending.swap(false, Ordering::AcqRel)
    }
}

pub enum Launch {
    Primary(SingleInstance),
    Existing,
}

/// Retain for the *entire* application and its shutdown cleanup. Releasing
/// the guard early would allow another instance to launch during shutdown.
pub struct SingleInstance {
    signal: Arc<RestoreSignal>,
    #[cfg(windows)]
    _platform: windows::Guard,
    #[cfg(unix)]
    _platform: unix::Guard,
}

impl SingleInstance {
    pub fn signal(&self) -> Arc<RestoreSignal> {
        Arc::clone(&self.signal)
    }
}

/// Startup failures must be visible on console-less Windows builds. Never
/// silently fall through into a second set of game and browser workers.
pub fn report_acquire_error(error: &str) {
    eprintln!("Moth Controller could not enforce single-instance mode: {error}");
    #[cfg(windows)]
    windows::show_error(error);
}

pub fn acquire() -> Result<Launch, String> {
    let signal = Arc::new(RestoreSignal::default());
    #[cfg(windows)]
    {
        return match windows::acquire(Arc::clone(&signal))? {
            Some(guard) => Ok(Launch::Primary(SingleInstance {
                signal, _platform: guard
            })),
            None => Ok(Launch::Existing),
        };
    }
    #[cfg(unix)]
    {
        return match unix::acquire(Arc::clone(&signal))? {
            Some(guard) => Ok(Launch::Primary(SingleInstance {
                signal, _platform: guard
            })),
            None => Ok(Launch::Existing),
        };
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = signal;
        Err("single-instance control is unsupported on this platform".into())
    }
}

#[cfg(windows)]
mod windows {
    use super::RestoreSignal;
    use std::ffi::c_void;
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    type Handle = *mut c_void;
    const ERROR_ALREADY_EXISTS: u32 = 183;
    const EVENT_MODIFY_STATE: u32 = 0x0002;
    const WAIT_OBJECT_0: u32 = 0;
    const INFINITE: u32 = 0xFFFF_FFFF;
    const MUTEX_NAME: &str = "Local\\MothController-Instance-v1";
    const EVENT_NAME: &str = "Local\\MothController-Restore-v1";
    const WINDOW_TITLE: &str = "Moth · PokéIdle";
    const SW_RESTORE: i32 = 9;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateMutexW(attributes: Handle, initial_owner: i32, name: *const u16) -> Handle;
        fn CreateEventW(attributes: Handle, manual_reset: i32, initial_state: i32,
            name: *const u16) -> Handle;
        fn OpenEventW(desired_access: u32, inherit_handle: i32, name: *const u16) -> Handle;
        fn SetEvent(handle: Handle) -> i32;
        fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
        fn GetLastError() -> u32;
        fn CloseHandle(handle: Handle) -> i32;
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> Handle;
        fn ShowWindow(window: Handle, command: i32) -> i32;
        fn SetForegroundWindow(window: Handle) -> i32;
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(window: Handle, message: *const u16, caption: *const u16,
            options: u32) -> i32;
    }

    pub fn show_error(error: &str) {
        let message = wide(&format!("Moth Controller could not open:\n{error}"));
        let caption = wide("Moth Controller");
        unsafe {
            // MB_OK | MB_ICONERROR
            MessageBoxW(std::ptr::null_mut(), message.as_ptr(), caption.as_ptr(), 0x10);
        }
    }

    pub struct Guard {
        mutex: usize,
        event: usize,
        stop: Arc<AtomicBool>,
        worker: Option<JoinHandle<()>>,
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            // Wake the kernel wait so the listener can exit promptly.
            unsafe { SetEvent(self.event as Handle); }
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
            unsafe {
                CloseHandle(self.event as Handle);
                CloseHandle(self.mutex as Handle);
            }
        }
    }

    pub fn acquire(signal: Arc<RestoreSignal>) -> Result<Option<Guard>, String> {
        let mutex_name = wide(MUTEX_NAME);
        let mutex = unsafe {
            CreateMutexW(std::ptr::null_mut(), 0, mutex_name.as_ptr())
        };
        if mutex.is_null() {
            return Err(format!("could not create controller mutex: {}",
                std::io::Error::last_os_error()));
        }
        let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        if already_exists {
            unsafe { CloseHandle(mutex); }
            notify_existing()?;
            return Ok(None);
        }

        let event_name = wide(EVENT_NAME);
        let event = unsafe {
            CreateEventW(std::ptr::null_mut(), 0, 0, event_name.as_ptr())
        };
        if event.is_null() {
            unsafe { CloseHandle(mutex); }
            return Err(format!("could not create restore event: {}",
                std::io::Error::last_os_error()));
        }
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        // Copy the raw handle as an integer: it remains owned by Guard, and
        // is closed only after the worker has joined.
        let event_id = event as usize;
        let worker = match thread::Builder::new()
            .name("moth-single-instance-listener".into())
            .spawn(move || loop {
                let result = unsafe { WaitForSingleObject(event_id as Handle, INFINITE) };
                if result != WAIT_OBJECT_0 || worker_stop.load(Ordering::Acquire) {
                    break;
                }
                signal.request();
            }) {
            Ok(worker) => worker,
            Err(error) => {
                unsafe {
                    CloseHandle(event);
                    CloseHandle(mutex);
                }
                return Err(format!("could not start restore listener: {error}"));
            }
        };
        Ok(Some(Guard {
            mutex: mutex as usize, event: event_id, stop, worker: Some(worker)
        }))
    }

    fn notify_existing() -> Result<(), String> {
        let name = wide(EVENT_NAME);
        // The primary might still be creating its event immediately after
        // creating the mutex. Never start a second GUI during that race.
        for _ in 0..40 {
            let event = unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, name.as_ptr()) };
            if !event.is_null() {
                let ok = unsafe { SetEvent(event) };
                unsafe { CloseHandle(event); }
                if ok == 0 {
                    return Err(format!("could not signal existing controller: {}",
                        std::io::Error::last_os_error()));
                }
                // A second launch originated from a user desktop action, so
                // also ask Windows directly to restore/foreground the window.
                // If the GUI is still initializing, the named event handles it.
                let title = wide(WINDOW_TITLE);
                let hwnd = unsafe {
                    FindWindowW(std::ptr::null(), title.as_ptr())
                };
                if !hwnd.is_null() {
                    unsafe {
                        ShowWindow(hwnd, SW_RESTORE);
                        SetForegroundWindow(hwnd);
                    }
                }
                return Ok(());
            }
            thread::sleep(Duration::from_millis(50));
        }
        Err("Another controller is running, but its window could not be signaled".into())
    }
}

#[cfg(unix)]
mod unix {
    use super::RestoreSignal;
    use std::fs::{self, File, OpenOptions};
    use std::io::{self, Read, Write};
    use std::os::fd::AsRawFd;
    use std::os::raw::c_int;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    const LOCK_EX: c_int = 2;
    const LOCK_NB: c_int = 4;
    const RESTORE: &[u8] = b"MOTH-RESTORE-v1\n";
    const STOP: &[u8] = b"MOTH-STOP----v1\n";
    const ACK: &[u8] = b"OK\n";

    unsafe extern "C" {
        fn flock(fd: c_int, operation: c_int) -> c_int;
    }

    pub struct Guard {
        // Keep the open file description locked for this entire lifetime.
        _lock: File,
        socket: PathBuf,
        stop: Arc<AtomicBool>,
        worker: Option<JoinHandle<()>>,
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            // Unblock accept() without a periodic wakeup/timer.
            if let Ok(mut stream) = UnixStream::connect(&self.socket) {
                let _ = stream.write_all(STOP);
            }
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
            let _ = fs::remove_file(&self.socket);
        }
    }

    pub fn acquire(signal: Arc<RestoreSignal>) -> Result<Option<Guard>, String> {
        let data_root = crate::config::data_root()?;
        acquire_in(&data_root, signal)
    }

    fn acquire_in(root: &Path, signal: Arc<RestoreSignal>) -> Result<Option<Guard>, String> {
        fs::create_dir_all(root)
            .map_err(|error| format!("could not create single-instance directory: {error}"))?;
        let lock_path = root.join("moth-controller-instance.lock");
        let socket = root.join("moth-controller-instance.sock");
        let lock = OpenOptions::new().read(true).write(true).create(true)
            .open(&lock_path).map_err(|error|
                format!("could not open controller instance lock: {error}"))?;
        if unsafe { flock(lock.as_raw_fd(), LOCK_EX | LOCK_NB) } != 0 {
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::WouldBlock {
                return Err(format!("could not claim controller instance lock: {error}"));
            }
            // Do not take the lock or start workers in the second process.
            notify_existing(&socket)?;
            return Ok(None);
        }

        // Only the lock owner may remove a stale socket left by an OS crash.
        match fs::remove_file(&socket) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("could not clear old instance socket: {error}")),
        }
        let listener = UnixListener::bind(&socket)
            .map_err(|error| format!("could not create controller restore socket: {error}"))?;
        if let Err(error) = fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)) {
            let _ = fs::remove_file(&socket);
            return Err(format!("could not protect controller restore socket: {error}"));
        }

        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker = match thread::Builder::new()
            .name("moth-single-instance-listener".into())
            .spawn(move || {
                while !worker_stop.load(Ordering::Acquire) {
                    let Ok((mut stream, _)) = listener.accept() else { break };
                    let _ = stream.set_read_timeout(Some(Duration::from_millis(750)));
                    let mut input = [0u8; RESTORE.len()];
                    if stream.read_exact(&mut input).is_ok() && input.as_slice() == RESTORE {
                        if !worker_stop.load(Ordering::Acquire) {
                            signal.request();
                            let _ = stream.write_all(ACK);
                        }
                    }
                }
            }) {
            Ok(worker) => worker,
            Err(error) => {
                let _ = fs::remove_file(&socket);
                return Err(format!("could not start controller restore listener: {error}"));
            }
        };
        Ok(Some(Guard {
            _lock: lock, socket, stop, worker: Some(worker)
        }))
    }

    fn notify_existing(socket: &Path) -> Result<(), String> {
        // A second launch can win the race after flock but before bind().
        // Retry the signal, never a new controller process.
        for _ in 0..40 {
            if let Ok(mut stream) = UnixStream::connect(socket) {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                if stream.write_all(RESTORE).is_ok() {
                    let mut ack = [0u8; ACK.len()];
                    if stream.read_exact(&mut ack).is_ok() && ack.as_slice() == ACK {
                        return Ok(());
                    }
                }
            }
            thread::sleep(Duration::from_millis(50));
        }
        Err("Another controller is running, but its window could not be signaled".into())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::sync::atomic::{AtomicU64, Ordering};

        static SEQ: AtomicU64 = AtomicU64::new(0);

        fn test_root() -> PathBuf {
            let directory = std::env::temp_dir().join(format!(
                "moth-single-instance-{}-{}", std::process::id(),
                SEQ.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir_all(&directory).unwrap();
            directory
        }

        #[test]
        fn a_second_instance_signals_primary_without_creating_a_listener() {
            let root = test_root();
            let signal = Arc::new(RestoreSignal::default());
            let first = acquire_in(&root, Arc::clone(&signal)).unwrap().unwrap();
            assert!(!signal.take());
            assert!(acquire_in(&root, Arc::new(RestoreSignal::default()))
                .unwrap().is_none());
            assert!(signal.take());
            assert!(!signal.take());
            drop(first);
            assert!(!root.join("moth-controller-instance.sock").exists());
            let third = acquire_in(&root, Arc::new(RestoreSignal::default())).unwrap();
            assert!(third.is_some());
            drop(third);
            fs::remove_dir_all(root).unwrap();
        }

        #[test]
        fn stale_socket_is_replaced_only_by_lock_owner() {
            let root = test_root();
            let socket = root.join("moth-controller-instance.sock");
            fs::write(&socket, b"stale").unwrap();
            let guard = acquire_in(&root, Arc::new(RestoreSignal::default()))
                .unwrap().unwrap();
            assert!(socket.exists());
            drop(guard);
            assert!(!socket.exists());
            fs::remove_dir_all(root).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restore_requests_remain_pending_until_consumed() {
        let signal = RestoreSignal::default();
        signal.request();
        signal.request();
        assert!(signal.take());
        assert!(!signal.take());
    }
}
