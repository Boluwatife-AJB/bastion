mod common;
use common::{TestServer, test_config};

#[tokio::test]
async fn test_exact_request_count_small() {
  let server = TestServer::start_ok().await;
  let config = test_config(&server.url(), 10, 2);

  let (snapshot, _windows) = bastion::engine::run(config).await.expect("Engine should not fail");

  assert_eq!(
    snapshot.total_requests, 10,
    "Should send exactly 10 requests"
  );

  assert_eq!(
    snapshot.total_responses, 10,
    "Should receive exactly 10 responses"
  );

  assert_eq!(
    snapshot.total_errors, 0,
    "Should not have any errors"
  );
}


#[tokio::test]
async fn test_exact_request_count_large() {
  let server = TestServer::start_ok().await;
  let config = test_config(&server.url(), 500, 20);

  let (snapshot, _windows) = bastion::engine::run(config).await.expect("Engine should not fail");

  assert_eq!(snapshot.total_requests, 500, "Should send exactly 500 requests");
  assert_eq!(snapshot.success_count, 500,);
  assert_eq!(snapshot.error_count, 0, "Should not have any errors");
}

#[tokio::test]
async fn test_request_count_equals_concurrency() {
  // Edge cases where exactly one request per worker is sent
  let server = TestServer::start_ok().await;
  let config = test_config(&server.url(), 10, 10);

  let (snapshot, _windows) = bastion::engine::run(config).await.expect("Engine should not fail");

  assert_eq!(snapshot.total_requests, 10);
}

#[tokio::test]
async fn test_request_count_less_than_concurrency() {
  // Edge cases where fewer requests than workers are sent and some workers get nothing
  let server = TestServer::start_ok().await;
  let config = test_config(&server.url(), 3, 10);

  let (snapshot, _windows) = bastion::engine::run(config).await.expect("Engine should not fail");

  assert_eq!(snapshot.total_requests, 3);
}

#[tokio::test]
async fn test_error_count_accurate() {
  let server = TestServer::start_failing().await;
  let config = test_config(&server.url(), 50, 5);

  let (snapshot, _windows) = bastion::engine::run(config).await.expect("Engine should not fail even when server errors");

  assert_eq!(snapshot.total_requests, 50, "Should have 50 requests");
  // 500 responses are counted as errors (status >=400)
  assert_eq!(snapshot.error_count, 50, "Should have 50 errors");
  assert_eq!(snapshot.success_count, 0, "Should have 0 successes");
}

#[tokio::test]
async fn test_partial_error_count() {
  let server = TestServer::start_degrading(30).await;
  let config = test_config(&server.url(), 50, 5);

  let (snapshot, _windows) = bastion::engine::run(config).await.expect("Engine should not fail");

  assert_eq!(snapshot.total_requests, 50, "Should have 50 requests");
  assert_eq!(snapshot.error_count + snapshot.success_count, 50);
}