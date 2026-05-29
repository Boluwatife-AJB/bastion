use clap::Parser;
use tracing_subscriber::EnvFilter;
use anyhow::Context;

mod reporter;
mod metrics;
mod http;
mod display;
mod error;
mod config;
mod cli;
mod worker;
mod engine;
mod scheduler;
mod thresholds;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))
    ).with_target(false).compact().init();

    let cli = cli::BastionCli::parse();
    let config = cli.into_config().context("Failed to build configuration from CLI arguments")?;

    let snapshot = engine::run(config.clone()).await.context("Benchmark run failed")?;

    match config.output_format {
        crate::config::OutputFormat::Terminal => {
            reporter::terminal::print_report(&snapshot, &config);
        }
        crate::config::OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&snapshot)?;
            match &config.output_file {
                Some(path) => std::fs::write(path, json)?,
                None => println!("{json}")
            }
        }
        crate::config::OutputFormat::Csv => {
            // TODO: CSV REPORTER
            eprintln!("CSV output coming soon!")
        }
    }

    Ok(())
}