use std::collections::VecDeque;
use std::time::Duration;

use tokio::sync::broadcast;
use tracing::debug;

use crate::metrics::window::WindowSnapshot;

const SPARKLINE_WIDTH: usize = 40;

pub struct LiveDisplay {
  rx: broadcast::Receiver<WindowSnapshot>,
  rps_history: VecDeque<f64>,
  peak_rps: f64,
}

impl LiveDisplay {
    pub fn new(rx: broadcast::Receiver<WindowSnapshot>) -> Self {
      Self { rx, rps_history: VecDeque::with_capacity(SPARKLINE_WIDTH), peak_rps: 0.0 }
    }

    pub async fn run(mut self) {
      println!("");

      loop {
          match self.rx.recv().await {
            Ok(snapshot) => {
              self.update_history(&snapshot);
              self.render(&snapshot);
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
              debug!("Live display lagged by {} windows", n);
            }
            Err(broadcast::error::RecvError::Closed) => {
              debug!("Live display channel closed");
              break;
            }
          }
      }
    }

    fn update_history(&mut self, snapshot: &WindowSnapshot) {
      if self.rps_history.len() >= SPARKLINE_WIDTH {
        self.rps_history.pop_front();
      }
      self.rps_history.push_back(snapshot.rps);
      self.peak_rps = self.peak_rps.max(snapshot.rps);
    }

    fn render(&self, snapshot: &WindowSnapshot) {
      use console::style;

      if snapshot.window_index > 0{
        println!("\x1B[5A");
      }

      // Line 1: Throughput + sparkline
      let sparkline = self.build_sparkline();
      println!(
        " {} {:.1} req/s {}",
        style("▶").cyan(),
        snapshot.rps,
        sparkline,
      );

      // Line 2: Latency percentiles
      let p99_style = if snapshot.p99.as_millis() > 100 {
        style(fmt_duration(snapshot.p99)).red()
      } else if snapshot.p99.as_millis() > 50 {
        style(fmt_duration(snapshot.p99)).yellow()
      } else {
        style(fmt_duration(snapshot.p99)).green()
      };

      println!(
        " {} p50={} p90={} p99={}",
        style("◆").dim(),
        style(fmt_duration(snapshot.p50)).dim(),
        style(fmt_duration(snapshot.p90)).dim(),
        p99_style
      );

      // Line 3: Error rate
      let error_display = if snapshot.error_rate > 0.01 {
        style(format!("{:.1}% errors", snapshot.error_rate * 100.0)).red()
      } else if snapshot.error_rate > 0.001 {
        style(format!("{:.2}% errors", snapshot.error_rate * 100.0)).yellow()
      } else {
        style("no errors".to_string()).green()
      };

      println!(
        " {} {} - {:.2} MB/s",
        style("◆").dim(),
        error_display,
        snapshot.throughput_bps / 1_048_576.0
      );

      // Line 4: TTFB if available
      if let Some(ttfb) = snapshot.ttfb_p99 {
        println!(
          " {} TTFB p99={} - window {}",
          style("◆").dim(),
          fmt_duration(ttfb),
          snapshot.window_index
        );
      } else {
        println!(
          " {} window {}",
          style("◆").dim(),
          snapshot.window_index
        );
      }

      // Line 5: New connections indicator
      if snapshot.new_connections > 0 {
        println!(
          " {} +{} new connections",
          style("◆").dim(),
          snapshot.new_connections
        );
      } else {
        println!(
          " {} all connections reused",
          style("◆").dim(),
        );
      }
    }

    fn build_sparkline(&self) -> String {
      const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

      if self.rps_history.is_empty() || self.peak_rps == 0.0 {
        return String::new();
      }

      self.rps_history.iter().map(|&rps| {
        let normalized = (rps / self.peak_rps).clamp(0.0, 1.0);
        let index = (normalized * 7.0).round() as usize;
        BLOCKS[index.min(7)]
      }).collect()
    }
}

fn fmt_duration(duration: Duration) -> String {
  let micros = duration.as_micros();
  if micros >= 1_000 {
    format!("{}μs", micros / 1_000)
  } else if micros >= 1_000_000 {
    format!("{:.1}ms", micros as f64 / 1_000.0)
  } else {
    format!("{:.2}s", micros as f64 / 1_000_000.0)
  }
}