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

#[derive(Clone, Debug)]
pub struct StoreMarketHandle {
    tree: sled::Tree,
    settings_tree: sled::Tree,
    pub id: MarketIdentifier,
}

#[derive(Debug, Clone, bincode::Encode, bincode::Decode, serde::Serialize, serde::Deserialize)]
pub struct MarketSettings {
    pub ohlc_refresh_rate: Option<Duration>,
}

impl std::default::Default for MarketSettings {
    fn default() -> Self {
        Self {
            ohlc_refresh_rate: Some(Duration::from_secs(60)),
        }
    }
}

impl StoreMarketHandle {
    pub fn new(tree: sled::Tree, settings_tree: sled::Tree, id: MarketIdentifier) -> Self {
        Self {
            tree,
            settings_tree,
            id,
        }
    }

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

    pub fn close_range(&self, start: i64, end: i64) -> Result<Vec<OHLC>> {
        let start = self.close_to(start)?.ok_or_else(|| Error::NoData)?;
        let end = self.close_to(end)?.ok_or_else(|| Error::NoData)?;
        Ok(self.exact_range(start, end)?)
    }

    pub fn exact_range(&self, start: i64, end: i64) -> Result<Vec<OHLC>> {
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
            0
        };
        let exchange_lock = exchange.lock().await;
        let chunk = exchange_lock.get_ohlc(&self.id, resume_from).await?;
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
