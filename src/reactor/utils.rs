#![allow(dead_code)]
use crate::prelude::*;

#[derive(Debug, Clone)]
pub enum PeakPosition {
    Top,
    Bottom,
}

#[derive(Debug, Clone)]
pub struct PeakFinder {
    pub dataset: StoreMarketHandle,
    pub window: (i64, i64),
    pub raw_peak: HashMap<i64, PeakPosition>,
}

impl PeakFinder {
    pub fn new(dataset: StoreMarketHandle, window: (i64, i64)) -> Result<Self> {
        let res = Self {
            dataset,
            window,
            raw_peak: HashMap::new(),
        };
        Ok(res.find_raw_peak()?)
    }

    fn find_raw_peak(mut self) -> Result<Self> {
        self.raw_peak.clear();
        let mut current = self.dataset.ohlc(self.window.0)?;
        while let Some(current) = current.take() {
            match (
                self.dataset.prev_ohlc(current.time)?,
                self.dataset.next_ohlc(current.time)?,
            ) {
                (Some(prev), Some(next)) => {
                    if prev.high_normalized < current.high_normalized
                        && next.high_normalized < current.high_normalized
                    {
                        self.raw_peak.insert(current.time, PeakPosition::Top);
                    } else if prev.low_normalized > current.low_normalized
                        && next.low_normalized > current.low_normalized
                    {
                        self.raw_peak.insert(current.time, PeakPosition::Bottom);
                    }
                }
                _ => {}
            }
            if current.time >= self.window.1 {
                break;
            }
        }
        Ok(self)
    }
}
