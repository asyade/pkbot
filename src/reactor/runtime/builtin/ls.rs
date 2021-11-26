use super::*;

#[derive(Serialize, Deserialize)]
struct LsEntry {
    exchange_name: String,
    quote: String,
    base: String,
}

async fn get_markets(
    reactor: Reactor,
    exchange_filter: Option<&str>,
    base_filter: Option<String>,
    quote_filter: Option<String>,
) -> Vec<MarketIdentifier> {
    let mut available = Vec::new();
    for (name, exchange) in reactor.exchanges
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
        stdout: Sender<ProgramOutput>,
    ) {
        args.insert(0, "ls".to_string());
        let app = clap::App::new("ls").arg(Arg::new("exchange_name").index(1).required(false));
        match app.try_get_matches_from(args) {
            Ok(app) => {
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
                let results: Vec<String> = 
                    get_markets(reactor, exchange, base, quote)
                    .await
                    .into_iter()
                    .map(|e| format!("{}/{}/{}", e.exchange_name, e.base, e.quote))
                    .collect();
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