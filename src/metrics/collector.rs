use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::http::timing::RequestTiming;
use crate::metrics::histogram::LatencyHistogram;

pub struct WorkerMetrics {
  pub histogram: LatencyHistogram,
  pub ttfb_histogram: LatencyHistogram,
  pub transfer_histogram: LatencyHistogram,
  pub scheduler_delay_histogram: LatencyHistogram,
  pub success_count: u64,
  pub error_count: u64,
  pub status_counts: std::collections::HashMap<u16, u64>,
  pub bytes_received: u64,
  pub new_connections: u64,
  pub http2_count: u64,
  pub http1_count: u64,
}

impl Default for LatencyHistogram {
  fn default() -> Self {
      Self::new()
  }
}

impl WorkerMetrics {
  pub fn new() -> Self {
    Self {
      histogram: LatencyHistogram::new(),
      ttfb_histogram: LatencyHistogram::new(),
      transfer_histogram: LatencyHistogram::new(),
      scheduler_delay_histogram: LatencyHistogram::new(),
      success_count: 0,
      error_count: 0,
      status_counts: std::collections::HashMap::new(),
      bytes_received: 0,
      new_connections: 0,
      http2_count: 0,
      http1_count: 0,
    }
  }

  pub fn record_success(&mut self, latency: Duration, status: u16, bytes: u64) {
    self.histogram.record(latency);
    self.success_count += 1;
    *self.status_counts.entry(status).or_insert(0) += 1;
    self.bytes_received += bytes;
  }

  pub fn record_error(&mut self, latency: Duration) {
    self.histogram.record(latency);
    self.error_count += 1;
  }

  pub fn record_timing(&mut self, timing:RequestTiming) {
    use crate::http::timing::HttpProtocol;

    if let Some(ttfb) = timing.ttfb() {
      self.ttfb_histogram.record(ttfb);
    }

    if let Some(transfer) = timing.transfer_time() {
      self.transfer_histogram.record(transfer);
    }

    if let Some(delay) = timing.scheduler_delay() {
      self.scheduler_delay_histogram.record(delay);
    }

    if !timing.connection_reused {
      self.new_connections += 1;
    }

    match timing.protocol {
      HttpProtocol::Http1 => self.http1_count += 1,
      HttpProtocol::Http2 => self.http2_count += 1,
      HttpProtocol::Unknown => {}
    }
  }
}

/// Shared atomic counter for live progress reporting
pub struct SharedProgress {
  pub completed: AtomicU64,
  pub errors: AtomicU64,
  pub bytes: AtomicU64,
  pub warmup_sent: AtomicU64,
}

impl SharedProgress {
  pub fn new() -> Arc<Self> {
    Arc::new(Self {
      completed: AtomicU64::new(0),
      errors: AtomicU64::new(0),
      bytes: AtomicU64::new(0),
      warmup_sent: AtomicU64::new(0),
    })
  }

  #[inline]
  pub fn increment_completed(&self) {
    self.completed.fetch_add(1, Ordering::Relaxed);
  }

  #[inline]
  pub fn increment_errors(&self) {
    self.errors.fetch_add(1, Ordering::Relaxed);
  }

  #[inline]
  pub fn add_bytes(&self, n: u64) {
    self.bytes.fetch_add(n, Ordering::Relaxed);
  }

  #[inline]
  pub fn increment_warmup(&self) {
    self.warmup_sent.fetch_add(1, Ordering::Relaxed);
  }
}