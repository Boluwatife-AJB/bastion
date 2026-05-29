use std::collections::VecDeque;
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, mpsc};
use tokio::time::{self, MissedTickBehavior};
use tracing::{debug, info, warn};

use crate::metrics::event::RequestEvent;
use crate::metrics::window::{MetricWindow, WindowSnapshot};

const MAX_HISTORY: usize = 3600;

const WINDOW_DURATION: Duration = Duration::from_secs(1);

pub struct Aggregator {
  event_rx: mpsc::Receiver<RequestEvent>,
  window_tx: broadcast::Sender<WindowSnapshot>,
  current_window: MetricWindow,
  history: VecDeque<WindowSnapshot>,
  window_index: u64,
}

impl Aggregator {
    pub fn new(event_rx: mpsc::Receiver<RequestEvent>, window_tx: broadcast::Sender<WindowSnapshot>) -> Self {
      let now = Instant::now();
      Self { event_rx, window_tx, current_window: MetricWindow::new(now, WINDOW_DURATION), history: VecDeque::with_capacity(MAX_HISTORY), window_index: 0 }
    }

    pub async fn run(mut self) -> Vec<WindowSnapshot> {
      let mut ticker = time::interval(Duration::from_millis(100));
      ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

      loop {
          tokio::select! {
            event = self.event_rx.recv() => {
              match event {
                Some(e) if !e.is_warmup => {
                  self.process_event(e);
                }
                Some(_) => {
                  // Warmup event, discard silently
                }
                None => {
                  self.close_current_window();
                  info!(
                    windows = self.window_index,
                    "Aggregator: event channel closed, finalizing"
                  );
                  break;
                }

              }
            }

            _ = ticker.tick() => {
              self.check_window_close();
            }
          }
      }

      self.history.into_iter().collect()
    }

    fn process_event(&mut self, event:RequestEvent) {
      let now = Instant::now();

      if self.current_window.is_closed(now) {
        self.close_current_window();
        self.open_new_window(now);
      }

      self.current_window.record(&event);
    }

    fn check_window_close(&mut self) {
      let now = Instant::now();
      if self.current_window.is_closed(now) {
        self.close_current_window();
        self.open_new_window(now);
      }
    }

    fn close_current_window(&mut self) {
      if self.current_window.request_count == 0 {
        self.current_window = MetricWindow::new(
          Instant::now(),
          WINDOW_DURATION,
        );
        return
      }

      let snapshot = WindowSnapshot::from_window(&self.current_window, self.window_index);

      debug!(
        window = self.window_index,
        rps = snapshot.rps,
        p99_ms = snapshot.p99.as_millis(),
        errors = snapshot.error_count,
        "Window closed"
      );

      // Broadcast to all consumer
      if let Err(e) = self.window_tx.send(snapshot.clone()) {
        debug!("No broadcast receivers: {e}");
      }

      if self.history.len() >= MAX_HISTORY {
        self.history.pop_front();
      }
      self.history.push_back(snapshot);
      self.window_index += 1;
    }

    fn open_new_window(&mut self, start: Instant) {
      self.current_window = MetricWindow::new(
        start,
        WINDOW_DURATION,
      );
    }
}