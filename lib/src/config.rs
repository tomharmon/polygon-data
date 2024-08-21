use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::types::Timespan;

#[derive(Deserialize, Clone)]
pub struct Tickers {
    /// A list of tickers to download data for.
    pub tickers: Vec<String>,
}

#[derive(Clone)]
pub struct Config {
    /// A list of tickers to download data for.
    pub tickers: Vec<String>,
    /// The timespan for each candlestick.
    pub timespan: Timespan,
    /// The folder to save the results. Results will be saved
    /// in this structure: `$output_dir/$ticker/$year/$month/$day.csv`
    pub output_dir: PathBuf,
    /// The starting date to pull data from
    pub from: DateTime<Utc>,
    /// The ending date to pull data to
    pub to: DateTime<Utc>,
    /// How many records to fetch in one chunk
    pub limit: u32,
}
