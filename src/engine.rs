use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinSet;
use tracing::info;

use crate::config::{Config, RunMode};
use crate::display::live::LiveDisplay;
use crate::http::pool::{build_hyper_client, PoolConfig};
use crate::metrics::aggregator::Aggregator;
use crate::metrics::collector::SharedProgress;
use crate::metrics::event::RequestEvent;
use crate::metrics::window::WindowSnapshot;
use crate::worker::run_worker;
use crate::metrics::snapshot::MetricsSnapshot;
use crate::metrics::summary::summarize_windows;
use crate::scheduler::Scheduler;
use anyhow::Result;


const EVENT_CHANNEL_CAPACITY: usize = 8_192;

const BROADCAST_CAPACITY: usize = 64;

pub async fn run(config: Config) -> Result<(MetricsSnapshot, Vec<WindowSnapshot>)> {
  let config = Arc::new(config);

  print_run_summary(&config);

  let (work_tx, work_rx) = mpsc::channel(config.channel_capacity);
  let work_rx = Arc::new(tokio::sync::Mutex::new(work_rx));

  let (event_tx, event_rx) = mpsc::channel::<RequestEvent>(EVENT_CHANNEL_CAPACITY);

  let (window_tx, _) = broadcast::channel::<crate::metrics::window::WindowSnapshot>(BROADCAST_CAPACITY);

  let live_display_rx = window_tx.subscribe();

  let pool_config = PoolConfig::from_config(&config);
  let client = build_hyper_client(&pool_config)?;

  let progress = SharedProgress::new();

  let aggregator = Aggregator::new(event_rx, window_tx.clone());
  let aggregator_task = tokio::spawn(aggregator.run());

  let display_task = if config.progress {
    let display = LiveDisplay::new(live_display_rx);
    Some(tokio::spawn(display.run()))
  } else {
    None
  };

  let progress_task = spawn_progress_updater(&config, Arc::clone(&progress));

  // Print warmup notice
  if let Some(warmup) = config.warmup {
    print!(
      " {} Warming up for {}...\n",
      console::style("◆").yellow(),
      humantime::format_duration(warmup)
    );
  }

  let wall_start = Instant::now();

  let mut worker_set = JoinSet::new();

  for worker_id in 0..config.concurrency {
    worker_set.spawn(run_worker(
      worker_id, 
      Arc::clone(&config), 
      Arc::clone(&client), 
      Arc::clone(&work_rx), 
      Arc::clone(&progress),
    event_tx.clone()));
  }

  drop(event_tx);

  let config_for_scheduler = Arc::clone(&config);
  let scheduler_task = tokio::spawn(async move {
    Scheduler::new(config_for_scheduler, work_tx).run().await;
  });

  scheduler_task.await?;

  while worker_set.join_next().await.is_some() {}

  let total_duration = wall_start.elapsed();
  let window_history = aggregator_task.await?;

  if let Some(task) = display_task {
    tokio::time::sleep(Duration::from_millis(150)).await;
    task.abort();
  }
  progress_task.abort();

  info!(
    duration_ms = total_duration.as_millis(),
    windows = window_history.len(),
    "Run complete"
  );

  let summary = summarize_windows(&window_history, total_duration);
  let snapshot = MetricsSnapshot::from_summary_and_windows(
    summary, &window_history, total_duration
  );

  Ok((snapshot, window_history))
}

// async fn run_worker_shared(worker_id: usize, config: Arc<Config>, client: Arc<BastionClient>, rx: Arc<tokio::sync::Mutex<mpsc::Receiver<crate::scheduler::WorkItem>>>, progress: Arc<SharedProgress>) -> crate::metrics::collector::WorkerMetrics {
//   use crate::metrics::collector::WorkerMetrics;
//   use crate::http::client::build_client;
//   use tracing::warn;

//   let client = match build_client(&config) {
//     Ok(c) => c,
//     Err(e) => {
//       warn!(error = %e, "Worker {worker_id} failed to build HTTP client");
//       return WorkerMetrics::new();
//     }
//   };
  
//   let mut metrics = WorkerMetrics::new();
//   let mut warmup_metrics = WorkerMetrics::new();

//   loop {
//     let item = {
//       let mut receiver = rx.lock().await;
//       receiver.recv().await
//     };

//     let item = match item {
//       Some(i) => i,
//       None => break,
//     };

//       let result = crate::worker::send_request_pub(&client, &config).await;
//       let latency = item.intended_at.elapsed();

//       let target = if item.is_warmup {
//         &mut warmup_metrics
//       } else {
//         &mut metrics
//       };

//       match result {
//         Ok((status, bytes)) => {
//           target.record_success(latency, status, bytes);
//           if !item.is_warmup {
//             progress.increment_completed();
//             progress.add_bytes(bytes);
//           } else {
//             progress.increment_warmup()
//           }
//         }
//         Err(e) => {
//           tracing::warn!(worker_id, seq = item.sequence, error = %e, "Request failed");
//           target.record_error(latency);
//           if !item.is_warmup {
//             progress.increment_errors();
//             progress.increment_completed();
//           }
//         }
//       }
//   }
  
//   metrics
// }

// fn build_rate_limiter(rate: Option<u32>) -> Option<Arc<governor::DefaultDirectRateLimiter>> {
//   rate.map(|rps| {
//     let quota = Quota::per_second(
//       std::num::NonZeroU32::new(rps).expect("Rate > 0 validated in Config")
//     );
//     Arc::new(RateLimiter::direct(quota))
//   })
// }

fn print_run_summary(config: &Config) {
  use console::style;
  println!();
  print!("  {}  Bastion\n", style("▲").cyan().bold());
  print!("  {}  {}\n", style("Target:").dim(), config.url);

  match config.run_mode {
    RunMode::RequestCount(n) => {
      print!("  {}  {}\n", style("Requests:").dim(), n);
    }
    RunMode::Duration(d) => {
      print!("  {}  {}\n", style("Duration:").dim(), humantime::format_duration(d));
    }
  }
  print!("  {}  {}\n", style("Concurrency:").dim(), config.concurrency);
  if let Some(rps) = config.rate_limit {
    print!("  {}  {} req/s\n", style("Rate limits:").dim(), rps);
  }
  if config.http2 {
    println!("  {} enabled", style("HTTP/2:").dim());
  }
  if let Some(warmup) = config.warmup {
    print!("  {}  {}\n", style("Warmup:").dim(), humantime::format_duration(warmup));
  }
  println!();
}

// fn setup_progress_bar(config: &Config, total: u64) -> Option<ProgressBar> {
//   if !config.progress {
//     return None;
//   }


//   let pb = if total == u64::MAX {
//     let pb = ProgressBar::new_spinner();
//     pb.set_style(
//       ProgressStyle::with_template(
//         "{spinner:.cyan} [{elapsed_precise}] {pos} requests {msg}"
//       ).unwrap()
//     );
//     pb
//   } else {
//     let pb = ProgressBar::new(total);
//     pb.set_style(
//       ProgressStyle::with_template(
//         "{spinner:.cyan} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta}) {msg}"
//       ).unwrap().progress_chars("█▉▊▋▌▍▎▏  ")
//     );
//     pb
//   };

//   pb.enable_steady_tick(std::time::Duration::from_millis(80));
//   Some(pb)
// }

// Spawn the progress bar update task
fn spawn_progress_updater(config: &Arc<Config>, progress: Arc<SharedProgress>) -> tokio::task::JoinHandle<()> {
  let total = match config.run_mode {
    RunMode::RequestCount(n) => n,
    RunMode::Duration(_) => u64::MAX,
  };
  let show = config.progress;

  tokio::spawn(async move {
    if !show { return; }
    loop {
        let done = progress.completed.load(std::sync::atomic::Ordering::Relaxed);
        let _errors = progress.errors.load(std::sync::atomic::Ordering::Relaxed);

        if done >= total { break; }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
  })
}