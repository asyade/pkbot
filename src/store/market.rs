use super::*;

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

#[repr(i32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Encode, Decode, Serialize, Deserialize)]
pub enum Interval {
    Min1 = 1,
    Min5 = 5,
    Min15 = 15,
    Min30 = 30,
    Hour1 = 60,
    Hour4 = 240,
    Day1 = 1_440,
    Day7 = 10_080,
    Day15 = 21_600,
}

impl Interval {
    pub fn as_secs(&self) -> i64 {
        (match self {
            Interval::Min1 => 1,
            Interval::Min5 => 5,
            Interval::Min15 => 15,
            Interval::Min30 => 30,
            Interval::Hour1 => 60,
            Interval::Hour4 => 240,
            Interval::Day1 => 1_440,
            Interval::Day7 => 10_080,
            Interval::Day15 => 21_600,
        }) * 60
    }

    #[allow(unused)]
    pub fn from_minuts(mins: i64) -> Result<Self> {
        match mins {
            1 => Ok(Interval::Min1),
            5 => Ok(Interval::Min5),
            15 => Ok(Interval::Min15),
            30 => Ok(Interval::Min30),
            60 => Ok(Interval::Hour1),
            240 => Ok(Interval::Hour4),
            1_440 => Ok(Interval::Day1),
            10_080 => Ok(Interval::Day7),
            21_600 => Ok(Interval::Day15),
            _ => Err(Error::InvalidInterval(mins)),
        }
    }
}

impl std::fmt::Display for Interval {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            fmt,
            "{}",
            match self {
                Interval::Min1 => "Min1",
                Interval::Min5 => "Min5",
                Interval::Min15 => "Min15",
                Interval::Min30 => "Min30",
                Interval::Hour1 => "Hour1",
                Interval::Hour4 => "Hour4",
                Interval::Day1 => "Day1",
                Interval::Day7 => "Day7",
                Interval::Day15 => "Day15",
            }
        )
    }
}

#[derive(Clone, Debug)]
pub struct StoreMarketHandle {
    db: Db,
    trees_cache: Arc<RwLock<HashMap<Interval, StoreMarketDataHandle>>>,
    #[allow(unused)]
    settings_tree: sled::Tree,
    pub id: MarketIdentifier,
}

#[derive(Debug, Clone, bincode::Encode, bincode::Decode, serde::Serialize, serde::Deserialize)]
pub struct MarketSettings {
    pub ohlc_refresh_rate: Option<Interval>,
}

impl std::default::Default for MarketSettings {
    fn default() -> Self {
        Self {
            ohlc_refresh_rate: None,
        }
    }
}

impl StoreMarketHandle {
    pub fn new(db: sled::Db, settings_tree: sled::Tree, id: MarketIdentifier) -> Self {
        Self {
            db,
            trees_cache: Arc::new(RwLock::new(HashMap::new())),
            settings_tree,
            id,
        }
    }

    #[allow(unused)]
    pub fn settings(&self) -> Result<MarketSettings> {
        if let Some(raw) = self
            .settings_tree
            .get(format!("{}", &self.id).as_bytes())
            .map_err(|e| {
                warn!(
                    "Failed to parse market settings, fallback to default: {:?}",
                    e
                )
            })
            .ok()
            .flatten()
        {
            Ok(bincode::decode_from_slice(
                raw.as_ref(),
                Configuration::standard(),
            )?)
        } else {
            let settings = MarketSettings::default();
            self.set_settings(&settings)?;
            Ok(settings)
        }
    }

    pub fn set_settings(&self, settings: &MarketSettings) -> Result<()> {
        let encoded = bincode::encode_to_vec(settings, Configuration::standard())?;
        self.settings_tree
            .insert(format!("{}", &self.id).as_bytes(), encoded)?;
        Ok(())
    }

    pub async fn interval(&self, interval: Interval) -> Result<StoreMarketDataHandle> {
        if let Some(handle) = self.trees_cache.read().await.get(&interval).cloned() {
            return Ok(handle)
        }
        log::trace!(
            "Register sync market data: EXCHANGE={}, BASE={}, QUOTE={}, INTERVAL={}",
            &self.id.exchange_name,
            &self.id.base,
            &self.id.quote,
            interval
        );
        let tree = self.db.open_tree(self.id.data_tree_uid(interval))?;
        let handle = StoreMarketDataHandle::new(tree, self.id.clone(), interval);
        self.trees_cache
            .write()
            .await
            .insert(interval, handle.clone());
        Ok(handle)
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct StoreMarketDataHandle {
    id: MarketIdentifier,
    tree: sled::Tree,
    interval: Interval,
}

impl StoreMarketDataHandle {
    pub fn new(tree: sled::Tree, id: MarketIdentifier, interval: Interval) -> Self {
        Self { id, tree, interval }
    }

    pub fn prev_close_to(&self, target_time: Timestamp) -> Result<Option<i64>> {
        let mut iter = self.tree.iter();
        while let Some(Ok((key, _value))) = iter.next() {
            let key_time = key.from_store();
            if key_time >= target_time {
                return Ok(Some(key_time));
            }
        }
        Ok(None)
    }

    pub fn next_close_to(&self, target_time: Timestamp) -> Result<Option<i64>> {
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
        Ok(closest.filter(|e| *e <= target_time))
    }

    pub fn close_range(&self, start: Timestamp, end: Timestamp) -> Result<Vec<OHLC>> {
        let start = self.prev_close_to(start)?.ok_or_else(|| Error::NoData)?;
        let end = self.next_close_to(end)?.ok_or_else(|| Error::NoData)?;
        Ok(self.exact_range(start, end)?)
    }

    pub fn exact_range(&self, start: Timestamp, end: Timestamp) -> Result<Vec<OHLC>> {
        let mut ret = Vec::new();
        let mut offset = start;
        loop {
            let ohlc = self.ohlc(offset)?.ok_or_else(|| Error::NoData)?;
            offset = self.next_ohlc(offset)?.ok_or_else(|| Error::NoData)?.time;
            ret.push(ohlc);
            if offset == end {
                break;
            }
        }
        Ok(ret)
    }

    pub fn extend<T: IntoIterator<Item = OHLC>>(&self, chunk: T) -> Result<()> {
        for item in chunk {
            self.insert(item)?;
        }
        Ok(())
    }

    pub fn insert(&self, ohlc: OHLC) -> Result<()> {
        let encoded =
            bincode::encode_to_vec(&ohlc, Configuration::standard()).unwrap_or_else(|_| Vec::new());
        self.tree.insert(&ohlc.time.to_be_bytes(), encoded)?;
        Ok(())
    }

    pub fn ohlc(&self, time: Timestamp) -> Result<Option<OHLC>> {
        try_result_opt!(self.tree.get(&time.to_be_bytes()))
    }

    #[allow(unused)]
    pub fn prev_ohlc(&self, time: Timestamp) -> Result<Option<OHLC>> {
        try_result_opt!(self
            .tree
            .get_lt(&time.to_be_bytes())
            .map(|e| e.map(|(_, e)| e)))
    }

    pub fn next_ohlc(&self, time: Timestamp) -> Result<Option<OHLC>> {
        try_result_opt!(self
            .tree
            .get_gt(&time.to_be_bytes())
            .map(|e| e.map(|(_, e)| e)))
    }

    pub fn first_ohlc(&self) -> Result<Option<OHLC>> {
        try_result_opt!(self.tree.first().map(|e| e.map(|(_, e)| e)))
    }

    #[allow(unused)]
    pub fn last_ohlc(&self) -> Result<Option<OHLC>> {
        try_result_opt!(self.tree.last().map(|e| e.map(|(_, e)| e)))
    }
}
