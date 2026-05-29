use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::{self, MissedTickBehavior};
use governor::{Quota, RateLimiter, clock::DefaultClock, state::direct::NotKeyed, state::InMemoryState};
use crate::config::{Config, RunMode};
use tracing::{debug, info};


/// A work item sent from teh scheduler to a worker.
#[derive(Debug)]
pub struct WorkItem {
  /// When the scheduler intended this request to be sent.
  pub intended_at: Instant,

  pub sequence: u64,

  pub is_warmup: bool,
}

type Limiter = governor::RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

pub struct Scheduler {
  config: Arc<Config>,
  tx: mpsc::Sender<WorkItem>,
}


impl Scheduler {
  pub fn new(config: Arc<Config>, tx: mpsc::Sender<WorkItem>) -> Self {
    Self { config, tx}
  }

  pub async fn run(self) {
    let warmup_end = self.config.warmup.map(|w| Instant::now() + w);

    match self.config.run_mode.clone() {
      RunMode::RequestCount(n) => {
        self.run_count_mode(n, warmup_end).await;
      }
      RunMode::Duration(d) => {
        self.run_duration_mode(d, warmup_end).await;
      }
    }

    info!("Scheduler finished - all work items sent");
  }

  async fn run_count_mode(&self, total: u64, warmup_end: Option<Instant>) {
    let limiter = self.build_limiter();

    for seq in 0..total {
      let now = Instant::now();
      let is_warmup = warmup_end.map(|end| now < end).unwrap_or(false);

      if let Some(ref l) = limiter {
        l.until_ready().await;
      }

      let intended_at = Instant::now();
      let item = WorkItem {intended_at, sequence: seq, is_warmup};

      if self.tx.send(item).await.is_err() {
        debug!("Scheduler: channel closed, stopping early at seq {seq}");
        break;
      }
    }
  }

  async fn run_duration_mode(&self, duration: Duration, warmup_end: Option<Instant>) {
    let deadline = Instant::now() + duration;
    let limiter = self.build_limiter();
    let mut seq = 0u64;

    loop {
      let now = Instant::now();
      if now >= deadline {
        debug!("Scheduler: deadline reached, stopping early at seq {seq}");
        break;
      }

      let is_warmup = warmup_end.map(|end| now < end).unwrap_or(false);

      if let Some(ref l) = limiter {
        tokio::select! {
          _ = l.until_ready() => {}
          _ = time::sleep_until(deadline.into()) => break,
        }
      }

      if Instant::now() >= deadline {
        debug!("Scheduler: deadline reached, stopping early at seq {seq}");
        break;
      }

      let intended_at = Instant::now();
      let item = WorkItem {intended_at, sequence: seq, is_warmup};

      if self.tx.send(item).await.is_err() {
        debug!("Scheduler: channel closed, stopping early at seq {seq}");
        break;
      }

      seq += 1;
    }

    info!(total_scheduled = seq, "Duration-mode scheduler completed");
  }

  fn build_limiter(&self) -> Option<Arc<Limiter>> {
    self.config.rate_limit.map(|rps| {
      let quota = Quota::per_second(
        std::num::NonZeroU32::new(rps).expect("Rate Limit > 0 validated in Config")
      );
      Arc::new(RateLimiter::direct(quota))
    })
  }
}