use std::time::SystemTime;

use crate::{prelude::*, reactor};
use crate::exchange::MarketIdentifier;
use crate::reactor::SyncMarket;
use crate::store::MarketSettings;

#[derive(Debug, Deserialize, Serialize)]
pub struct GetAllMarketResult {
    available: Option<Vec<MarketIdentifier>>,
    loaded: Option<Vec<MarketIdentifier>>,
}

#[get("/?<available>&<loaded>")]
pub async fn get_all(
    reactor: &State<Reactor>,
    available: Option<bool>,
    loaded: Option<bool>,
) -> Result<Json<GetAllMarketResult>> {
    let available = if available.unwrap_or(false) {
        let mut available = Vec::new();
        for (name, exchange) in reactor.exchanges.lock().await.iter() {
            match exchange.lock().await.get_markets().await {
                Ok(mut markets) => available.append(&mut markets),
                Err(e) => error!("Failed to fetch markets: EXCHANGE={}, ERROR={}", name, e),
            }
        }
        Some(available)
    } else {
        None
    };
    let loaded = if loaded.unwrap_or(false) {
        let mut loaded = Vec::new();
        for (_, exchange) in reactor.store.trees.lock().unwrap().iter() {
            loaded.push(exchange.id.clone())
        }
        Some(loaded)
    } else {
        None
    };
    Ok(Json(GetAllMarketResult { available, loaded }))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetMarketResult {
    settings: MarketSettings,
    first_ohlc: Option<i64>,
    last_ohlc: Option<i64>,
}


#[get("/<exchange>/<base>/<quote>")]
pub async fn get(
    exchange: String,
    base: String,
    quote: String,
    reactor: &State<Reactor>,
) -> Result<Json<GetMarketResult>> {
    let id = MarketIdentifier {
        exchange_name: exchange,
        base,
        quote,
    };
    let market = reactor.get_or_register_market(id).await?;
    Ok(Json(GetMarketResult {
        settings: market.store.settings()?,
        first_ohlc: market.store.first_ohlc()?.map(|e| e.time),
        last_ohlc: market.store.last_ohlc()?.map(|e| e.time),
    }))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetMarketOhlcResult {
    data: Vec<OHLC>,
}

#[get("/<exchange>/<base>/<quote>/ohlc?<from>&<to>&<exact>")]
pub async fn get_ohlc(
    exchange: String,
    base: String,
    quote: String,
    from: i64,
    to: Option<i64>,
    exact: Option<bool>,
    reactor: &State<Reactor>,
) -> Result<Json<GetMarketOhlcResult>> {
    let id = MarketIdentifier {
        exchange_name: exchange,
        base,
        quote,
    };
    let market = reactor.get_or_register_market(id).await?;
    let to = if let Some(end) = to {
        end
    } else {
        market.store.last_ohlc()?.map(|e| e.time).unwrap_or(0)
    };
    let ohlc = if exact.unwrap_or(false) {
        market.store.exact_range(from, to)?
    } else {
        market.store.close_range(from, to)?
    };
    Ok(Json(GetMarketOhlcResult {
        data: ohlc,
    }))
}
