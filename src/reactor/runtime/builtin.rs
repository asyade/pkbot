use std::time::UNIX_EPOCH;

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

#[derive(Debug, Clone)]
pub struct ArgumentInterval {
    pub raw: String,
    pub normalized: Interval,
}

impl ArgumentInterval {
    pub fn new(raw: &str) -> Result<ArgumentInterval> {
        let raw = raw.to_lowercase();
        match raw.as_str() {
            "1m" => Ok(Self {
                raw,
                normalized: Interval::Min1,
            }),
            "5m" => Ok(Self {
                raw,
                normalized: Interval::Min5,
            }),
            "15m" => Ok(Self {
                raw,
                normalized: Interval::Min15,
            }),
            "30m" => Ok(Self {
                raw,
                normalized: Interval::Min30,
            }),
            "1h" => Ok(Self {
                raw,
                normalized: Interval::Hour1,
            }),
            "4h" => Ok(Self {
                raw,
                normalized: Interval::Hour4,
            }),
            "1d" => Ok(Self {
                raw,
                normalized: Interval::Day1,
            }),
            "7d" => Ok(Self {
                raw,
                normalized: Interval::Day7,
            }),
            "15d" => Ok(Self {
                raw,
                normalized: Interval::Day15,
            }),
            _ => Err(Error::Parsing(raw, 0..0)),
        }
    }

    pub fn validator(raw: &str) -> std::result::Result<(), String> {
        Self::new(raw).map_err(|_| {
            format!(
                "Wrong interval format\n    Found `{}`\n    Expedted one of: 1m, 5m, 15m, 30m, 1h, 4h, 1d, 7d, 15d",
                raw,
            )
        })?;
        Ok(())
    }
}

fn human_duration(str: &str) -> Result<Duration> {
    let str = str.to_lowercase();
    if str.ends_with("s") {
        let parsed: u64 = str[..str.len() - 1].parse()?;
        Ok(Duration::from_secs(parsed))
    } else if str.ends_with("m") {
        let parsed: u64 = str[..str.len() - 1].parse()?;
        Ok(Duration::from_secs(parsed * 60))
    } else if str.ends_with("h") {
        let parsed: u64 = str[..str.len() - 1].parse()?;
        Ok(Duration::from_secs(parsed * 60 * 60))
    } else if str.ends_with("d") {
        let parsed: u64 = str[..str.len() - 1].parse()?;
        Ok(Duration::from_secs(parsed * 60 * 60 * 24))
    } else {
        Ok(Duration::from_secs(str[..str.len()].parse()?))
    }
}

#[derive(Debug, Clone)]
pub enum ArgumentTimestamp {
    Absolute { date: DateTime<FixedOffset> },
    RelativeToNow { delta: Duration, crtime: SystemTime },
}

impl ArgumentTimestamp {
    pub fn new(raw: &str, crtime: SystemTime) -> Result<Self> {
        if raw.starts_with("NOW-") {
            let mut splited = raw.split("-");
            let left = splited.next();
            let right = splited.next();
            match (left, right) {
                (Some(_), Some(raw)) => {
                    return Ok(ArgumentTimestamp::RelativeToNow {
                        crtime,
                        delta: human_duration(raw)?,
                    })
                }
                _ => return Err(Error::Parsing(raw.to_string(), 3..0)),
            }
        } else {
            Ok(ArgumentTimestamp::Absolute {
                date: DateTime::parse_from_rfc3339(raw)
                    .map_err(|_| Error::Parsing(raw.to_string(), 3..0))?,
            })
        }
    }

    pub fn validator(raw: &str) -> std::result::Result<(), String> {
        let _ = Self::new(raw, SystemTime::now()).map_err(|_| {
            String::from("Wrong time format, format must be one of:\n    rfc3339: `1996-12-19T16:39:57-08:00`\n    UNIX timestamp: `1639239687``")
        })?;
        Ok(())
    }

    pub fn timestamp(&self) -> i64 {
        match self {
            ArgumentTimestamp::Absolute { date } => date.timestamp(),
            ArgumentTimestamp::RelativeToNow { delta, crtime } => {
                crtime.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64 - delta.as_secs() as i64
            }
        }
    }
}

pub async fn try_builtin<F: Future<Output = Result<ProgramOutput>> + 'static>(
    f: F,
    stdout: Sender<ProgramOutput>,
) {
    match f.await {
        Ok(res) => {
            let _ = stdout.send(res).await;
        }
        Err(e) => {
            let _ = stdout.send(e.into()).await;
        }
    }
}
