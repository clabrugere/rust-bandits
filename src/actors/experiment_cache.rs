use crate::errors::PersistenceError;
use crate::{config::ExperimentCacheConfig, policies::Policy};

use actix::prelude::*;
use log::{info, warn};
use std::{collections::HashMap, fs::File, io::BufReader, time::Duration};
use uuid::Uuid;

pub struct ExperimentCache {
    storage: HashMap<Uuid, Box<dyn Policy + Send>>,
    config: ExperimentCacheConfig,
}

impl ExperimentCache {
    pub fn new(config: ExperimentCacheConfig) -> Self {
        let storage = File::open(&config.path)
            .map(|file| {
                let reader = BufReader::new(file);
                serde_json::from_reader(reader).unwrap_or_default()
            })
            .unwrap_or_default();

        Self { storage, config }
    }

    fn persist(&self) -> Result<(), PersistenceError> {
        if self.storage.is_empty() {
            return Ok(());
        }

        info!(
            "Persisting cache to '{}'",
            self.config.path.to_str().unwrap_or_default()
        );

        let serialized = serde_json::to_string(&self.storage)?;
        std::fs::write(&self.config.path, serialized)?;
        Ok(())
    }
}

impl Actor for ExperimentCache {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Starting experiment cache actor");
        ctx.run_interval(
            Duration::from_secs(self.config.persist_every),
            |cache, _| {
                if let Err(err) = cache.persist() {
                    warn!("Failed to persist cache: {}", err);
                }
            },
        );
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct InsertExperimentCache {
    pub experiment_id: Uuid,
    pub policy: Box<dyn Policy + Send>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct RemoveExperimentCache {
    pub experiment_id: Uuid,
}

#[derive(Message)]
#[rtype(result = "Option<Box<dyn Policy + Send>>")]
pub struct ReadExperimentCache {
    pub experiment_id: Uuid,
}

#[derive(Message)]
#[rtype(result = "HashMap<Uuid, Box<dyn Policy + Send>>")]
pub struct ReadFullExperimentCache;

impl Handler<InsertExperimentCache> for ExperimentCache {
    type Result = ();

    fn handle(&mut self, msg: InsertExperimentCache, _: &mut Self::Context) -> Self::Result {
        self.storage.insert(msg.experiment_id, msg.policy);
    }
}

impl Handler<RemoveExperimentCache> for ExperimentCache {
    type Result = ();

    fn handle(&mut self, msg: RemoveExperimentCache, _: &mut Self::Context) -> Self::Result {
        self.storage.remove(&msg.experiment_id);
    }
}

impl Handler<ReadExperimentCache> for ExperimentCache {
    type Result = Option<Box<dyn Policy + Send>>;

    fn handle(&mut self, msg: ReadExperimentCache, _: &mut Self::Context) -> Self::Result {
        self.storage.get(&msg.experiment_id).cloned()
    }
}

impl Handler<ReadFullExperimentCache> for ExperimentCache {
    type Result = MessageResult<ReadFullExperimentCache>;

    fn handle(&mut self, _: ReadFullExperimentCache, _: &mut Self::Context) -> Self::Result {
        MessageResult(self.storage.clone())
    }
}
