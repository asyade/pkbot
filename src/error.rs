use thiserror::Error;

use crate::interpretor::{ProgramOutput, ProgramStatus};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Kraken api error: {0}")]
    KrakenAPIError(#[from] kraken_sdk_rest::error::Error),
    #[error("io (std): {0}")]
    IO(#[from] std::io::Error),
    #[error("database: {0}")]
    Database(#[from] sled::Error),
    #[error("text encoding: {0}")]
    EncodingText(#[from] serde_json::error::Error),
    #[error("encoding: {0}")]
    Encoding(#[from] bincode::error::EncodeError),
    #[error("decode: {0}")]
    Decoding(#[from] bincode::error::DecodeError),
    #[error("api error: {0}")]
    ApiServer(#[from] rocket::Error),
    #[error("Missing environ: {0}")]
    MissingEnviron(&'static str),
    #[error("Exchange not found: {0}")]
    ExchangeNotFound(String),
    #[error("No data")]
    NoData,
    #[error("Pairs are not loaded")]
    PairNotLoaded,
    #[error("Parsing error: {0}")]
    Parsing(String, std::ops::Range<usize>),
    #[error("Reference not found: `{0}`")]
    ReferenceNotFound(String),
    #[error("The referenced scoop not exist: `{0}`")]
    ScoopNotFound(usize),
    #[error("{0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("Wrong interval: {0}")]
    InvalidInterval(i64),
    #[error("Arguments parsing: {0}")]
    Clap(#[from] clap::Error),
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

impl Into<ProgramOutput> for Error {
    fn into(self) -> ProgramOutput {
        ProgramOutput::Exit {
            message: Some(format!("{}", self)),
            status: ProgramStatus::Error,
        }
    }
}
