use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use http_body_util::BodyExt;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::http::pool::BastionClient;
use crate::http::request::build_request;
use crate::http::timing::HttpProtocol;
use crate::metrics::collector::SharedProgress;
use crate::metrics::event::RequestEvent;
use crate::scheduler::WorkItem;
use tracing::warn;

pub async fn run_worker(worker_id: usize, config: Arc<Config>,
    client: Arc<BastionClient>,
    rx: Arc<tokio::sync::Mutex<mpsc::Receiver<WorkItem>>>,
    progress: Arc<SharedProgress>, event_tx: mpsc::Sender<RequestEvent>) {

  // Pre-encode body bytes once
  let body_bytes: Option<Bytes> = config.body.as_deref().map(|b| Bytes::from(b.to_owned()));

  loop {
      let item = {
        let mut receiver = rx.lock().await;
        receiver.recv().await
      };

      let item = match item {
        Some(i) => i,
        None => break,
      };

      let request = match build_request(&config, body_bytes.clone()) {
        Ok(r) => r,
        Err(e) => {
          warn!(worker_id, error = %e, "Failed to build request");
          continue;
        }
      };

      let send_start = Instant::now();

      let response = match client.request(request).await {
          Ok(r) => r,
          Err(e) => {
            warn!(worker_id, seq = item.sequence, error = %e, "Request failed");

            // Emit a failure event
            let event = RequestEvent {
              completed_at: Instant::now(),
              total_latency: item.intended_at.elapsed(),
              ttfb: None,
              status: None,
              bytes: 0,
              new_connection: false,
              protocol: HttpProtocol::Unknown,
              is_warmup: item.is_warmup,
            };

            emit_event(&event_tx, event, &progress, item.is_warmup).await;
            continue;
          }
      };

      let ttfb = send_start.elapsed();
      let status = response.status().as_u16();
      let protocol = match response.version() {
        hyper::Version::HTTP_11 => HttpProtocol::Http1,
        hyper::Version::HTTP_2 => HttpProtocol::Http2,
        _ => HttpProtocol::Unknown,
      };

      let body = match response.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
          warn!(worker_id, seq = item.sequence, error = %e, "Failed to collect response body");
          Bytes::new()
        }
      };

      let event = RequestEvent {
        completed_at: Instant::now(),
        total_latency: item.intended_at.elapsed(),
        ttfb: Some(ttfb),
        status: Some(status),
        bytes: body.len() as u64,
        new_connection: false,
        protocol: protocol,
        is_warmup: item.is_warmup,
      };

      emit_event(&event_tx, event, &progress, item.is_warmup).await;
  }
}

// async fn execute_request(worker_id: usize, config: &Config, client: &BastionClient, item: &WorkItem, body_bytes: Option<Bytes>,) -> RequestTiming {
//   let mut timing = RequestTiming {
//     scheduled_at: Some(item.intended_at),
//     ..Default::default()
//   };

//   let send_start = Instant::now();
//   timing.send_start = Some(send_start);

//   let request = match build_request(config, body_bytes) {
//     Ok(r) => r, 
//     Err(e) => {
//       warn!(worker_id, seq = item.sequence, error = %e, "Failed to build request");
//       return timing;
//     }
//   };

//   // let use_http2 = config.http2;

//   let response = match client.request(request).await {
//     Ok(r) => r,
//     Err(e) => {
//       warn!(worker_id, seq = item.sequence, error = %e, "HTTP request failed");
//       return timing;
//     }
//   };
  
//   // Record TTFB
//   timing.first_byte_at = Some(Instant::now());
//   timing.status = Some(response.status().as_u16());

//   // Detect HTTP version
//   timing.protocol = match response.version() {
//     hyper::Version::HTTP_11 => HttpProtocol::Http1,
//     hyper::Version::HTTP_2 => HttpProtocol::Http2,
//     _ => HttpProtocol::Unknown,
//   };

//   debug!(worker_id, seq = item.sequence, status = timing.status, ttfb_ms = timing.ttfb().map(|d| d.as_millis()), protocol = ?timing.protocol, "Response headers received");

//   // Read response body
//   let body = match response.into_body().collect().await {
//     Ok(collected) => collected.to_bytes(),
//     Err(e) => {
//       warn!(worker_id, error = %e, "Failed to collect response body");
//       timing.last_byte_at = Some(Instant::now());
//       return timing;
//     }
//   };

//   timing.last_byte_at = Some(Instant::now());
//   timing.response_bytes = body.len() as u64;

//   debug!(worker_id, seq = item.sequence, bytes = timing.response_bytes, ttlb_ms = timing.total_latency().map(|d| d.as_millis()), "Response body received");

//   timing
// }

async fn emit_event(tx: &mpsc::Sender<RequestEvent>, event: RequestEvent, progress: &SharedProgress, is_warmup: bool) {
  if is_warmup {
    progress.increment_warmup();
  } else {
    progress.increment_completed();
    if event.status.map(|s| s >= 400).unwrap_or(true) {
      progress.increment_errors();
    } 
    progress.add_bytes(event.bytes);
  }

  let _ = tx.send(event).await;
}

