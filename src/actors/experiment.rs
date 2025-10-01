use super::experiment_cache::{ExperimentCache, InsertExperimentCache};

use crate::errors::ExperimentOrPolicyError;
use crate::policies::{BatchUpdateElement, DrawResult, Policy, PolicyStats};

use actix::prelude::*;
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

pub struct Experiment {
    id: Uuid,
    policy: Box<dyn Policy + Send>,
    cache: Addr<ExperimentCache>,
    save_every: u64,
}

impl Experiment {
    pub fn new(
        id: Uuid,
        policy: Box<dyn Policy + Send>,
        cache: Addr<ExperimentCache>,
        save_every: u64,
    ) -> Self {
        Self {
            id,
            policy,
            cache,
            save_every,
        }
    }

    fn persist(&self) {
        info!(id = %self.id, "Persisting policy state for experiment");
        self.cache.do_send(InsertExperimentCache {
            experiment_id: self.id,
            policy: self.policy.clone_box(),
        });
    }
}

impl Actor for Experiment {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!(id = %self.id, "Starting actor for experiment");
        ctx.run_interval(Duration::from_secs(self.save_every), |experiment, _| {
            experiment.persist();
        });
    }
}

// Messages
#[derive(Message)]
#[rtype(result = "()")]
pub struct Ping;

#[derive(Message)]
#[rtype(result = "Result<(), ExperimentOrPolicyError>")]
pub struct Reset {
    pub arm_id: Option<usize>,
    pub reward: Option<f64>,
    pub count: Option<u64>,
}

#[derive(Message)]
#[rtype(result = "usize")]
pub struct AddArm {
    pub initial_reward: Option<f64>,
    pub initial_count: Option<u64>,
}

#[derive(Message)]
#[rtype(result = "Result<(), ExperimentOrPolicyError>")]
pub struct DeleteArm {
    pub arm_id: usize,
}

#[derive(Message)]
#[rtype(result = "Result<DrawResult, ExperimentOrPolicyError>")]
pub struct Draw;

#[derive(Message)]
#[rtype(result = "Result<(), ExperimentOrPolicyError>")]
pub struct Update {
    pub timestamp: u128,
    pub arm_id: usize,
    pub reward: f64,
}

#[derive(Message)]
#[rtype(result = "Result<(), ExperimentOrPolicyError>")]
pub struct UpdateBatch {
    pub updates: Vec<BatchUpdateElement>,
}

#[derive(Message)]
#[rtype(result = "PolicyStats")]
pub struct GetStats;

impl Handler<Ping> for Experiment {
    type Result = ();

    fn handle(&mut self, _: Ping, _: &mut Self::Context) -> Self::Result {}
}

impl Handler<Reset> for Experiment {
    type Result = Result<(), ExperimentOrPolicyError>;

    fn handle(&mut self, msg: Reset, _: &mut Self::Context) -> Self::Result {
        self.policy
            .reset(msg.arm_id, msg.reward, msg.count)
            .map_err(ExperimentOrPolicyError::from)
    }
}

impl Handler<AddArm> for Experiment {
    type Result = usize;

    fn handle(&mut self, msg: AddArm, _: &mut Self::Context) -> Self::Result {
        self.policy.add_arm(
            msg.initial_reward.unwrap_or_default(),
            msg.initial_count.unwrap_or_default(),
        )
    }
}

impl Handler<DeleteArm> for Experiment {
    type Result = Result<(), ExperimentOrPolicyError>;

    fn handle(&mut self, msg: DeleteArm, _: &mut Self::Context) -> Self::Result {
        self.policy
            .delete_arm(msg.arm_id)
            .map_err(ExperimentOrPolicyError::from)
    }
}

impl Handler<Draw> for Experiment {
    type Result = Result<DrawResult, ExperimentOrPolicyError>;

    fn handle(&mut self, _: Draw, _: &mut Self::Context) -> Self::Result {
        self.policy.draw().map_err(ExperimentOrPolicyError::from)
    }
}

impl Handler<Update> for Experiment {
    type Result = Result<(), ExperimentOrPolicyError>;

    fn handle(&mut self, msg: Update, _: &mut Self::Context) -> Self::Result {
        self.policy
            .update(msg.timestamp, msg.arm_id, msg.reward)
            .map_err(ExperimentOrPolicyError::from)
    }
}

impl Handler<UpdateBatch> for Experiment {
    type Result = Result<(), ExperimentOrPolicyError>;

    fn handle(&mut self, msg: UpdateBatch, _: &mut Self::Context) -> Self::Result {
        self.policy
            .update_batch(&msg.updates)
            .map_err(ExperimentOrPolicyError::from)
    }
}

impl Handler<GetStats> for Experiment {
    type Result = MessageResult<GetStats>;

    fn handle(&mut self, _: GetStats, _: &mut Self::Context) -> Self::Result {
        MessageResult(self.policy.stats())
    }
}
