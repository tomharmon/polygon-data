use std::{
    fs::{File, OpenOptions},
    path::PathBuf,
    time::Duration,
};

use crate::{
    client::Client,
    config::Config,
    error::{self, Error},
    types::{
        AggregateRecord, AggregateRequest, AggregateRequestBuilder, Timespan,
    },
};
use chrono::{DateTime, Utc};
use csv::WriterBuilder;
use futures::stream::{self, BoxStream, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use tokio::{fs, time::sleep};
use tracing::{debug, error, info, instrument, warn};

const CONCURRENCY_LIMIT: usize = 10;

pub struct Service {
    client: Client,
    config: Config,
}

impl Service {
    pub fn new(config: Config, polygon_api_key: &str) -> Result<Self, Error> {
        let client = Client::new(polygon_api_key)?;
        Ok(Self { client, config })
    }

    #[instrument(skip_all)]
    pub async fn fetch_data(&self) {
        info!(
            num_tickers = self.config.tickers.len(),
            timespan = %self.config.timespan,
            output_dir = ?self.config.output_dir,
            from = %self.config.from,
            to = %self.config.to,
            "Starting to fetch data..."
        );

        let num_chunks = num_chunks(
            self.config.timespan,
            self.config.from,
            self.config.to,
            self.config.limit,
        );
        let progress_bar = ProgressBar::new(
            self.config.tickers.len() as u64 * num_chunks as u64,
        )
        .with_style(style());
        stream::iter(&self.config.tickers)
            .for_each_concurrent(CONCURRENCY_LIMIT,|ticker| {
                let pb = progress_bar.clone();
                async move {
                    let request = match AggregateRequestBuilder::default()
                        .timespan(self.config.timespan)
                        .ticker(ticker)
                        .from(self.config.from)
                        .to(self.config.to)
                        .limit(self.config.limit)
                        .build() {
                            Ok(request) => request,
                            Err(e) => {
                                error!(error = %e, ticker = %ticker, "Encountered an error when building a request");
                                return;
                            }
                        };
                    tracing::info!(ticker = %ticker, "Fetching data for ticker");
                    let _result = self.save_aggregates_to_disk(request, pb).await.inspect_err(|e| {
                        error!(error = %e, ticker = %ticker, "Encountered an error when processing a ticker");
                    });
                    tracing::info!(ticker = %ticker, "Finished fetching data for ticker");
                }
            })
            .await;

        progress_bar.finish();
        info!("Finished fetching data!");
    }

    #[instrument(skip_all, fields(ticker = %request.ticker))]
    async fn get_aggregates<'a>(
        &'a self,
        request: AggregateRequest<'a>,
    ) -> BoxStream<Result<Vec<AggregateRecord>, Error>> {
        let client = self.client.clone();
        let stream = stream::unfold(
            (request, None, false),
            move |(mut request, next_url, final_page)| {
                let client = client.clone();
                async move {
                    if final_page {
                        return None;
                    } else if let Some(url) = next_url {
                        request.next_url = Some(url);
                    }

                    match client.get_aggregate(&request).await {
                        Ok(response) if response.next_url.is_some() => Some((
                            Ok(response.results),
                            (request, response.next_url, false),
                        )),
                        Ok(response) => {
                            debug!(
                                num_results = response.results_count,
                                "Got final page of data"
                            );
                            Some((
                                Ok(response.results),
                                (request, response.next_url, true),
                            ))
                        }
                        Err(e) => Some((Err(e), (request, None, false))),
                    }
                }
            },
        );

        stream.boxed()
    }

    #[instrument(skip_all, err, fields(ticker = %request.ticker))]
    pub async fn save_aggregates_to_disk<'a>(
        &'a self,
        request: AggregateRequest<'a>,
        progress_bar: ProgressBar,
    ) -> Result<(), Error> {
        let ticker = &request.ticker;
        let timespan = &request.timespan;
        let file_path = self
            .config
            .output_dir
            .join(format!("{ticker}/{timespan}.csv"));
        let parent_dir = file_path
            .parent()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Output directory must have at least one parent",
                )
            })
            .map_err(error::FileIo::CreateFile)?;
        fs::create_dir_all(parent_dir)
            .await
            .map_err(error::FileIo::CreateFile)?;
        let file = create_or_open_file(file_path)?;
        let mut writer = WriterBuilder::new().flexible(true).from_writer(file);
        let mut stream = self.get_aggregates(request).await;
        while let Some(result) = stream.next().await {
            match result {
                Ok(records) if records.is_empty() => {
                    warn!("Got no results");
                }
                Ok(records) => {
                    debug!(num_records = %records.len(), "Processing batch of recrods");
                    for record in records {
                        writer.serialize(record).map_err(error::FileIo::Csv)?;
                    }
                    writer.flush().map_err(error::FileIo::FileWrite)?;
                }
                Err(e) => {
                    error!("Error when getting next item from stream");
                    // Once we are more intelligent about appending data
                    // we could potentially remove this return
                    return Err(e);
                }
            }
            progress_bar.inc(1);
            sleep(Duration::from_millis(20)).await
        }

        Ok(())
    }
}

// According to Polygon docs, it should work
/// Estimate the number of chunks for the given `timespan` and the time interval
fn num_chunks(
    timespan: Timespan,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    limit: u32,
) -> i64 {
    let duration = to - from;
    // according to https://polygon.io/blog/aggs-api-updates
    let num_intervals = match timespan {
        // tbh I'm not sure Polygon's behavior for seconds, beware this is untested
        Timespan::Second => duration.num_seconds(),
        Timespan::Minute => duration.num_minutes(),
        Timespan::Hour => duration.num_hours(),
        Timespan::Day => duration.num_days(),
        Timespan::Week => duration.num_weeks(),
        // the following branches are obviously technically incorrect,
        // but this is just used for an estimate so it's fine
        Timespan::Month => duration.num_weeks() / 4,
        Timespan::Quarter => duration.num_weeks() / 12,
        Timespan::Year => duration.num_days() / 365,
    };

    num_intervals / i64::from(limit)
}

fn create_or_open_file(file_path: PathBuf) -> Result<File, error::FileIo> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .map_err(error::FileIo::CreateFile)
}

fn style() -> ProgressStyle {
    ProgressStyle::with_template(
        "[{elapsed}] {bar:40.cyan/blue} {pos:>4}/{len:4} {percent}% {msg}",
    )
    .expect("always valid if tests pass")
}

#[cfg(test)]
mod tests {
    use super::style;

    #[test]
    fn style_is_valid() {
        let _ = style();
    }
}
