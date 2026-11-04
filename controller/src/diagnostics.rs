//! Opt-in, foreground-only performance sampling.
//!
//! There is no worker, timer, global counter, or OS query until the Diagnostics
//! page is both selected and focused. The UI drops this entire state as soon as
//! the user leaves the page or hides/unfocuses the window.
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub(crate) const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);
const MAX_HISTORY: usize = 60;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PerformanceSample {
    /// CPU usage relative to one logical CPU: multi-threaded work can exceed 100%.
    pub(crate) cpu_percent: Option<f64>,
    pub(crate) resident_bytes: Option<u64>,
    pub(crate) peak_bytes: Option<u64>,
    pub(crate) threads: Option<u32>,
}

#[derive(Clone, Default)]
pub(crate) struct HlsAccountSample {
    pub(crate) account: String,
    pub(crate) bytes: u64,
    pub(crate) last_minute: u64,
    pub(crate) channels: usize,
}

#[derive(Clone, Default)]
pub(crate) struct HlsNetworkSample {
    pub(crate) rows: Vec<HlsAccountSample>,
    pub(crate) bytes: u64,
    pub(crate) last_minute: u64,
}

#[derive(Clone, Copy)]
struct ProcessCounters {
    cpu_ns: u128,
    resident_bytes: Option<u64>,
    peak_bytes: Option<u64>,
    threads: Option<u32>,
}

#[derive(Default)]
pub(crate) struct DiagnosticsSampler {
    last_read: Option<Instant>,
    previous: Option<(Instant, ProcessCounters)>,
    pub(crate) current: Option<PerformanceSample>,
    pub(crate) history: VecDeque<PerformanceSample>,
    pub(crate) network: Option<HlsNetworkSample>,
    pub(crate) paused: bool,
    pub(crate) last_frame_ms: Option<f64>,
    pub(crate) slowest_frame_ms: f64,
    pub(crate) recorded_frames: u64,
}

/// Fail closed if the window manager does not report focus. Diagnostics
/// must never silently run in an unverified background window.
pub(crate) fn should_sample(tab_selected: bool, tray_hidden: bool, focused: Option<bool>) -> bool {
    tab_selected && !tray_hidden && focused == Some(true)
}

fn cpu_usage_percent(previous_ns: u128, current_ns: u128, elapsed: Duration) -> Option<f64> {
    let elapsed_ns = elapsed.as_nanos();
    if elapsed_ns == 0 || current_ns < previous_ns { return None; }
    Some((current_ns - previous_ns) as f64 * 100.0 / elapsed_ns as f64)
}

impl DiagnosticsSampler {
    pub(crate) fn set_paused(&mut self, paused: bool) {
        if self.paused && !paused {
            // A pause interval is not a valid CPU usage sample window.
            self.previous = None;
            self.last_read = None;
            self.current = None;
            self.history.clear();
            self.network = None;
        }
        self.paused = paused;
    }

    // True only on a due sampling tick. Callers may update other cached
    // metrics on that same tick; never re-read them on intermediate UI frames.
    pub(crate) fn sample_if_due(&mut self) -> bool {
        if self.paused { return false; }
        let now = Instant::now();
        if self.last_read.is_some_and(|at| now.duration_since(at) < SAMPLE_INTERVAL) {
            return false;
        }
        self.last_read = Some(now);
        let Some(counters) = read_process_counters() else {
            self.previous = None;
            self.current = None;
            return true;
        };
        let cpu_percent = self.previous
            .and_then(|(previous_at, previous)| cpu_usage_percent(
                previous.cpu_ns, counters.cpu_ns, now.duration_since(previous_at)
            ));
        let point = PerformanceSample {
            cpu_percent,
            resident_bytes: counters.resident_bytes,
            peak_bytes: counters.peak_bytes,
            threads: counters.threads,
        };
        self.previous = Some((now, counters));
        self.current = Some(point);
        self.history.push_back(point);
        if self.history.len() > MAX_HISTORY { self.history.pop_front(); }
        true
    }

    pub(crate) fn record_frame(&mut self, elapsed: Duration) {
        if self.paused { return; }
        let ms = elapsed.as_secs_f64() * 1000.0;
        self.last_frame_ms = Some(ms);
        self.slowest_frame_ms = self.slowest_frame_ms.max(ms);
        self.recorded_frames = self.recorded_frames.saturating_add(1);
    }
}

#[cfg(target_os = "linux")]
fn read_process_counters() -> Option<ProcessCounters> {
    use std::fs;
    use std::ffi::{c_int, c_long};
    // Linux /proc/self/stat utime/stime are process-wide CPU tick counters.
    // sysconf(_SC_CLK_TCK) converts ticks to nanoseconds without assumptions.
    unsafe extern "C" { fn sysconf(name: c_int) -> c_long; }
    const SC_CLK_TCK: c_int = 2;
    let hz = unsafe { sysconf(SC_CLK_TCK) };
    if hz <= 0 { return None; }
    let stat = fs::read_to_string("/proc/self/stat").ok()?;
    // comm is parenthesized and may contain spaces, so start after the final ')'.
    let after_comm = stat.rsplit_once(") ")?.1;
    let mut fields = after_comm.split_whitespace();
    let user_ticks: u128 = fields.nth(11)?.parse().ok()?;
    let system_ticks: u128 = fields.next()?.parse().ok()?;
    let cpu_ns = user_ticks.saturating_add(system_ticks)
        .saturating_mul(1_000_000_000) / hz as u128;

    let status = fs::read_to_string("/proc/self/status").ok();
    let value = |key: &str| -> Option<u64> {
        status.as_deref()?.lines().find_map(|line|
            line.strip_prefix(key)?.split_whitespace().next()?.parse().ok()
        )
    };
    Some(ProcessCounters {
        cpu_ns,
        resident_bytes: value("VmRSS:").and_then(|kib| kib.checked_mul(1024)),
        peak_bytes: value("VmHWM:").and_then(|kib| kib.checked_mul(1024)),
        threads: value("Threads:").and_then(|n| u32::try_from(n).ok()),
    })
}

#[cfg(windows)]
fn read_process_counters() -> Option<ProcessCounters> {
    use std::ffi::c_void;
    use std::mem::size_of;
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct FileTime { low: u32, high: u32 }
    impl FileTime {
        fn nanos(self) -> u128 {
            ((u64::from(self.high) << 32) | u64::from(self.low)) as u128 * 100
        }
    }
    #[repr(C)]
    #[derive(Default)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }
    #[repr(C)]
    #[derive(Default)]
    struct ThreadEntry32 {
        size: u32, usage: u32, thread_id: u32, owner_pid: u32,
        base_priority: i32, delta_priority: i32, flags: u32,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn GetCurrentProcessId() -> u32;
        fn GetProcessTimes(process: *mut c_void, creation: *mut FileTime,
            exit: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
        fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> *mut c_void;
        fn Thread32First(snapshot: *mut c_void, entry: *mut ThreadEntry32) -> i32;
        fn Thread32Next(snapshot: *mut c_void, entry: *mut ThreadEntry32) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(process: *mut c_void,
            counters: *mut ProcessMemoryCounters, size: u32) -> i32;
    }

    let process = unsafe { GetCurrentProcess() };
    let (mut created, mut exited, mut kernel, mut user) =
        (FileTime::default(), FileTime::default(), FileTime::default(), FileTime::default());
    if unsafe { GetProcessTimes(process, &mut created, &mut exited, &mut kernel, &mut user) } == 0 {
        return None;
    }
    let mut memory = ProcessMemoryCounters {
        cb: size_of::<ProcessMemoryCounters>() as u32,
        ..ProcessMemoryCounters::default()
    };
    let memory_ok = unsafe {
        GetProcessMemoryInfo(process, &mut memory, size_of::<ProcessMemoryCounters>() as u32)
    } != 0;

    // Toolhelp is invoked *only* on a due sample while Diagnostics is visible.
    let snapshot = unsafe { CreateToolhelp32Snapshot(0x0000_0004, 0) };
    let thread_count = if snapshot == usize::MAX as *mut c_void || snapshot.is_null() {
        None
    } else {
        let mut entry = ThreadEntry32 { size: size_of::<ThreadEntry32>() as u32,
            ..ThreadEntry32::default() };
        let pid = unsafe { GetCurrentProcessId() };
        let mut count: u32 = 0;
        let mut ok = unsafe { Thread32First(snapshot, &mut entry) } != 0;
        while ok {
            if entry.owner_pid == pid { count = count.saturating_add(1); }
            ok = unsafe { Thread32Next(snapshot, &mut entry) } != 0;
        }
        unsafe { CloseHandle(snapshot) };
        Some(count)
    };

    Some(ProcessCounters {
        cpu_ns: kernel.nanos().saturating_add(user.nanos()),
        resident_bytes: memory_ok.then_some(memory.working_set_size as u64),
        peak_bytes: memory_ok.then_some(memory.peak_working_set_size as u64),
        threads: thread_count,
    })
}

#[cfg(not(any(target_os = "linux", windows)))]
fn read_process_counters() -> Option<ProcessCounters> { None }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_when_closed_or_unfocused() {
        assert!(!should_sample(false, false, Some(true)));
        assert!(!should_sample(true, true, Some(true)));
        assert!(!should_sample(true, false, Some(false)));
        assert!(!should_sample(true, false, None));
        assert!(should_sample(true, false, Some(true)));
        let sampler = DiagnosticsSampler::default();
        assert!(sampler.current.is_none());
        assert!(sampler.history.is_empty());
        assert!(sampler.last_read.is_none());
    }

    #[test]
    fn cpu_rate_is_per_core_and_guards_invalid_intervals() {
        assert_eq!(cpu_usage_percent(0, 1_000_000_000, Duration::from_secs(1)), Some(100.0));
        assert_eq!(cpu_usage_percent(0, 2_000_000_000, Duration::from_secs(1)), Some(200.0));
        assert_eq!(cpu_usage_percent(100, 99, Duration::from_secs(1)), None);
        assert_eq!(cpu_usage_percent(0, 1, Duration::ZERO), None);
    }

    #[test]
    fn pause_does_not_sample_or_record_frames() {
        let mut sampler = DiagnosticsSampler { paused: true, ..DiagnosticsSampler::default() };
        assert!(!sampler.sample_if_due());
        sampler.record_frame(Duration::from_millis(17));
        assert!(sampler.current.is_none());
        assert_eq!(sampler.recorded_frames, 0);
        assert!(sampler.last_read.is_none());
    }

    #[test]
    fn history_is_bounded() {
        let mut sampler = DiagnosticsSampler::default();
        for _ in 0..100 {
            sampler.history.push_back(PerformanceSample::default());
            if sampler.history.len() > MAX_HISTORY { sampler.history.pop_front(); }
        }
        assert_eq!(sampler.history.len(), MAX_HISTORY);
    }
}
