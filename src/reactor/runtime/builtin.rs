use std::{time::UNIX_EPOCH, process::ChildStdout};

use super::*;
use chrono::{DateTime, FixedOffset};
use clap::Arg;

#[macro_export]
macro_rules! buitlin_panic {
    ($stdout:expr, $fmt:expr $(, $($arg:tt)*)?) => {
        let fmt = format!($fmt, $($($arg)*)?);
        let _ = $stdout.send(ProgramOutput::Exit {
            message: Some(fmt),
            status: ProgramStatus::Error,
        }).await.map_err(|e| log::error!("Failed to send output of buitlin to stdout: ERROR={}", e));
    };
}

#[macro_export]
macro_rules! buitlin_print {
    ($stdout:expr, $fmt:expr $(, $($arg:tt)*)?) => {
        let fmt = format!($fmt, $($($arg)*)?);
        let _ = $stdout.send(ProgramOutput::Text {
            message: fmt,
        }).await.map_err(|e| log::error!("Failed to send output of buitlin to stdout: ERROR={}", e));
    };
}

#[macro_export]
macro_rules! buitlin_result {
    ($stdout:expr, $payload:expr) => {{
        let _ = $stdout
            .send(ProgramOutput::Json { content: $payload })
            .await
            .map_err(|e| log::error!("Failed to send output of buitlin to stdout: ERROR={}", e));
        buitlin_result!($stdout)
    }};
    ($stdout:expr) => {{
        let _ = $stdout
            .send(ProgramOutput::Exit {
                message: None,
                status: ProgramStatus::Success,
            })
            .await
            .map_err(|e| log::error!("Failed to send output of buitlin to stdout: ERROR={}", e));
    }};
}

pub mod cat;
pub mod echo;
pub mod ls;
pub mod sleep;

enum ArgumentTimestamp {
    Absolute {
        date: DateTime<FixedOffset>,
    },
    Relative {
        since: Duration,
        crtime: SystemTime,
    }
}

impl ArgumentTimestamp {
    pub fn new(raw: &str, crtime: SystemTime) -> Result<Self> {
        if raw.starts_with("NOW-") {
            let mut splited = raw.split("-");
            let since = splited.nth(1).map(|e| e.parse().ok()).flatten().ok_or_else(|| Error::Parsing(raw.to_string(), 3..0))?;
            Ok(ArgumentTimestamp::Relative {
                since: Duration::from_secs(since),
                crtime,
            })
        } else {
            Ok(ArgumentTimestamp::Absolute {
                date: unimplemented!(),
            })
        }
    }

    pub fn timestamp(&self) -> i64 {
        match self {
            ArgumentTimestamp::Absolute{..} => unimplemented!(),
            ArgumentTimestamp::Relative { since, crtime } => {
                crtime.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64 - since.as_secs() as i64
            },
        }
    }
}

pub async fn try_builtin<F: Future<Output = Result<ProgramOutput>> + 'static>(f: F, stdout: Sender<ProgramOutput>) {
    match f.await {
        Ok(res) => {
            let _ = stdout.send(res).await;
        },
        Err(e) => {
            let _ = stdout.send(e.into()).await;
        }
    }
}