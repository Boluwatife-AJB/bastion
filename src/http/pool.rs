use std::sync::Arc;
use std::time::Duration;

use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use http_body_util::Full;
use bytes::Bytes;

use crate::config::Config;
use crate::http::connector::MeteredConnector;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct PoolConfig {
  pub max_idle_per_host: usize,
  pub idle_timeout: Duration,
  pub connection_timeout: Duration,
  pub http2: bool,
  pub http2_max_concurrent: Option<u32>,
}

impl PoolConfig {
  pub fn from_config(config: &Config) -> Self {
    Self { max_idle_per_host: config.concurrency, idle_timeout: Duration::from_secs(90), connection_timeout: config.timeout, http2: config.http2, http2_max_concurrent: None }
  }
}


pub type BastionClient = Client<MeteredConnector, Full<Bytes>>;

pub fn build_hyper_client(pool_config: &PoolConfig) -> Result<Arc<BastionClient>> {
  let connector = MeteredConnector::new();

  let mut builder = Client::builder(TokioExecutor::new());

  // Pool configuration
  builder.pool_max_idle_per_host(pool_config.max_idle_per_host).pool_idle_timeout(pool_config.idle_timeout);

  if pool_config.http2 {
    builder.http2_only(true);
  }

  let client = builder.build(connector);

  Ok(Arc::new(client))
}