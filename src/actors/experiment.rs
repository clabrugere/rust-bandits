use super::errors::ExperimentOrPolicyError;
use super::policy_cache::{InsertPolicyCache, PolicyCache};

use crate::policies::{Policy, PolicyStats};

use actix::prelude::*;
use log::info;
use std::time::Duration;
use uuid::Uuid;

pub struct Experiment {
    id: Uuid,
    policy: Box<dyn Policy + Send>,
    cache: Addr<PolicyCache>,
    cache_every: u64,
}

impl Experiment {
    pub fn new(
        id: Uuid,
        policy: Box<dyn Policy + Send>,
        cache: Addr<PolicyCache>,
        cache_every: u64,
    ) -> Self {
        Self {
            id,
            policy,
            cache,
            cache_every,
        }
    }

    fn cache(&self) {
        info!("Caching policy for experiment {}", &self.id);
        self.cache.do_send(InsertPolicyCache {
            experiment_id: self.id,
            policy: self.policy.clone_box(),
        });
    }
}

impl Actor for Experiment {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Starting actor for experiment {}", self.id);
        ctx.run_interval(Duration::from_secs(self.cache_every), |experiment, _| {
            experiment.cache();
        });
    }
}

#[derive(Message)]
#[rtype(result = "Pong")]
pub struct Ping;

#[derive(MessageResponse)]
pub struct Pong;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Reset;

#[derive(Message)]
#[rtype(result = "usize")]
pub struct AddArm;

#[derive(Message)]
#[rtype(result = "Result<(), ExperimentOrPolicyError>")]
pub struct DeleteArm {
    pub arm_id: usize,
}

#[derive(Message)]
#[rtype(result = "Result<usize, ExperimentOrPolicyError>")]
pub struct Draw;

#[derive(Message)]
#[rtype(result = "Result<(), ExperimentOrPolicyError>")]
pub struct Update {
    pub arm_id: usize,
    pub reward: f64,
}

#[derive(Message)]
#[rtype(result = "Result<(), ExperimentOrPolicyError>")]
pub struct UpdateBatch {
    pub updates: Vec<(u64, usize, f64)>,
}

#[derive(Message)]
#[rtype(result = "PolicyStats")]
pub struct GetStats;

impl Handler<Ping> for Experiment {
    type Result = Pong;

    fn handle(&mut self, _: Ping, _: &mut Self::Context) -> Self::Result {
        Pong
    }
}

impl Handler<Reset> for Experiment {
    type Result = ();

    fn handle(&mut self, _: Reset, _: &mut Self::Context) -> Self::Result {
        self.policy.reset()
    }
}

impl Handler<AddArm> for Experiment {
    type Result = usize;

    fn handle(&mut self, _: AddArm, _: &mut Self::Context) -> Self::Result {
        self.policy.add_arm()
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
    type Result = Result<usize, ExperimentOrPolicyError>;

    fn handle(&mut self, _: Draw, _: &mut Self::Context) -> Self::Result {
        self.policy.draw().map_err(ExperimentOrPolicyError::from)
    }
}

impl Handler<Update> for Experiment {
    type Result = Result<(), ExperimentOrPolicyError>;

    fn handle(&mut self, msg: Update, _: &mut Self::Context) -> Self::Result {
        self.policy
            .update(msg.arm_id, msg.reward)
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
