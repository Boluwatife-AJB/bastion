use bytes::Bytes;
use http_body_util::Full;
use hyper::Request;
use hyper::header::{HeaderName, HeaderValue, CONTENT_TYPE, USER_AGENT};

use crate::config::{Config, HttpMethod};
use crate::error::{BastionError, Result};


/// Build a hyper Request from config
pub fn build_request(config: &Config, body_bytes: Option<Bytes>) -> Result<Request<Full<Bytes>>> {
  let method = to_hyper_method(&config.method)?;
  let uri: hyper::Uri = config.url.parse().map_err(|e| BastionError::InvalidConfig(format!("Invalid URI: {e}")))?;

  let body = body_bytes.unwrap_or_else(Bytes::new);
  
  let mut req = Request::builder().method(method).uri(uri);

  // Add user agent
  req = req.header(USER_AGENT, "bastion/0.2.0");

  // Add content-type for bodies
  if !body.is_empty() {
    req = req.header(CONTENT_TYPE, "application/json");
  }

  for (key,value) in &config.headers {
    let name = HeaderName::from_bytes(key.as_bytes()).map_err(|e| BastionError::InvalidConfig(format!("Invalid header name '{key}': {e}")))?;
    let val = HeaderValue::from_str(value).map_err(|e| BastionError::InvalidConfig(format!("Invalid header value '{value}': {e}")))?;
    req = req.header(name, val);
  }

  req.body(Full::new(body)).map_err(|e| BastionError::InvalidConfig(format!("Failed to build request: {e}")))
}

fn to_hyper_method(method: &HttpMethod) -> Result<hyper::Method> {
  Ok(match method {
      HttpMethod::Get => hyper::Method::GET,
      HttpMethod::Post => hyper::Method::POST,
      HttpMethod::Put => hyper::Method::PUT,
      HttpMethod::Patch => hyper::Method::PATCH,
      HttpMethod::Delete => hyper::Method::DELETE,
      HttpMethod::Head => hyper::Method::HEAD,
  })
}