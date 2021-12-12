use crate::{exchange::MarketIdentifier, prelude::*, reactor::SyncExchange};
use sled::{Db, IVec};

mod market;
pub use market::*;

pub struct Store {
    db: Db,
}

#[derive(Clone)]
pub struct StoreHandle {
    db: Db,
    settings_tree: sled::Tree,
    pub trees: Arc<std::sync::Mutex<HashMap<String, StoreMarketHandle>>>,
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

impl Store {
    pub fn new(path: PathBuf) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    pub fn handle(&self) -> StoreHandle {
        StoreHandle {
            settings_tree: self
                .db
                .open_tree("settings")
                .expect("Failed to create settings store"),
            db: self.db.clone(),
            trees: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }
}

impl StoreHandle {
    pub fn market(&self, market: MarketIdentifier) -> Result<StoreMarketHandle> {
        let uid = market.tree_uid();
        if let Some(exchange) = self.trees.lock().unwrap().get(&uid) {
            return Ok(exchange.clone());
        }
        self.trees.lock().unwrap().insert(
            uid.clone(),
            StoreMarketHandle::new(self.db.clone(), self.settings_tree.clone(), market),
        );
        Ok(self.trees.lock().unwrap()[&uid].clone())
    }
}
