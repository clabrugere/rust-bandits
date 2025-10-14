use super::state_store::{SaveState, StateStore};

use crate::actors::state_store::{DeleteState, LoadState};
use crate::errors::{ExperimentError, PolicyError};
use crate::policies::{BatchUpdateElement, DrawResult, Policy, PolicyStats};

use actix::prelude::*;
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

pub struct Experiment {
    id: Uuid,
    policy: Option<Box<dyn Policy + Send>>,
    state_store: Addr<StateStore>,
    save_every: u64,
}

impl Experiment {
    pub fn new(
        id: Uuid,
        policy: Option<Box<dyn Policy + Send>>,
        state_store: Addr<StateStore>,
        save_every: u64,
    ) -> Self {
        Self {
            id,
            policy,
            state_store,
            save_every,
        }
    }

    fn persist(&self) {
        if let Some(policy) = &self.policy {
            self.state_store.do_send(SaveState {
                experiment_id: self.id,
                policy: policy.clone_box(),
            });
        }
    }

    fn with_policy_mut<F, R, E>(&mut self, f: F) -> Result<R, ExperimentError>
    where
        F: FnOnce(&mut dyn Policy) -> Result<R, E>,
        ExperimentError: From<E>,
    {
        let policy = self.policy.as_mut().ok_or(ExperimentError::NoPolicy)?;
        f(policy.as_mut()).map_err(Into::into)
    }
}

impl Actor for Experiment {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!(id = %self.id, "Starting actor for experiment");

        if self.policy.is_none() {
            let experiment_id = self.id;
            let state_store = self.state_store.clone();

            ctx.spawn(
                async move {
                    match state_store.clone().send(LoadState { experiment_id }).await {
                        Ok(Some(policy)) => Some(policy),
                        _ => None,
                    }
                }
                .into_actor(self)
                .map(|maybe_policy, actor, _| {
                    if let Some(policy) = maybe_policy {
                        actor.policy = Some(policy);
                        info!(id = %actor.id, "Reloaded policy state for experiment");
                    }
                }),
            );
        }

        ctx.run_interval(Duration::from_secs(self.save_every), |experiment, _| {
            experiment.persist();
        });
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        info!(id = %self.id, "Stopped actor for experiment");
    }
}

impl Supervised for Experiment {}

// Messages
#[derive(Message)]
#[rtype(result = "()")]
pub struct Ping;

#[derive(Message)]
#[rtype(result = "Result<(), ExperimentError>")]
pub struct Delete;

#[derive(Message)]
#[rtype(result = "Result<(), ExperimentError>")]
pub struct Reset {
    pub arm_id: Option<usize>,
    pub cumulative_reward: Option<f64>,
    pub count: Option<u64>,
}

#[derive(Message)]
#[rtype(result = "Result<usize, ExperimentError>")]
pub struct AddArm {
    pub initial_reward: Option<f64>,
    pub initial_count: Option<u64>,
}

#[derive(Message)]
#[rtype(result = "Result<(), ExperimentError>")]
pub struct DisableArm {
    pub arm_id: usize,
}

#[derive(Message)]
#[rtype(result = "Result<(), ExperimentError>")]
pub struct EnableArm {
    pub arm_id: usize,
}

#[derive(Message)]
#[rtype(result = "Result<(), ExperimentError>")]
pub struct DeleteArm {
    pub arm_id: usize,
}

#[derive(Message)]
#[rtype(result = "Result<DrawResult, ExperimentError>")]
pub struct Draw;

#[derive(Message)]
#[rtype(result = "Result<(), ExperimentError>")]
pub struct Update {
    pub timestamp: f64,
    pub arm_id: usize,
    pub reward: f64,
}

#[derive(Message)]
#[rtype(result = "Result<(), ExperimentError>")]
pub struct UpdateBatch {
    pub updates: Vec<BatchUpdateElement>,
}

#[derive(Message)]
#[rtype(result = "Result<PolicyStats, ExperimentError>")]
pub struct GetStats;

// Handlers
impl Handler<Ping> for Experiment {
    type Result = ();

    fn handle(&mut self, _: Ping, _: &mut Self::Context) -> Self::Result {}
}

impl Handler<Delete> for Experiment {
    type Result = Result<(), ExperimentError>;

    fn handle(&mut self, _: Delete, ctx: &mut Self::Context) -> Self::Result {
        self.state_store.do_send(DeleteState {
            experiment_id: self.id,
        });
        ctx.stop();
        Ok(())
    }
}

impl Handler<Reset> for Experiment {
    type Result = Result<(), ExperimentError>;

    fn handle(&mut self, msg: Reset, _: &mut Self::Context) -> Self::Result {
        self.with_policy_mut(|policy| policy.reset(msg.arm_id, msg.cumulative_reward, msg.count))
    }
}

impl Handler<AddArm> for Experiment {
    type Result = Result<usize, ExperimentError>;

    fn handle(&mut self, msg: AddArm, _: &mut Self::Context) -> Self::Result {
        self.with_policy_mut(|policy| {
            let arm_id = policy.add_arm(
                msg.initial_reward.unwrap_or_default(),
                msg.initial_count.unwrap_or_default(),
            );
            Ok::<usize, PolicyError>(arm_id)
        })
    }
}

impl Handler<DisableArm> for Experiment {
    type Result = Result<(), ExperimentError>;

    fn handle(&mut self, msg: DisableArm, _: &mut Self::Context) -> Self::Result {
        self.with_policy_mut(|policy| policy.disable_arm(msg.arm_id))
    }
}

impl Handler<EnableArm> for Experiment {
    type Result = Result<(), ExperimentError>;

    fn handle(&mut self, msg: EnableArm, _: &mut Self::Context) -> Self::Result {
        self.with_policy_mut(|policy| policy.enable_arm(msg.arm_id))
    }
}

impl Handler<DeleteArm> for Experiment {
    type Result = Result<(), ExperimentError>;

    fn handle(&mut self, msg: DeleteArm, _: &mut Self::Context) -> Self::Result {
        self.with_policy_mut(|policy| policy.delete_arm(msg.arm_id))
    }
}

impl Handler<Draw> for Experiment {
    type Result = Result<DrawResult, ExperimentError>;

    fn handle(&mut self, _: Draw, _: &mut Self::Context) -> Self::Result {
        self.with_policy_mut(|policy| policy.draw())
    }
}

impl Handler<Update> for Experiment {
    type Result = Result<(), ExperimentError>;

    fn handle(&mut self, msg: Update, _: &mut Self::Context) -> Self::Result {
        self.with_policy_mut(|policy| policy.update(msg.timestamp, msg.arm_id, msg.reward))
    }
}

impl Handler<UpdateBatch> for Experiment {
    type Result = Result<(), ExperimentError>;

    fn handle(&mut self, msg: UpdateBatch, _: &mut Self::Context) -> Self::Result {
        let mut updates = msg.updates;
        updates.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));

        self.with_policy_mut(|policy| policy.update_batch(&updates))
    }
}

impl Handler<GetStats> for Experiment {
    type Result = Result<PolicyStats, ExperimentError>;

    fn handle(&mut self, _: GetStats, _: &mut Self::Context) -> Self::Result {
        self.with_policy_mut(|policy| Ok::<PolicyStats, PolicyError>(policy.stats()))
    }
}
