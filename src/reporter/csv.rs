use crate::metrics::window::WindowSnapshot;
use crate::error::Result;

pub fn write_csv_report(windows: &[WindowSnapshot], writer: &mut impl std::io::Write) -> Result<()> {
  let mut wtr = csv::Writer::from_writer(writer);

  // Header
  wtr.write_record(&["second", "rps", "p50(ms)", "p90(ms)", "mean(ms)", "request_count", "error_count", "error_rate(%)", "bytes_received", "throughput(kbps)", "new_connections"])?;

  // Data
  for window in windows {
    wtr.write_record(&[
      window.window_index.to_string(),
      format!("{:.2}", window.rps),
      format!("{:.3}", window.p50.as_secs_f64() * 1000.0),
      format!("{:.3}", window.p90.as_secs_f64() * 1000.0),
      format!("{:.3}", window.p99.as_secs_f64() * 1000.0),
      format!("{:.3}", window.mean.as_secs_f64() * 1000.0),
      window.request_count.to_string(),
      window.error_count.to_string(),
      format!("{:.6}", window.error_rate),
      window.bytes_received.to_string(),
      format!("{:.2}", window.throughput_bps / 1024.0),
      window.new_connections.to_string(),
    ])?;
  }

  wtr.flush()?;
  Ok(())
}