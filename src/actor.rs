use crate::policies::bandit::{Bandit, BanditError, BanditStats};
use actix::prelude::*;

pub struct BanditActor {
    bandit: Box<dyn Bandit + Send>,
}

impl BanditActor {
    pub fn new(bandit: Box<dyn Bandit + Send>) -> Self {
        Self { bandit }
    }
}

impl Actor for BanditActor {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Reset;

#[derive(Message)]
#[rtype(result = "usize")]
pub struct AddArm;

#[derive(Message)]
#[rtype(result = "Result<(), BanditError>")]
pub struct DeleteArm {
    pub arm_id: usize,
}

#[derive(Message)]
#[rtype(result = "Result<usize, BanditError>")]
pub struct Draw;

#[derive(Message)]
#[rtype(result = "Result<(), BanditError>")]
pub struct Update {
    pub arm_id: usize,
    pub reward: f64,
}

#[derive(Message)]
#[rtype(result = "BanditStats")]
pub struct GetStats;

impl Handler<Reset> for BanditActor {
    type Result = ();

    fn handle(&mut self, _: Reset, _: &mut Self::Context) -> Self::Result {
        self.bandit.reset()
    }
}

impl Handler<AddArm> for BanditActor {
    type Result = usize;

    fn handle(&mut self, _: AddArm, _: &mut Context<Self>) -> Self::Result {
        self.bandit.add_arm()
    }
}

impl Handler<DeleteArm> for BanditActor {
    type Result = Result<(), BanditError>;

    fn handle(&mut self, msg: DeleteArm, _: &mut Context<Self>) -> Self::Result {
        self.bandit.delete_arm(msg.arm_id)
    }
}

impl Handler<Draw> for BanditActor {
    type Result = Result<usize, BanditError>;

    fn handle(&mut self, _: Draw, _: &mut Context<Self>) -> Self::Result {
        self.bandit.draw()
    }
}

impl Handler<Update> for BanditActor {
    type Result = Result<(), BanditError>;

    fn handle(&mut self, msg: Update, _: &mut Context<Self>) -> Self::Result {
        self.bandit.update(msg.arm_id, msg.reward)
    }
}

impl Handler<GetStats> for BanditActor {
    type Result = MessageResult<GetStats>;

    fn handle(&mut self, _: GetStats, _: &mut Context<Self>) -> Self::Result {
        MessageResult(self.bandit.stats())
    }
}
