use crate::exchange::MarketIdentifier;
use crate::prelude::*;

#[derive(Debug, Deserialize, Serialize)]
pub struct GetMarketDataResult {}

#[get("/<market_name>/data?<from>&<to>")]
pub fn get(
    from: u64,
    to: u64,
    market_name: String,
    reactor: &State<Reactor>,
) -> Result<Json<GetMarketDataResult>> {
    Ok(unimplemented!())
}
