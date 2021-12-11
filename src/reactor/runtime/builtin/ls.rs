use super::*;

#[derive(Serialize, Deserialize)]
struct LsEntry {
    exchange_name: String,
    quote: String,
    base: String,
}

#[derive(Serialize)]
struct DereferencedLsEntry {
    exchange_name: String,
    quote: String,
    base: String,
    first_ohlc: Option<i64>,
    last_ohlc: Option<i64>,
    definition: MarketDefinition,
}

async fn get_markets(
    reactor: Reactor,
    exchange_filter: Option<&str>,
    base_filter: Option<String>,
    quote_filter: Option<String>,
) -> Vec<MarketIdentifier> {
    let mut available = Vec::new();
    for (name, exchange) in reactor
        .exchanges
        .read()
        .await
        .iter()
        .filter(|(name, _)| exchange_filter.as_ref().map(|e| e == *name).unwrap_or(true))
    {
        match exchange.lock().await.get_markets().await {
            Ok(markets) => available.extend(markets.into_iter().filter(|e| {
                quote_filter
                    .as_deref()
                    .map(|f| f == e.quote)
                    .unwrap_or(true)
                    && base_filter.as_deref().map(|f| f == e.base).unwrap_or(true)
            })),
            Err(e) => error!("Failed to fetch markets: EXCHANGE={}, ERROR={}", name, e),
        }
    }
    available
}
pub async fn main(
    reactor: Reactor,
    mut args: Vec<String>,
    _stdin: Option<Receiver<ProgramOutput>>,
    _stdout: Sender<ProgramOutput>,
) -> Result<ProgramOutput> {
    args.insert(0, "ls".to_string());
    let app = clap::App::new("ls")
        .arg(
            Arg::new("exchange_name")
                .takes_value(true)
                .index(1)
                .required(false),
        )
        .arg(Arg::new("definition").short('d').required(false));
    let app = app.try_get_matches_from(args)?;
    let exchange = app.value_of("exchange_name");
    let mut splited = exchange.unwrap_or("").split("/");
    let exchange = splited.next().filter(|e| *e != "");
    let base = splited
        .next()
        .map(|e| e.trim().to_uppercase())
        .filter(|e| *e != "" && *e != "*");
    let quote = splited
        .next()
        .map(|e| e.trim().to_uppercase())
        .filter(|e| *e != "" && *e != "*");
    let results = get_markets(reactor.clone(), exchange, base, quote).await;
    if app.is_present("definition") {
        let mut ret: Vec<DereferencedLsEntry> = Vec::with_capacity(results.len());
        for market in results {
            let mut first_ohlc = None;
            let mut last_ohlc = None;
            match reactor.get_or_register_market(&market).await {
                Ok(market) => {
                    first_ohlc = market.store.first_ohlc().ok().flatten().map(|e| e.time);
                    last_ohlc = market.store.last_ohlc().ok().flatten().map(|e| e.time);
                }
                Err(e) => {
                    let _ = dbg!(e);
                }
            }
            match reactor.exchanges.read().await[&market.exchange_name]
                .lock()
                .await
                .get_market_definition(&market, None)
                .await
            {
                Ok(definition) => {
                    ret.push(DereferencedLsEntry {
                        exchange_name: market.exchange_name,
                        quote: market.quote,
                        base: market.base,
                        first_ohlc,
                        last_ohlc,
                        definition,
                    });
                }
                Err(e) => {
                    let _ = dbg!(e);
                }
            }
        }
        Ok(ProgramOutput::Json { content: serde_json::to_value(&ret).unwrap_or_else(|_| Value::Null) })
    } else {
        let results: Vec<_> = results
            .into_iter()
            .map(|e| format!("{}/{}/{}", e.exchange_name, e.base, e.quote))
            .collect();
        Ok(ProgramOutput::json(&results)?)
    }
}
