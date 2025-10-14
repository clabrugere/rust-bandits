use super::actors::experiment::Experiment;
use super::actors::state_store::{LoadAllStates, StateStore};
use super::errors::RepositoryError;

use crate::actors::experiment::{
    AddArm, DeleteArm, DisableArm, Draw, EnableArm, GetStats, Ping, Reset, Stop, Update,
    UpdateBatch,
};
use crate::config::ExperimentConfig;
use crate::policies::{BatchUpdateElement, DrawResult, Policy, PolicyStats, PolicyType};

use actix::{prelude::*, Supervisor};
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
            .send(LoadAllStates)
            .await
            .map(|experiments| {
                info!(num_experiments = %experiments.len(), "Loaded experiments");
                experiments.into_iter().for_each(|(experiment_id, policy)| {
                    self.create_experiment(Some(experiment_id), policy);
                    info!(id = %experiment_id, "Loaded experiment");
                });
            })
            .map_err(|_| RepositoryError::StorageUnavailable)
    }

    fn get_experiment_address(
        &self,
        experiment_id: Uuid,
    ) -> Result<&Addr<Experiment>, RepositoryError> {
        self.experiments
            .get(&experiment_id)
            .map(|e| &e.address)
            .ok_or(RepositoryError::ExperimentNotFound(experiment_id))
    }

    async fn send_to_experiment<M>(
        &self,
        experiment_id: Uuid,
        message: M,
    ) -> Result<M::Result, RepositoryError>
    where
        M: Message + Send + 'static,
        M::Result: Send + 'static,
        Experiment: Handler<M>,
    {
        self.get_experiment_address(experiment_id)?
            .send(message)
            .await
            .map_err(|_| RepositoryError::ExperimentUnavailable(experiment_id))
    }

    pub async fn ping_experiment(&self, experiment_id: Uuid) -> Result<(), RepositoryError> {
        self.send_to_experiment(experiment_id, Ping).await
    }

    pub fn iter_experiments(
        &self,
    ) -> Result<impl Iterator<Item = (&Uuid, &PolicyType)>, RepositoryError> {
        Ok(self
            .experiments
            .iter()
            .map(|(id, el)| (id, &el.policy_type)))
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
        // use a Supervisor to handle auto restart of crashed experiments
        let address = Supervisor::start({
            let state_store = self.state_store.clone();
            let save_every = self.experiment_config.save_every;

            move |_| Experiment::new(experiment_id, Some(policy), state_store.clone(), save_every)
        });

        self.experiments.insert(
            experiment_id,
            RepositoryElement {
                address,
                policy_type,
            },
        );

        experiment_id
    }

    pub async fn delete_experiment(&mut self, experiment_id: Uuid) -> Result<(), RepositoryError> {
        self.get_experiment_address(experiment_id)?.do_send(Stop);
        self.experiments.remove(&experiment_id);
        Ok(())
    }

    pub async fn reset_experiment(
        &self,
        experiment_id: Uuid,
        arm_id: Option<usize>,
        cumulative_reward: Option<f64>,
        count: Option<u64>,
    ) -> Result<(), RepositoryError> {
        self.send_to_experiment(
            experiment_id,
            Reset {
                arm_id,
                cumulative_reward,
                count,
            },
        )
        .await?
        .map_err(RepositoryError::from)
    }

    pub async fn add_experiment_arm(
        &self,
        experiment_id: Uuid,
        initial_reward: Option<f64>,
        initial_count: Option<u64>,
    ) -> Result<usize, RepositoryError> {
        self.send_to_experiment(
            experiment_id,
            AddArm {
                initial_reward,
                initial_count,
            },
        )
        .await?
        .map_err(RepositoryError::from)
    }

    pub async fn enable_experiment_arm(
        &self,
        experiment_id: Uuid,
        arm_id: usize,
    ) -> Result<(), RepositoryError> {
        self.send_to_experiment(experiment_id, EnableArm { arm_id })
            .await?
            .map_err(RepositoryError::from)
    }

    pub async fn disable_experiment_arm(
        &self,
        experiment_id: Uuid,
        arm_id: usize,
    ) -> Result<(), RepositoryError> {
        self.send_to_experiment(experiment_id, DisableArm { arm_id })
            .await?
            .map_err(RepositoryError::from)
    }

    pub async fn delete_experiment_arm(
        &self,
        experiment_id: Uuid,
        arm_id: usize,
    ) -> Result<(), RepositoryError> {
        self.send_to_experiment(experiment_id, DeleteArm { arm_id })
            .await?
            .map_err(RepositoryError::from)
    }

    pub async fn draw_experiment(
        &self,
        experiment_id: Uuid,
    ) -> Result<DrawResult, RepositoryError> {
        self.send_to_experiment(experiment_id, Draw)
            .await?
            .map_err(RepositoryError::from)
    }

    pub async fn update_experiment(
        &self,
        experiment_id: Uuid,
        timestamp: f64,
        arm_id: usize,
        reward: f64,
    ) -> Result<(), RepositoryError> {
        self.send_to_experiment(
            experiment_id,
            Update {
                timestamp,
                arm_id,
                reward,
            },
        )
        .await?
        .map_err(RepositoryError::from)
    }

    pub async fn batch_update_experiment(
        &self,
        experiment_id: Uuid,
        updates: Vec<BatchUpdateElement>,
    ) -> Result<(), RepositoryError> {
        self.send_to_experiment(experiment_id, UpdateBatch { updates })
            .await?
            .map_err(RepositoryError::from)
    }

    pub async fn get_experiment_stats(
        &self,
        experiment_id: Uuid,
    ) -> Result<PolicyStats, RepositoryError> {
        self.send_to_experiment(experiment_id, GetStats)
            .await?
            .map_err(RepositoryError::from)
    }
}
