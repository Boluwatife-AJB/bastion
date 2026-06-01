use std::path::Path;
use std::process;

use clap::Parser;
use tracing_subscriber::EnvFilter;
use anyhow::Context;
use bastion::{cli, config, engine, metrics, reporter, thresholds, compare, error};

// mod reporter;
// mod metrics;
// mod http;
// mod display;
// mod error;
// mod config;
// mod cli;
// mod worker;
// mod engine;
// mod scheduler;
// mod thresholds;
// mod compare;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))
    ).with_target(false).compact().init();

    let cli = cli::BastionCli::parse();
    let config = cli.into_config().context("Configuration error")?;

    let assertions: Vec<thresholds::Threshold> = config.assertions.iter().map(|s| thresholds::parse_threshold(s)).collect::<error::Result<Vec<_>>>().context("Invalid threshold assertions")?;

    let baseline = config.compare.as_deref().map(|path| reporter::json::load_baseline(Path::new(path))).transpose().context("Failed to load baseline report")?;

    let (snapshot, windows) = engine::run(config.clone()).await.context("Benchmark run failed")?;

    let _use_stderr_for_human = matches!(config.output_format, crate::config::OutputFormat::Json);

    // Terminal report
    if matches!(config.output_format, config::OutputFormat::Terminal) {
        reporter::terminal::print_report(&snapshot, &config);
        reporter::terminal::print_time_series(&windows, 20);

        if let Some(ref summary) = windows.first().map(|_| {metrics::summary::summarize_windows(&windows, snapshot.total_duration)}) {
            reporter::terminal::print_spike_analysis(summary, &windows);
        }
    }

    // Comparison report
    let comparison = baseline.as_ref().map(|base| {
        compare::ComparisonReport::compare(base, &snapshot, config.regression_threshold)
    });

    if let Some(ref cmp) = comparison {
        reporter::terminal::print_comparison(cmp);
    }

    // Threshold assertions
    let threshold_results: Vec<thresholds::ThresholdResult> = assertions.iter().map(|t| t.clone().evaluate(&snapshot)).collect();

    if !threshold_results.is_empty() {
        reporter::terminal::print_thresholds(&threshold_results);
    }

    // JSON output
    match (&config.output_format, &config.output_file) {
        (config::OutputFormat::Json, Some(path)) => {
            let mut file = std::fs::File::create(path).context("Failed to create JSON output file")?;
            reporter::json::write_json_report(&snapshot, &windows, comparison.as_ref(), &mut file)?;
            eprintln!("JSON report written to {path}");
        }
        (config::OutputFormat::Json, None) => {
            reporter::json::write_json_report(&snapshot, &windows, comparison.as_ref(), &mut std::io::stdout())?;
        }
        _ => {}
    }

    // CSV time-series output
    if let Some(ref csv_path) = config.csv_file {
        let mut file = std::fs::File::create(csv_path).context("Failed to create CSV output file")?;
        reporter::csv::write_csv_report(&windows, &mut file)?;
        eprintln!("CSV time-series written to {csv_path}")
    }


    // Exit code
    let assertion_failed = threshold_results.iter().any(|r| !r.passed);
    let comparison_regressed = comparison.as_ref().map(|c| c.any_regression()).unwrap_or(false);

    if assertion_failed || comparison_regressed {
        process::exit(1)
    }

    Ok(())
}