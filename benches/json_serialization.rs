use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::hint::black_box;
use std::time::{Duration, Instant};
use bastion::metrics::window::WindowSnapshot;

fn make_windows(count: usize) -> Vec<WindowSnapshot> {
  (0..count).map(|i| WindowSnapshot {
    window_index: i as u64,
    start: Instant::now(),
    rps: 100.0 + (1 % 20) as f64,
    throughput_bps: 102400.0,
    error_rate: 0.001,
    request_count: 100,
    error_count: 0,
    bytes_received: 102400,
    new_connections: 0,
    p50: Duration::from_millis(38),
    p50: Duration::from_millis(80),
    p50: Duration::from_millis(150),
    mean: Duration::from_millis(45),
    ttfb_p99: Some(Duration::from_millis(30)),
  }).collect()
}

fn bench_json_serialization(c: &mut Criterion) {
  let mut group = c.benchmark_group("json_output");

  for window_count in [60, 300, 3600 ] {
    let windows = make_windows(window_count);
    group.throughput(Throughput::Elements(window_count as u64));

    group.bench_with_input(
      criterion::BenchmarkId::from_parameter(
        format!("{window_count}_windows")
      ), &windows, |b, windows| {
        b.iter(|| {
          let mut buf = Vec::new();
          let snapshot = make_snapshot();
          bastion::reporter::json::write_json_report(
            black_box(&snapshot),
            black_box(windows),
            None,
            &mut buf,
          ).unwrap();
          black_box(buf)
        })
      });
  }

  group.finish();
}

fn make_snapshot() -> bastion::metrics::snapshot::MetricSnapshot {
  use bastion::metrics::snapshot::*;
  MetricsSnapshot {
    total_requests: 10000,
    success_count: 9995, 
    error_count: 5,
    total_duration: Duration::from_secs(100),
    requests_per_second: 100.0,
    bytes_received: 10_486_760,
    status_counts: Default::default(),
    latency: LatencySnapshot {
      min: Duration::from_millis(2),
      max: Duration::from_millis(400),
      mean: Duration::from_millis(45),
      stddev: Duration::from_millis(20),
      p50: Duration::from_millis(38),
      p75: Duration::from_millis(55),
      p90: Duration::from_millis(80),
      p95: Duration::from_millis(110),
      p99: Duration::from_millis(180),
      p999: Duration::from_millis(380),
    },
    ttfb: None,
    transfer: None,
    scheduler_delay: None,
    connections: ConnectionSnapshot::default()
  }
}

criterion_group!(benches, bench_json_serialization);
criterion_main!(benches);