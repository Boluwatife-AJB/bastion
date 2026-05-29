use clap::Parser;
use std::collections::HashMap;
use crate::config::{HttpMethod, Config, OutputFormat, RunMode};
use crate::error::{Result, BastionError};

/// Bastion - A production-grade HTTP load testing tool.
/// 
/// Examples:
/// bastion -u https://example.com -n 1000 -c 50
/// bastion -u https://example.com -d 30s -c 50 -r 200
/// bastion -u https://api.example.com -n 500 -c 20 --warmup 5s
/// bastion -u https://api.example.com -m POST -b '{"key":"val"}' -H "Authorization: Bearer token"
#[derive(Parser, Debug)]
#[command(
  name = "bastion",
  version,
  about = "High performance HTTP load testing and benchmarking tool",
  long_about = None,
)]
pub struct BastionCli {
  /// Target URL to benchmark
  #[arg(short = 'u', long, value_name = "URL")]
  pub url: String,

  /// Total number of request to send (conflicts with --duration)
  // #[arg(short = 'n', long, conflicts_with = "duration", value_name = "COUNT", default_value_if("duration", None::<&str>, None))]
  #[arg(short = 'n', long, conflicts_with = "duration", value_name = "COUNT", default_value = "100")]
  pub requests: Option<u64>,

  // How long to run the benchmark, e.g. "30s", "2m" (conflicts with --requests)
  #[arg(short = 'd', long, value_name = "DURATION", conflicts_with = "requests")]
  pub duration: Option<String>,

  /// Number of concurrent workers
  #[arg(short = 'c', long, default_value = "10", value_name = "WORKERS")]
  pub concurrency: usize,

  /// Request per second rate limit (default: unlimited)
  #[arg(short = 'r', long, value_name = "RPS")]
  pub rate_limit: Option<u32>,

  /// Per-request timeout, e.g "5s", "500ms"
  #[arg(short = 't', long, default_value = "30s", value_name = "DURATION")]
  pub timeout: String,

  /// Warmup duration - traffic is sent but metrics are discarded
  #[arg(long, value_name = "DURATION")]
  pub warmup: Option<String>,

  /// HTTP method
  #[arg(short = 'm', long, default_value = "GET", value_name = "METHOD")]
  pub method: String,

  /// Request body (for POST/PUT/PATCH)
  #[arg(short = 'b', long, value_name = "BODY")]
  pub body: Option<String>,

  /// Custom headers: "Key: Value" (repeatable)
  #[arg(short = 'H', long = "header", value_name = "HEADER")]
  pub headers: Vec<String>,

  /// Output format: terminal, json, csv
  #[arg(short = 'o', long, default_value = "terminal", value_name = "FORMAT")]
  pub output_format: String,

  /// File path to write report output
  #[arg(long, value_name = "FILE")]
  pub output_file: Option<String>,

  /// Disable live progress bar
  #[arg(long)]
  pub no_progress: bool,

  /// Disable HTTP keep-alive
  #[arg(long)]
  pub no_keep_alive: bool,

  /// Follow HTTP redirects
  #[arg(long, default_value = "true")]
  pub follow_redirects: bool,

  /// Enable HTTP/2 (negotiated via ALPN, requires server support)
  #[arg(long)]
  pub http2: bool,

  /// Threshold assertions
  #[arg(
    long = "assert",
    value_name = "THRESHOLD",
    help = "Fail if threshold is violated (e.g. p99<200ms, error_rate>1%, rps>50)",
  )]
  pub assertions: Vec<String>,

  /// Path to a baseline JSON report for comparison
  #[arg(long, value_name = "FILE")]
  pub compare: Option<String>,

  /// Maximum regression allowed when comparing
  #[arg(long, value_name = "FRACTION", default_value = "0.10")]
  pub regression_threshold: f64,

  /// Write per-second CSV time-series to this file
  #[arg(long, value_name = "FILE")]
  pub csv_file: Option<String>,
}

impl BastionCli {
  /// Convert raw CLI args into a validated Config
  pub fn into_config(self) -> Result<Config> {
    let timeout = parse_duration_arg(&self.timeout, "timeout")?;

    let method = self.method.parse::<HttpMethod>()?;

    let headers = parse_headers(&self.headers)?;

    let warmup = self.warmup.as_deref().map(|s| parse_duration_arg(&s, "warmup")).transpose()?;

    let output_format = match self.output_format.to_lowercase().as_str() {
      "terminal" | "term" => OutputFormat::Terminal,
      "csv" => OutputFormat::Csv,
      "json" => OutputFormat::Json,
      other => return Err(BastionError::InvalidConfig(format!("Invalid output format '{}': must be one of 'terminal', 'csv', or 'json'", other))),
    };

    let run_mode = if let Some(d) = self.duration {
      let dur = parse_duration_arg(&d, "duration")?;
      RunMode::Duration(dur)
    } else {
      let n = self.requests.unwrap_or(100);
      RunMode::RequestCount(n)
    };

    let concurrency = self.concurrency;

    let config = Config {
      url: self.url,
      run_mode,
      concurrency,
      rate_limit: self.rate_limit,
      timeout,
      method,
      body: self.body,
      headers,
      output_format,
      output_file: self.output_file,
      progress: !self.no_progress,
      keep_alive: !self.no_keep_alive,
      follow_redirects: self.follow_redirects,
      warmup,
      channel_capacity: concurrency * 2,
      http2: self.http2,
      assertions: self.assertions,
      compare: self.compare,
      regression_threshold: self.regression_threshold,
      csv_file: self.csv_file,
    };

    config.validate()?;

    Ok(config)
  }
}

/// Parse a human-readable duration string like "5s", "200ms", "2m45s"
fn parse_duration_arg(s: &str, field: &str) -> Result<std::time::Duration> {
  humantime::parse_duration(s).map_err(|e| {
    BastionError::InvalidConfig(format!("Invalid {field} '{s}': {e}. Use format like '5s', '200ms', '2m45s'."))
  })
}

/// Parse "Key: Value" headers into a HashMap
fn parse_headers(raw: &[String]) -> Result<HashMap<String, String>> {
  let mut map = HashMap::new();
  for header in raw {
    let parts:Vec<&str> = header.splitn(2, ':').collect();

    if parts.len() != 2 {
      return Err(BastionError::InvalidConfig(format!("Invalid header '{}': must be in 'Key: Value' format", header)));
    }

    map.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
  }

  Ok(map)
}