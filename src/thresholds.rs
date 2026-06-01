use std::time::Duration;
use crate::metrics::snapshot::MetricsSnapshot;
use crate::error::{BastionError, Result};

#[derive(Debug, Clone)]
pub enum Threshold {
  P99Lt(Duration),
  P95Lt(Duration),
  P90Lt(Duration),
  P50Lt(Duration),
  MeanLt(Duration),
  ErrorRateLt(f64),
  RpsGt(f64),
}

#[derive(Debug)]
pub struct ThresholdResult {
  pub threshold: Threshold,
  pub passed: bool,
  pub actual: String,
  pub expected: String,
}

impl Threshold {
    pub fn evaluate(self, snapshot: &MetricsSnapshot) -> ThresholdResult {
      match self {
          Threshold::P99Lt(limit) => {
            let actual = snapshot.latency.p99;
            ThresholdResult {
              passed: actual < limit,
              actual: fmt_duration(actual),
              expected: format!("< {}", fmt_duration(limit)),
              threshold: self.clone(),
            }
          }
          Threshold::P95Lt(limit) => {
            let actual = snapshot.latency.p95;
            ThresholdResult {
              passed: actual < limit,
              actual: fmt_duration(actual),
              expected: format!("< {}", fmt_duration(limit)),
              threshold: self.clone(),
            }
          }
          Threshold::P90Lt(limit) => {
            let actual = snapshot.latency.p90;
            ThresholdResult {
              passed: actual < limit,
              actual: fmt_duration(actual),
              expected: format!("< {}", fmt_duration(limit)),
              threshold: self.clone(),
            }
          }
          Threshold::P50Lt(limit) => {
            let actual = snapshot.latency.p50;
            ThresholdResult {
              passed: actual < limit,
              actual: fmt_duration(actual),
              expected: format!("< {}", fmt_duration(limit)),
              threshold: self.clone(),
            }
          }
          Threshold::MeanLt(limit) => {
            let actual = snapshot.latency.mean;
            ThresholdResult {
              passed: actual < limit,
              actual: fmt_duration(actual),
              expected: format!("< {}", fmt_duration(limit)),
              threshold: self.clone(),
            }
          }
          Threshold::ErrorRateLt(limit) => {
            let total = snapshot.total_requests as f64;
            let actual_rate = if total > 0.0 { snapshot.error_count as f64 / total } else { 0.0 };
            ThresholdResult {
              passed: actual_rate < limit,
              actual: format!("{:.3}%", actual_rate * 100.0),
              expected: format!("< {:.3}%", limit * 100.0),
              threshold: self.clone(),
            }
          }
          Threshold::RpsGt(limit) => {
            let actual = snapshot.requests_per_second;
            ThresholdResult {
              passed: actual > limit,
              actual: format!("{actual:.2} req/s"),
              expected: format!("> {limit:.2} req/s",),
              threshold: self.clone(),
            }
          }
      }
    }

    pub fn label(&self) -> &'static str {
      match self {
        Threshold::P99Lt(_) => "p99 latency",
        Threshold::P95Lt(_) => "p95 latency",
        Threshold::P90Lt(_) => "p90 latency",
        Threshold::P50Lt(_) => "p50 latency",
        Threshold::MeanLt(_) => "mean latency",
        Threshold::ErrorRateLt(_) => "error rate",
        Threshold::RpsGt(_) => "throughput",
      }
    }
}

pub fn parse_threshold(s: &str) -> Result<Threshold> {
  let s = s.trim();

  // Try each pattern
  if let Some(val) = s.strip_prefix("p99<") {
    return Ok(Threshold::P99Lt(parse_duration(val)?));
  }
  if let Some(val) = s.strip_prefix("p95<") {
    return Ok(Threshold::P95Lt(parse_duration(val)?));
  }
  if let Some(val) = s.strip_prefix("p90<") {
    return Ok(Threshold::P90Lt(parse_duration(val)?));
  }
  if let Some(val) = s.strip_prefix("p50<") {
    return Ok(Threshold::P50Lt(parse_duration(val)?));
  }
  if let Some(val) = s.strip_prefix("mean<") {
    return Ok(Threshold::MeanLt(parse_duration(val)?));
  }
  if let Some(val) = s.strip_prefix("error-rate<") {
    let pct: f64 = val.trim_end_matches('%').parse().map_err(|_| BastionError::InvalidConfig(format!("Invalid error rate: '{}': expected percentage", val)))?;
    return Ok(Threshold::ErrorRateLt(pct / 100.0));
  }
  if let Some(val) = s.strip_prefix("rps>") {
    let rps: f64 = val.parse().map_err(|_| BastionError::InvalidConfig(format!("Invalid RPS: '{}': expected number", val)))?;
    return Ok(Threshold::RpsGt(rps));
  }

  Err(BastionError::InvalidConfig(format!("Unknown threshold format: '{s}'. expected pattern like 'p99<100ms', 'rps>1000', error-rate<1%', 'mean<50ms'")))
}

fn parse_duration(s: &str) -> Result<Duration> {
  humantime::parse_duration(s).map_err(|e| BastionError::InvalidConfig(format!("Invalid duration: '{}': {}", s, e)))
}

fn fmt_duration(d: Duration) -> String {
  let ms = d.as_millis();
  if ms == 0 {
    format!("{}μs", d.as_micros())
  } else {
    format!("{}ms", ms)
  }
}
