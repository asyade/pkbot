use super::*;
use crate::prelude::*;
use kraken_sdk_rest::{api::AddOrderRequest, Client, Interval, PairName};

pub struct KrakenExchange {
    pub api_key: String,
    pub api_private_key: String,
    pub client: Client,
}

impl KrakenExchange {
    pub fn new(api_key: String, api_private_key: String) -> Self {
        Self {
            client: Client::new(&api_key, &api_private_key),
            api_key,
            api_private_key,
        }
    }
}

#[async_trait]
impl Exchange for KrakenExchange {
    fn name(&self) -> String {
        String::from("kraken")
    }

    async fn get_severt_time(&self) -> Result<NaiveDateTime> {
        let server_time = self.client.get_server_time().send().await?;
        Ok(NaiveDateTime::from_timestamp(server_time.unixtime, 0))
    }
    async fn get_ohlc(&self, id: &MarketIdentifier, since: u64) -> Result<OHLCChunk> {
        Ok(OHLCChunk::new(
            self.client
                .get_ohlc_data(PairName::from(&id.base, &id.quote))
                .interval(Interval::Min1)
                .since(since)
                .send()
                .await?
                .into_iter()
                .map(|e| super::OHLC::new(e.0, e.1, e.2, e.3, e.4, e.5, e.6, e.7))
                .collect(),
        ))
    }

    async fn get_markets(&self) -> Result<Vec<MarketIdentifier>> {
        let pairs = self.client.get_asset_pairs().send().await?;
        Ok(pairs
            .into_iter()
            .map(|(k, v)| MarketIdentifier {
                quote: v.quote,
                base: v.base,
                exchange_name: self.name(),
            })
            .collect())
    }
}
