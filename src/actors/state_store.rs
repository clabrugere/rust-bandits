use crate::config::StateStoreConfig;
use crate::policies::Policy;

use actix::prelude::*;
use std::{collections::HashMap, fs, io::BufReader, path::PathBuf};
use tracing::{info, warn};
use uuid::Uuid;

pub struct StateStore {
    config: StateStoreConfig,
}

impl StateStore {
    pub fn new(config: StateStoreConfig) -> Self {
        if let Err(err) = fs::create_dir_all(&config.dir) {
            warn!(error = %err, path = ?config.dir, "Could not create state store directory");
        }
        Self { config }
    }

    fn path_for(&self, experiment_id: Uuid) -> PathBuf {
        self.config.dir.join(format!("{experiment_id}.json"))
    }
}

impl Actor for StateStore {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        info!("Starting StateStore");
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
        let path = self.path_for(msg.experiment_id);
        match serde_json::to_string(&msg.policy) {
            Ok(serialized) => {
                if let Err(err) = fs::write(&path, serialized) {
                    warn!(error = %err, id = %msg.experiment_id, "Failed to write experiment state");
                }
            }
            Err(err) => {
                warn!(error = %err, id = %msg.experiment_id, "Failed to serialize experiment state");
            }
        }
    }
}

impl Handler<DeleteState> for StateStore {
    type Result = ();

    fn handle(&mut self, msg: DeleteState, _: &mut Self::Context) -> Self::Result {
        info!(id = %msg.experiment_id, "Deleting state for experiment");
        let path = self.path_for(msg.experiment_id);
        if let Err(err) = fs::remove_file(&path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                warn!(error = %err, id = %msg.experiment_id, "Failed to delete experiment state");
            }
        }
    }
}

impl Handler<LoadState> for StateStore {
    type Result = Option<Box<dyn Policy + Send>>;

    fn handle(&mut self, msg: LoadState, _: &mut Self::Context) -> Self::Result {
        let path = self.path_for(msg.experiment_id);
        let file = fs::File::open(&path).ok()?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader)
            .map_err(
                |err| warn!(error = %err, id = %msg.experiment_id, "Failed to deserialize state"),
            )
            .ok()
    }
}

impl Handler<LoadAllStates> for StateStore {
    type Result = MessageResult<LoadAllStates>;

    fn handle(&mut self, _: LoadAllStates, _: &mut Self::Context) -> Self::Result {
        let mut states: HashMap<Uuid, Box<dyn Policy + Send>> = HashMap::new();

        let entries = match fs::read_dir(&self.config.dir) {
            Ok(e) => e,
            Err(err) => {
                warn!(error = %err, path = ?self.config.dir, "Failed to read state store directory");
                return MessageResult(states);
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let Ok(experiment_id) = Uuid::try_parse(stem) else {
                warn!(path = ?path, "Skipping file with non-UUID name in state store directory");
                continue;
            };
            match fs::File::open(&path).map(BufReader::new) {
                Ok(reader) => match serde_json::from_reader(reader) {
                    Ok(policy) => {
                        states.insert(experiment_id, policy);
                    }
                    Err(err) => {
                        warn!(error = %err, id = %experiment_id, "Failed to deserialize state");
                    }
                },
                Err(err) => {
                    warn!(error = %err, id = %experiment_id, "Failed to open state file");
                }
            }
        }

        MessageResult(states)
    }
}
