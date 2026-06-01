use std::time::Duration;
use bastion::metrics::histogram::LatencyHistogram;

#[test]
fn test_single_value_all_percentiles_equal() {
    let mut h = LatencyHistogram::new();
    h.record(Duration::from_millis(100));

    assert_eq!(h.percentile(50.0), h.percentile(99.0));
    assert_eq!(h.count(), 1);
}

#[test]
fn test_percentile_ordering_invariant() {
  let mut h = LatencyHistogram::new();
  for ms in [1, 5, 10, 20, 50, 100, 200, 500, 1000] {
    for _ in 0..100 {
      h.record(Duration::from_millis(ms))
    }
  }

  assert!(h.percentile(50.0) <= h.percentile(75.0));
  assert!(h.percentile(75.0) <= h.percentile(90.0));
  assert!(h.percentile(90.0) <= h.percentile(95.0));
  assert!(h.percentile(95.0) <= h.percentile(99.0));
  assert!(h.percentile(99.0) <= h.percentile(99.9));
}

#[test]
fn test_merge_is_equivalent_to_single_histogram() {
  let values_a: Vec<u64> = (1..=100).map(|i| i * 5).collect();
  let values_b: Vec<u64> = (1..=100).map(|i| i * 7).collect(); 

  let mut combined = LatencyHistogram::new();
  for &v in values_a.iter().chain(values_b.iter()) {
    combined.record(Duration::from_millis(v));
  } 

  let mut h_a = LatencyHistogram::new();
  let mut h_b = LatencyHistogram::new();
  for &v in &values_a { h_a.record(Duration::from_millis(v)); }
  for &v in &values_b { h_b.record(Duration::from_millis(v)); }
  h_a.merge(&h_b);

  let p99_combined = combined.percentile(99.0).as_nanos() as f64;
  let p99_merged   = h_a.percentile(99.0).as_nanos() as f64;
  let diff = (p99_combined - p99_merged).abs() / p99_combined;

  assert!(
    diff < 0.001,
    "Merge should be equivalent to combined: combined={:?} merged={:?} diff={:.4}%",
    combined.percentile(99.0),
    h_a.percentile(99.0),
    diff * 100.0
  );
}

#[test]
fn test_saturate_at_max_does_not_panic() {
    let mut h = LatencyHistogram::new();
   
    h.record(Duration::from_secs(3600)); 
    assert_eq!(h.count(), 1);
}

#[test]
fn test_count_matches_records() {
    let mut h = LatencyHistogram::new();
    for i in 0..1000 {
        h.record(Duration::from_millis(i % 100));
    }
    assert_eq!(h.count(), 1000);
}

#[test]
fn test_zero_duration_recorded_without_panic() {
    let mut h = LatencyHistogram::new();
    
    h.record(Duration::ZERO);
    assert_eq!(h.count(), 1);
}