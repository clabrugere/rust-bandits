use crate::actors::experiment::{
    AddArm, Delete, DeleteArm, DisableArm, Draw, EnableArm, Experiment, GetStats, Ping, Reset,
    Update, UpdateBatch,
};
use crate::actors::state_store::{LoadAllStates, StateStore};
use crate::config::ExperimentConfig;
use crate::errors::{RepositoryError, ServiceError};
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

    pub async fn load_experiments(&mut self) -> Result<(), ServiceError> {
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
            .map_err(|err| ServiceError::Mailbox {
                actor: "StateStore",
                source: err,
            })
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
    ) -> Result<M::Result, ServiceError>
    where
        M: Message + Send + 'static,
        M::Result: Send + 'static,
        Experiment: Handler<M>,
    {
        self.get_experiment_address(experiment_id)?
            .send(message)
            .await
            .map_err(|err| ServiceError::Mailbox {
                actor: "Experiment",
                source: err,
            })
    }

    pub async fn ping_experiment(&self, experiment_id: Uuid) -> Result<(), ServiceError> {
        self.send_to_experiment(experiment_id, Ping).await
    }

    pub fn iter_experiments(&self) -> impl Iterator<Item = (&Uuid, &PolicyType)> {
        self.experiments
            .iter()
            .map(|(id, el)| (id, &el.policy_type))
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

    pub async fn delete_experiment(&mut self, experiment_id: Uuid) -> Result<(), ServiceError> {
        self.get_experiment_address(experiment_id)?.do_send(Delete);
        self.experiments.remove(&experiment_id);
        Ok(())
    }

    pub async fn reset_experiment(
        &self,
        experiment_id: Uuid,
        arm_id: Option<usize>,
        cumulative_reward: Option<f64>,
        count: Option<u64>,
    ) -> Result<(), ServiceError> {
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
        .map_err(ServiceError::from)
    }

    pub async fn add_experiment_arm(
        &self,
        experiment_id: Uuid,
        initial_reward: Option<f64>,
        initial_count: Option<u64>,
    ) -> Result<usize, ServiceError> {
        self.send_to_experiment(
            experiment_id,
            AddArm {
                initial_reward,
                initial_count,
            },
        )
        .await?
        .map_err(RepositoryError::from)
        .map_err(ServiceError::from)
    }

    pub async fn enable_experiment_arm(
        &self,
        experiment_id: Uuid,
        arm_id: usize,
    ) -> Result<(), ServiceError> {
        self.send_to_experiment(experiment_id, EnableArm { arm_id })
            .await?
            .map_err(RepositoryError::from)
            .map_err(ServiceError::from)
    }

    pub async fn disable_experiment_arm(
        &self,
        experiment_id: Uuid,
        arm_id: usize,
    ) -> Result<(), ServiceError> {
        self.send_to_experiment(experiment_id, DisableArm { arm_id })
            .await?
            .map_err(RepositoryError::from)
            .map_err(ServiceError::from)
    }

    pub async fn delete_experiment_arm(
        &self,
        experiment_id: Uuid,
        arm_id: usize,
    ) -> Result<(), ServiceError> {
        self.send_to_experiment(experiment_id, DeleteArm { arm_id })
            .await?
            .map_err(RepositoryError::from)
            .map_err(ServiceError::from)
    }

    pub async fn draw_experiment(&self, experiment_id: Uuid) -> Result<DrawResult, ServiceError> {
        self.send_to_experiment(experiment_id, Draw)
            .await?
            .map_err(RepositoryError::from)
            .map_err(ServiceError::from)
    }

    pub async fn update_experiment(
        &self,
        experiment_id: Uuid,
        timestamp: f64,
        arm_id: usize,
        reward: f64,
    ) -> Result<(), ServiceError> {
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
        .map_err(ServiceError::from)
    }

    pub async fn batch_update_experiment(
        &self,
        experiment_id: Uuid,
        updates: Vec<BatchUpdateElement>,
    ) -> Result<(), ServiceError> {
        self.send_to_experiment(experiment_id, UpdateBatch { updates })
            .await?
            .map_err(RepositoryError::from)
            .map_err(ServiceError::from)
    }

    pub async fn get_experiment_stats(
        &self,
        experiment_id: Uuid,
    ) -> Result<PolicyStats, ServiceError> {
        self.send_to_experiment(experiment_id, GetStats)
            .await?
            .map_err(RepositoryError::from)
            .map_err(ServiceError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::state_store::SaveState;
    use crate::config::{ExperimentConfig, StateStoreConfig};
    use crate::errors::{RepositoryError, ServiceError};
    use crate::policies::{Policy, PolicyType};

    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    const EPSILON: f64 = 0.01;
    const DEFAULT_SEED: Option<u64> = Some(1234);

    fn make_policy() -> Box<dyn Policy + Send> {
        PolicyType::EpsilonGreedy {
            epsilon: EPSILON,
            epsilon_decay: None,
            seed: DEFAULT_SEED,
        }
        .into_inner()
    }

    struct TestContext {
        repository: Repository,
        state_store: Addr<StateStore>,
        state_path: PathBuf,
    }

    impl TestContext {
        fn new() -> Self {
            let state_path =
                std::env::temp_dir().join(format!("state-store-{}.json", Uuid::new_v4()));
            let state_store_config = StateStoreConfig {
                path: state_path.clone(),
                persist_every: 86_400,
            };
            let state_store = StateStore::new(state_store_config).start();
            let experiment_config = ExperimentConfig { save_every: 86_400 };
            let repository = Repository::new(experiment_config, state_store.clone());

            Self {
                repository,
                state_store,
                state_path,
            }
        }
    }

    impl Drop for TestContext {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.state_path);
        }
    }

    #[actix::test]
    async fn create_and_ping_experiment() {
        let mut ctx = TestContext::new();
        let experiment_id = ctx.repository.create_experiment(None, make_policy());

        ctx.repository
            .ping_experiment(experiment_id)
            .await
            .expect("experiment should respond to ping");

        assert!(ctx
            .repository
            .iter_experiments()
            .any(|(id, policy)| {
                *id == experiment_id
                    && matches!(policy, PolicyType::EpsilonGreedy { epsilon, .. } if *epsilon == EPSILON)
            }));
    }

    #[actix::test]
    async fn manages_arm_lifecycle() {
        let mut ctx = TestContext::new();
        let experiment_id = ctx.repository.create_experiment(None, make_policy());

        let arm_id = ctx
            .repository
            .add_experiment_arm(experiment_id, Some(1.0), Some(1))
            .await
            .expect("arm creation should succeed");

        let mut stats = ctx
            .repository
            .get_experiment_stats(experiment_id)
            .await
            .expect("stats should be available");
        let arm_stats = stats.arms.get(&arm_id).expect("arm should exist");
        assert_eq!(arm_stats.pulls, 1);
        assert!(arm_stats.is_active);

        ctx.repository
            .disable_experiment_arm(experiment_id, arm_id)
            .await
            .expect("disable should succeed");
        stats = ctx
            .repository
            .get_experiment_stats(experiment_id)
            .await
            .expect("stats after disable");
        assert!(!stats.arms[&arm_id].is_active);

        ctx.repository
            .enable_experiment_arm(experiment_id, arm_id)
            .await
            .expect("enable should succeed");
        stats = ctx
            .repository
            .get_experiment_stats(experiment_id)
            .await
            .expect("stats after enable");
        assert!(stats.arms[&arm_id].is_active);

        let draw = ctx
            .repository
            .draw_experiment(experiment_id)
            .await
            .expect("draw should succeed");
        assert_eq!(draw.arm_id, arm_id);

        ctx.repository
            .update_experiment(experiment_id, 42.0, arm_id, 2.0)
            .await
            .expect("update should succeed");
        ctx.repository
            .batch_update_experiment(experiment_id, vec![(1.0, arm_id, 3.0), (2.0, arm_id, 1.0)])
            .await
            .expect("batch update should succeed");

        stats = ctx
            .repository
            .get_experiment_stats(experiment_id)
            .await
            .expect("stats after updates");
        assert_eq!(stats.arms[&arm_id].pulls, 4);
        assert!((stats.arms[&arm_id].mean_reward - 1.75).abs() < 1e-6);

        ctx.repository
            .reset_experiment(experiment_id, Some(arm_id), Some(0.0), Some(0))
            .await
            .expect("reset should succeed");
        stats = ctx
            .repository
            .get_experiment_stats(experiment_id)
            .await
            .expect("stats after reset");
        let arm_stats = stats.arms.get(&arm_id).expect("arm present after reset");
        assert_eq!(arm_stats.pulls, 0);
        assert_eq!(arm_stats.mean_reward, 0.0);

        ctx.repository
            .delete_experiment_arm(experiment_id, arm_id)
            .await
            .expect("delete arm should succeed");
        stats = ctx
            .repository
            .get_experiment_stats(experiment_id)
            .await
            .expect("stats after delete");
        assert!(stats.arms.get(&arm_id).is_none());

        ctx.repository
            .delete_experiment(experiment_id)
            .await
            .expect("delete experiment should succeed");
        assert!(!ctx
            .repository
            .iter_experiments()
            .any(|(id, _)| *id == experiment_id));

        let err = ctx
            .repository
            .ping_experiment(experiment_id)
            .await
            .expect_err("ping should fail after deletion");
        match err {
            ServiceError::Repository(RepositoryError::ExperimentNotFound(id)) => {
                assert_eq!(id, experiment_id)
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[actix::test]
    async fn loads_experiments_from_state_store() {
        let mut ctx = TestContext::new();
        let experiment_id = Uuid::new_v4();

        let mut saved_policy = make_policy();
        saved_policy.add_arm(5.0, 2);

        ctx.state_store
            .send(SaveState {
                experiment_id,
                policy: saved_policy,
            })
            .await
            .expect("state should be saved");

        ctx.repository
            .load_experiments()
            .await
            .expect("loading from state store should succeed");

        assert!(ctx
            .repository
            .iter_experiments()
            .any(|(id, _)| *id == experiment_id));

        let stats = ctx
            .repository
            .get_experiment_stats(experiment_id)
            .await
            .expect("stats should be retrievable");
        assert_eq!(stats.arms.len(), 1);
        let arm = stats.arms.values().next().expect("arm data present");
        assert_eq!(arm.pulls, 2);
        assert_eq!(arm.mean_reward, 5.0);
    }
}
