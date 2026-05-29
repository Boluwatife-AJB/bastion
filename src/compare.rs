use std::time::Duration;
use serde::Serialize;
use crate::metrics::snapshot::MetricsSnapshot;

#[derive(Debug, Serialize)]
pub struct ComparisonReport {
  pub baseline_requests: u64,
  pub current_requests: u64,

  pub rps_delta_pct: f64,
  pub p50_delta_pct: f64,
  pub p90_delta_pct: f64,
  pub p99_delta_pct: f64,
  pub error_rate_delta_pct: f64,

  pub rps_regressed: bool,
  pub p99_regressed: bool,
  pub error_rate_regressed: bool,

  pub  verdicts:Vec<ComparisonVerdict>,
}

#[derive(Debug, Serialize)]
pub struct ComparisonVerdict {
  pub metric: String,
  pub baseline: String,
  pub current: String,
  pub delta_pct: f64,
  pub status: VerdictStatus,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VerdictStatus {
  Stable,
  Regressed,
  Improved,
}

impl ComparisonReport {
  pub fn compare(baseline: &MetricsSnapshot, current: &MetricsSnapshot, regression_threshold: f64) -> Self {
    let rps_delta = pct_change(baseline.requests_per_second, current.requests_per_second);
    let p50_delta = duration_pct_change(baseline.latency.p50, current.latency.p50);
    let p90_delta = duration_pct_change(baseline.latency.p90, current.latency.p90);
    let p99_delta = duration_pct_change(baseline.latency.p99, current.latency.p99);


    let baseline_error_rate = error_rate(baseline);
    let current_error_rate = error_rate(current);
    let error_rate_delta = pct_change(baseline_error_rate, current_error_rate);

    // For latency: positive delta
    let p99_regressed = p99_delta > regression_threshold * 100.0;
    let rps_regressed = rps_delta < -(regression_threshold * 100.0);
    let error_rate_regressed = error_rate_delta > regression_threshold * 100.0;

    let mut verdicts = Vec::new();
    
    verdicts.push(ComparisonVerdict {
      metric: "RPS".into(),
      baseline: format!("{:.2} req/s", baseline.requests_per_second),
      current: format!("{:.2} req/s", current.requests_per_second),
      delta_pct: rps_delta,
      status: classify_rps(rps_delta, regression_threshold),
    });

    verdicts.push(ComparisonVerdict {
      metric: "P50 latency".into(),
      baseline: fmt_dur(baseline.latency.p50),
      current: fmt_dur(current.latency.p50),
      delta_pct: p50_delta,
      status: classify_latency(p50_delta, regression_threshold),
    });
    
    verdicts.push(ComparisonVerdict {
      metric: "P90 latency".into(),
      baseline: fmt_dur(baseline.latency.p90),
      current: fmt_dur(current.latency.p90),
      delta_pct: p90_delta,
      status: classify_latency(p90_delta, regression_threshold),
    });
    
    verdicts.push(ComparisonVerdict {
      metric: "P99 latency".into(),
      baseline: fmt_dur(baseline.latency.p99),
      current: fmt_dur(current.latency.p99),
      delta_pct: p99_delta,
      status: classify_latency(p99_delta, regression_threshold),
    });
    
    verdicts.push(ComparisonVerdict {
      metric: "Error Rate".into(),
      baseline: format!("{:.3}%", baseline_error_rate * 100.0),
      current: format!("{:.3}%", current_error_rate * 100.0),
      delta_pct: error_rate_delta,
      status: classify_error_rate(error_rate_delta, regression_threshold),
    });
    
    Self {
      baseline_requests: baseline.total_requests,
      current_requests: current.total_requests,
      rps_delta_pct: rps_delta,
      p50_delta_pct: p50_delta,
      p90_delta_pct: p90_delta,
      // p95_delta_pct: 0.0,
      p99_delta_pct: p99_delta,
      error_rate_delta_pct: error_rate_delta,
      rps_regressed,
      p99_regressed,
      error_rate_regressed,
      verdicts,
    }
  }

  pub fn any_regression(&self) -> bool {
    self.rps_regressed || self.p99_regressed || self.error_rate_regressed
  }
}

fn pct_change(baseline: f64, current: f64) -> f64 {
  if baseline == 0.0 {
    return 0.0;
  }
  ((current - baseline) / baseline) * 100.0
}

fn duration_pct_change(baseline: Duration, current: Duration) -> f64 {
  pct_change(baseline.as_secs_f64(), current.as_secs_f64())
}

fn error_rate(s: &MetricsSnapshot) -> f64 {
  if s.total_requests == 0 {
    return 0.0;
  }
  s.error_count as f64 / s.total_requests as f64
}

fn classify_latency(delta_pct: f64, threshold: f64) -> VerdictStatus {
  let t = threshold * 100.0;
  if delta_pct > t {
    VerdictStatus::Regressed
  } else if delta_pct < -5.0 {
    VerdictStatus::Improved
  } else {
    VerdictStatus::Stable
  }
}

fn classify_rps(delta_pct: f64, threshold: f64) -> VerdictStatus {
  let t = threshold * 100.0;
  if delta_pct < -t {
    VerdictStatus::Regressed
  } else if delta_pct > 5.0 {
    VerdictStatus::Improved
  } else {
    VerdictStatus::Stable
  }
}

fn classify_error_rate(delta_pct: f64, threshold: f64) -> VerdictStatus {
  let t = threshold * 100.0;
  if delta_pct > t {
    VerdictStatus::Regressed
  } else if delta_pct < -5.0 {
    VerdictStatus::Improved
  } else {
    VerdictStatus::Stable
  }
}

fn fmt_dur(d: Duration) -> String {
  format!("{:.1}ms", d.as_secs_f64() * 1000.0)
}