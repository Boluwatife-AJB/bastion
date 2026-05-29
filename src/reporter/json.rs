use std::path::Path;
use crate::metrics::snapshot::MetricsSnapshot;
use crate::metrics::window::WindowSnapshot;
use crate::compare::ComparisonReport;
use crate::error::{BastionError, Result};
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct JsonReport<'a> {
  pub version: &'static str,
  pub summary: &'a MetricsSnapshot,
  pub windows: Vec<JsonWindow>,
  pub comparison: Option<&'a ComparisonReport>
}

#[derive(Serialize, Debug)]
pub struct JsonWindow {
  pub second: u64,
  pub rps: f64,
  pub p50_ms: f64,
  pub p90_ms: f64,
  pub p99_ms: f64,
  pub error_count: u64,
  pub error_rate: f64,
  pub bytes_received: u64,
  pub throughput_kbps: f64,
}

impl From<&WindowSnapshot> for JsonWindow {
    fn from(w: &WindowSnapshot) -> Self {
        Self {
            second: w.window_index,
            rps: round2(w.rps),
            p50_ms: round2(w.p50.as_secs_f64() * 1000.0),
            p90_ms: round2(w.p90.as_secs_f64() * 1000.0),
            p99_ms: round2(w.p99.as_secs_f64() * 1000.0),
            error_count: w.error_count,
            error_rate: round4(w.error_rate),
            bytes_received: w.bytes_received,
            throughput_kbps: round2(w.throughput_bps / 1024.0),
        }
    }
}

pub fn write_json_report(snapshot: &MetricsSnapshot, windows: &[WindowSnapshot], comparison: Option<&ComparisonReport>, writer: &mut impl std::io::Write) -> Result<()> {
  let report = JsonReport {
    version: env!("CARGO_PKG_VERSION"),
    summary: snapshot,
    windows: windows.iter().map(JsonWindow::from).collect(),
    comparison,
  };
  serde_json::to_writer_pretty(&mut *writer, &report)?;
  writeln!(writer)?;
  Ok(())
}

pub fn load_baseline(path: &Path) -> Result<MetricsSnapshot> {
  let file = std::fs::File::open(path).map_err(BastionError::Io)?;
  let reader = std::io::BufReader::new(file);

  #[derive(serde::Deserialize)]
  struct SavedReport {
    summary: MetricsSnapshot,
  }

  let saved: SavedReport = serde_json::from_reader(reader)?;
  Ok(saved.summary)
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round4(value: f64) -> f64 {
    (value * 10000.0).round() / 10000.0
}