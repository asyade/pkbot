use super::*;
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
pub mod ls;
pub mod sleep;
