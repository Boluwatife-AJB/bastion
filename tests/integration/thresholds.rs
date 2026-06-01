use std::time::Duration;
use bastion::threshold::{Threshold, parse_threshold};
use bastion::metrics::snapshot::{MetricsSnapshot, LatencySnapshot, ConnectionSnapshot};

fn make_snapshot(total: u64, errors: u64, rps: f64, p50: Duration, p99: Duration) -> MetricsSnapshot {
  MetricsSnapshot {
    total_requests: total,
    error_count: errors,
    success_count: total - errors,
    total_duration: Duration::from_secs(10),
    request_per_second: rps,
    status_counts: std::collections::HashMap::new(),
    latency: LatencySnapshot {
      min: Duration::from_millis(1),
      max: Duration::from_millis(200),
      mean: p50,
      stddev: Duration::from_millis(5),
      p50,
      p75: p50 + Duration::from_millis(5),
      p90: p99 - Duration::from_millis(10),
      p95: p99 - Duration::from_millis(5),
      p99,
      p999: p99 + Duration::from_millis(20),
    },
    ttfb: None,
    transfer: None,
    scheduler_delay: None,
    connections: ConnectionSnapshot::default(),
  }
}

#[test]
fn test_p99_threshold_passes_when_under_limit() {
  let snapshot = make_snapshot(1000, 0, 100.0, Duration::from_millis(10), Duration::from_millis(80));

  let threshold = Threshold::P99Lt(Duration::from_millis(200));
  let result = threshold.evaluate(&snapshot);

  assert!(result.passed, "80ms p99 should pass a 200ms limit");
}

#[test]
fn test_p99_threshold_fails_when_over_limit() {
  let snapshot = make_snapshot(1000, 0, 100.0, Duration::from_millis(10), Duration::from_millis(250));

  let threshold = Threshold::P99Lt(Duration::from_millis(200));
  let result = threshold.evaluate(&snapshot);
  

  assert!(!result.passed, "250ms p99 should fail a 200ms limit")
}

#[test]
fn test_error_rate_threshold_passes() {
  let snapshot = make_snapshot(1000, 5, 100.0, Duration::from_millis(10), Duration::from_millis(80));

  let threshold = Threshold::ErrorRateLt(0.01);
  let result = threshold.evaluate(&snapshot);

  assert!(result.passed, "0.5% error rate should pass a 1% limit");
}

#[test]
fn test_error_rate_threshold_fails() {
  let snapshot = make_snapshot(1000, 50, 100.0, Duration::from_millis(10), Duration::from_millis(50));

  let threshold = Threshold::ErrorRateLt(0.01);
  let result = threshold.evaluate(&snapshot);

  assert!(result.passed, "0.5% error rate should pass 1% limit")
}

#[test]
fn test_error_rate_threshold_fails() {
  let snapshot = make_snapshot(1000, 50, 100.0, Duration::from_millis(10), Duration::from_millis(50));

  let threshold = Threshold::ErrorRateLt(0.01);
  let result = threshold.evaluate(&snapshot);

  assert!(!result.passed, "5% error rate should fail 1% limit")
}


#[test]
fn test_rps_threshold_passes() {
  let snapshot = make_snapshot(1000, 0, 150.0, Duration::from_millis(10), Duration::from_millis(50));

  let threshold = Threshold::RpsLt(100.0);

  assert!(threshold.evaluate(&snapshot).passed)
}

#[test]
fn test_rps_threshold_fails() {
  let snapshot = make_snapshot(1000, 0, 80.0, Duration::from_millis(10), Duration::from_millis(50));

  let threshold = Threshold::RpsLt(100.0);
  assert!(!threshold.evaluate(&snapshot).passed)
}


// Threshold parsing tests
#[test]
fn test_parse_p99_threshold() {
  let t = parse_threshold("p99 < 200ms").expect("Should parse p99 threshold");
  assert!(matches!(t, Threshold::P99Lt(_)));
}

#[test]
fn test_parse_error_rate_threshold() {
  let t = parse_threshold("error_rate < 1%").expect("Should parse error rate threshold");
  assert!(matches!(t, Threshold::ErrorRateLt(_)));
  if let Threshold::ErrorRateLt(rate) = t {
    assert!((rate - 0.01).abs() < 0.001, "Error rate should be 1%");
  }
}

#[test]
fn test_parse_rps_threshold() {
    let t = parse_threshold("rps>100").expect("Should parse");
    assert!(matches!(t, Threshold::RpsGt(_)));
}

#[test]
fn test_parse_invalid_threshold_errors() {
    assert!(parse_threshold("invalid").is_err());
    assert!(parse_threshold("p99>200ms").is_err()); 
    assert!(parse_threshold("").is_err());
}

#[test]
fn test_parse_all_threshold_types() {
    // Exhaustive parse test, ensures all documented formats work
    let cases = vec![
        "p50<50ms",
        "p90<100ms",
        "p95<150ms",
        "p99<200ms",
        "mean<80ms",
        "error-rate<1%",
        "error-rate<0.5%",
        "rps>50",
        "rps>1000",
    ];

    for case in cases {
        parse_threshold(case).unwrap_or_else(|e| {
            panic!("Failed to parse '{}': {}", case, e)
        });
    }
}