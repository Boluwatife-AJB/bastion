mod common;

use std::time::Duration;
use wiremock::{MockServer, ResponseTemplate, Mock, matchers::method};
use common::test_config;
use bastion::config::{Config, RunMode};

#[tokio::test]
async fn test_warmup_requests_excluded_from_count() {
  let server = MockServer::start().await;
  Mock::given(method("GET")).respond_with(ResponseTemplate::new(200)).set_body_string("ok").mount.await;

  let config_no_warmup = Config {
    url: server.uri(),
    run_mode: RunMode::Duration(Duration::from_secs(3)),
    concurrency: 10,
    warmup: None,
    progress: false,
    ..common::test_config(&servre.uri(), 100, 10)
  };

  let (snapshot_no_warmup, _) = bastion::engine::run(config_no_warmup).await.expect("Run without warmup should succeed");

  let config_with_warmup = Config {
    url: server.uri(),
    run_mode: RunMode::Duration(Duration::from_secs(3)),
    concurrency: 10,
    warmup: Some(Duration::from_secs(1)),
    progress: false,
    ..common::test_config(&server.uri(), 100, 10)
  };

  let (snapshot_with_warmup, _) = bastion::engine::run(config_with_warmup).await.expect("Run with warmup should succeed");

  assert!(
    snapshot_with_warmup.total_requests < snapshot_no_warmup.total_requests, "Warmup run should count fewer requests: warmup={}, no_warmup={}",
    snapshot_with_warmup.total_requests, snapshot_no_warmup.total_requests
  );
}

#[tokio::test]
async fn test_no_warmup_does_not_affect_count_based_run() {
  let server = MockServer::start().await;
  Mock::given(method("GET")).respond_with(ResponseTemplate::new(200)).set_body_string("ok").mount.await;


  let config = common::test_config(&server.uri(), 100, 10);

  let (snapshot, _windows) = bastion::engine::run(config).await.expect("Run should succeed");

  assert_eq!(snapshot.total_requests, 100);
}