use std::{path::PathBuf, str::FromStr};

use anyhow::{bail, Context, Error, Result};
use chrono::{DateTime, NaiveDate, Utc};
use clap::Parser;
use polygon_data::{
    config::{Config, Tickers},
    service::Service,
    types::Timespan,
};
use std::fs;
use tracing_subscriber::{fmt, layer::SubscriberExt, prelude::*, EnvFilter};

const DEFAULT_CHUNK_SIZE: u32 = 5_000;

/// CLI tool to download data from Polygon
#[derive(Parser, Debug)]
struct Args {
    /// File path to a config file that lists all the tickers to download data for
    #[clap(short, long)]
    config: PathBuf,
    /// The length of time for each candlestick.
    #[clap(short, long, default_value_t, value_parser = Timespan::from_str)]
    span: Timespan,
    /// The folder to save the downloaded data. Will be saved
    /// in this structure: `$output_dir/$ticker/$year/$month/$day.csv`
    #[clap(short, long)]
    output_dir: PathBuf,
    /// The starting date to pull data from
    #[clap(short, long)]
    from: NaiveDate,
    /// The ending date to pull data to
    #[clap(short, long)]
    to: NaiveDate,
    #[clap(env = "POLYGON_API_KEY")]
    polygon_api_key: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let file_appender = tracing_appender::rolling::daily(
        args.output_dir.clone(),
        "polygon-data.log",
    );
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(fmt::layer().with_ansi(false).with_writer(non_blocking))
        .with(EnvFilter::from_default_env())
        .init();
    let api_key = args.polygon_api_key.clone();
    let config = args.try_into()?;
    let service = Service::new(config, &api_key)?;
    service.fetch_data().await;
    Ok(())
}

impl TryFrom<Args> for Config {
    type Error = Error;
    fn try_from(args: Args) -> Result<Self, Self::Error> {
        let tickers = parse_config(args.config)?.tickers;
        let from = args.from.and_hms_opt(0, 0, 0).ok_or_else(|| {
            Error::msg("couldn't construct date with --from argument")
        })?;
        let from = DateTime::<Utc>::from_naive_utc_and_offset(from, Utc);
        let to = args.to.and_hms_opt(0, 0, 0).ok_or_else(|| {
            Error::msg("couldn't construct date with --to argument")
        })?;
        let to = DateTime::<Utc>::from_naive_utc_and_offset(to, Utc);
        Ok(Self {
            tickers,
            timespan: args.span,
            output_dir: args.output_dir,
            from,
            to,
            limit: DEFAULT_CHUNK_SIZE,
        })
    }
}

fn parse_config(path: PathBuf) -> Result<Tickers, Error> {
    let contents = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read file: {:?}", path))?;

    let extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("");

    match extension {
        "yaml" | "yml" => serde_yaml::from_str(&contents)
            .with_context(|| "Failed to parse YAML"),
        "toml" => {
            toml::from_str(&contents).with_context(|| "Failed to parse TOML")
        }
        "json" => serde_json::from_str(&contents)
            .with_context(|| "Failed to parse JSON"),
        _ => {
            bail!("Unknown extension")
        }
    }
}
