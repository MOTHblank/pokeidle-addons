//! Optional, on-demand CPU inspection of controller threads.
//!
//! Invoked only from the focused Diagnostics page, when its detailed thread
//! inspection checkbox is enabled and the 15-second interval is due. There is
//! no background thread, persistent OS handle, or global sampling counter.
#[derive(Clone, Debug)]
pub(crate) struct ThreadCounter {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) cpu_ns: u128,
}

pub(crate) struct ThreadSnapshot {
    pub(crate) count: u32,
    pub(crate) threads: Vec<ThreadCounter>,
}

/// Limit untrusted OS thread descriptions before placing them in the UI.
fn display_name(name: &str, id: u64) -> String {
    let name = name.trim();
    if name.is_empty() {
        return format!("Thread {id}");
    }
    name.chars()
        .filter(|ch| !ch.is_control())
        .take(48)
        .collect()
}

#[cfg(target_os = "linux")]
pub(crate) fn inspect() -> Option<ThreadSnapshot> {
    use std::fs;
    use std::ffi::{c_int, c_long};
    use std::time::Duration;

    unsafe extern "C" { fn sysconf(name: c_int) -> c_long; }
    const SC_CLK_TCK: c_int = 2;
    let hz = unsafe { sysconf(SC_CLK_TCK) };
    if hz <= 0 { return None; }
    let mut threads = Vec::new();
    let mut count = 0u32;
    for entry in fs::read_dir("/proc/self/task").ok()?.flatten() {
        let Some(tid) = entry.file_name().to_str().and_then(|id| id.parse::<u64>().ok())
        else { continue; };
        count = count.saturating_add(1);
        let Ok(stat) = fs::read_to_string(entry.path().join("stat")) else { continue; };
        // The comm field can contain whitespace and parentheses. The final
        // ") " precedes state (field 3); utime is field 14.
        let Some((comm_prefix, fields)) = stat.rsplit_once(") ") else { continue; };
        let name = comm_prefix.split_once('(').map(|(_, name)| name).unwrap_or("");
        let mut fields = fields.split_whitespace();
        let Some(user_ticks) = fields.nth(11).and_then(|v| v.parse::<u128>().ok()) else {
            continue;
        };
        let Some(system_ticks) = fields.next().and_then(|v| v.parse::<u128>().ok()) else {
            continue;
        };
        let cpu_ns = user_ticks.saturating_add(system_ticks)
            .saturating_mul(Duration::from_secs(1).as_nanos()) / hz as u128;
        threads.push(ThreadCounter { id: tid, name: display_name(name, tid), cpu_ns });
    }
    Some(ThreadSnapshot { count, threads })
}

#[cfg(windows)]
pub(crate) fn inspect() -> Option<ThreadSnapshot> {
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::ptr;

    const TH32CS_SNAPTHREAD: u32 = 0x0000_0004;
    const THREAD_QUERY_LIMITED_INFORMATION: u32 = 0x0800;
    #[repr(C)]
    #[derive(Default)]
    struct ThreadEntry32 {
        size: u32, usage: u32, thread_id: u32, owner_pid: u32,
        base_priority: i32, delta_priority: i32, flags: u32,
    }
    #[repr(C)]
    #[derive(Default)]
    struct FileTime { low: u32, high: u32 }
    impl FileTime {
        fn nanos(&self) -> u128 {
            ((u64::from(self.high) << 32) | u64::from(self.low)) as u128 * 100
        }
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcessId() -> u32;
        fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> *mut c_void;
        fn Thread32First(snapshot: *mut c_void, entry: *mut ThreadEntry32) -> i32;
        fn Thread32Next(snapshot: *mut c_void, entry: *mut ThreadEntry32) -> i32;
        fn OpenThread(access: u32, inherit: i32, id: u32) -> *mut c_void;
        fn GetThreadTimes(thread: *mut c_void, created: *mut FileTime,
            exited: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
        fn GetThreadDescription(thread: *mut c_void, description: *mut *mut u16) -> i32;
        fn LocalFree(mem: *mut c_void) -> *mut c_void;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot.is_null() || snapshot == usize::MAX as *mut c_void {
        return None;
    }
    let pid = unsafe { GetCurrentProcessId() };
    let mut count = 0u32;
    let mut threads = Vec::new();
    let mut entry = ThreadEntry32 { size: size_of::<ThreadEntry32>() as u32,
        ..ThreadEntry32::default() };
    let mut ok = unsafe { Thread32First(snapshot, &mut entry) } != 0;
    while ok {
        if entry.owner_pid == pid {
            count = count.saturating_add(1);
            let thread = unsafe { OpenThread(THREAD_QUERY_LIMITED_INFORMATION, 0, entry.thread_id) };
            if !thread.is_null() {
                let (mut created, mut exited, mut kernel, mut user) =
                    (FileTime::default(), FileTime::default(),
                     FileTime::default(), FileTime::default());
                if unsafe { GetThreadTimes(thread,
                    &mut created, &mut exited, &mut kernel, &mut user) } != 0
                {
                    let mut description: *mut u16 = ptr::null_mut();
                    let name = if unsafe { GetThreadDescription(thread, &mut description) } == 0
                        && !description.is_null()
                    {
                        let mut len = 0usize;
                        while len < 64 && unsafe { *description.add(len) } != 0 {
                            len += 1;
                        }
                        let utf16 = unsafe { std::slice::from_raw_parts(description, len) };
                        let label = String::from_utf16_lossy(utf16);
                        unsafe { LocalFree(description.cast()); }
                        display_name(&label, u64::from(entry.thread_id))
                    } else {
                        format!("Thread {}", entry.thread_id)
                    };
                    threads.push(ThreadCounter {
                        id: u64::from(entry.thread_id),
                        name,
                        cpu_ns: kernel.nanos().saturating_add(user.nanos()),
                    });
                }
                unsafe { CloseHandle(thread); }
            }
        }
        ok = unsafe { Thread32Next(snapshot, &mut entry) } != 0;
    }
    unsafe { CloseHandle(snapshot); }
    Some(ThreadSnapshot { count, threads })
}

#[cfg(not(any(windows, target_os = "linux")))]
pub(crate) fn inspect() -> Option<ThreadSnapshot> { None }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_bounded_and_do_not_embed_control_characters() {
        assert_eq!(display_name("", 10), "Thread 10");
        assert_eq!(display_name(" game\n socket ", 1), "game socket");
        assert_eq!(display_name(&"a".repeat(80), 1).chars().count(), 48);
    }

    #[cfg(any(target_os = "linux", windows))]
    #[test]
    fn current_process_threads_are_sampleable() {
        let snapshot = inspect().expect("running process has thread metadata");
        assert!(snapshot.count > 0);
        assert!(!snapshot.threads.is_empty());
    }
}
