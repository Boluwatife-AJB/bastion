use std::time::Duration;
use crate::metrics::snapshot::MetricsSnapshot;
use crate::error::{BastionError, Result};

#[derive(Debug, Clone)]
pub enum Threshold {
  p99Lt(Duration),
  p95Lt(Duration),
  p90Lt(Duration),
  p50Lt(Duration),
  MeanLt(Duration),
  ErrorRateLt(Duration),
  RpsGt(f64),
}

#[derive(Debug)]
pub struct ThresholdResult {
  pub threshold: Threshold,
  pub passed: bool,
  pub actual: String,
  pub expected: String,
}