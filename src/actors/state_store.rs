use crate::config::StateStoreConfig;
use crate::errors::PersistenceError;
use crate::policies::Policy;

use actix::prelude::*;
use std::{collections::HashMap, fs::File, io::BufReader, time::Duration};
use tracing::{info, warn};
use uuid::Uuid;

pub struct StateStore {
    storage: HashMap<Uuid, Box<dyn Policy + Send>>,
    config: StateStoreConfig,
}

impl StateStore {
    pub fn new(config: StateStoreConfig) -> Self {
        let storage = File::open(&config.path)
            .map(|file| {
                let reader = BufReader::new(file);
                serde_json::from_reader(reader).unwrap_or_else(|err| {
                    warn!(error = %err, "Starting with empty store");
                    HashMap::new()
                })
            })
            .unwrap_or_else(|err| {
                warn!(error = %err, "Starting with empty store");
                HashMap::new()
            });

        Self { storage, config }
    }

    fn persist(&self) -> Result<(), PersistenceError> {
        if self.storage.is_empty() {
            return Ok(());
        }

        info!(
            path = ?self.config.path,
            "Persisting state store to"
        );

        let serialized = serde_json::to_string(&self.storage)?;
        std::fs::write(&self.config.path, serialized)?;
        Ok(())
    }
}

impl Actor for StateStore {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Starting StateStore");
        ctx.run_interval(
            Duration::from_secs(self.config.persist_every),
            |state_store, _| {
                if let Err(err) = state_store.persist() {
                    warn!(error = %err, "Failed to persist state");
                }
            },
        );
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        info!("Stopped StateStore");
    }
}

// Messages
#[derive(Message)]
#[rtype(result = "()")]
pub struct SaveState {
    pub experiment_id: Uuid,
    pub policy: Box<dyn Policy + Send>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct DeleteState {
    pub experiment_id: Uuid,
}

#[derive(Message)]
#[rtype(result = "Option<Box<dyn Policy + Send>>")]
pub struct LoadState {
    pub experiment_id: Uuid,
}

#[derive(Message)]
#[rtype(result = "HashMap<Uuid, Box<dyn Policy + Send>>")]
pub struct LoadAllStates;

// Handlers
impl Handler<SaveState> for StateStore {
    type Result = ();

    fn handle(&mut self, msg: SaveState, _: &mut Self::Context) -> Self::Result {
        info!(id = %msg.experiment_id, "Saving state for experiment");
        self.storage.insert(msg.experiment_id, msg.policy);
    }
}

impl Handler<DeleteState> for StateStore {
    type Result = ();

    fn handle(&mut self, msg: DeleteState, _: &mut Self::Context) -> Self::Result {
        info!(id = %msg.experiment_id, "Deleting state for experiment");
        self.storage.remove(&msg.experiment_id);
    }
}

impl Handler<LoadState> for StateStore {
    type Result = Option<Box<dyn Policy + Send>>;

    fn handle(&mut self, msg: LoadState, _: &mut Self::Context) -> Self::Result {
        self.storage.get(&msg.experiment_id).cloned()
    }
}

impl Handler<LoadAllStates> for StateStore {
    type Result = MessageResult<LoadAllStates>;

    fn handle(&mut self, _: LoadAllStates, _: &mut Self::Context) -> Self::Result {
        MessageResult(self.storage.clone())
    }
}
