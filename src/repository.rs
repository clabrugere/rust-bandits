use super::actors::experiment::Experiment;
use super::actors::experiment_cache::{ExperimentCache, ReadFullExperimentCache};
use super::errors::{RepositoryError, RepositoryOrExperimentError};

use crate::actors::experiment::{
    AddArm, DeleteArm, Draw, GetStats, Ping, Reset, Update, UpdateBatch,
};
use crate::config::ExperimentConfig;
use crate::policies::{DrawResult, Policy, PolicyStats};

use actix::prelude::*;
use log::info;
use std::collections::HashMap;
use uuid::Uuid;

pub struct Repository {
    experiments: HashMap<Uuid, Addr<Experiment>>,
    experiment_config: ExperimentConfig,
    cache: Addr<ExperimentCache>,
}

impl Repository {
    pub fn new(experiment_config: ExperimentConfig, cache: Addr<ExperimentCache>) -> Self {
        Self {
            experiments: HashMap::new(),
            experiment_config,
            cache,
        }
    }

    pub async fn load_experiments(&mut self) -> Result<(), RepositoryError> {
        self.cache
            .send(ReadFullExperimentCache)
            .await
            .map(|experiments| {
                info!("Loading {} experiment(s)", experiments.len());
                experiments.iter().for_each(|(&experiment_id, policy)| {
                    self.create_experiment(Some(experiment_id), policy.clone_box());
                    info!("Loaded experiment {}", experiment_id);
                });
            })
            .map_err(|err| RepositoryError::StorageError(err.to_string()))
    }

    fn get_experiment(
        &self,
        experiment_id: Uuid,
    ) -> Result<Addr<Experiment>, RepositoryOrExperimentError> {
        self.experiments
            .get(&experiment_id)
            .cloned()
            .ok_or_else(|| {
                RepositoryOrExperimentError::Repository(RepositoryError::ExperimentNotFound(
                    experiment_id,
                ))
            })
    }

    async fn send_to_experiment<M>(
        &self,
        experiment_id: Uuid,
        message: M,
    ) -> Result<M::Result, RepositoryOrExperimentError>
    where
        M: actix::Message + Send + 'static,
        M::Result: Send + 'static,
        Experiment: actix::Handler<M>,
    {
        self.get_experiment(experiment_id)?
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

    pub fn list_experiments(&self) -> Result<Vec<Uuid>, RepositoryError> {
        Ok(self.experiments.keys().cloned().collect())
    }

    pub fn clear(&mut self) -> () {
        self.experiments.clear();
    }

    pub fn create_experiment(
        &mut self,
        experiment_id: Option<Uuid>,
        policy: Box<dyn Policy + Send>,
    ) -> Uuid {
        let experiment_id = experiment_id.unwrap_or(Uuid::new_v4());
        let actor = Experiment::new(
            experiment_id,
            policy,
            self.cache.clone(),
            self.experiment_config.save_every,
        )
        .start();
        self.experiments.insert(experiment_id, actor);

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
        reward: Option<f64>,
        count: Option<u64>,
    ) -> Result<(), RepositoryOrExperimentError> {
        self.send_to_experiment(
            experiment_id,
            Reset {
                arm_id,
                reward,
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
        draw_id: Uuid,
        timestamp: u128,
        arm_id: usize,
        reward: f64,
    ) -> Result<(), RepositoryOrExperimentError> {
        self.send_to_experiment(
            experiment_id,
            Update {
                draw_id,
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
        updates: Vec<(Uuid, u128, usize, f64)>,
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
