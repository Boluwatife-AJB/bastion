use thiserror::Error;

/// The error type for the Bastion crate.
#[derive(Error, Debug)]
pub enum BastionError {
  #[error("HTTP request failed: {0}")]
  RequestFailed(#[from] reqwest::Error),

  #[error("Invalid configuration: {0}")]
  InvalidConfig(String),

  #[error("Rate limiter error: {0}")]
  RateLimiter(String),

  #[error("Metrics error: {0}")]
  Metrics(String),

  #[error("I/O error: {0}")]
  Io(#[from] std::io::Error),

  #[error("Serialization error: {0}")]
  Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, BastionError>;
