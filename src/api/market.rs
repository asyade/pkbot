use crate::prelude::*;

pub mod data;
use crate::exchange::MarketIdentifier;

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
            loaded.push(exchange.market.clone())
        }
        Some(loaded)
    } else {
        None
    };
    Ok(Json(GetAllMarketResult { available, loaded }))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetMarketResult {
}

#[get("/?<exchange>&<base>&<quote>")]
pub fn get(exchange: String, base: String, quote: String, reactor: &State<Reactor>) -> Result<Json<GetMarketResult>> {
    Ok(unimplemented!())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostMarket {
    market: MarketIdentifier,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostMarketResult {}

#[post("/", data = "<data>")]
pub fn post(data: Json<PostMarket>, reactor: &State<Reactor>) -> Result<Json<PostMarketResult>> {
    Ok(Json(PostMarketResult {}))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteMarketResult {
    success: bool,
}

#[delete("/<market_name>")]
pub fn delete(market_name: String, reactor: &State<Reactor>) -> Result<Json<DeleteMarketResult>> {
    Ok(Json(DeleteMarketResult { success: false }))
}
