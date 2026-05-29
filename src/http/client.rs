use reqwest::Client;
use crate::config::Config;
use crate::error::Result;

pub fn build_client(config: &Config) -> Result<Client> {
  let mut builder = Client::builder().timeout(config.timeout).tcp_keepalive(if config.keep_alive {
    Some(std::time::Duration::from_secs(60))  
  } else {
    None
  }).connection_verbose(false).pool_max_idle_per_host(if config.keep_alive {10} else {0});

  if !config.follow_redirects {
    builder = builder.redirect(reqwest::redirect::Policy::none());
  }

  // Add default headers
  let mut default_headers = reqwest::header::HeaderMap::new();
  default_headers.insert(
    reqwest::header::USER_AGENT,
    reqwest::header::HeaderValue::from_static("bastion/0.1.0"),
  );

  for (key, value) in &config.headers {
    let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes()).map_err(|e| crate::error::BastionError::InvalidConfig(format!("Invalid header name '{}': {}", key, e)))?;

    let header_value = reqwest::header::HeaderValue::from_str(value).map_err(|e| crate::error::BastionError::InvalidConfig(format!("Invalid header value '{}': {}", value, e)))?;
    default_headers.insert(header_name, header_value);
  }

  builder = builder.default_headers(default_headers);

  builder.build().map_err(Into::into)
}