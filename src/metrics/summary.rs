use std::time::Duration;
use crate::metrics::window::WindowSnapshot;


/// Derive a full-run MetricsSnapshot from the window history
pub fn summarize_windows(windows: &[WindowSnapshot], total_duration: Duration) -> RunSummary {
  if windows.is_empty() {
    return RunSummary::default();
  }

  let total_requests = windows.iter().map(|w| w.request_count).sum();
  let total_errors = windows.iter().map(|w| w.error_count).sum();
  let total_bytes = windows.iter().map(|w| w.bytes_received).sum();

  let peak_rps = windows.iter().map(|w| w.rps).fold(0.0_f64, f64::max);
  let min_rps = windows.iter().filter(|w| w.rps > 0.0).map(|w| w.rps).fold(f64::MAX, f64::min);

  let avg_rps = windows.iter().map(|w| w.rps).sum::<f64>() / windows.len() as f64;

  let peak_p99 = windows.iter().map(|w| w.p99).max().unwrap_or(Duration::ZERO);

  let avg_p99_nanos = windows.iter().map(|w| w.p99.as_nanos() as f64).sum::<f64>() / windows.len() as f64;
  let avg_p99 = Duration::from_nanos(avg_p99_nanos as u64);

  // Detect latency spikes
  let mut p99s: Vec<Duration> = windows.iter().map(|w| w.p99).collect();
  p99s.sort();
  let median_p99 = p99s[p99s.len()/2];
  let spike_threshold = median_p99 * 2;
  let spike_count = windows.iter().filter(|w| w.p99 > spike_threshold).count();

  RunSummary { window_count: windows.len(), total_requests, total_errors, total_bytes, total_duration, avg_rps, peak_rps, min_rps, avg_p99, peak_p99, error_rate: total_errors as f64 / total_requests.max(1) as f64, spike_count, spike_threshold }
}

#[derive(Debug, Default)]
pub struct RunSummary {
  pub window_count: usize,
  pub total_requests: u64,
  pub total_errors: u64,
  pub total_bytes: u64,
  pub total_duration: Duration,
  pub avg_rps: f64,
  pub peak_rps: f64,
  pub min_rps: f64,
  pub avg_p99: Duration,
  pub peak_p99: Duration,
  pub error_rate: f64,
  pub spike_count: usize,
  pub spike_threshold: Duration,
}