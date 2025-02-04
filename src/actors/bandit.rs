use super::cache::{InsertPolicyCache, PolicyCache};
use super::errors::BanditOrPolicyError;

use crate::policies::{Policy, PolicyStats};

use actix::prelude::*;
use log::info;
use std::time::Duration;
use uuid::Uuid;

pub struct Bandit {
    id: Uuid,
    policy: Box<dyn Policy + Send>,
    cache: Addr<PolicyCache>,
    cache_every: u64,
}

impl Bandit {
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
        info!("Caching bandit {}", &self.id);
        self.cache.do_send(InsertPolicyCache {
            bandit_id: self.id,
            policy: self.policy.clone_box(),
        });
    }
}

impl Actor for Bandit {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Started bandit {}", self.id);
        ctx.run_interval(Duration::from_secs(self.cache_every), |bandit, _| {
            bandit.cache();
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
    pub updates: Vec<(u64, usize, f64)>,
}

#[derive(Message)]
#[rtype(result = "PolicyStats")]
pub struct GetStats;

impl Handler<Ping> for Bandit {
    type Result = Pong;

    fn handle(&mut self, _: Ping, _: &mut Self::Context) -> Self::Result {
        Pong
    }
}

impl Handler<Reset> for Bandit {
    type Result = ();

    fn handle(&mut self, _: Reset, _: &mut Self::Context) -> Self::Result {
        self.policy.reset()
    }
}

impl Handler<AddArm> for Bandit {
    type Result = usize;

    fn handle(&mut self, _: AddArm, _: &mut Self::Context) -> Self::Result {
        self.policy.add_arm()
    }
}

impl Handler<DeleteArm> for Bandit {
    type Result = Result<(), BanditOrPolicyError>;

    fn handle(&mut self, msg: DeleteArm, _: &mut Self::Context) -> Self::Result {
        self.policy
            .delete_arm(msg.arm_id)
            .map_err(BanditOrPolicyError::from)
    }
}

impl Handler<Draw> for Bandit {
    type Result = Result<usize, BanditOrPolicyError>;

    fn handle(&mut self, _: Draw, _: &mut Self::Context) -> Self::Result {
        self.policy.draw().map_err(BanditOrPolicyError::from)
    }
}

impl Handler<Update> for Bandit {
    type Result = Result<(), BanditOrPolicyError>;

    fn handle(&mut self, msg: Update, _: &mut Self::Context) -> Self::Result {
        self.policy
            .update(msg.arm_id, msg.reward)
            .map_err(BanditOrPolicyError::from)
    }
}

impl Handler<UpdateBatch> for Bandit {
    type Result = Result<(), BanditOrPolicyError>;

    fn handle(&mut self, msg: UpdateBatch, _: &mut Self::Context) -> Self::Result {
        self.policy
            .update_batch(&msg.updates)
            .map_err(BanditOrPolicyError::from)
    }
}

impl Handler<GetStats> for Bandit {
    type Result = MessageResult<GetStats>;

    fn handle(&mut self, _: GetStats, _: &mut Self::Context) -> Self::Result {
        MessageResult(self.policy.stats())
    }
}
