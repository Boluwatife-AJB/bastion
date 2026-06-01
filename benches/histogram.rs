use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;
use std::hint::black_box;
use bastion::metrics::histogram::LatencyHistogram;

fn bench_record(c: &mut Criterion) {
  let mut h = LatencyHistogram::new();

  c.bench_function("histogram_record_1ms", |b| {
    b.iter(|| {
      h.record(black_box(Duration::from_millis(1)));
    });
  });

  c.bench_function("histogram_record_100ms", |b| {
    b.iter(|| {
      h.record(black_box(Duration::from_millis(100)))
    })
  });

  c.bench_function("histogram_record_realistic", |b| {
    let mut i = 0u64;
    b.iter(|| {
      let ms = match i % 100 {
        0..=89 => 5,
        90..=98 => 50,
        _ => 200,
      };
      h.record(black_box(Duration::from_millis(ms)));
      i += 1;
    })
  });
}

fn bench_percentile(c: &mut Criterion) {
  let mut h = LatencyHistogram::new();
  for i in 0..10_000 {
    let ms = (i % 200) + 1;
    h.record(Duration::from_millis(ms))
  }

  c.bench_function("histogram_p99", |b| {
    b.iter(|| {
      black_box(h.percentile(99.0))
    })
  });

  c.bench_function("histogram_p50", |b| {
    b.iter(|| {
      black_box(h.percentile(50.0))
    })
  });

  c.bench_function("histogram_all_percentiles", |b| {
    b.iter(|| {
      let _ = black_box(h.percentile(50.0));
      let _ = black_box(h.percentile(75.0));
      let _ = black_box(h.percentile(90.0));
      let _ = black_box(h.percentile(95.0));
      let _ = black_box(h.percentile(99.0));
      let _ = black_box(h.percentile(99.9));
    })
  });
}

fn bench_merge(c: &mut Criterion) {
  let mut group = c.benchmark_group("histogram_merge");

  for shards in [2, 10, 50, 100] {
    group.bench_with_input(
      BenchmarkId::from_parameter(shards), 
      &shards, 
    |b, &shards| {
      b.iter(|| {
        let mut histograms: Vec<LatencyHistogram> = (0..shards).map(|_| {
          let mut h = LatencyHistogram::new();
          for ms in 1..=100u64 {
            h.record(Duration::from_millis(ms))
          }
          h
        }).collect();

        let mut merged = histograms.remove(0);
        for h in histograms {
          merged.merge(&h);
        }
        black_box(merged.percentile(99.0))
      })
    });
  }

  group.finish();
}

criterion_group!(benches, bench_record, bench_percentile, bench_merge);
criterion_main!(benches);