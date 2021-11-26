use super::*;
use crate::exchange::MarketIdentifier;

async fn get_metrics(
    reactor: &Reactor,
    market: String,
    from: i64,
    to: Option<i64>,
    exact: bool,
) -> Result<Vec<OHLC>> {
    let market = reactor.get_or_register_market(market.into()).await?;
    let to = if let Some(end) = to {
        end
    } else {
        market.store.last_ohlc()?.map(|e| e.time).unwrap_or(0)
    };
    let ohlc = if exact {
        market.store.exact_range(from, to)?
    } else {
        market.store.close_range(from, to)?
    };
    Ok(ohlc)
}

pub async fn main(
        reactor: Reactor,
        mut args: Vec<String>,
        _stdin: Option<Receiver<ProgramOutput>>,
        stdout: Sender<ProgramOutput>,
    ) {
        args.insert(0, "cat".to_string());
        let app = clap::App::new("cat").arg(
            Arg::new("market_name")
                .required(true)
                .takes_value(true)
                .multiple_values(true),
        );
        match app.try_get_matches_from(args) {
            Ok(app) => {
                let input = app.values_of("market_name").unwrap();
                let mut results = Vec::new();

                for val in input.into_iter() {
                    results.push(
                        get_metrics(&reactor, val.to_string(), 0, None, false)
                            .await
                            .unwrap_or_else(|_| Vec::new()),
                    );
                }
                buitlin_result!(
                    stdout,
                    serde_json::to_value(&results).unwrap_or_else(|_| Value::Null)
                );
            }
            Err(e) => {
                buitlin_panic!(stdout, "{}", e);
            }
        }
    }