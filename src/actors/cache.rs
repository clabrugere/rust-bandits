use crate::config::PolicyCacheConfig;

use actix::prelude::*;
use log::{info, warn};
use std::{
    collections::HashMap,
    fs::{write, File},
    io::BufReader,
    time::Duration,
};
use uuid::Uuid;

#[derive(Debug)]
pub struct PolicyCache {
    storage: HashMap<Uuid, String>,
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
            if let Ok(serialized) = serde_json::to_string_pretty(&self.storage) {
                match write(&self.config.path, serialized) {
                    Ok(_) => info!(
                        "Persisted cache to '{}'",
                        self.config.path.to_str().unwrap_or_default()
                    ),
                    Err(err) => warn!("Error while persisting cache to disk: {}", err),
                }
            }
        }
    }
}

impl Actor for PolicyCache {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        info!("Started policy cache");
        let persist_every = Duration::from_secs(self.config.persist_every);
        ctx.run_interval(persist_every, |cache, _| {
            cache.persist();
        });
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct InsertPolicyCache {
    pub bandit_id: Uuid,
    pub serialized: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct RemovePolicyCache {
    pub bandit_id: Uuid,
}

#[derive(Message)]
#[rtype(result = "Option<String>")]
pub struct ReadPolicyCache {
    pub bandit_id: Uuid,
}

#[derive(Message)]
#[rtype(result = "HashMap<Uuid, String>")]
pub struct ReadAllPolicyCache;

impl Handler<InsertPolicyCache> for PolicyCache {
    type Result = ();

    fn handle(&mut self, msg: InsertPolicyCache, _: &mut Context<Self>) -> Self::Result {
        self.storage.insert(msg.bandit_id, msg.serialized);
    }
}

impl Handler<RemovePolicyCache> for PolicyCache {
    type Result = ();

    fn handle(&mut self, msg: RemovePolicyCache, _: &mut Context<Self>) -> Self::Result {
        self.storage.remove(&msg.bandit_id);
    }
}

impl Handler<ReadPolicyCache> for PolicyCache {
    type Result = Option<String>;

    fn handle(&mut self, msg: ReadPolicyCache, _: &mut Context<Self>) -> Self::Result {
        self.storage.get(&msg.bandit_id).cloned()
    }
}

impl Handler<ReadAllPolicyCache> for PolicyCache {
    type Result = MessageResult<ReadAllPolicyCache>;

    fn handle(&mut self, _: ReadAllPolicyCache, _: &mut Self::Context) -> Self::Result {
        MessageResult(self.storage.clone())
    }
}
