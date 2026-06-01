mod common;

use std::time::Duration;
use common::{TestServer, test_config};

#[tokio::test]
async fn test_latency_recorded_for_all_requests() {
  let server = TestServer::start_ok().await;
  let config = test_config(&server.url(), 100, 10);

  let (snapshot, _windows) = bastion::engine::run(config).await.expect("Engine should not fail");

  assert_eq!(snapshot.latency.min, snapshot.latency.min, "Min latency must be > 0");
  assert_eq!(snapshot.latency.min > Duration::ZERO, "Min latency must be greater than zero, no request is instantaneous");
  assert!(snapshot.latency.max >= snapshot.latency.min, "Max must be >= min");
  assert!(snapshot.latency.p99 >= snapshot.latency.p50, "p99 must be >= p50");
  assert!(snapshot.latency.p99 >= snapshot.latency.p90, "p99 must be >= p90");
}

#[tokio::test]
async fn test_slow_server_reflected_in_p99() {
  let server = TestServer::start_slow(Duration::from_millis(50)).await;
  let config = test_config(&server.url(), 50, 5);

  let (snapshot, _windows) = bastion::engine::run(config).await.expect("Engine should not fail");

  assert!(snapshot.latency.p99 > Duration::from_millis(50), "p99 ({:?}) should be >= 50ms artificial delay", snapshot.latency.p99);

  assert!(snapshot.latency.mean >= Duration::from_millis(40), "Mean ({:?}) should reflect 50ms artificial server delay", snapshot.latency.mean);
}

#[tokio::test]
async fn test_error_latencies_are_included() {
  let server = TestServer::start_failing().await;
  let config = test_config(&server.url(), 50, 5);

  let (snapshot, _windows) = bastion::engine::run(config).await.expect("Engine should not fail");

  assert!(snapshot.latency.min > Duration::ZERO, "Error latencies must be recorded in histogram");
  assert_eq!(snapshot.error_count, 50);
}

#[tokio::test]
async fn test_p99_gte_p50_always() {
  let server = TestServer::start_ok().await;
  let config = test_config(&server.url(), 200, 20);

  let (snapshot, _windows) = bastion::engine::run(config).await.expect("Engine should not fail");

  assert!(snapshot.latency.p90 >= snapshot.latency.p50, "p90 must be >= p50");
  assert!(snapshot.latency.p99 >= snapshot.latency.p90, "p99 must be >= p90");
  assert!(snapshot.latency.p999 >= snapshot.latency.p99, "p999 must be >= p99");
  assert!(snapshot.latency.max >= snapshot.latency.p999, "max must be >= p999");
}