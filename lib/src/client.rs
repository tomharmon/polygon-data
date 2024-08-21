use std::str::FromStr;

use reqwest::header::{self, HeaderMap, HeaderName, HeaderValue};
use tracing::{debug, instrument};
use url::Url;

use crate::{
    error::{self, Error},
    types::{AggregateRequest, AggregateResponse},
};

const MULIPLIER: usize = 1;
const BASE_URL: &str = "https://api.polygon.io";

#[derive(Clone)]
pub struct Client {
    inner: reqwest::Client,
}

impl Client {
    pub fn new(polygon_api_key: &str) -> Result<Self, error::Init> {
        let mut bearer =
            HeaderValue::from_str(&format!("Bearer {}", polygon_api_key))
                .map_err(|_| {
                    error::Init::InvalidApiKey(polygon_api_key.to_string())
                })?;
        bearer.set_sensitive(true);
        let headers = HeaderMap::from_iter([
            (HeaderName::from_static("authorization"), bearer),
            (header::ACCEPT, HeaderValue::from_static("application/json")),
        ]);
        let inner = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(error::Init::ClientInitialization)?;
        Ok(Self { inner })
    }

    #[instrument(skip_all, err, fields(ticker = %request.ticker))]
    pub async fn get_aggregate(
        &self,
        request: &AggregateRequest<'_>,
    ) -> Result<AggregateResponse, Error> {
        let AggregateRequest {
            ticker,
            timespan,
            from,
            to,
            next_url,
            limit,
        } = request;
        let from = from.timestamp_millis();
        let to = to.timestamp_millis();
        let url = if let Some(url) = next_url {
            Url::from_str(url)?
        } else {
            Url::from_str(&format!(
                "{BASE_URL}/v2/aggs/ticker/{ticker}/range/{MULIPLIER}/{timespan}/{from}/{to}?limit={limit}"
            ))?
        };

        let response = self
            .inner
            .get(url)
            .send()
            .await
            .map_err(Error::SendRequest)?;
        let status = response.status();
        let response: AggregateResponse = response
            .error_for_status()
            .map_err(Error::UnexpectedStatus)?
            .json()
            .await
            .map_err(Error::Deserialization)?;
        debug!(status = %status, num_results = %response.results.len(), "Got response");
        Ok(response)
    }
}
