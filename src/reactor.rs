use crate::exchange::*;
use crate::prelude::*;
use std::time::SystemTime;

pub mod utils;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct HistoryIdentifier {
    pub market_name: String,
    pub exchange_name: String,
}

pub type SyncMarketHistory = Arc<Mutex<MarketHistory>>;

pub struct MarketHistory {
    pub ohlc: VecDeque<OHLC>,
    previous_update: Option<(SystemTime, std::ops::Range<i64>)>,
    refresh: bool,
}

impl MarketHistory {
    pub fn new() -> Self {
        Self {
            previous_update: None,
            ohlc: VecDeque::new(),
            refresh: true,
        }
    }

    pub fn append(&mut self, chunk: Vec<OHLC>) {
        if chunk.len() == 0 {
            return;
        }
        self.previous_update = Some((
            SystemTime::now(),
            chunk.first().unwrap().time..chunk.last().unwrap().time,
        ));
        // dbg!(&self.previous_update);
        let chunk_first_time = chunk.first().unwrap().time;
        let mut useless = 0;
        loop {
            let previous_last_time = if self.ohlc.len() > 0 {
                self.ohlc.get(self.ohlc.len() - 1).unwrap().time
            } else {
                0
            };
            if previous_last_time > chunk_first_time {
                self.ohlc.pop_back();
                useless += 1;
            } else {
                break;
            }
        }
        // dbg!(useless);
        for item in chunk.into_iter() {
            self.ohlc.push_back(item);
        }
    }
}

pub struct Reactor {
    pub store: StoreHandle,
    pub exchanges: Arc<Mutex<HashMap<String, SyncExchange>>>,
}

pub type SyncExchange = Arc<Mutex<Box<dyn Exchange + Sync + Send>>>;

impl Reactor {
    pub fn new(store: StoreHandle) -> Self {
        Self {
            store,
            exchanges: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn register_exchange(&mut self, exchange: SyncExchange) {
        let name = exchange.lock().await.name();
        self.exchanges.lock().await.insert(name, exchange);
    }

    pub async fn set_refresh_ohlc(
        &mut self,
        market: MarketIdentifier,
        interval: Duration,
        exchange: SyncExchange,
    ) -> Result<tokio::task::JoinHandle<()>> {
        let exchange_name = exchange.lock().await.name();
        log::info!(
            "Registering market refresh: EXCHANGE={}, INTERVAL={:#?}",
            &exchange_name,
            interval
        );
        let store = self.store.market(market)?;
        Ok(tokio::spawn(Self::refresh_routine(
            interval, exchange, store,
        )))
    }

    async fn refresh_routine(interval: Duration, exchange: SyncExchange, store: StoreMarketHandle) {
        loop {
            if let Err(e) = store.refresh(exchange.clone()).await {
                log::error!("Failed to refresh market: {}", e);
            }
            tokio::time::sleep(interval).await;
        }
    }
}
