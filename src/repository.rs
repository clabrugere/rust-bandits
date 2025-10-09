use super::actors::experiment::Experiment;
use super::actors::state_store::{ReadFullExperimentState, StateStore};
use super::errors::{RepositoryError, RepositoryOrExperimentError};

use crate::actors::experiment::{
    AddArm, DeleteArm, Draw, GetStats, Ping, Reset, Update, UpdateBatch,
};
use crate::config::ExperimentConfig;
use crate::policies::{BatchUpdateElement, DrawResult, Policy, PolicyStats, PolicyType};

use actix::prelude::*;
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

struct RepositoryElement {
    address: Addr<Experiment>,
    policy_type: PolicyType,
}

pub struct Repository {
    experiments: HashMap<Uuid, RepositoryElement>,
    experiment_config: ExperimentConfig,
    state_store: Addr<StateStore>,
}

impl Repository {
    pub fn new(experiment_config: ExperimentConfig, state_store: Addr<StateStore>) -> Self {
        Self {
            experiments: HashMap::new(),
            experiment_config,
            state_store,
        }
    }

    pub async fn load_experiments(&mut self) -> Result<(), RepositoryError> {
        self.state_store
            .send(ReadFullExperimentState)
            .await
            .map(|experiments| {
                info!(num_experiments = %experiments.len(), "Loaded experiments");

                experiments.iter().for_each(|(&experiment_id, policy)| {
                    self.create_experiment(Some(experiment_id), policy.clone_box());
                    info!(id = %experiment_id, "Loaded experiment");
                });
            })
            .map_err(|err| RepositoryError::StorageError(err.to_string()))
    }

    fn get_experiment_address(
        &self,
        experiment_id: Uuid,
    ) -> Result<Addr<Experiment>, RepositoryOrExperimentError> {
        // cloning the address of an actor is cheap
        self.experiments
            .get(&experiment_id)
            .map(|e| &e.address)
            .cloned()
            .ok_or(RepositoryOrExperimentError::Repository(
                RepositoryError::ExperimentNotFound(experiment_id),
            ))
    }

    async fn send_to_experiment<M>(
        &self,
        experiment_id: Uuid,
        message: M,
    ) -> Result<M::Result, RepositoryOrExperimentError>
    where
        M: Message + Send + 'static,
        M::Result: Send + 'static,
        Experiment: Handler<M>,
    {
        self.get_experiment_address(experiment_id)?
            .send(message)
            .await
            .map_err(|_| {
                RepositoryOrExperimentError::Repository(RepositoryError::ExperimentNotAvailable(
                    experiment_id,
                ))
            })
    }

    pub async fn ping_experiment(
        &self,
        experiment_id: Uuid,
    ) -> Result<(), RepositoryOrExperimentError> {
        self.send_to_experiment(experiment_id, Ping).await
    }

    pub fn list_experiments(&self) -> Result<HashMap<Uuid, PolicyType>, RepositoryError> {
        Ok(self
            .experiments
            .iter()
            .map(|(&id, el)| (id, el.policy_type.clone()))
            .collect())
    }

    pub fn clear(&mut self) {
        self.experiments.clear();
    }

    pub fn create_experiment(
        &mut self,
        experiment_id: Option<Uuid>,
        policy: Box<dyn Policy + Send>,
    ) -> Uuid {
        let experiment_id = experiment_id.unwrap_or(Uuid::new_v4());
        let policy_type = policy.policy_type();
        let address = Experiment::new(
            experiment_id,
            policy,
            self.state_store.clone(),
            self.experiment_config.save_every,
        )
        .start();
        self.experiments.insert(
            experiment_id,
            RepositoryElement {
                address,
                policy_type,
            },
        );

        experiment_id
    }

    pub fn delete_experiment(&mut self, experiment_id: Uuid) -> Result<(), RepositoryError> {
        if self.experiments.contains_key(&experiment_id) {
            self.experiments.remove(&experiment_id);
            Ok(())
        } else {
            Err(RepositoryError::ExperimentNotFound(experiment_id))
        }
    }

    pub async fn reset_experiment(
        &self,
        experiment_id: Uuid,
        arm_id: Option<usize>,
        cumulative_reward: Option<f64>,
        count: Option<u64>,
    ) -> Result<(), RepositoryOrExperimentError> {
        self.send_to_experiment(
            experiment_id,
            Reset {
                arm_id,
                cumulative_reward,
                count,
            },
        )
        .await?
        .map_err(RepositoryOrExperimentError::from)
    }

    pub async fn add_experiment_arm(
        &self,
        experiment_id: Uuid,
        initial_reward: Option<f64>,
        initial_count: Option<u64>,
    ) -> Result<usize, RepositoryOrExperimentError> {
        self.send_to_experiment(
            experiment_id,
            AddArm {
                initial_reward,
                initial_count,
            },
        )
        .await
    }

    pub async fn delete_experiment_arm(
        &self,
        experiment_id: Uuid,
        arm_id: usize,
    ) -> Result<(), RepositoryOrExperimentError> {
        self.send_to_experiment(experiment_id, DeleteArm { arm_id })
            .await?
            .map_err(RepositoryOrExperimentError::from)
    }

    pub async fn draw_experiment(
        &self,
        experiment_id: Uuid,
    ) -> Result<DrawResult, RepositoryOrExperimentError> {
        self.send_to_experiment(experiment_id, Draw)
            .await?
            .map_err(RepositoryOrExperimentError::from)
    }

    pub async fn update_experiment(
        &self,
        experiment_id: Uuid,
        timestamp: f64,
        arm_id: usize,
        reward: f64,
    ) -> Result<(), RepositoryOrExperimentError> {
        self.send_to_experiment(
            experiment_id,
            Update {
                timestamp,
                arm_id,
                reward,
            },
        )
        .await?
        .map_err(RepositoryOrExperimentError::from)
    }

    pub async fn batch_update_experiment(
        &self,
        experiment_id: Uuid,
        updates: Vec<BatchUpdateElement>,
    ) -> Result<(), RepositoryOrExperimentError> {
        self.send_to_experiment(experiment_id, UpdateBatch { updates })
            .await?
            .map_err(RepositoryOrExperimentError::from)
    }

    pub async fn get_experiment_stats(
        &self,
        experiment_id: Uuid,
    ) -> Result<PolicyStats, RepositoryOrExperimentError> {
        self.send_to_experiment(experiment_id, GetStats).await
    }
}
