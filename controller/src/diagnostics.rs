//! Opt-in, foreground-only performance sampling.
//!
//! There is no worker, timer, global counter, or OS query until the Diagnostics
//! page is both selected and focused. The UI drops this entire state as soon as
//! the user leaves the page or hides/unfocuses the window.
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

pub(crate) const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);
const MAX_HISTORY: usize = 60;
const THREAD_SAMPLE_INTERVAL: Duration = Duration::from_secs(15);
const MAX_UI_HISTORY: usize = 120;
const RECORDING_SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
const RECORDING_THREAD_INTERVAL: Duration = Duration::from_secs(2);
const MAX_RECORDING_HISTORY: usize = 3_600;
const MAX_RECORDING_UI_HISTORY: usize = 2_000;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PerformanceSample {
    /// Monotonic timestamp, preserving real gaps and pause intervals.
    pub(crate) at: Option<Instant>,
    /// CPU usage relative to one logical CPU: multi-threaded work can exceed 100%.
    /// Kept for thread correlation and backwards-compatible exports.
    pub(crate) cpu_percent: Option<f64>,
    /// Process CPU normalized across the machine's logical processors.
    /// This is the closest inexpensive analogue to Task Manager's process scale.
    pub(crate) cpu_system_percent: Option<f64>,
    pub(crate) resident_bytes: Option<u64>,
    pub(crate) peak_bytes: Option<u64>,
    /// Windows private commit charge. This is not private working set.
    pub(crate) private_bytes: Option<u64>,
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

/// Queue and worker data taken from existing snapshots while diagnostics is open.
/// These are queue occupancy metrics, NOT a per-feature CPU estimate.
#[derive(Clone, Debug, Default)]
pub(crate) struct GameQueueSample {
    pub(crate) slot: usize,
    pub(crate) phase: String,
    pub(crate) capture_queued: usize,
    pub(crate) automation_queued: usize,
    pub(crate) automation_pending: bool,
    pub(crate) market_pending: bool,
    pub(crate) restock_pending: bool,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct OperationalSample {
    pub(crate) game: Vec<GameQueueSample>,
    pub(crate) game_busy: usize,
    pub(crate) kick_busy: usize,
    pub(crate) kick_channels: Option<crate::native_kick_pool::PoolUsage>,
    pub(crate) kick_hls: Option<crate::native_kick_pool::PoolUsage>,
}

/// Calculated from two on-demand per-thread OS CPU snapshots.
#[derive(Clone, Debug)]
pub(crate) struct ThreadHotspot {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) cpu_percent: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct ThreadCpuSample {
    pub(crate) at: Instant,
    pub(crate) hottest: Vec<ThreadHotspot>,
}

#[derive(Clone, Copy)]
struct ProcessCounters {
    cpu_ns: u128,
    resident_bytes: Option<u64>,
    peak_bytes: Option<u64>,
    private_bytes: Option<u64>,
    threads: Option<u32>,
}

#[derive(Clone, Default)]
pub(crate) struct DiagnosticsSampler {
    /// Only set by the explicit recording command. No recorder exists at startup.
    pub(crate) recording: bool,
    last_read: Option<Instant>,
    last_thread_read: Option<Instant>,
    last_thread_count: Option<u32>,
    previous: Option<(Instant, ProcessCounters)>,
    previous_thread_read: Option<Instant>,
    previous_thread_cpu: HashMap<u64, u128>,
    pub(crate) inspect_threads: bool,
    pub(crate) hot_threads: Vec<ThreadHotspot>,
    pub(crate) thread_history: VecDeque<ThreadCpuSample>,
    pub(crate) threads_unavailable: bool,
    pub(crate) resident_baseline: Option<u64>,
    pub(crate) current: Option<PerformanceSample>,
    pub(crate) history: VecDeque<PerformanceSample>,
    pub(crate) network: Option<HlsNetworkSample>,
    pub(crate) operational: Option<OperationalSample>,
    pub(crate) paused: bool,
    pub(crate) last_frame_ms: Option<f64>,
    pub(crate) slowest_frame_ms: f64,
    pub(crate) recorded_frames: u64,
    pub(crate) ui_frames: VecDeque<f64>,
    pub(crate) last_preflight_ms: Option<f64>,
    pub(crate) last_panel_ms: Option<f64>,
    pub(crate) last_process_probe_ms: Option<f64>,
    pub(crate) last_network_probe_ms: Option<f64>,
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

#[cfg(windows)]
fn logical_processor_count() -> u32 {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetActiveProcessorCount(group_number: u16) -> u32;
    }
    const ALL_PROCESSOR_GROUPS: u16 = 0xffff;
    let count = unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) };
    if count > 0 { count } else {
        std::thread::available_parallelism()
            .map(|value| value.get() as u32).unwrap_or(1).max(1)
    }
}

#[cfg(not(windows))]
fn logical_processor_count() -> u32 {
    std::thread::available_parallelism()
        .map(|value| value.get() as u32).unwrap_or(1).max(1)
}

fn system_cpu_percent(one_core_percent: Option<f64>) -> Option<f64> {
    one_core_percent.map(|percent| percent / f64::from(logical_processor_count().max(1)))
}

fn thread_sample_due(last: Option<Instant>, now: Instant) -> bool {
    last.is_none_or(|at| now.saturating_duration_since(at) >= THREAD_SAMPLE_INTERVAL)
}

fn calculate_hotspots(
    previous: &HashMap<u64, u128>,
    previous_at: Instant,
    current: &[crate::diagnostics_threads::ThreadCounter],
    now: Instant,
) -> Vec<ThreadHotspot> {
    let elapsed = now.saturating_duration_since(previous_at);
    let mut hot: Vec<_> = current.iter().filter_map(|thread| {
        let previous_ns = *previous.get(&thread.id)?;
        let cpu_percent = cpu_usage_percent(previous_ns, thread.cpu_ns, elapsed)?;
        if !cpu_percent.is_finite() { return None; }
        Some(ThreadHotspot {
            id: thread.id,
            name: thread.name.clone(),
            cpu_percent,
        })
    }).collect();
    hot.sort_by(|a, b| b.cpu_percent.total_cmp(&a.cpu_percent).then_with(|| a.id.cmp(&b.id)));
    hot.truncate(8);
    hot
}

impl DiagnosticsSampler {
    pub(crate) fn reset(&mut self) {
        // Reset is a data action; it must never silently resume sampling.
        let paused = self.paused;
        let inspect_threads = self.inspect_threads;
        let recording = self.recording;
        *self = Self { paused, inspect_threads, recording, ..Self::default() };
    }

    /// Detailed collection is deliberately more expensive and exists only
    /// inside a user-created RecordingSession, never for ordinary users.
    pub(crate) fn for_recording() -> Self {
        Self { recording: true, inspect_threads: true, ..Self::default() }
    }

    fn sample_interval(&self) -> Duration {
        if self.recording { RECORDING_SAMPLE_INTERVAL } else { SAMPLE_INTERVAL }
    }

    fn thread_interval(&self) -> Duration {
        if self.recording { RECORDING_THREAD_INTERVAL } else { THREAD_SAMPLE_INTERVAL }
    }

    pub(crate) fn set_thread_inspection(&mut self, enabled: bool) {
        if self.inspect_threads == enabled { return; }
        self.inspect_threads = enabled;
        // A newly enabled inspector must establish its own CPU baseline.
        self.last_thread_read = None;
        self.last_thread_count = None;
        self.previous_thread_read = None;
        self.previous_thread_cpu.clear();
        self.hot_threads.clear();
        self.threads_unavailable = false;
    }

    pub(crate) fn set_paused(&mut self, paused: bool) {
        if self.paused && !paused {
            // Time spent paused must not enter a CPU delta. Keep earlier
            // history and insert a visible gap on the time-based graphs.
            self.previous = None;
            self.previous_thread_read = None;
            self.previous_thread_cpu.clear();
            self.hot_threads.clear();
            self.last_thread_read = None;
            self.last_thread_count = None;
            self.last_read = None;
            self.current = None;
            self.network = None;
            self.operational = None;
            self.history.push_back(PerformanceSample {
                at: Some(Instant::now()),
                ..PerformanceSample::default()
            });
            self.trim_history();
        }
        self.paused = paused;
    }

    fn trim_history(&mut self) {
        let limit = if self.recording { MAX_RECORDING_HISTORY } else { MAX_HISTORY };
        while self.history.len() > limit { self.history.pop_front(); }
    }

    /// Returns true only on a due tick. Never polls while paused.
    pub(crate) fn sample_if_due(&mut self) -> bool {
        if self.paused { return false; }
        let now = Instant::now();
        if self.last_read.is_some_and(|at| now.saturating_duration_since(at) < self.sample_interval()) {
            return false;
        }
        self.last_read = Some(now);
        let include_threads = if self.recording {
            self.last_thread_read.is_none_or(|at|
                now.saturating_duration_since(at) >= self.thread_interval())
        } else {
            thread_sample_due(self.last_thread_read, now)
        };
        let probe_start = Instant::now();
        // The detailed thread snapshot includes the thread count, so avoid
        // enumerating the same OS threads twice on the same tick.
        let counters = read_process_counters(include_threads && !self.inspect_threads);
        if include_threads {
            self.last_thread_read = Some(now);
            if self.inspect_threads {
                match crate::diagnostics_threads::inspect() {
                    Some(snapshot) => {
                        self.last_thread_count = Some(snapshot.count);
                        self.threads_unavailable = false;
                        self.hot_threads = self.previous_thread_read.map(|prior| {
                            calculate_hotspots(&self.previous_thread_cpu, prior, &snapshot.threads, now)
                        }).unwrap_or_default();
                        if self.recording && !self.hot_threads.is_empty() {
                            self.thread_history.push_back(ThreadCpuSample {
                                at: now, hottest: self.hot_threads.clone(),
                            });
                            while self.thread_history.len() > MAX_RECORDING_HISTORY / 2 {
                                self.thread_history.pop_front();
                            }
                        }
                        self.previous_thread_cpu = snapshot.threads.iter()
                            .map(|thread| (thread.id, thread.cpu_ns)).collect();
                        self.previous_thread_read = Some(now);
                    }
                    None => {
                        self.last_thread_count = None;
                        self.previous_thread_read = None;
                        self.previous_thread_cpu.clear();
                        self.hot_threads.clear();
                        self.threads_unavailable = true;
                    }
                }
            } else {
                self.last_thread_count = counters.as_ref().and_then(|stats| stats.threads);
            }
        }
        self.last_process_probe_ms = Some(probe_start.elapsed().as_secs_f64() * 1000.0);

        let Some(counters) = counters else {
            self.previous = None;
            self.current = None;
            self.history.push_back(PerformanceSample {
                at: Some(now), ..PerformanceSample::default()
            });
            self.trim_history();
            return true;
        };
        let cpu_percent = self.previous
            .and_then(|(previous_at, previous)| cpu_usage_percent(
                previous.cpu_ns, counters.cpu_ns, now.saturating_duration_since(previous_at)
            ));
        let point = PerformanceSample {
            at: Some(now),
            cpu_percent,
            cpu_system_percent: system_cpu_percent(cpu_percent),
            resident_bytes: counters.resident_bytes,
            peak_bytes: counters.peak_bytes,
            private_bytes: counters.private_bytes,
            threads: self.last_thread_count,
        };
        if let Some(rss) = counters.resident_bytes {
            self.resident_baseline.get_or_insert(rss);
        }
        self.previous = Some((now, counters));
        self.current = Some(point);
        self.history.push_back(point);
        self.trim_history();
        true
    }

    pub(crate) fn record_frame(
        &mut self, elapsed: Duration, preflight: Option<Duration>, panel: Option<Duration>,
    ) {
        if self.paused { return; }
        let ms = elapsed.as_secs_f64() * 1000.0;
        self.last_frame_ms = Some(ms);
        self.slowest_frame_ms = self.slowest_frame_ms.max(ms);
        self.recorded_frames = self.recorded_frames.saturating_add(1);
        self.ui_frames.push_back(ms);
        let limit = if self.recording { MAX_RECORDING_UI_HISTORY } else { MAX_UI_HISTORY };
        while self.ui_frames.len() > limit { self.ui_frames.pop_front(); }
        self.last_preflight_ms = preflight.map(|t| t.as_secs_f64() * 1000.0);
        self.last_panel_ms = panel.map(|t| t.as_secs_f64() * 1000.0);
    }

    /// Report creation is an explicit user action, never an automatic save.
    /// Account names, credentials, network URLs and game state are excluded.
    pub(crate) fn report_json(&self) -> serde_json::Value {
        let now = Instant::now();
        serde_json::json!({
            "schema": "moth-controller-diagnostics-v1",
            "version": env!("CARGO_PKG_VERSION"),
            "sampling_interval_ms": self.sample_interval().as_millis(),
            "recording": self.recording,
            "thread_sampling_interval_ms": self.thread_interval().as_millis(),
            "cpu_units": "percent_of_one_logical_cpu",
            "cpu_system_units": "percent_of_total_logical_processor_capacity",
            "logical_processors": logical_processor_count(),
            "memory_baseline_bytes": self.resident_baseline,
            "detailed_thread_inspection": self.inspect_threads,
            "thread_cpu_hotspots": self.hot_threads.iter().map(|thread| serde_json::json!({
                "thread_id": thread.id,
                "thread_name": thread.name.as_str(),
                "cpu_percent": thread.cpu_percent,
            })).collect::<Vec<_>>(),
            "thread_cpu_timeline": self.thread_history.iter().map(|point| serde_json::json!({
                "age_ms": now.saturating_duration_since(point.at).as_millis(),
                "top": point.hottest.iter().map(|thread| serde_json::json!({
                    "thread_id": thread.id,
                    "thread_name": thread.name.as_str(),
                    "cpu_percent": thread.cpu_percent,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "process_samples": self.history.iter().map(|point| serde_json::json!({
                "age_ms": point.at.map(|at| now.saturating_duration_since(at).as_millis()),
                "cpu_percent": point.cpu_percent,
                "cpu_system_percent": point.cpu_system_percent,
                "resident_bytes": point.resident_bytes,
                "peak_bytes": point.peak_bytes,
                "private_bytes": point.private_bytes,
                "threads": point.threads,
            })).collect::<Vec<_>>(),
            "ui": {
                "frame_pass_samples_ms": self.ui_frames.iter().copied().collect::<Vec<_>>(),
                "frame_passes_total": self.recorded_frames,
                "last_preflight_ms": self.last_preflight_ms,
                "last_panel_ms": self.last_panel_ms,
                "last_process_probe_ms": self.last_process_probe_ms,
                "last_diagnostics_refresh_ms": self.last_network_probe_ms,
            },
            "kick_hls": self.network.as_ref().map(|network| serde_json::json!({
                "session_bytes": network.bytes,
                "last_minute_bytes": network.last_minute,
            })),
            "worker_queues": self.operational.as_ref().map(|ops| serde_json::json!({
                "game_snapshots_busy": ops.game_busy,
                "kick_snapshots_busy": ops.kick_busy,
                "game": ops.game.iter().map(|game| serde_json::json!({
                    "profile_slot": game.slot + 1,
                    "phase": game.phase,
                    "capture_queued": game.capture_queued,
                    "automation_queued": game.automation_queued,
                    "automation_pending": game.automation_pending,
                    "market_pending": game.market_pending,
                    "restock_pending": game.restock_pending,
                })).collect::<Vec<_>>(),
                "kick_channels": ops.kick_channels.map(|pool| serde_json::json!({
                    "workers": pool.workers, "idle": pool.idle, "pending": pool.pending,
                })),
                "kick_hls": ops.kick_hls.map(|pool| serde_json::json!({
                    "workers": pool.workers, "idle": pool.idle, "pending": pool.pending,
                })),
            })),
        })
    }

    pub(crate) fn export_report(&self) -> Result<std::path::PathBuf, String> {
        write_report(&self.report_json())
    }

    pub(crate) fn mean_frame_ms(&self) -> Option<f64> {
        (!self.ui_frames.is_empty()).then(||
            self.ui_frames.iter().sum::<f64>() / self.ui_frames.len() as f64
        )
    }

    pub(crate) fn p95_frame_ms(&self) -> Option<f64> {
        if self.ui_frames.is_empty() { return None; }
        let mut values: Vec<_> = self.ui_frames.iter().copied().collect();
        values.sort_unstable_by(f64::total_cmp);
        let index = (values.len() * 95).div_ceil(100).saturating_sub(1);
        values.get(index).copied()
    }
}

/// Shared explicit-export path for foreground and background recordings.
/// No file is ever written by a sampling timer or passive UI rendering.
pub(crate) fn write_report(report: &serde_json::Value) -> Result<std::path::PathBuf, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = crate::config::data_root()?.join("Diagnostics");
    std::fs::create_dir_all(&root)
        .map_err(|error| format!("Could not create diagnostics directory: {error}"))?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|error| format!("Invalid system clock: {error}"))?.as_millis();
    let path = root.join(format!("diagnostics-{stamp}.json"));
    let bytes = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("Could not serialize diagnostics: {error}"))?;
    std::fs::write(&path, bytes)
        .map_err(|error| format!("Could not save diagnostics report: {error}"))?;
    Ok(path)
}

#[cfg(target_os = "linux")]
fn read_process_counters(include_threads: bool) -> Option<ProcessCounters> {
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
        private_bytes: None,
        threads: include_threads.then(|| value("Threads:")
            .and_then(|n| u32::try_from(n).ok())).flatten(),
    })
}

#[cfg(windows)]
fn read_process_counters(include_threads: bool) -> Option<ProcessCounters> {
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
        private_usage: usize,
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
    let thread_count = if !include_threads {
        None
    } else {
        let snapshot = unsafe { CreateToolhelp32Snapshot(0x0000_0004, 0) };
        if snapshot == usize::MAX as *mut c_void || snapshot.is_null() {
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
        }
    };

    Some(ProcessCounters {
        cpu_ns: kernel.nanos().saturating_add(user.nanos()),
        resident_bytes: memory_ok.then_some(memory.working_set_size as u64),
        peak_bytes: memory_ok.then_some(memory.peak_working_set_size as u64),
        private_bytes: memory_ok.then_some(memory.private_usage as u64),
        threads: thread_count,
    })
}

#[cfg(not(any(target_os = "linux", windows)))]
fn read_process_counters(_include_threads: bool) -> Option<ProcessCounters> { None }

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
    fn system_cpu_rate_is_normalized_by_logical_processors() {
        let count = f64::from(logical_processor_count().max(1));
        assert_eq!(system_cpu_percent(Some(100.0)), Some(100.0 / count));
        assert_eq!(system_cpu_percent(None), None);
    }

    #[test]
    fn pause_does_not_sample_or_record_frames() {
        let mut sampler = DiagnosticsSampler { paused: true, ..DiagnosticsSampler::default() };
        assert!(!sampler.sample_if_due());
        sampler.record_frame(Duration::from_millis(17), None, None);
        assert!(sampler.current.is_none());
        assert_eq!(sampler.recorded_frames, 0);
        assert!(sampler.last_read.is_none());
    }

    #[test]
    fn resuming_preserves_history_but_clears_cpu_baseline() {
        let mut sampler = DiagnosticsSampler {
            paused: true,
            current: Some(PerformanceSample::default()),
            network: Some(HlsNetworkSample::default()),
            last_read: Some(Instant::now()),
            previous: Some((Instant::now(), ProcessCounters {
                cpu_ns: 100, resident_bytes: None, peak_bytes: None,
                private_bytes: None, threads: None,
            })),
            ..DiagnosticsSampler::default()
        };
        sampler.history.push_back(PerformanceSample::default());
        sampler.set_paused(false);
        assert!(!sampler.paused);
        assert!(sampler.current.is_none());
        assert!(sampler.network.is_none());
        assert!(sampler.previous.is_none());
        assert!(sampler.last_read.is_none());
        assert_eq!(sampler.history.len(), 2);
        assert!(sampler.history.back().is_some_and(|point| point.cpu_percent.is_none()));
    }

    #[test]
    fn reset_keeps_pause_and_clears_data() {
        let mut sampler = DiagnosticsSampler {
            paused: true, recorded_frames: 12,
            current: Some(PerformanceSample::default()),
            ..DiagnosticsSampler::default()
        };
        sampler.reset();
        assert!(sampler.paused);
        assert!(sampler.current.is_none());
        assert_eq!(sampler.recorded_frames, 0);
        assert!(!sampler.sample_if_due());
    }

    #[test]
    fn samples_threads_at_lower_frequency() {
        let now = Instant::now();
        assert!(thread_sample_due(None, now));
        assert!(!thread_sample_due(Some(now), now));
        assert!(!thread_sample_due(Some(now), now + Duration::from_secs(14)));
        assert!(thread_sample_due(Some(now), now + Duration::from_secs(15)));
    }

    #[test]
    fn ui_statistics_are_bounded_and_correct() {
        let mut sampler = DiagnosticsSampler::default();
        for millis in 1..=130 {
            sampler.record_frame(Duration::from_millis(millis), None, None);
        }
        assert_eq!(sampler.ui_frames.len(), MAX_UI_HISTORY);
        assert_eq!(sampler.mean_frame_ms(), Some(70.5));
        assert_eq!(sampler.p95_frame_ms(), Some(124.0));
    }

    #[test]
    fn exported_report_excludes_account_names_and_private_state() {
        let mut sampler = DiagnosticsSampler::default();
        sampler.network = Some(HlsNetworkSample {
            rows: vec![HlsAccountSample {
                account: "PRIVATE_ACCOUNT".into(), bytes: 100,
                last_minute: 50, channels: 1,
            }],
            bytes: 100, last_minute: 50,
        });
        sampler.operational = Some(OperationalSample {
            game: vec![GameQueueSample {
                slot: 1, phase: "Online".into(),
                capture_queued: 3, ..GameQueueSample::default()
            }],
            ..OperationalSample::default()
        });
        let report = sampler.report_json().to_string();
        assert!(!report.contains("PRIVATE_ACCOUNT"));
        assert!(report.contains("profile_slot"));
        assert!(report.contains("capture_queued"));
        assert!(report.contains("moth-controller-diagnostics-v1"));
    }

    #[test]
    fn hotspot_cpu_rates_ignore_new_and_replaced_threads() {
        let now = Instant::now();
        let previous_at = now - Duration::from_secs(10);
        let previous = HashMap::from([(101, 1_000_000_000), (202, 5_000_000_000)]);
        let current = vec![
            crate::diagnostics_threads::ThreadCounter {
                id: 101, name: "moth-game".into(), cpu_ns: 3_000_000_000,
            },
            crate::diagnostics_threads::ThreadCounter {
                id: 202, name: "moth-ui".into(), cpu_ns: 5_500_000_000,
            },
            crate::diagnostics_threads::ThreadCounter {
                id: 303, name: "new".into(), cpu_ns: 1_000_000_000,
            },
            crate::diagnostics_threads::ThreadCounter {
                id: 404, name: "reset".into(), cpu_ns: 2_000_000_000,
            },
        ];
        let results = calculate_hotspots(&previous, previous_at, &current, now);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 101);
        assert!((results[0].cpu_percent - 20.0).abs() < 0.0001);
        assert_eq!(results[1].id, 202);
        assert!((results[1].cpu_percent - 5.0).abs() < 0.0001);
    }

    #[test]
    fn thread_inspection_is_opt_in_and_clears_old_baselines() {
        let mut sampler = DiagnosticsSampler::default();
        assert!(!sampler.inspect_threads);
        assert!(sampler.previous_thread_cpu.is_empty());
        sampler.set_thread_inspection(true);
        assert!(sampler.inspect_threads);
        sampler.previous_thread_cpu.insert(123, 99);
        sampler.last_thread_read = Some(Instant::now());
        sampler.set_thread_inspection(false);
        assert!(!sampler.inspect_threads);
        assert!(sampler.previous_thread_cpu.is_empty());
        assert!(sampler.last_thread_read.is_none());
        sampler.set_thread_inspection(true);
        assert!(sampler.previous_thread_read.is_none());
    }

    #[test]
    fn reset_preserves_thread_inspection_preference() {
        let mut sampler = DiagnosticsSampler::default();
        sampler.set_thread_inspection(true);
        sampler.reset();
        assert!(sampler.inspect_threads);
        assert!(sampler.previous_thread_cpu.is_empty());
        assert!(sampler.hot_threads.is_empty());
    }

    #[test]
    fn background_recorder_is_explicit_and_retains_a_longer_bounded_trace() {
        let ordinary = DiagnosticsSampler::default();
        assert!(!ordinary.recording);
        assert!(!ordinary.inspect_threads);
        assert_eq!(ordinary.sample_interval(), Duration::from_secs(2));
        let mut capture = DiagnosticsSampler::for_recording();
        assert!(capture.recording);
        assert!(capture.inspect_threads);
        assert_eq!(capture.sample_interval(), Duration::from_secs(1));
        assert_eq!(capture.thread_interval(), Duration::from_secs(2));
        for _ in 0..MAX_RECORDING_HISTORY + 20 {
            capture.history.push_back(PerformanceSample::default());
            capture.trim_history();
        }
        assert_eq!(capture.history.len(), MAX_RECORDING_HISTORY);
        capture.reset();
        assert!(capture.recording);
        assert!(capture.inspect_threads);
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
