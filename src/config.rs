use std::time::Duration;
use std::collections::HashMap;
use crate::error::{BastionError, Result};

#[derive(Debug, Clone)]
pub enum HttpMethod {
  Get,
  Post,
  Put,
  Patch,
  Delete,
  Head,
}

impl std::str::FromStr for HttpMethod {
  type Err = BastionError;

  fn from_str(s: &str) -> Result<Self> {
    match s.to_uppercase().as_str() {
      "GET" => Ok(HttpMethod::Get),
      "POST" => Ok(HttpMethod::Post),
      "PUT" => Ok(HttpMethod::Put),
      "PATCH" => Ok(HttpMethod::Patch),
      "DELETE" => Ok(HttpMethod::Delete),
      "HEAD" => Ok(HttpMethod::Head),
      other => Err(BastionError::InvalidConfig(format!("Invalid HTTP method: {}", other))),
    }
  }
}

#[derive(Debug, Clone)]
pub enum RunMode {
  RequestCount(u64),
  Duration(Duration),
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
  Csv,
  Json,
  Terminal,
}

#[derive(Debug, Clone)]
pub struct Config {
  pub url: String,
  pub run_mode: RunMode,
  pub concurrency: usize,
  pub rate_limit: Option<u32>,
  pub timeout: Duration,
  pub method: HttpMethod,
  pub body: Option<String>,
  pub headers: HashMap<String, String>,
  pub output_format: OutputFormat,
  pub output_file: Option<String>,
  pub progress: bool,
  pub keep_alive: bool,
  pub follow_redirects: bool,
  // pub duration: Option<Duration>,
  pub warmup: Option<Duration>,
  pub channel_capacity: usize,
  pub http2: bool,
}



impl Config {
  pub fn validate(&self) -> Result<()> {
    if self.url.is_empty() {
      return Err(BastionError::InvalidConfig("URL cannot be empty".into()));
    }

    // Validate URL is parseable
    reqwest::Url::parse(&self.url).map_err(|e| BastionError::InvalidConfig(format!("Invalid URL: '{}': {}", self.url, e)))?;

    match &self.run_mode {
      RunMode::RequestCount(n) if *n == 0 => {
        return Err(BastionError::InvalidConfig("Request count must be greater than 0".into()));
      }
      RunMode::Duration(d) if d.is_zero() => {
        return Err(BastionError::InvalidConfig("Duration must be greater than 0".into()));
      }
      _ => {}
    }

    // if self.requests == 0 {
    //   return Err(BastionError::InvalidConfig("Number of requests must be greater than 0".into()));
    // }

    if self.concurrency == 0 {
      return Err(BastionError::InvalidConfig("Concurrency must be greater than 0".into()));
    }

    if self.concurrency > 10_000 {
      return Err(BastionError::InvalidConfig("Concurrency above 10,000 is unsupported (check OS fd limits)".into()));
    }

    if let Some(rate_limit) = self.rate_limit {
      if rate_limit == 0 {
        return Err(BastionError::InvalidConfig("Rate limit must be greater than 0 req/s".into()));
      }
    }

    if self.http2 && !self.keep_alive {
      return Err(BastionError::InvalidConfig("HTTP/2 requires keep-alive connections. Remove --no-keep-alive".into()));
    }

    Ok(())
  }
}
