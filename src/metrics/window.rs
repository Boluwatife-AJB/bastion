use std::time::{Duration, Instant};
use hdrhistogram::Histogram;

pub struct MetricWindow {
  pub start: Instant,
  pub duration: Duration,
  pub histogram: Histogram<u64>,
  pub ttfb_histogram: Histogram<u64>,
  pub request_count: u64,
  pub success_count: u64,
  pub error_count: u64,
  pub server_error_count: u64,
  pub bytes_received: u64,
  pub new_connections: u64,
}

impl MetricWindow {
    pub fn new(start: Instant, duration: Duration) -> Self {
      Self { start, duration, histogram: Histogram::new_with_bounds(1, 60_000_000_000, 3).expect("Valid histogram bounds"), ttfb_histogram: Histogram::new_with_bounds(1, 60_000_000_000, 3).expect("Valid histogram bounds"), request_count: 0, success_count: 0, error_count:0, server_error_count: 0, bytes_received: 0, new_connections: 0 }
    }

    pub fn record(&mut self, event: &crate::metrics::event::RequestEvent) {
      let nanos = event.total_latency.as_nanos() as u64;
      let _ = self.histogram.record(nanos.min(60_000_000_000));

      if let Some(ttfb) = event.ttfb {
        let _ = self.ttfb_histogram.record((ttfb.as_nanos() as u64).min(60_000_000_000));
      }

      self.request_count += 1;
      self.bytes_received += event.bytes;

      if event.is_success() {
        self.success_count += 1;  
      } else {
          self.error_count += 1;
      }

      if event.is_server_error() {
        self.server_error_count += 1;
      }

      if event.new_connection {
        self.new_connections += 1;
      }
    }

    pub fn is_closed(&self, now: Instant) -> bool {
      now.duration_since(self.start) >= self.duration
    }

    pub fn rps(&self) -> f64 {
      self.request_count as f64 / self.duration.as_secs_f64()
    }

    pub fn throughput_bps(&self) -> f64 {
      self.bytes_received as f64 / self.duration.as_secs_f64()
    }

    /// Error rate as a fraction [0.0, 1.0]
    pub fn error_rate(&self) -> f64 {
      if self.request_count == 0 {
        return 0.0;
      }
      self.error_count as f64 / self.request_count as f64
    }

    pub fn p50(&self) -> Duration {
      nanos_to_duration(self.histogram.value_at_percentile(50.0))
    }
    pub fn p90(&self) -> Duration {
      nanos_to_duration(self.histogram.value_at_percentile(90.0))
    }
    pub fn p99(&self) -> Duration {
      nanos_to_duration(self.histogram.value_at_percentile(99.0))
    }
    pub fn mean(&self) -> Duration {
      nanos_to_duration(self.histogram.mean() as u64)
    }

    pub fn ttfb_p99(&self) -> Option<Duration> {
      if self.ttfb_histogram.len() == 0 {
        Some(nanos_to_duration(self.ttfb_histogram.value_at_percentile(99.0)))
      } else {
        None
      }

    }

}

fn nanos_to_duration(nanos: u64) -> Duration {
  Duration::from_nanos(nanos)
}

/// A finalized, serializable snapshot of a closed window.
#[derive(Debug, Clone)]
pub struct WindowSnapshot {
  pub window_index: u64,
  pub start: Instant,
  pub rps: f64,
  pub throughput_bps: f64,
  pub error_rate: f64,
  pub request_count: u64,
  pub error_count: u64,
  pub bytes_received: u64,
  pub new_connections: u64,
  pub p50: Duration,
  pub p90: Duration,
  pub p99: Duration,
  pub mean: Duration,
  pub ttfb_p99: Option<Duration>,
}

impl WindowSnapshot {
    pub fn from_window(window: &MetricWindow, index: u64) -> Self {
      Self { window_index: index, start: window.start, rps: window.rps(), throughput_bps: window.throughput_bps(), error_rate: window.error_rate(), request_count: window.request_count, error_count: window.error_count, bytes_received: window.bytes_received, new_connections: window.new_connections, p50: window.p50(), p90: window.p90(), p99: window.p99(), mean: window.mean(), ttfb_p99: window.ttfb_p99() }
    }
    
}