use std::{sync::mpsc::sync_channel, time::SystemTime};

use crate::{exchange::MarketIdentifier, prelude::*, reactor::SyncExchange};
use sled::{transaction::TransactionError, Db, IVec, Tree};

const DEFAULT_REFRESH_SINCE: u64 = 60 * 60 * 24;

macro_rules! try_result_opt {
    ($ivec:expr) => {
        Ok(if let Some(raw) = $ivec? {
            let decoded = bincode::decode_from_slice(raw.as_ref(), Configuration::standard())?;
            Some(decoded)
        } else {
            None
        })
    };
}

pub struct Store {
    db: Db,
}

pub struct StoreHandle {
    db: Db,
    pub trees: Arc<std::sync::Mutex<HashMap<String, StoreMarketHandle>>>,
}

#[derive(Clone, Debug)]
pub struct StoreMarketHandle {
    tree: sled::Tree,
    pub market: MarketIdentifier,
}

impl Store {
    pub fn new(path: PathBuf) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    pub fn handle(&self) -> StoreHandle {
        StoreHandle {
            db: self.db.clone(),
            trees: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }
}

impl StoreHandle {
    pub fn market(&mut self, market: MarketIdentifier) -> Result<StoreMarketHandle> {
        let uid = market.uid();
        if let Some(exchange) = self.trees.lock().unwrap().get(&uid) {
            return Ok(exchange.clone());
        }
        let tree = self.db.open_tree(&uid)?;
        self.trees
            .lock()
            .unwrap()
            .insert(uid.clone(), StoreMarketHandle::new(tree, market));
        Ok(self.trees.lock().unwrap()[&uid].clone())
    }
}

pub trait StoredTimestamp {
    fn from_store(self) -> i64;
}

impl StoredTimestamp for IVec {
    fn from_store(self) -> i64 {
        let b: &[u8] = &self;
        i64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
    }
}

impl StoreMarketHandle {
    pub fn new(tree: sled::Tree, market: MarketIdentifier) -> Self {
        Self { tree, market }
    }

    pub fn close_to(&self, target_time: i64) -> Result<Option<i64>> {
        let mut iter = self.tree.iter();
        let mut closest = None;
        while let Some(Ok((key, _value))) = iter.next() {
            let key_time = key.from_store();
            match closest {
                Some(closest_in) if closest_in >= target_time => break,
                _ => {
                    closest = Some(key_time);
                }
            }
        }
        Ok(closest)
    }

    pub fn iter_range(&self, start: i64, end: i64) -> Result<()> {
        assert!(start <= end);
        Ok(())
    }

    pub fn ohlc(&self, time: i64) -> Result<Option<OHLC>> {
        try_result_opt!(self.tree.get(&time.to_be_bytes()))
    }

    pub fn prev_ohlc(&self, time: i64) -> Result<Option<OHLC>> {
        try_result_opt!(self
            .tree
            .get_lt(&time.to_be_bytes())
            .map(|e| e.map(|(_, e)| e)))
    }

    pub fn next_ohlc(&self, time: i64) -> Result<Option<OHLC>> {
        try_result_opt!(self
            .tree
            .get_gt(&time.to_be_bytes())
            .map(|e| e.map(|(_, e)| e)))
    }

    pub fn first_ohlc(&self) -> Result<Option<OHLC>> {
        try_result_opt!(self.tree.first().map(|e| e.map(|(_, e)| e)))
    }

    pub fn last_ohlc(&self) -> Result<Option<OHLC>> {
        try_result_opt!(self.tree.last().map(|e| e.map(|(_, e)| e)))
    }

    pub async fn refresh(&self, exchange: SyncExchange) -> Result<()> {
        let last = self.last_ohlc()?;

        let resume_from = if let Some(last) = last.as_ref() {
            last.time as u64
        } else {
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - DEFAULT_REFRESH_SINCE
        };
        let exchange_lock = exchange.lock().await;
        let chunk = exchange_lock.get_ohlc(&self.market, resume_from).await?;
        let exchange_name = exchange_lock.name();
        log::info!("Appending {} OHLC metric into store: EXCHANGE={}, REQUEST_FROM={}, CHUNK_FROM={}, CHUNK_TO={}, PREVIOUS_LAST={:?}",
        chunk.data.len(),
        &exchange_name,
        resume_from,
        chunk.begin,
        chunk.end,
        last.as_ref().map(|e| e.time),
    );
        for ohlc in chunk.data {
            let encoded = bincode::encode_to_vec(&ohlc, Configuration::standard())
                .unwrap_or_else(|_| Vec::new());
            self.tree.insert(&ohlc.time.to_be_bytes(), encoded)?;
        }

        Ok(())
    }
}
