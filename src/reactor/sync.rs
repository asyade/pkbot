use super::*;
use crate::prelude::*;

#[derive(Clone)]
pub struct CancelTask {
    is_running: Arc<Mutex<bool>>,
}

impl CancelTask {
    pub fn new() -> Self {
        Self { is_running: Arc::new(Mutex::new(true)) }
    }

    pub async fn is_running(&self) -> bool {
        *self.is_running.lock().await
    }

    pub async fn cancel(self) {
        *self.is_running.lock().await = false;
    }
}

pub struct RefreshOhlcTask {
    cancel: CancelTask,
    interval: Duration,
}

impl RefreshOhlcTask {

    pub async fn spawn(interval: Duration, exchange: SyncExchange, store: StoreMarketHandle) -> Result<Self> {
        let cancel = CancelTask::new();
        tokio::spawn(Self::refresh_ohlc_routine(interval, exchange, store, cancel.clone()));
        Ok(Self {
            cancel,
            interval,
        })
    }

    pub async fn cancel(self) {
        self.cancel.cancel().await
    }
    
    async fn refresh_ohlc_routine(interval: Duration, exchange: SyncExchange, store: StoreMarketHandle, cancel: CancelTask) {
        log::info!("Begin ohlc refresh routine: MARKET={} INTERVAL={:#?}", &store.id, interval);
        while cancel.is_running().await {
            if let Err(e) = store.refresh(exchange.clone()).await {
                log::error!("Failed to refresh market: {}", e);
            }
            tokio::time::sleep(interval).await;
        }
    }
}

#[derive(Clone)]
pub struct SyncMarket {
    pub exchange: SyncExchange,
    pub store: StoreMarketHandle,
    refresh_ohlc: Arc<Mutex<Option<RefreshOhlcTask>>>,
}

impl SyncMarket{

    pub async fn new(reactor: &Reactor, identifier: MarketIdentifier) -> Result<Self> {
        let exchange = reactor.exchanges.lock().await.get(&identifier.exchange_name)
            .ok_or_else(|| Error::ExchangeNotFound(identifier.exchange_name.clone()))?.clone();
        let store = reactor.store.market(identifier)?;
        Ok(SyncMarket {
            store,
            exchange,
            refresh_ohlc: Arc::new(Mutex::new(None)),
        })
    }

    pub fn id(&self) -> &MarketIdentifier {
        &self.store.id
    }

    pub async fn sync(&self) -> Result<()> {
        let settings = self.store.settings()?;
        log::info!("Begin sync of {}", &self.store.id);
        let existing = self.refresh_ohlc.lock().await.take();
        match (existing, settings.ohlc_refresh_rate) {
            (Some(task), None) => { task.cancel().await; }, 
            (Some(current), Some(new)) if current.interval != new => {
                current.cancel().await;
                self.refresh_ohlc.lock().await.replace(RefreshOhlcTask::spawn(new, self.exchange.clone(), self.store.clone()).await?);
            }
            (None, Some(new)) => {
                self.refresh_ohlc.lock().await.replace(RefreshOhlcTask::spawn(new, self.exchange.clone(), self.store.clone()).await?);
            }
            (Some(task), Some(_)) => {
                self.refresh_ohlc.lock().await.replace(task);
            },
            (None, None) => {},
        }
        log::info!("Sync done {}", &self.store.id);
        Ok(())
    }

    
}

pub type SyncExchange = Arc<Mutex<Box<dyn Exchange + Sync + Send>>>;
