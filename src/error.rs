use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Kraken api error: {0}")]
    KrakenAPIError(#[from] kraken_sdk_rest::error::Error),
    #[error("The specified pair dosent exist: {0}")]
    PairNotFound(String),
    #[error("io (std): {0}")]
    IO(#[from] std::io::Error),
    #[error("database: {0}")]
    Database(#[from] sled::Error),
    #[error("encoding: {0}")]
    Encoding(#[from] bincode::error::EncodeError),
    #[error("decode: {0}")]
    Decoding(#[from] bincode::error::DecodeError),
    #[error("peak finder error: {0}")]
    PeakFinder(&'static str),
    #[error("api error: {0}")]
    ApiServer(#[from] rocket::Error),
}
