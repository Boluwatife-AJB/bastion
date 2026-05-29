use hdrhistogram::Histogram;
use std::time::Duration;

/// A latency histogram with nanosecond precision up to 60 seconds.
pub struct LatencyHistogram {
  inner: Histogram<u64>,
}

impl LatencyHistogram {
  /// Creates a new histogram tracking nanoseconds with 3 significant figures
  pub fn new() -> Self {
    let inner = Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3).expect("Valid histogram bounds");
    Self {inner}
  }

  /// Record a single latency measurement
  #[inline]
  pub fn record(&mut self, duration: Duration) {
    let nanos = duration.as_nanos() as u64;
    let clamped = nanos.min(self.inner.high());
    let _ = self.inner.record(clamped);
  }

  pub fn percentile(&self, pct: f64) -> Duration {
    let nanos = self.inner.value_at_percentile(pct);
    Duration::from_nanos(nanos)
  }

  pub fn min(&self) -> Duration {
    Duration::from_nanos(self.inner.min())
  }

  pub fn max(&self) -> Duration {
    Duration::from_nanos(self.inner.max())
  }

  pub fn mean(&self) -> Duration {
    Duration::from_nanos(self.inner.mean() as u64)
  }

  pub fn stddev(&self) -> Duration {
    Duration::from_nanos(self.inner.stdev() as u64)
  }

  pub fn count(&self) -> u64 {
    self.inner.len()
  }

  pub fn merge(&mut self, other: &LatencyHistogram) {
    self.inner.add(&other.inner).expect("Compatible histogram can always merge");
  }
}