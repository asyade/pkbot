use crate::store::StoreMarketDataHandle;

use super::*;

#[derive(Clone)]
pub struct SyncMarket {
    exchange: SyncExchange,
    store: StoreMarketHandle,
}

impl SyncMarket {
    pub async fn new(reactor: &Reactor, identifier: MarketIdentifier) -> Result<Self> {
        log::trace!(
            "Register sync market: EXCHANGE={}, BASE={}, QUOTE={}",
            &identifier.exchange_name,
            &identifier.base,
            &identifier.quote
        );
        let exchange = reactor
            .exchanges
            .read()
            .await
            .get(&identifier.exchange_name)
            .ok_or_else(|| Error::ExchangeNotFound(identifier.exchange_name.clone()))?
            .clone();
        let store = reactor.store.market(identifier)?;
        Ok(SyncMarket { store, exchange })
    }

    pub async fn interval(&self, interval: Interval) -> Result<StoreMarketDataHandle> {
        self.store.interval(interval).await
    }

    pub async fn sync_periode(
        &self,
        from: Timestamp,
        to: Timestamp,
        interval: Interval,
    ) -> Result<Range<Timestamp>> {
        log::trace!("Begin period synchronization: EXCHANGE={}, BASE={}, QUOTE={}, FROM={}, TO={}, INTERVAL={}",
            &self.store.id.exchange_name,
            &self.store.id.base,
            &self.store.id.quote,
            from,
            to,
            interval
        );
        if self.check_periode_availability(from, to, interval).await? {
            return Ok(from..to)
        }

        let exchange_lock = self.exchange.lock().await;
        let chunk = exchange_lock
            .get_ohlc(&self.store.id, from, interval)
            .await?;
        let exchange_name = exchange_lock.name();
        log::trace!("Appending {} OHLC metric into store: EXCHANGE={}, REQUEST_FROM={}, CHUNK_FROM={}, CHUNK_TO={}",
            chunk.data.len(),
            &exchange_name,
            from,
            chunk.begin,
            chunk.end,
        );
        drop(exchange_lock);
        let tree = self.store.interval(interval).await?;
        tree.extend(chunk.data)?;
        Ok(chunk.begin..chunk.end)
    }

    async fn check_periode_availability(&self, from: Timestamp, to: Timestamp, interval: Interval) -> Result<bool> {
        let store = self.store.interval(interval).await?;
        if from != 0 {
            let _close_from = match store.prev_close_to(from)? {
                None => return Ok(false),
                Some(close_from) if close_from - from > interval.as_secs() => return Ok(false),
                Some(close_from) => close_from,
            };
        } else if store.first_ohlc()?.map(|e| e.first_available).unwrap_or(true) {
            return Ok(false);
        }
        let _close_to = match store.next_close_to(to)? {
            None => return Ok(false),
            Some(close_from) => close_from,
        };
        Ok(true)
    }
}

pub type SyncExchange = Arc<Mutex<Box<dyn Exchange + Sync + Send>>>;
