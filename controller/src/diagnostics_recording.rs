//! Developer-triggered background performance recording.
//!
//! No thread, OS polling, global tracing or allocation is installed at
//! application startup. The user must explicitly start a recording from the
//! Diagnostics page. Stop/Drop sends a wakeable cancellation signal; the
//! recorder never depends on GUI refresh, focus, workspace tabs or the tray.
use crate::diagnostics::DiagnosticsSampler;
use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const RECORDING_TICK: Duration = Duration::from_secs(1);
const MAX_MARKERS: usize = 256;
const MAX_PAGE_FRAMES: usize = 500;

#[derive(Clone)]
pub(crate) struct RecordingMarker {
    pub(crate) at: Instant,
    pub(crate) label: String,
}

#[derive(Default, Clone)]
pub(crate) struct PageTimings {
    pub(crate) passes: u64,
    pub(crate) sum_ms: f64,
    pub(crate) maximum_ms: f64,
    samples: VecDeque<f64>,
}
impl PageTimings {
    fn observe(&mut self, duration: Duration) {
        let ms = duration.as_secs_f64() * 1000.0;
        self.passes = self.passes.saturating_add(1);
        self.sum_ms += ms;
        self.maximum_ms = self.maximum_ms.max(ms);
        self.samples.push_back(ms);
        while self.samples.len() > MAX_PAGE_FRAMES { self.samples.pop_front(); }
    }

    pub(crate) fn average_ms(&self) -> f64 {
        if self.passes == 0 { 0.0 } else { self.sum_ms / self.passes as f64 }
    }

    pub(crate) fn p95_ms(&self) -> f64 {
        if self.samples.is_empty() { return 0.0; }
        let mut sorted: Vec<_> = self.samples.iter().copied().collect();
        sorted.sort_unstable_by(f64::total_cmp);
        sorted[(sorted.len() * 95).div_ceil(100).saturating_sub(1)]
    }
}

pub(crate) struct RecordingSession {
    sampler: Arc<Mutex<DiagnosticsSampler>>,
    stop_tx: Option<mpsc::Sender<()>>,
    running: Arc<AtomicBool>,
    began: Instant,
    ended: Option<Instant>,
    pub(crate) draft_marker: String,
    pub(crate) markers: VecDeque<RecordingMarker>,
    pub(crate) page_timings: BTreeMap<&'static str, PageTimings>,
    repaint_causes: BTreeMap<String, u64>,
    last_page: Option<&'static str>,
}

impl RecordingSession {
    pub(crate) fn start() -> Result<Self, String> {
        let sampler = Arc::new(Mutex::new(DiagnosticsSampler::for_recording()));
        let running = Arc::new(AtomicBool::new(true));
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let thread_sampler = Arc::clone(&sampler);
        let thread_running = Arc::clone(&running);
        thread::Builder::new()
            .name("moth-diagnostics-recorder".into())
            .stack_size(512 * 1024)
            .spawn(move || {
                // Never request GUI repaints: collection continues while the
                // window is unfocused, hidden, or rendering a different tab.
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| loop {
                    let tick = Instant::now();
                    match thread_sampler.lock() {
                        Ok(mut state) => { state.sample_if_due(); },
                        Err(_) => break,
                    }
                    let delay = RECORDING_TICK.saturating_sub(tick.elapsed());
                    match stop_rx.recv_timeout(delay) {
                        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                }));
                thread_running.store(false, Ordering::Release);
            })
            .map_err(|error| format!("Could not start diagnostic recording: {error}"))?;

        let began = Instant::now();
        let mut recorder = Self {
            sampler, stop_tx: Some(stop_tx), running, began, ended: None,
            draft_marker: String::new(), markers: VecDeque::new(),
            page_timings: BTreeMap::new(), repaint_causes: BTreeMap::new(),
            last_page: None,
        };
        recorder.mark("Recording started");
        Ok(recorder)
    }

    pub(crate) fn is_running(&self) -> bool {
        self.stop_tx.is_some() && self.running.load(Ordering::Acquire)
    }

    pub(crate) fn stop(&mut self) {
        if self.ended.is_none() {
            self.mark("Recording stopped");
            self.ended = Some(Instant::now());
        }
        self.running.store(false, Ordering::Release);
        if let Some(stop) = self.stop_tx.take() { let _ = stop.send(()); }
    }

    pub(crate) fn elapsed(&self) -> Duration {
        self.ended.unwrap_or_else(Instant::now)
            .saturating_duration_since(self.began)
    }

    pub(crate) fn mark(&mut self, label: impl Into<String>) {
        self.markers.push_back(RecordingMarker {
            at: Instant::now(),
            label: label.into().chars().filter(|c| !c.is_control()).take(100).collect(),
        });
        while self.markers.len() > MAX_MARKERS { self.markers.pop_front(); }
    }

    /// UI observations are recorded while a user explicitly records,
    /// regardless of the active tab. Never wait for the OS sampling thread.
    pub(crate) fn note_ui_pass(&mut self, page: &'static str, duration: Duration) {
        if !self.is_running() { return; }
        if self.last_page != Some(page) {
            self.last_page = Some(page);
            self.mark(format!("Page: {page}"));
        }
        self.page_timings.entry(page).or_default().observe(duration);
        if let Ok(mut sampler) = self.sampler.try_lock() {
            sampler.record_frame(duration, None, None);
        }
    }

    /// Record egui's own explanation for why the current pass happened.
    /// This is intentionally recording-only: exact file/line aggregation is
    /// developer telemetry and has no cost in normal application use.
    pub(crate) fn note_repaint_causes(
        &mut self,
        causes: impl IntoIterator<Item = String>,
    ) {
        if !self.is_running() { return; }
        for cause in causes {
            let cause: String = cause.chars().filter(|c| !c.is_control()).take(240).collect();
            if cause.is_empty() { continue; }
            let count = self.repaint_causes.entry(cause).or_default();
            *count = count.saturating_add(1);
        }
    }

    /// Cloning up to an hour of history is intentional only while the
    /// developer is viewing the explicit recording interface.
    pub(crate) fn snapshot(&self) -> Option<DiagnosticsSampler> {
        self.sampler.try_lock().ok().map(|sampler| sampler.clone())
    }

    pub(crate) fn export_report(&self) -> Result<std::path::PathBuf, String> {
        // Serialize/export an owned snapshot without holding the sampler
        // mutex, so explicit disk writes never stall subsequent OS samples.
        let snapshot = self.sampler.lock()
            .map_err(|_| "Diagnostic recorder data unavailable".to_owned())?.clone();
        let mut report = snapshot.report_json();
        let now = Instant::now();
        report["recording_duration_ms"] = serde_json::json!(self.elapsed().as_millis());
        report["recording_active"] = serde_json::json!(self.is_running());
        report["markers"] = serde_json::json!(self.markers.iter().map(|marker| serde_json::json!({
            "age_ms": now.saturating_duration_since(marker.at).as_millis(),
            "label": marker.label,
        })).collect::<Vec<_>>());
        report["pages"] = serde_json::json!(self.page_timings.iter().map(|(name, timings)| {
            serde_json::json!({
                "page": name,
                "passes": timings.passes,
                "average_ms": timings.average_ms(),
                "p95_ms": timings.p95_ms(),
                "maximum_ms": timings.maximum_ms,
            })
        }).collect::<Vec<_>>());
        let mut repaint_causes: Vec<_> = self.repaint_causes.iter()
            .map(|(cause, count)| (cause, *count))
            .collect();
        repaint_causes.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        report["repaint_causes"] = serde_json::json!(repaint_causes.into_iter()
            .map(|(cause, count)| serde_json::json!({
                "cause": cause,
                "count": count,
            }))
            .collect::<Vec<_>>());
        crate::diagnostics::write_report(&report)
    }
}

impl Drop for RecordingSession {
    fn drop(&mut self) {
        // Dropping the sender wakes the worker immediately out of recv_timeout.
        // Never join an OS-inspection operation on the GUI or shutdown thread.
        if let Some(stop) = self.stop_tx.take() { let _ = stop.send(()); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_timings_track_percentiles_and_limit_storage() {
        let mut times = PageTimings::default();
        for milliseconds in 1..=600 {
            times.observe(Duration::from_millis(milliseconds));
        }
        assert_eq!(times.passes, 600);
        assert_eq!(times.samples.len(), MAX_PAGE_FRAMES);
        assert!((times.average_ms() - 300.5).abs() < 0.01);
        assert_eq!(times.maximum_ms, 600.0);
        assert_eq!(times.p95_ms(), 575.0);
    }

    #[test]
    fn marker_text_is_bounded_and_safe_to_export() {
        let mut recorder = RecordingSession::start().unwrap();
        recorder.mark(format!("marker\n{}", "x".repeat(300)));
        let latest = recorder.markers.back().unwrap();
        assert!(!latest.label.contains('\n'));
        assert_eq!(latest.label.chars().count(), 100);
        recorder.stop();
        assert!(!recorder.is_running());
    }

    #[test]
    fn explicit_stop_finishes_and_preserves_the_capture() {
        let mut recorder = RecordingSession::start().unwrap();
        recorder.note_repaint_causes([
            "src/app.rs:900 worker wake".to_owned(),
            "src/app.rs:900 worker wake".to_owned(),
            "egui/input.rs:1 pointer".to_owned(),
        ]);
        assert_eq!(recorder.repaint_causes["src/app.rs:900 worker wake"], 2);
        recorder.note_ui_pass("Overview", Duration::from_millis(12));
        recorder.note_ui_pass("Hunt Atlas", Duration::from_millis(90));
        assert_eq!(recorder.page_timings.len(), 2);
        recorder.stop();
        let previous = recorder.page_timings["Overview"].passes;
        recorder.note_ui_pass("Overview", Duration::from_secs(1));
        assert_eq!(recorder.page_timings["Overview"].passes, previous);
        assert!(recorder.ended.is_some());
    }
}
