use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use hyper::Uri;
use hyper_rustls::HttpsConnector;
use hyper_util::rt::TokioIo;
use hyper_util::client::legacy::connect::HttpConnector;
use tokio::net::TcpStream;
use tower_service::Service;
use tracing::debug;

#[derive(Debug, Clone, Default)]
pub struct ConnectionTiming {
  pub dns_lookup: Option<Duration>,
  pub tcp_connect: Option<Duration>,
  pub tls_handshake: Option<Duration>,
  pub reuses: bool,
  pub remote_addr: Option<SocketAddr>,
}

impl ConnectionTiming {
  pub fn total_overhead(&self) -> Duration {
    let dns = self.dns_lookup.unwrap_or(Duration::ZERO);
    let tcp = self.tcp_connect.unwrap_or(Duration::ZERO);
    let tls = self.tls_handshake.unwrap_or(Duration::ZERO);

    dns + tcp + tls
  }
}

/// A channel for sending connection timing from the connector to the worker.
pub type TimingSender = tokio::sync::oneshot::Sender<ConnectionTiming>;
pub type TimingReceiver = tokio::sync::oneshot::Receiver<ConnectionTiming>;

#[derive(Clone)]
pub struct MeteredConnector {
  inner: HttpsConnector<HttpConnector>
}

impl MeteredConnector {
  pub fn new() -> Self {
    let mut http = HttpConnector::new();
    // Enable TCP_NODELAY — disables Nagle's algorithm.
    // Nagle buffers small packets, waiting to coalesce them.
    http.set_nodelay(true);
    http.set_keepalive(Some(Duration::from_secs(60)));

    let https = hyper_rustls::HttpsConnectorBuilder::new().with_native_roots().expect("Failed to load native TLS roots").https_or_http().enable_http1().enable_http2().wrap_connector(http);

    Self { inner: https }
  }
}

pub type MeteredStream = hyper_rustls::MaybeHttpsStream<TokioIo<TcpStream>>;

impl Service<Uri> for MeteredConnector {
  type Response = MeteredStream;
  type Error = Box<dyn std::error::Error + Send + Sync>;
  type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

  fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    self.inner.poll_ready(cx).map_err(Into::into)
  }

  fn call(&mut self, uri: Uri) -> Self::Future {
    let mut inner = self.inner.clone();

    Box::pin(async move {
      let connect_start = Instant::now();

      let stream = inner.call(uri).await?;

      let connect_duration = connect_start.elapsed();
      debug!(connect_ms = connect_duration.as_millis(), "New connection established");

      Ok(stream)
    })
  }
}