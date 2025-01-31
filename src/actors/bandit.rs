use super::errors::BanditOrPolicyError;
use super::supervisor::{BanditCrashed, Supervisor};

use crate::policies::{Policy, PolicyStats};

use actix::prelude::*;
use uuid::Uuid;

pub struct Bandit {
    id: Uuid,
    policy: Box<dyn Policy + Send>,
    supervisor: Addr<Supervisor>,
}

impl Bandit {
    pub fn new(id: Uuid, policy: Box<dyn Policy + Send>, supervisor: Addr<Supervisor>) -> Self {
        Self {
            id,
            policy,
            supervisor,
        }
    }
}

impl Actor for Bandit {
    type Context = Context<Self>;

    fn stopping(&mut self, _: &mut Context<Self>) -> Running {
        self.supervisor
            .do_send(BanditCrashed { bandit_id: self.id });
        Running::Stop
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Reset;

#[derive(Message)]
#[rtype(result = "usize")]
pub struct AddArm;

#[derive(Message)]
#[rtype(result = "Result<(), BanditOrPolicyError>")]
pub struct DeleteArm {
    pub arm_id: usize,
}

#[derive(Message)]
#[rtype(result = "Result<usize, BanditOrPolicyError>")]
pub struct Draw;

#[derive(Message)]
#[rtype(result = "Result<(), BanditOrPolicyError>")]
pub struct Update {
    pub arm_id: usize,
    pub reward: f64,
}

#[derive(Message)]
#[rtype(result = "Result<(), BanditOrPolicyError>")]
pub struct UpdateBatch {
    pub updates: Vec<(usize, usize, f64)>,
}

#[derive(Message)]
#[rtype(result = "PolicyStats")]
pub struct GetStats;

impl Handler<Reset> for Bandit {
    type Result = ();

    fn handle(&mut self, _: Reset, _: &mut Context<Self>) -> Self::Result {
        self.policy.reset()
    }
}

impl Handler<AddArm> for Bandit {
    type Result = usize;

    fn handle(&mut self, _: AddArm, _: &mut Context<Self>) -> Self::Result {
        self.policy.add_arm()
    }
}

impl Handler<DeleteArm> for Bandit {
    type Result = Result<(), BanditOrPolicyError>;

    fn handle(&mut self, msg: DeleteArm, _: &mut Context<Self>) -> Self::Result {
        self.policy
            .delete_arm(msg.arm_id)
            .map_err(BanditOrPolicyError::from)
    }
}

impl Handler<Draw> for Bandit {
    type Result = Result<usize, BanditOrPolicyError>;

    fn handle(&mut self, _: Draw, _: &mut Context<Self>) -> Self::Result {
        self.policy.draw().map_err(BanditOrPolicyError::from)
    }
}

impl Handler<Update> for Bandit {
    type Result = Result<(), BanditOrPolicyError>;

    fn handle(&mut self, msg: Update, _: &mut Context<Self>) -> Self::Result {
        self.policy
            .update(msg.arm_id, msg.reward)
            .map_err(BanditOrPolicyError::from)
    }
}

impl Handler<UpdateBatch> for Bandit {
    type Result = Result<(), BanditOrPolicyError>;

    fn handle(&mut self, msg: UpdateBatch, _: &mut Context<Self>) -> Self::Result {
        self.policy
            .update_batch(&msg.updates)
            .map_err(BanditOrPolicyError::from)
    }
}

impl Handler<GetStats> for Bandit {
    type Result = MessageResult<GetStats>;

    fn handle(&mut self, _: GetStats, _: &mut Context<Self>) -> Self::Result {
        MessageResult(self.policy.stats())
    }
}
