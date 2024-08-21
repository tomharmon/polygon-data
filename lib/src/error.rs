use displaydoc::Display;
use thiserror::Error;

use crate::types::AggregateRequestBuilderError;

#[derive(Debug, Display, Error)]
pub enum Error {
    /// Init error: {0}
    Init(#[from] Init),
    /// Io: {0}
    File(#[from] FileIo),
    /// URL is not valid
    InvalidUrl(#[from] url::ParseError),
    /// Error sending request: {0}
    SendRequest(reqwest::Error),
    /// Failed to deserialize response: {0}
    Deserialization(reqwest::Error),
    /// Unexpected status code: {0}
    UnexpectedStatus(reqwest::Error),
    /// Failed to deserialize response: {0}
    Serde(#[from] serde_json::Error),
    /// Invalid aggregate request: {0}
    InvalidRequest(#[from] AggregateRequestBuilderError),
}

#[derive(Debug, Display, Error)]
pub enum Init {
    /// Failed to initialize the client: {0}
    ClientInitialization(reqwest::Error),
    /// Invalid API key {0}
    InvalidApiKey(String),
    /// Invalid base URL: {0}
    InvalidBaseUrl(String),
}

#[derive(Debug, Display, Error)]
pub enum FileIo {
    /// Error writing CSV: {0}
    Csv(#[from] csv::Error),
    /// Error writing file: {0}
    FileWrite(std::io::Error),
    /// Error creating file: {0}
    CreateFile(std::io::Error),
}
