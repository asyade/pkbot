use crate::exchange::*;
use crate::prelude::*;
use std::time::SystemTime;

pub mod utils;

mod sync;
pub use sync::*;

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
    pub markets: Arc<Mutex<HashMap<MarketIdentifier, SyncMarket>>>,
}

impl Reactor {
    pub fn new(store: StoreHandle) -> Self {
        Self {
            store,
            exchanges: Arc::new(Mutex::new(HashMap::new())),
            markets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn register_exchange(&self, exchange: SyncExchange) {
        let name = exchange.lock().await.name();
        self.exchanges.lock().await.insert(name, exchange);
    }

    pub async fn get_or_register_market(&self, id: MarketIdentifier) -> Result<SyncMarket> {
        let (market, fresh) = {
            let mut lock = self.markets.lock().await; 
            if let Some(market) = lock.get(&id) {
                (market.clone(), false)
            } else {
                let market = SyncMarket::new(&self, id.clone()).await?;
                lock.insert(id, market.clone());
                (market, true)
            }
        };
        if fresh {
            let _ = market.sync().await.map_err(|e| error!("Failed to sync market: {:?}", e));
        }
        Ok(market)
    }
}
