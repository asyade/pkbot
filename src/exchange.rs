use crate::prelude::*;
mod kraken;
pub use kraken::*;

pub struct OHLCChunk {
    pub data: Vec<OHLC>,
    pub begin: i64,
    pub end: i64,
    pub duration: i64,
    pub interval: i64,
}

impl OHLCChunk {
    pub fn new(data: Vec<OHLC>) -> Self {
        let begin = data.first().unwrap().time;
        let end = data.last().unwrap().time;
        let diff = end - begin;
        let interval = data.iter().skip(1).next().map(|e| e.time).unwrap_or(begin) - begin;
        let duration = (diff + interval) as i64;
        Self {
            begin,
            end,
            duration,
            interval,
            data,
        }
    }
}

#[derive(Clone, Debug, bincode::Encode, bincode::Decode, serde::Serialize, serde::Deserialize)]
pub struct OHLC {
    pub time: i64,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub open_normalized: f64,
    pub high_normalized: f64,
    pub low_normalized: f64,
    pub close_normalized: f64,
    pub vwap: String,
    pub volume: String,
    pub count: u64,
}

impl OHLC {
    pub fn new(
        time: i64,
        open: String,
        high: String,
        low: String,
        close: String,
        vwap: String,
        volume: String,
        count: u64,
    ) -> Self {
        Self {
            time,
            open_normalized: open.parse().unwrap(),
            high_normalized: high.parse().unwrap(),
            low_normalized: low.parse().unwrap(),
            close_normalized: close.parse().unwrap(),
            open,
            high,
            low,
            close,
            vwap,
            volume,
            count,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Hash, Serialize)]
pub struct MarketIdentifier {
    pub base: String,
    pub quote: String,
    pub exchange_name: String,
}

impl<T: AsRef<str>> From<T> for MarketIdentifier {
    fn from(f: T) -> Self {
        let mut splited = f.as_ref().split("/");
        MarketIdentifier {
            exchange_name: splited.next().unwrap_or("").to_string(),
            base: splited.next().unwrap_or("").to_uppercase(),
            quote: splited.next().unwrap_or("").to_uppercase(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketDefinition {
    pub age: SystemTime,
    pub pairname: String,
    pub pair_decimals: i32,
    pub lot_decimals: i32,
    pub lot_multiplier: i32,
    pub leverage_buy: Vec<f64>,
    pub leverage_sell: Vec<f64>,
    pub fees: Vec<(f64, f64)>,
    pub fees_maker: Option<Vec<(f64, f64)>>,
    pub margin_call: f64,
    pub margin_stop: f64,
    pub ordermin: Option<String>,
}

impl std::fmt::Display for MarketIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}/{}", &self.exchange_name, &self.base, &self.quote)
    }
}

impl MarketIdentifier {
    pub fn uid(&self) -> String {
        format!("{}_{}", self.exchange_name, self.pair_name())
    }

    pub fn pair_name(&self) -> String {
        format!("{}/{}", self.base, self.quote)
    }
}

#[async_trait]
pub trait Exchange {
    fn name(&self) -> String;

    async fn get_severt_time(&self) -> Result<NaiveDateTime>;

    async fn get_ohlc(&self, id: &MarketIdentifier, since: u64) -> Result<OHLCChunk>;

    async fn refresh_market_cache(&self) -> Result<()>;

    async fn get_markets(&self) -> Result<Vec<MarketIdentifier>>;

    async fn get_market_definition(
        &self,
        id: &MarketIdentifier,
        max_age: Option<Duration>,
    ) -> Result<MarketDefinition>;
}
