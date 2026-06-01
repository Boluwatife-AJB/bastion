use std::time::Duration;
use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use crate::metrics::histogram::LatencyHistogram;
use crate::metrics::collector::WorkerMetrics;
use crate::metrics::summary::RunSummary;
use crate::metrics::window::WindowSnapshot;

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsSnapshot {
  pub total_requests: u64,
  pub success_count: u64,
  pub error_count: u64,
  pub total_duration: Duration,
  pub requests_per_second: f64,
  pub bytes_received: u64,
  pub status_counts: HashMap<u16, u64>,
  pub latency: LatencySnapshot,
  pub ttfb: Option<LatencySnapshot>,
  pub transfer: Option<LatencySnapshot>,
  pub scheduler_delay: Option<LatencySnapshot>,
  pub connections: ConnectionSnapshot,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LatencySnapshot {
  pub min: Duration,
  pub max: Duration,
  pub mean: Duration,
  pub stddev: Duration,
  pub p50: Duration,
  pub p75: Duration,
  pub p90: Duration,
  pub p95: Duration,
  pub p99: Duration,
  pub p999: Duration,
}

impl LatencySnapshot {
  pub fn from_historgram(h: &LatencyHistogram) -> Self {
    Self {
      min: h.min(),
      max: h.max(),
      mean: h.mean(),
      stddev: h.stddev(),
      p50: h.percentile(50.0),
      p75: h.percentile(75.0),
      p90: h.percentile(90.0),
      p95: h.percentile(95.0),
      p99: h.percentile(99.0),
      p999: h.percentile(99.9),
    }
  }

  pub fn zeroed() -> Self {
    Self { min: std::time::Duration::ZERO, max: std::time::Duration::ZERO, mean: std::time::Duration::ZERO, stddev: std::time::Duration::ZERO, p50: std::time::Duration::ZERO, p75: std::time::Duration::ZERO, p90: std::time::Duration::ZERO, p95: std::time::Duration::ZERO, p99: std::time::Duration::ZERO, p999: std::time::Duration::ZERO }
  }
}

#[derive(Debug, Serialize, Default, Deserialize)]
pub struct ConnectionSnapshot {
  pub new_connections: u64,
  pub http2_requests: u64,
  pub http1_requests: u64,
  pub reuse_rate: f64,
}

impl MetricsSnapshot {
  /// Build a snapshot by merging all per-worker metrics
  pub fn from_workers(workers: Vec<WorkerMetrics>, total_duration: Duration) -> Self {
    let mut merged = LatencyHistogram::new();
    let mut merged_ttfb = LatencyHistogram::new();
    let mut merged_transfer = LatencyHistogram::new();
    let mut merged_delay = LatencyHistogram::new();
        
    let mut success_count = 0u64;
    let mut error_count = 0u64;
    let mut status_counts: HashMap<u16, u64> = HashMap::new();
    let mut bytes_received = 0u64;
    let mut new_connections = 0u64;
    let mut http2_count = 0u64;
    let mut http1_count = 0u64;

    for worker in workers {
      merged.merge(&worker.histogram);
      merged_ttfb.merge(&worker.ttfb_histogram);
      merged_transfer.merge(&worker.transfer_histogram);
      merged_delay.merge(&worker.scheduler_delay_histogram);

      success_count += worker.success_count;
      error_count += worker.error_count;
      bytes_received += worker.bytes_received;
      new_connections += worker.new_connections;
      http2_count += worker.http2_count;
      http1_count += worker.http1_count;

      for (status, count) in worker.status_counts {
        *status_counts.entry(status).or_insert(0) += count;
      }
    }

    let total_requests = success_count + error_count;
    let requests_per_second = total_requests as f64 / total_duration.as_secs_f64().max(f64::EPSILON); 

    let reuse_rate = if total_requests > 0 {
      new_connections as f64 / total_requests as f64
    } else {
      0.0
    };

    Self {
      total_requests,
      success_count,
      error_count,
      total_duration,
      requests_per_second,
      bytes_received,
      status_counts,
      latency: LatencySnapshot::from_historgram(&merged),
      ttfb: if merged_ttfb.count() > 0 {
         Some(LatencySnapshot::from_historgram(&merged_ttfb)) 
        } else {
          None
        },
      transfer: if merged_transfer.count() > 0 {
        Some(LatencySnapshot::from_historgram(&merged_transfer))
      } else {
        None
      },
      scheduler_delay: if merged_delay.count() > 0 {
        Some(LatencySnapshot::from_historgram(&merged_delay))
      } else {
        None
      },
      connections: ConnectionSnapshot {
        new_connections,
        http2_requests: http2_count,
        http1_requests: http1_count,
        reuse_rate,
      },
    }
  }

  /// Builds full snapshot from the run summary and window history
  pub fn from_summary_and_windows(summary: RunSummary, windows: &[WindowSnapshot], total_duration: Duration) -> Self {
    let status_counts = std::collections::HashMap::new();

    let new_connections = windows.iter().map(|w| w.new_connections).sum();
    let total_requests = summary.total_requests;
    let reuse_rate = if total_requests > 0 {
      1.0 - (new_connections as f64 / total_requests as f64)
    } else {
      0.0
    };

    let latency = if windows.is_empty() {
      LatencySnapshot::zeroed()
    } else {
      let p50_avg = avg_duration(windows.iter().map(|w| w.p50));
      let p90_avg = avg_duration(windows.iter().map(|w| w.p90));
      let p99_avg = avg_duration(windows.iter().map(|w| w.p99));
      let p99_peak = windows.iter().map(|w| w.p99).max().unwrap_or_default();
      let mean_avg = avg_duration(windows.iter().map(|w| w.mean));
      let min = windows.iter().map(|w| w.p50).min().unwrap_or_default();

      LatencySnapshot { min, max: p99_peak, mean: mean_avg, stddev: Duration::ZERO, p50: p50_avg, p75: p90_avg, p90: p90_avg, p95: p99_avg, p99: p99_avg, p999: p99_avg }
    };

    // TTFB snapshot
    let ttfb = if windows.iter().any(|w| w.ttfb_p99.is_some()) {
      let p99_avg = avg_duration(windows.iter().filter_map(|w| w.ttfb_p99));

      Some(LatencySnapshot {
        min: p99_avg,
        max: windows.iter().filter_map(|w| w.ttfb_p99).max().unwrap_or_default(),
        mean: p99_avg,
        stddev: Duration::ZERO,
        p50: p99_avg,
        p75: p99_avg,
        p90: p99_avg,
        p95: p99_avg,
        p99: p99_avg,
        p999: p99_avg,
      })
    } else {
      None
    };

    Self { total_requests: summary.total_requests, success_count: summary.total_requests.saturating_sub(summary.total_errors), error_count: summary.total_errors, total_duration, requests_per_second: summary.avg_rps, bytes_received: summary.total_bytes, status_counts, latency, ttfb, transfer: None, scheduler_delay: None, connections: ConnectionSnapshot { new_connections, http2_requests: 0, http1_requests: 0, reuse_rate } }
  }
}


fn avg_duration(iter: impl Iterator<Item = Duration>) -> Duration {
  let (sum, count) = iter.fold((Duration::ZERO, 0u64), |(acc, n), d| (acc + d, n + 1));

  if count == 0 {
    Duration::ZERO
  } else {
    sum / count as u32
  }
}