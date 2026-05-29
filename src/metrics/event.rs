use std::time::{Duration, Instant};
use crate::http::timing::HttpProtocol;

#[derive(Debug, Clone)]
pub struct RequestEvent {
  pub completed_at: Instant,
  pub total_latency: Duration,
  pub ttfb: Option<Duration>,
  pub status: Option<u16>,
  pub bytes: u64,
  pub new_connection: bool,
  pub protocol: HttpProtocol,
  pub is_warmup: bool
}

impl RequestEvent {
    pub fn is_success(&self) -> bool {
      self.status.map(|s| s < 400).unwrap_or(false) 
    }

    pub fn is_server_error(&self) -> bool {
      self.status.map(|s| s >= 500).unwrap_or(false)
    }
}