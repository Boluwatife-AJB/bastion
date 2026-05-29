use comfy_table::{Table, Cell, Attribute, Color};
use console::style;
use crate::metrics::snapshot::MetricsSnapshot;
use humantime::format_duration;
use crate::metrics::summary::RunSummary;
use crate::metrics::window::WindowSnapshot;
use crate::thresholds::{Threshold, ThresholdResult};
use crate::compare::{ComparisonReport, VerdictStatus};

pub fn print_report(snapshot: &MetricsSnapshot, config: &crate::config::Config) {
  use crate::config::RunMode;

  println!();

  if config.warmup.is_some() {
    println!(
      " {} Warmup period excluded from results",
      console::style("ℹ").blue()
    );
    println!();
  }

  println!();
  println!("  {} Bastion Results", console::style("■").cyan().bold());
  println!();

  let mut summary = Table::new();
  summary.set_header(vec![
    Cell::new("Metrics").add_attribute(Attribute::Bold),
    Cell::new("Value").add_attribute(Attribute::Bold),
  ]);

  summary.add_row(vec![
    "Total Requests",
    &snapshot.total_requests.to_string(),
  ]);
  summary.add_row(vec![
    "Successful",
    &format!("{}  ({:.1}%)", snapshot.success_count, (snapshot.success_count as f64 / snapshot.total_requests as f64) * 100.0),
  ]);
  summary.add_row(vec![
    "Errors",
    &format!("{}  ({:.1}%)", snapshot.error_count, (snapshot.error_count as f64 / snapshot.total_requests as f64) * 100.0),
  ]);
  summary.add_row(vec![
    "Duration",
    &format_duration(snapshot.total_duration).to_string(),
  ]);
  summary.add_row(vec![
    "Throughput",
    &format!("{:2} req/s", snapshot.requests_per_second),
  ]);
  summary.add_row(vec![
    "Data Received",
    &format_bytes(snapshot.bytes_received),
  ]);

  println!("{summary}");
  println!();

  // Latency table
  let mut latency = Table::new();
  latency.set_header(vec![
    Cell::new("Percentile").add_attribute(Attribute::Bold),
    Cell::new("Latency").add_attribute(Attribute::Bold),
  ]);

  let lat = &snapshot.latency;
  latency.add_row(vec!["Min", &fmt_duration(lat.min)]);
  latency.add_row(vec!["Mean", &fmt_duration(lat.mean)]);
  latency.add_row(vec!["p50", &fmt_duration(lat.p50)]);
  latency.add_row(vec!["p75", &fmt_duration(lat.p75)]);
  latency.add_row(vec!["p90", &fmt_duration(lat.p90)]);
  latency.add_row(vec!["p95", &fmt_duration(lat.p95)]);
  latency.add_row(vec!["p99", &fmt_duration(lat.p99)]);
  latency.add_row(vec!["p99.9", &fmt_duration(lat.p999)]);
  latency.add_row(vec!["Max", &fmt_duration(lat.max)]);
  latency.add_row(vec!["StdDev", &fmt_duration(lat.stddev)]);

  println!("{latency}");
  println!();

  // Status code distribution 
  if !snapshot.status_counts.is_empty() {
    println!();
    let mut statuses = Table::new();
    statuses.set_header(vec![
      Cell::new("Status Code").add_attribute(Attribute::Bold),
      Cell::new("Count").add_attribute(Attribute::Bold),
    ]);
    
    let mut sorted: Vec<_> = snapshot.status_counts.iter().collect();
    sorted.sort_by_key(|(code, _)| *code);
    for (status, count) in sorted {
      let cell = if *status >= 200 && *status < 300 {
        Cell::new(format!("{}", status)).add_attribute(Attribute::Bold).fg(Color::Green)
      } else if *status >= 300 && *status < 400 {
        Cell::new(format!("{}", status)).add_attribute(Attribute::Bold).fg(Color::Yellow)
      } else if *status >= 400 && *status < 500 {
        Cell::new(format!("{}", status)).add_attribute(Attribute::Bold).fg(Color::Red)
      } else {
        Cell::new(format!("{}", status)).add_attribute(Attribute::Bold)
      };
      statuses.add_row(vec![cell, Cell::new(count.to_string())]);
    }
    println!("{statuses}");
    // println!();
  }

  // TTFB Section
  if let Some(ref ttfb) = snapshot.ttfb {
    println!("");
    println!("  {} Time to First Byte (server processing)", console::style("◆").cyan());
    let mut table = Table::new();
    table.set_header(vec![
      Cell::new("Percentile").add_attribute(Attribute::Bold),
      Cell::new("TTFB").add_attribute(Attribute::Bold),
    ]);
    table.add_row(vec!["p50", &fmt_duration(ttfb.p50)]);
    table.add_row(vec!["p75", &fmt_duration(ttfb.p75)]);
    table.add_row(vec!["p90", &fmt_duration(ttfb.p90)]);
    table.add_row(vec!["p99", &fmt_duration(ttfb.p99)]);
    table.add_row(vec!["p999", &fmt_duration(ttfb.p999)]);
    println!("{table}");
  }

  // Scheduler delay section
  if let Some(ref delay) = snapshot.scheduler_delay {
    let p99_delay_ms = delay.p99.as_millis();
    if p99_delay_ms > 5 {
      // Only show if meaningful, high delay means workers are saturated
      println!("");
      println!(
        " {} Scheduler delay p99: {}ms - workers may be saturated", console::style("▲").yellow(), p99_delay_ms);
    }
  }

  // Connection pool section
  println!("");
  let mut conn_table = Table::new();
  conn_table.set_header(vec![
    Cell::new("Connection Metric").add_attribute(Attribute::Bold),
    Cell::new("Value").add_attribute(Attribute::Bold),
  ]);
  conn_table.add_row(vec![
    "New Connections",
    &snapshot.connections.new_connections.to_string(),
  ]);
  conn_table.add_row(vec![
    "Reuse Rate",
    &format!("{:.1}%", snapshot.connections.reuse_rate * 100.0)
  ]);
  conn_table.add_row(vec![
    "HTTP/1.1 Requests",
    &snapshot.connections.http1_requests.to_string(),
  ]);
  conn_table.add_row(vec![
    "HTTP/2 Requests",
    &snapshot.connections.http2_requests.to_string(),
  ]);
  println!("{conn_table}");
}

/// Print a per-second breakdown table (last N windows)
pub fn print_time_series(windows: &[WindowSnapshot], show_last: usize) {
  // use comfy_table::{Table, Cell, Attribute, Color}
  // use console::style;

  println!();
  println!(
    " {} Per-second breakdown (last {} seconds)",
    style("◆").cyan(),
    show_last.min(windows.len())
  );

  let mut table = Table::new();
  table.set_header(vec![
    Cell::new("T").add_attribute(Attribute::Bold),
    Cell::new("RPS").add_attribute(Attribute::Bold),
    Cell::new("p50").add_attribute(Attribute::Bold),
    Cell::new("p99").add_attribute(Attribute::Bold),
    Cell::new("Errors").add_attribute(Attribute::Bold),
    Cell::new("KB/s").add_attribute(Attribute::Bold),
  ]);

  let start = windows.len().saturating_sub(show_last);
  for w in &windows[start..] {
    let p99_cell = if w.p99.as_millis() > 100 {
      Cell::new(fmt_duration(w.p99)).fg(Color::Red)
    } else if w.p99.as_millis() > 50 {
      Cell::new(fmt_duration(w.p99)).fg(Color::Yellow) 
    } else {
      Cell::new(fmt_duration(w.p99)).fg(Color::Green)
    };

    let error_cell = if w.error_count > 0 {
      Cell::new(w.error_count.to_string()).fg(Color::Red)
    } else {
      Cell::new("-".to_string()).fg(Color::Green)
    };

    table.add_row(vec![
      Cell::new(format!("T+{}s", w.window_index)),
      Cell::new(format!("{:.1}", w.rps)),
      Cell::new(fmt_duration(w.p50)), 
      p99_cell, 
      error_cell,
      Cell::new(format!("{:.1}", w.throughput_bps / 1024.0))
    ]);
  }

  println!("{table}");

}

/// Print spike analysis section
pub fn print_spike_analysis(summary: &RunSummary, windows: &[WindowSnapshot]) {
  if summary.spike_count == 0 {
    println!(
      "\n {} Latency was stable throughout the run.", style("✓").green()
    );
    return;
  }

  println!(
    "\n {} {} Latency spike(s) detected (p99 > {} threshold)", style("▲").yellow(), summary.spike_count, fmt_duration(summary.spike_threshold)
  );

  for w in windows.iter().filter(|w| w.p99 > summary.spike_threshold) {
    println!(
      " T+{}s: p99={} rps={:.1}, errors={}",
      w.window_index,
      fmt_duration(w.p99),
      w.rps,
      w.error_count
    );
  }  
} 

/// Print threshold assertion results
pub fn print_thresholds(results: &[ThresholdResult]) {
  use comfy_table::{Table, Cell, Attribute, Color};
  use console::style;

  println!();
  println!(" {} Threshold Assertion Results", style("◆").cyan().bold());

  let mut table = Table::new();
  table.set_header(vec![
    Cell::new("Metric").add_attribute(Attribute::Bold),
    Cell::new("Expected").add_attribute(Attribute::Bold),
    Cell::new("Actual").add_attribute(Attribute::Bold),
    Cell::new("Result").add_attribute(Attribute::Bold),
  ]);

  for result in results {
    let status_cell = if result.passed {
      Cell::new("PASSED").fg(Color::Green)
    } else {
      Cell::new("FAILED").fg(Color::Red)
    };

    table.add_row(vec![
      Cell::new(result.threshold.label()),
      Cell::new(&result.expected),
      Cell::new(&result.actual),
      status_cell,
    ]);
  }

  println!("{table}");

  let failed = results.iter().filter(|r| !r.passed).count();
  if failed > 0 {
    println!("\n {} {}/{} assertion(s) failed", style("✕").red().bold(), failed, results.len());
  } else {
    println!("\n {} All {} assertion(s) passed", style("✓").green().bold(), results.len());
  }
}

/// Print comparison report
pub fn print_comparison(report: &ComparisonReport) {
  use comfy_table::{Table, Cell, Attribute, Color};
  use console::style;

  println!();
  println!("  {} Comparison vs Baseline", style("◆").cyan().bold());

  let mut table = Table::new();
  table.set_header(vec![
    Cell::new("Metric").add_attribute(Attribute::Bold),
    Cell::new("Baseline").add_attribute(Attribute::Bold),
    Cell::new("Current").add_attribute(Attribute::Bold),
    Cell::new("Δ").add_attribute(Attribute::Bold),
    Cell::new("Status").add_attribute(Attribute::Bold),
  ]);

  for v in &report.verdicts {
    let delta_str = if v.delta_pct >= 0.0 {
      format!("+{:.1}%", v.delta_pct)
    } else {
      format!("{:.1}%", v.delta_pct)
    };

    let (delta_cell, status_cell) = match v.status {
      VerdictStatus::Improved => (
        Cell::new(delta_str).fg(Color::Green),
        Cell::new("▲ improved").fg(Color::Green),
      ),
      VerdictStatus::Stable => (
        Cell::new(delta_str).fg(Color::Yellow),
        Cell::new("● stable").fg(Color::Yellow),
      ),
      VerdictStatus::Regressed => (
        Cell::new(delta_str).fg(Color::Red),
        Cell::new("▼ regressed").fg(Color::Red),
      ),
    };


    table.add_row(vec![
      Cell::new(&v.metric),
      Cell::new(&v.baseline),
      Cell::new(&v.current),
      delta_cell,
      status_cell,
    ]);
  }

  println!("{table}");

  if report.any_regression() {
    println!("\n {} Performance regression detected", style("✕").red().bold());
  } else {
    println!("\n {} No regression detected", style("✓").green().bold());
  }
}

fn fmt_duration(d: std::time::Duration) -> String {
  let micros = d.as_micros();
  if micros < 1_000 {
    format!("{} μs", micros)
  } else if micros < 1_000_000 {
    format!("{:.2} ms", micros as f64 / 1_000.0)
  } else {
    format!("{:.2} s", micros as f64 / 1_000_000.0)
  }
}

fn format_bytes(bytes: u64) -> String {
  if bytes < 1_024 {
    format!("{} B", bytes)
  } else if bytes < 1_048_576 {
    format!("{:.2} KB", bytes as f64 / 1_024.0)
  } else if bytes < 1_073_741_824 {
    format!("{:.2} MB", bytes as f64 / 1_048_576.0)
  } else {
    format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
  }
}