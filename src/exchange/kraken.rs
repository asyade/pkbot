use super::*;
use crate::reactor::SyncExchange;
use kraken_sdk_rest::{Client, Interval};
use tokio::sync::OwnedMutexGuard;

pub static EXCHANGE_NAME: &str = "kraken";

pub struct KrakenExchange {
    pub api_key: String,
    pub api_private_key: String,
    pub client: Client,
    pub markets_cache: Arc<Mutex<MarketCacheCell>>,
}

type MarketCacheCell = Option<HashMap<MarketIdentifier, MarketDefinition>>;

impl KrakenExchange {
    pub fn new(api_key: String, api_private_key: String) -> Self {
        Self {
            client: Client::new(&api_key, &api_private_key),
            api_key,
            api_private_key,
            markets_cache: Arc::new(Mutex::new(None)),
        }
    }

    pub fn from_env() -> Result<KrakenExchange> {
        let api_private_key = std::env::var("KRAKEN_API_PRIVATE_KEY")
            .map_err(|_| Error::MissingEnviron("KRAKEN_API_PRIVATE_KEY"))?;
        let api_key =
            std::env::var("KRAKEN_API_KEY").map_err(|_| Error::MissingEnviron("KRAKEN_API_KEY"))?;
        Ok(Self::new(api_key, api_private_key))
    }

    pub fn boxed(self) -> SyncExchange {
        Arc::new(Mutex::new(Box::new(self)))
    }

    async fn refresh_market_cache(
        mut lock: OwnedMutexGuard<MarketCacheCell>,
        client: &Client,
    ) -> Result<()> {
        // let mut lock = self.markets_cache.lock().await;
        let pairs = client.get_asset_pairs().send().await?;
        let mut map = HashMap::new();
        for (_, pair) in pairs {
            let id = MarketIdentifier {
                exchange_name: EXCHANGE_NAME.to_string(),
                base: pair.base,
                quote: pair.quote,
            };
            let age = std::time::SystemTime::now();
            let def = MarketDefinition {
                pairname: pair.altname,
                pair_decimals: pair.pair_decimals,
                lot_decimals: pair.lot_decimals,
                lot_multiplier: pair.lot_multiplier,
                leverage_buy: pair.leverage_buy,
                leverage_sell: pair.leverage_sell,
                fees: pair.fees.into_iter().map(|e| (e.0, e.1)).collect(),
                fees_maker: pair
                    .fees_maker
                    .map(|e| e.into_iter().map(|e| (e.0, e.1)).collect()),
                margin_call: pair.margin_call,
                margin_stop: pair.margin_stop,
                ordermin: pair.ordermin,
                age,
            };
            map.insert(id, def);
        }
        log::trace!(
            "Refresh market cache DONE: EXCHANGE={}, NBR_ENTRIES={}",
            EXCHANGE_NAME,
            map.len()
        );
        lock.replace(map.clone());
        Ok(())
    }
}

#[async_trait]
impl Exchange for KrakenExchange {
    fn name(&self) -> String {
        String::from(EXCHANGE_NAME)
    }

    async fn get_severt_time(&self) -> Result<NaiveDateTime> {
        let server_time = self.client.get_server_time().send().await?;
        Ok(NaiveDateTime::from_timestamp(server_time.unixtime, 0))
    }

    async fn get_ohlc(
        &self,
        id: &MarketIdentifier,
        mut since: Timestamp,
        interval: crate::prelude::Interval,
    ) -> Result<OHLCChunk> {
        let original_since = since;
        let market = self.get_market_definition(id, None).await?;
        let mut chunk = Vec::new();
        loop {
            log::trace!("Request OHLC chunk to external API: EXCHANGE={}, BASE={}, QUOTE={}, SINCE={}, INTERVAL={}", &id.exchange_name, &id.base, &id.quote, since, interval);
            let mut sub_chunk_len = 0;
            let sub_chunk = self
                .client
                .get_ohlc_data(market.pairname.clone())
                .interval(interval.into())
                .since(since as u64)
                .send()
                .await?;
            log::trace!("OHLC chunk received from API: EXCHANGE={}, BASE={}, QUOTE={}, SINCE={}, INTERVAL={}, NBR_CANDLES={}", &id.exchange_name, &id.base, &id.quote, since, interval, sub_chunk.len());
            let sub_chunk = sub_chunk.into_iter().map(|e| {
                since = e.0;
                sub_chunk_len += 1;
                super::OHLC::new(false, e.0, e.1, e.2, e.3, e.4, e.5, e.6, e.7)
            });
            chunk.extend(sub_chunk);
            if original_since == 0 && sub_chunk_len > 0 {
                chunk[0].first_available = true;
            }
            if sub_chunk_len <= 1 {
                break;
            }
        }
        Ok(OHLCChunk::new(chunk))
    }

    async fn refresh_market_cache(&self) -> Result<()> {
        log::trace!("Refresh market cache: EXCHANGE={}", EXCHANGE_NAME);
        let lock = self.markets_cache.clone().lock_owned().await;
        Self::refresh_market_cache(lock, &self.client).await?;
        Ok(())
    }

    async fn get_market_definition(
        &self,
        id: &MarketIdentifier,
        max_age: Option<Duration>,
    ) -> Result<MarketDefinition> {
        let lock = self.markets_cache.clone().lock_owned().await;
        if let Some(market) = lock
            .as_ref()
            .and_then(|markets| markets.get(id).map(|e| e.clone()))
        {
            if max_age
                .map(|e| market.age.elapsed().unwrap() < e)
                .unwrap_or(true)
            {
                return Ok(market.clone());
            }
        }
        Self::refresh_market_cache(lock, &self.client).await?;
        let lock = self.markets_cache.clone().lock_owned().await;
        if let Some(markets) = lock.as_ref() {
            return markets.get(id).cloned().ok_or(Error::NoData);
        } else {
            return Err(Error::NoData);
        }
    }

    async fn get_markets(&self) -> Result<Vec<MarketIdentifier>> {
        let lock = self.markets_cache.clone().lock_owned().await;
        if let Some(markets) = lock.as_ref().map(|e| e.keys().map(|e| e.clone()).collect()) {
            return Ok(markets);
        }
        Self::refresh_market_cache(lock, &self.client).await?;
        let lock = self.markets_cache.clone().lock_owned().await;
        if let Some(markets) = lock.as_ref().map(|e| e.keys().map(|e| e.clone()).collect()) {
            return Ok(markets);
        } else {
            return Err(Error::NoData);
        }
    }
}

impl Into<Interval> for crate::prelude::Interval {
    fn into(self) -> Interval {
        match self {
            crate::prelude::Interval::Min1 => Interval::Min1,
            crate::prelude::Interval::Min5 => Interval::Min5,
            crate::prelude::Interval::Min15 => Interval::Min15,
            crate::prelude::Interval::Min30 => Interval::Min30,
            crate::prelude::Interval::Hour1 => Interval::Hour1,
            crate::prelude::Interval::Hour4 => Interval::Hour4,
            crate::prelude::Interval::Day1 => Interval::Day1,
            crate::prelude::Interval::Day7 => Interval::Day7,
            crate::prelude::Interval::Day15 => Interval::Day15,
        }
    }
}
