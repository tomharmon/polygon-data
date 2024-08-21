use chrono::{DateTime, Utc};
use derive_builder::Builder;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Default,
    Deserialize,
    Serialize,
    Clone,
    Copy,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Timespan {
    Second,
    #[default]
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

#[derive(Builder)]
#[builder(setter(strip_option))]
pub struct AggregateRequest<'a> {
    pub(crate) ticker: &'a str,
    #[builder(default)]
    pub(crate) timespan: Timespan,
    pub(crate) from: DateTime<Utc>,
    pub(crate) to: DateTime<Utc>,
    #[builder(default)]
    pub(crate) next_url: Option<String>,
    pub(crate) limit: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AggregateRecord {
    /// The Unix Msec timestamp for the start of the aggregate window.
    #[serde(alias = "t", default)]
    pub timestamp: i64,
    /// The open price for the symbol in the given time period.
    #[serde(alias = "o")]
    pub open: Decimal,
    /// The highest price for the symbol in the given time period.
    #[serde(alias = "h")]
    pub high: Decimal,
    /// The lowest price for the symbol in the given time period.
    #[serde(alias = "l")]
    pub low: Decimal,
    /// The close price for the symbol in the given time period.
    #[serde(alias = "c")]
    pub close: Decimal,
    /// The trading volume of the symbol in the given time period.
    #[serde(alias = "v", default)]
    pub volume: Decimal,
    /// The number of transactions in the aggregate window.
    #[serde(alias = "n", default, skip_serializing_if = "Option::is_none")]
    pub transactions: Option<usize>,
    /// Whether or not this aggregate is for an OTC ticker. This field will be left off if false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub otc: Option<bool>,
    /// The volume weighted average price
    #[serde(alias = "vw", default, skip_serializing_if = "Option::is_none")]
    pub vwap: Option<Decimal>,
}

#[derive(Deserialize)]
pub struct AggregateResponse {
    pub ticker: String,
    pub adjusted: bool,
    #[serde(alias = "queryCount")]
    pub query_count: i64,
    pub request_id: String,
    #[serde(alias = "resultsCount")]
    pub results_count: usize,
    pub status: String,
    #[serde(default)]
    pub results: Vec<AggregateRecord>,
    pub next_url: Option<String>,
}
