use super::*;

async fn get_metrics(
    reactor: &Reactor,
    market: String,
    from: ArgumentTimestamp,
    to: Option<ArgumentTimestamp>,
) -> Result<Vec<OHLC>> {
    let id = market.into();
    let market = reactor.get_or_register_market(&id).await?;
    let to = if let Some(end) = to {
        end.timestamp()
    } else {
        market.store.last_ohlc()?.map(|e| e.time).unwrap_or(0)
    };
    let ohlc = market.store.close_range(from.timestamp(), to)?;
    Ok(ohlc)
}

pub async fn main(
    reactor: Reactor,
    mut args: Vec<String>,
    _stdin: Option<Receiver<ProgramOutput>>,
    _stdout: Sender<ProgramOutput>,
) -> Result<ProgramOutput> {
    args.insert(0, "cat".to_string());
    let app = clap::App::new("cat")
        .arg(Arg::new("from").takes_value(true).short('f').long("from"))
        .arg(Arg::new("to").takes_value(true).short('t').long("to"))
        .arg(Arg::new("interval").takes_value(true).short('i').long("interval"))
        .arg(
            Arg::new("market_name")
                .required(true)
                .takes_value(true)
                .multiple_values(true),
        );
    let app = app.try_get_matches_from(args)?;
    let input = app.values_of("market_name").unwrap();
    let mut results = Vec::new();
    for val in input.into_iter() {
        results.push(
            get_metrics(&reactor, val.to_string(), unimplemented!(), None)
                .await
                .unwrap_or_else(|_| Vec::new()),
        );
    }
    Ok(ProgramOutput::json(&results)?)
}
