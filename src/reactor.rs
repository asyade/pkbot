use crate::exchange::*;
use crate::interpretor::*;
use crate::prelude::*;

mod runtime;
mod sync;
pub mod utils;

pub use sync::*;

use self::runtime::ProgramRuntime;

pub type ListenerIdentifier = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReactorEvent {
    ProgramOutput {
        id: ProgramIdentifier,
        content: ProgramOutput,
    },
    ProgramStatus {
        id: ProgramIdentifier,
        status: ProgramStatus,
    },
    RuntimeCreated {
        id: ProgramIdentifier,
    },
    RuntimeDestroyed {
        id: ProgramIdentifier,
    },
}

pub type SyncMap<K, V> = Arc<RwLock<HashMap<K, V>>>;

pub struct ReactorEventListener {
    sender: Sender<ReactorEvent>,
}

pub struct ReactorEventListenerHandle {
    id: ListenerIdentifier,
    receiver: Receiver<ReactorEvent>,
    pool: SyncMap<ListenerIdentifier, ReactorEventListener>,
}

#[derive(Clone)]
pub struct Reactor {
    pub store: StoreHandle,
    pub exchanges: SyncMap<String, SyncExchange>,
    pub markets: SyncMap<MarketIdentifier, SyncMarket>,
    pub programs: SyncMap<ProgramIdentifier, ProgramRuntime>,
    pub listeners: SyncMap<ListenerIdentifier, ReactorEventListener>,
    listener_counter: Arc<AtomicU64>,
    process_counter: Arc<AtomicU64>,
}

impl Reactor {
    pub async fn new(store: StoreHandle) -> Self {
        let reactor = Self {
            store,
            exchanges: Arc::new(RwLock::new(HashMap::new())),
            markets: Arc::new(RwLock::new(HashMap::new())),
            listeners: Arc::new(RwLock::new(HashMap::new())),
            programs: Arc::new(RwLock::new(HashMap::new())),
            process_counter: Arc::new(AtomicU64::new(0)),
            listener_counter: Arc::new(AtomicU64::new(0)),
        };
        reactor
    }

    pub async fn event_listener(&self) -> ReactorEventListenerHandle {
        let id = self.listener_counter.fetch_add(1, Ordering::SeqCst);
        let (sender, receiver) = channel(1024);
        self.listeners
            .write()
            .await
            .insert(id, ReactorEventListener { sender });
        ReactorEventListenerHandle {
            receiver,
            pool: self.listeners.clone(),
            id,
        }
    }

    async fn runtime_handler(reactor: Reactor, mut runtime: ProgramRuntime) {
        let mut receiver = runtime.stdout.take().unwrap();
        let runtime_id = runtime.id;
        reactor.programs.write().await.insert(runtime_id, runtime);
        reactor
            .listeners
            .broadcast(ReactorEvent::RuntimeCreated { id: runtime_id })
            .await;
        log::trace!("Handling runtime: ID={}", runtime_id);
        while let Some(message) = receiver.recv().await {
            reactor
                .listeners
                .broadcast(ReactorEvent::ProgramOutput {
                    id: runtime_id,
                    content: message,
                })
                .await;
        }
        log::trace!("Removing runtime: ID={}", runtime_id);
        reactor.programs.write().await.remove(&runtime_id);
        reactor
            .listeners
            .broadcast(ReactorEvent::RuntimeDestroyed { id: runtime_id })
            .await;
    }

    pub async fn spawn_program(&self, program: Program) {
        let runtime = ProgramRuntime::spawn(program.root, self.clone()).await;
        tokio::spawn(Self::runtime_handler(self.clone(), runtime));
    }

    pub async fn register_exchange(&self, exchange: SyncExchange) {
        let name = exchange.lock().await.name();
        self.exchanges.write().await.insert(name, exchange);
    }

    pub async fn get_or_register_market(&self, id: &MarketIdentifier) -> Result<SyncMarket> {
        let (market, fresh) = {
            let mut lock = self.markets.write().await;
            if let Some(market) = lock.get(&id) {
                (market.clone(), false)
            } else {
                let market = SyncMarket::new(&self, id.clone()).await?;
                lock.insert(id.clone(), market.clone());
                (market, true)
            }
        };
        if fresh {
            let _ = market
                .sync()
                .await
                .map_err(|e| error!("Failed to sync market: {:?}", e));
        }
        Ok(market)
    }
}

impl ReactorEventListenerHandle {
    async fn cleanup(
        pool: SyncMap<ListenerIdentifier, ReactorEventListener>,
        id: ListenerIdentifier,
    ) {
        pool.write().await.remove(&id);
    }

    pub async fn recv(&mut self) -> Option<ReactorEvent> {
        self.receiver.recv().await
    }
}

impl Drop for ReactorEventListenerHandle {
    fn drop(&mut self) {
        tokio::spawn(Self::cleanup(self.pool.clone(), self.id));
    }
}

#[async_trait]
pub trait ListenerPool {
    async fn broadcast(&self, event: ReactorEvent);
}

#[async_trait]
impl ListenerPool for SyncMap<ListenerIdentifier, ReactorEventListener> {
    async fn broadcast(&self, event: ReactorEvent) {
        for (id, ReactorEventListener { sender }) in self.read().await.iter() {
            let _ = sender.send(event.clone()).await.map_err(|e| {
                log::error!("Failed to broadcast listener: ID={} ERROR={}", id, e);
            });
        }
    }
}
