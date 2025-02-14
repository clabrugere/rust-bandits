use crate::{config::PolicyCacheConfig, policies::Policy};

use actix::prelude::*;
use log::{info, warn};
use std::{
    collections::HashMap,
    fs::{write, File},
    io::BufReader,
    time::Duration,
};
use uuid::Uuid;

pub struct PolicyCache {
    storage: HashMap<Uuid, Box<dyn Policy + Send>>,
    config: PolicyCacheConfig,
}

impl PolicyCache {
    pub fn new(config: PolicyCacheConfig) -> Self {
        let storage = File::open(&config.path)
            .map(|file| {
                let reader = BufReader::new(file);
                serde_json::from_reader(reader).unwrap_or_default()
            })
            .unwrap_or_default();

        Self { storage, config }
    }

    fn persist(&self) {
        if !&self.storage.is_empty() {
            info!(
                "Persisting cache to '{}'",
                self.config.path.to_str().unwrap_or_default()
            );
            match serde_json::to_string(&self.storage) {
                Ok(serialized) => {
                    if let Err(err) = write(&self.config.path, serialized) {
                        warn!("Error while persisting cache to disk: {}", err);
                    }
                }
                Err(err) => warn!("Error while serializing cache: {}", err),
            }
        }
    }
}

impl Actor for PolicyCache {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Starting policy cache");
        let persist_every = Duration::from_secs(self.config.persist_every);
        ctx.run_interval(persist_every, |cache, _| {
            cache.persist();
        });
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct InsertPolicyCache {
    pub experiment_id: Uuid,
    pub policy: Box<dyn Policy + Send>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct RemovePolicyCache {
    pub experiment_id: Uuid,
}

#[derive(Message)]
#[rtype(result = "Option<Box<dyn Policy + Send>>")]
pub struct ReadPolicyCache {
    pub experiment_id: Uuid,
}

#[derive(Message)]
#[rtype(result = "HashMap<Uuid, Box<dyn Policy + Send>>")]
pub struct ReadFullPolicyCache;

impl Handler<InsertPolicyCache> for PolicyCache {
    type Result = ();

    fn handle(&mut self, msg: InsertPolicyCache, _: &mut Self::Context) -> Self::Result {
        self.storage.insert(msg.experiment_id, msg.policy);
    }
}

impl Handler<RemovePolicyCache> for PolicyCache {
    type Result = ();

    fn handle(&mut self, msg: RemovePolicyCache, _: &mut Self::Context) -> Self::Result {
        self.storage.remove(&msg.experiment_id);
    }
}

impl Handler<ReadPolicyCache> for PolicyCache {
    type Result = Option<Box<dyn Policy + Send>>;

    fn handle(&mut self, msg: ReadPolicyCache, _: &mut Self::Context) -> Self::Result {
        self.storage.get(&msg.experiment_id).cloned()
    }
}

impl Handler<ReadFullPolicyCache> for PolicyCache {
    type Result = MessageResult<ReadFullPolicyCache>;

    fn handle(&mut self, _: ReadFullPolicyCache, _: &mut Self::Context) -> Self::Result {
        MessageResult(self.storage.clone())
    }
}
