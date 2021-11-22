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
    #[error("Missing environ: {0}")]
    MissingEnviron(&'static str),
    #[error("Exchange not found: {0}")]
    ExchangeNotFound(String),
    #[error("Market not found: EXCHANGE={0} MARKET={1}")]
    MarketNotFound(String, String),
    #[error("No data")]
    NoData,
    #[error("Pairs are not loaded")]
    PairNotLoaded,
}

impl<'r> rocket::response::Responder<'r, 'static> for crate::error::Error {
    fn respond_to(self, r: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        log::error!(
            "An error occured {} {:?} {:?} {:?}",
            self,
            r.method(),
            r.uri().path(),
            r.uri().query(),
        );
        rocket::response::Result::Ok(
            rocket::response::status::Custom(
                rocket::http::Status::InternalServerError,
                rocket::serde::json::Json(format!("{}", self)),
            )
            .respond_to(r)?,
        )
    }
}