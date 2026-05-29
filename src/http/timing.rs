use std::time::{Duration, Instant};
use serde::Serialize;

/// Full timing breakdown for a single HTTP request
#[derive(Debug, Clone, Default)]
pub struct RequestTiming {
  pub scheduled_at: Option<Instant>,
  pub send_start: Option<Instant>,
  pub first_byte_at: Option<Instant>,
  pub last_byte_at: Option<Instant>,
  pub status: Option<u16>,
  pub response_bytes: u64,
  pub connection_reused: bool,
  pub protocol: HttpProtocol,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub enum HttpProtocol {
  #[default]
  Http1,
  Http2,
  Unknown,
}

impl RequestTiming {
  /// Total latency from scheduler's intended fire time
  pub fn total_latency(&self) -> Option<Duration> {
    match (self.scheduled_at, self.last_byte_at) {
      (Some(start), Some(end)) => end.checked_duration_since(start),
      _ => None,
    }
  }

  /// Time to first byte
  pub fn ttfb(&self) -> Option<Duration> {
    match (self.send_start, self.first_byte_at) {
      (Some(start), Some(end)) => end.checked_duration_since(start),
      _ => None,
    }
  }

  /// Transfer time
  pub fn transfer_time(&self) -> Option<Duration> {
    match (self.first_byte_at, self.last_byte_at) {
      (Some(start), Some(end)) => end.checked_duration_since(start),
      _ => None,
    }
  }

  /// Scheduler delay
  pub fn scheduler_delay(&self) -> Option<Duration> {
    match (self.scheduled_at, self.send_start) {
      (Some(scheduled), Some(started)) => started.checked_duration_since(scheduled),
      _ => None,
    }
  }
}