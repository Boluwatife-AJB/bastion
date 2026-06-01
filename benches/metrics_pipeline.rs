use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::hint::black_box;
use std::time::{Duration, Instant};
use bastion::metrics::event::RequestEvent;
use bastion::metrics::window::MetricWindow;
use bastion::http::timing::HttpProtocol;

fn bench_window_record(c: &mut Criterion) {
  let mut group = c.benchmark_group("window_record");
  group.throughput(Throughput::Elements(1));

  group.bench_function("single_event", |b| {
    let mut window = MetricWindow::new(Instant::now(), Duration::from_secs(1));
    let event = RequestEvent {
      completed_at: Instant::now(),
      total_latency: Duration::from_millis(10),
      ttfb: Some(Duration::from_millis(8)),
      status: Some(200),
      bytes: 1024,
      new_connection: false,
      protocol: HttpProtocol::Http1,
      is_warmup: false
    };

    b.iter(|| {
      window.record(black_box(&event));
    })
  });

  group.throughput(Throughput::Elements(1000));
  group.bench_function("1000_events_per_window", |b| {
    b.iter(|| {
      let mut window = MetricWindow::new(Instant::now(), Duration::from_secs(1));
      for ms in 0..1000u64 {
        let event = RequestEvent {
          completed_at: Instant::now(),
          total_latency: Duration::from_millis(ms % 100 +1),
          ttfb: Some(Duration::from_millis(ms % 80 + 1)),
          status: Some(200),
          bytes: 512,
          new_connection: ms == 0,
          protocol: HttpProtocol::Http1,
          is_warmup: false,
        };
        window.record(black_box(&event));
      }
      black_box(window.p99())
    })
  });

  group.finish();
}

fn bench_window_snapshot(c: &mut Criterion) {
  c.bench_function("window_to_snapshot", |b| {
    let mut window = MetricWindow::new(Instant::now(), Duration::from_secs(1));
    for ms in 0..1000u64 {
      let event = RequestEvent {
        completed_at: Instant::now(),
        total_latency: Duration::from_millis(ms % 100 +1),
        ttfb: Some(Duration::from_millis(ms % 80 + 1)),
        status: Some(200),
        bytes: 512,
        new_connection: false,
        protocol: HttpProtocol::Http1,
        is_warmup: false,
      };
      window.record(black_box(&event));
      
    }

    b.iter(|| {
      black_box(
        bastion::metrics::window::WindowSnapshot::from_window(black_box(&window), 0)
      ) 
    })
  });
}

criterion_group!(benches, bench_window_record, bench_window_snapshot);
criterion_main!(benches);