use super::*;

pub async fn main(
    reactor: Reactor,
    mut args: Vec<String>,
    _stdin: Option<Receiver<ProgramOutput>>,
    _stdout: Sender<ProgramOutput>,
) -> Result<ProgramOutput> {
    args.insert(0, "cat".to_string());
    let app = clap::App::new("cat")
        .arg(
            Arg::new("from")
                .validator(ArgumentTimestamp::validator)
                .takes_value(true)
                .short('f')
                .long("from"),
        )
        .arg(
            Arg::new("to")
                .validator(ArgumentTimestamp::validator)
                .takes_value(true)
                .short('t')
                .long("to"),
        )
        .arg(
            Arg::new("interval")
                .validator(ArgumentInterval::validator)
                .takes_value(true)
                .required(true)
                .short('i')
                .long("interval"),
        )
        .arg(
            Arg::new("market_name")
                .required(true)
                .takes_value(true)
                .multiple_values(true),
        );
    let app = app.try_get_matches_from(args)?;
    let now = SystemTime::now();
    let from = app
        .value_of("from")
        .and_then(|e| ArgumentTimestamp::new(e, now).ok())
        .map(|e| e.timestamp())
        .unwrap_or(0);
    let to = app
        .value_of("to")
        .and_then(|e| ArgumentTimestamp::new(e, now).ok())
        .map(|e| e.timestamp())
        .unwrap_or(now.duration_since(UNIX_EPOCH).unwrap().as_secs() as Timestamp);
    let interval = ArgumentInterval::new(app.value_of("interval").unwrap()).unwrap();
    let markets = app.values_of("market_name").unwrap();
    let mut results = Vec::new();
    for val in markets.into_iter() {
        let id = MarketIdentifier::from(val);
        let target_market = reactor.get_or_register_market(&id).await?;
        let range = target_market
            .sync_periode(from, to, interval.normalized)
            .await?;
        let _ = dbg!(range);
        let chunk = target_market
            .interval(interval.normalized)
            .await?
            .close_range(from, to)?;
        results.push(chunk);
    }
    Ok(unimplemented!())
    // Ok(ProgramOutput::json(&results)?)
}
