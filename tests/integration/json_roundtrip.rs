use std::time::Duration;
use bastion::metrics::snapshot::{MetricsSnapshot, LatencySnapshot, ConnectionSnapshot};
use bastion::reporter::json::{write_json_report, load_baseline};

fn make_test_snapshot() -> MetricsSnapshot {
    MetricsSnapshot {
        total_requests: 1000,
        success_count: 995,
        error_count: 5,
        total_duration: Duration::from_secs(10),
        requests_per_second: 100.0,
        bytes_received: 1_048_576,
        status_counts: {
            let mut m = std::collections::HashMap::new();
            m.insert(200u16, 995);
            m.insert(500u16, 5);
            m
        },
        latency: LatencySnapshot {
            min:    Duration::from_millis(2),
            max:    Duration::from_millis(500),
            mean:   Duration::from_millis(45),
            stddev: Duration::from_millis(30),
            p50:    Duration::from_millis(38),
            p75:    Duration::from_millis(60),
            p90:    Duration::from_millis(90),
            p95:    Duration::from_millis(120),
            p99:    Duration::from_millis(200),
            p999:   Duration::from_millis(450),
        },
        ttfb: None,
        transfer: None,
        scheduler_delay: None,
        connections: ConnectionSnapshot::default(),
    }
}

#[test]
fn test_json_round_trip_preserves_latency_values() {
  let original = make_test_snapshot();

  // Serialize to JSON
  let mut buf = Vec::new();
  write_json_report(&original, &[], None, &mut buf)
    .expect("Serialization should succeed");

  let json_str = String::from_utf8(buf).expect("Valid UTF-8");
  assert!(!json_str.is_empty());
  assert!(json_str.contains("\"total_requests\""));

  let tmp = tempfile::NamedTempFile::new()
    .expect("Should create temp file");
  write_json_report(&original, &[], None,  &mut tmp.reopen().unwrap())
    .expect("Write should succeed");

  let reloaded = load_baseline(tmp.path())
    .expect("Should reload saved report");

  assert_eq!(reloaded.total_requests, original.total_requests);
  assert_eq!(reloaded.error_count, original.error_count);
  assert_eq!(reloaded.latency.p99, original.latency.p99);
  assert_eq!(reloaded.latency.p50, original.latency.p50);
  assert_eq!(
    (reloaded.requests_per_second * 100.0).round(),
    (original.requests_per_second * 100.0).round(),
    "RPS should survive round-trip"
  );
}

#[test]
fn test_json_output_contains_windows() {
  use bastion::metrics::windows::WindowSnapshot;
  use std::time::Instant;

  let snapshot = make_test_snapshot();
  let windows = vec![
    WindowSnapshot {
      window_index: 0,
      start: Instant::now(),
      rps: 95.0,
      throughput_bps: 102400.0,
      error_rate: 0.01,
      request_count: 95,
      error_count: 1,
      bytes_received: 102400,
      new_connections: 5,
      p50: Duration::from_millis(38),
      p90: Duration::from_millis(80),
      p99: Duration::from_millis(150),
      mean: Duration::from_millis(45),
      ttfb_p99: None,
    }
  ];

  let mut buf = Vec::new();
  write_json_report(&snapshot, &windows, None, &mut buf).expect("Serialization should succeed");

  let json: serde_json::Value = serde_json::from_slice(&buf).expect("Should be valid JSON");

  assert!(json["windows"].is_array());
  assert_eq!(json["windows"].as_array().unwrap().len(), 1);
  assert_eq!(json["windows"][0]["second"], 0);
}