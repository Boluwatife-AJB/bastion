// //! Shared test infrastructure

use std::time::Duration;
use wiremock::{MockServer, ResponseTemplate, Mock};
use wiremock::matchers::{method, path};
use crate::config::{Config, HttpMethod, OutputFormat, RunMode};

pub struct TestServer {
  pub server: MockServer,
}

impl TestServer { 
  /// Start a mock server with a simple 200 OK response
  pub async fn start_ok() -> Self {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
      .respond_with(ResponseTemplate::new(200).set_body_string("OK")).insert_header("content-type", "text/plain").mount(&server).await;

    Self { server }
  }

  /// Start a mock server that responds slowly (simulates latency)
  pub async fn start_slow(delay: Duration) -> Self {
    let server = MockServer::start().await;

    Mock::given(method("GET")).respond_with(ResponseTemplate::new(200).set_delay(delay).set_body_string("slow")).mount(&server).await;

    Self { server }
  }

  /// Start a mock server that returns 500 for all requests
  pub async fn start_error() -> Self {
    let server = MockServer::start().await;

    Mock::given(method("GET")).respond_with(ResponseTemplate::new(500).set_body_string("error")).mount(&server).await;

    Self { server }
  }

  pub async fn start_degrading(ok_count: u64) -> Self {
    let server = MockServer::start().await;

    Mock::given(method("GET")).respond_with(ResponseTemplate::new(200).set_body_string("ok")).up_to_n_times(ok_count).mount(&server).await;

    // Remaining request will fail
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(500).set_body_string("overloaded")).mount(&server).await;

    Self { server }
  }

  pub fn url(&self) -> String {
    self.server.uri()
  }
}

/// Minimal Config for testing
pub fn test_config(url: &str, requests: u64, concurrency: usize) -> Config {
  Config {
    url: url.to_string(),
    run_mode: RunMode::RequestCount(requests),
    concurrency,
    rate_limit: None,
    timeout: Duration::from_secs(5),
    method: HttpMethod::Get,
    body: None,
    headers: std::collections::HashMap::new(),
    output_format: OutputFormat::Terminal,
    output_file: None,
    progress: false,
    keep_alive: true,
    follow_redirects: true,
    warmup: None,
    channel_capacity: concurrency * 2,
    http2: false,
    assertions: vec![],
    compare: None,
    regression_threshold: 0.10,
    csv_file: None,
  }
} 

#[macro_export]
macro_rules! assert_duration_gt {
  ($dur:expr, $min:expr, $msg:expr) => {
    assert_eq!($dur > $min, "{}: expected > {:?}, got {:?}", $msg, $min, $dur);
  };
}

#[macro_export]
macro_rules! assert_approximately_equal {
  ($a:expr, $b:expr, $tolerance:expr) => {
    let diff = (($a as f64) - ($b as f64)).abs();
    let max = ($a as f64).max($b as f64);
    
    assert!((diff / max) < $tolerance, "Expected {} ≈ {} (tolerance: {}) diff was {:.1}", $a, $b, $tolerance, (diff / max) * 100.0);
  };
}