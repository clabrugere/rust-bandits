use super::errors::{SupervisorError, SupervisorOrBanditError};
use crate::{
    bandit::{AddArm, Bandit, DeleteArm, Draw, GetStats, Reset, Update, UpdateBatch},
    policy::{create_policy, PolicyStats, PolicyType},
};
use actix::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

pub struct Supervisor {
    bandits: HashMap<Uuid, Addr<Bandit>>,
}

impl Supervisor {
    pub fn new() -> Self {
        Self {
            bandits: HashMap::new(),
        }
    }

    pub fn create_bandit(&mut self, policy_type: PolicyType, ctx: &mut Context<Self>) -> Uuid {
        let bandit_id = Uuid::new_v4();
        let policy = create_policy(&policy_type);
        let actor = Bandit::new(bandit_id, policy, ctx.address()).start();

        self.bandits.insert(bandit_id, actor);
        bandit_id
    }

    pub fn delete_bandit(&mut self, bandit_id: &Uuid) -> Result<(), SupervisorError> {
        if self.bandits.contains_key(bandit_id) {
            self.bandits.remove(bandit_id);
            Ok(())
        } else {
            Err(SupervisorError::BanditNotFound(*bandit_id))
        }
    }
}

impl Actor for Supervisor {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "Vec<Uuid>")]
pub struct ListBandits;

#[derive(Message)]
#[rtype(result = "Uuid")]
pub struct CreateBandit {
    pub bandit_type: PolicyType,
}

#[derive(Message)]
#[rtype(result = "Result<(), SupervisorOrBanditError>")]
pub struct DeleteBandit {
    pub bandit_id: Uuid,
}

#[derive(Message)]
#[rtype(result = "Result<(), SupervisorOrBanditError>")]
pub struct ResetBandit {
    pub bandit_id: Uuid,
}

#[derive(Message)]
#[rtype(result = "Result<usize, SupervisorOrBanditError>")]
pub struct AddArmBandit {
    pub bandit_id: Uuid,
}

#[derive(Message)]
#[rtype(result = "Result<(), SupervisorOrBanditError>")]
pub struct DeleteArmBandit {
    pub bandit_id: Uuid,
    pub arm_id: usize,
}

#[derive(Message)]
#[rtype(result = "Result<usize, SupervisorOrBanditError>")]
pub struct DrawBandit {
    pub bandit_id: Uuid,
}

#[derive(Message)]
#[rtype(result = "Result<(), SupervisorOrBanditError>")]
pub struct UpdateBandit {
    pub bandit_id: Uuid,
    pub arm_id: usize,
    pub reward: f64,
}

#[derive(Message)]
#[rtype(result = "Result<(), SupervisorOrBanditError>")]
pub struct UpdateBatchBandit {
    pub bandit_id: Uuid,
    pub updates: Vec<(usize, usize, f64)>,
}

#[derive(Message)]
#[rtype(result = "Result<PolicyStats, SupervisorOrBanditError>")]
pub struct GetBanditStats {
    pub bandit_id: Uuid,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct BanditCrashed {
    pub bandit_id: Uuid,
}

impl Handler<ListBandits> for Supervisor {
    type Result = MessageResult<ListBandits>;

    fn handle(&mut self, _: ListBandits, _: &mut Context<Self>) -> Self::Result {
        MessageResult(self.bandits.keys().cloned().collect())
    }
}

impl Handler<CreateBandit> for Supervisor {
    type Result = MessageResult<CreateBandit>;

    fn handle(&mut self, msg: CreateBandit, ctx: &mut Context<Self>) -> Self::Result {
        MessageResult(self.create_bandit(msg.bandit_type, ctx))
    }
}

impl Handler<DeleteBandit> for Supervisor {
    type Result = Result<(), SupervisorOrBanditError>;

    fn handle(&mut self, msg: DeleteBandit, _: &mut Context<Self>) -> Self::Result {
        self.delete_bandit(&msg.bandit_id)
            .map_err(SupervisorOrBanditError::from)
    }
}

impl Handler<ResetBandit> for Supervisor {
    type Result = ResponseFuture<Result<(), SupervisorOrBanditError>>;

    fn handle(&mut self, msg: ResetBandit, _: &mut Context<Self>) -> Self::Result {
        if let Some(actor) = self.bandits.get(&msg.bandit_id).cloned() {
            Box::pin(async move {
                actor
                    .send(Reset)
                    .await
                    .map_err(|_| SupervisorError::BanditNotAvailable(msg.bandit_id).into())
            })
        } else {
            Box::pin(async move { Err(SupervisorError::BanditNotFound(msg.bandit_id).into()) })
        }
    }
}

impl Handler<AddArmBandit> for Supervisor {
    type Result = ResponseFuture<Result<usize, SupervisorOrBanditError>>;

    fn handle(&mut self, msg: AddArmBandit, _: &mut Context<Self>) -> Self::Result {
        if let Some(actor) = self.bandits.get(&msg.bandit_id).cloned() {
            Box::pin(async move {
                actor
                    .send(AddArm)
                    .await
                    .map_err(|_| SupervisorError::BanditNotAvailable(msg.bandit_id).into())
            })
        } else {
            Box::pin(async move { Err(SupervisorError::BanditNotFound(msg.bandit_id).into()) })
        }
    }
}

impl Handler<DeleteArmBandit> for Supervisor {
    type Result = ResponseFuture<Result<(), SupervisorOrBanditError>>;

    fn handle(&mut self, msg: DeleteArmBandit, _: &mut Context<Self>) -> Self::Result {
        if let Some(actor) = self.bandits.get(&msg.bandit_id).cloned() {
            Box::pin(async move {
                actor
                    .send(DeleteArm { arm_id: msg.arm_id })
                    .await
                    .map_err(|_| {
                        SupervisorOrBanditError::from(SupervisorError::BanditNotAvailable(
                            msg.bandit_id,
                        ))
                    })?
                    .map_err(SupervisorOrBanditError::from)
            })
        } else {
            Box::pin(async move { Err(SupervisorError::BanditNotFound(msg.bandit_id).into()) })
        }
    }
}

impl Handler<DrawBandit> for Supervisor {
    type Result = ResponseFuture<Result<usize, SupervisorOrBanditError>>;

    fn handle(&mut self, msg: DrawBandit, _: &mut Context<Self>) -> Self::Result {
        if let Some(actor) = self.bandits.get(&msg.bandit_id).cloned() {
            Box::pin(async move {
                actor
                    .send(Draw)
                    .await
                    .map_err(|_| {
                        SupervisorOrBanditError::from(SupervisorError::BanditNotAvailable(
                            msg.bandit_id,
                        ))
                    })?
                    .map_err(SupervisorOrBanditError::from)
            })
        } else {
            Box::pin(async move { Err(SupervisorError::BanditNotFound(msg.bandit_id).into()) })
        }
    }
}

impl Handler<UpdateBandit> for Supervisor {
    type Result = ResponseFuture<Result<(), SupervisorOrBanditError>>;

    fn handle(&mut self, msg: UpdateBandit, _: &mut Context<Self>) -> Self::Result {
        if let Some(actor) = self.bandits.get(&msg.bandit_id).cloned() {
            Box::pin(async move {
                actor
                    .send(Update {
                        arm_id: msg.arm_id,
                        reward: msg.reward,
                    })
                    .await
                    .map_err(|_| {
                        SupervisorOrBanditError::from(SupervisorError::BanditNotAvailable(
                            msg.bandit_id,
                        ))
                    })?
                    .map_err(SupervisorOrBanditError::from)
            })
        } else {
            Box::pin(async move { Err(SupervisorError::BanditNotFound(msg.bandit_id).into()) })
        }
    }
}

impl Handler<UpdateBatchBandit> for Supervisor {
    type Result = ResponseFuture<Result<(), SupervisorOrBanditError>>;

    fn handle(&mut self, msg: UpdateBatchBandit, _: &mut Context<Self>) -> Self::Result {
        if let Some(actor) = self.bandits.get(&msg.bandit_id).cloned() {
            Box::pin(async move {
                actor
                    .send(UpdateBatch {
                        updates: msg.updates,
                    })
                    .await
                    .map_err(|_| {
                        SupervisorOrBanditError::from(SupervisorError::BanditNotAvailable(
                            msg.bandit_id,
                        ))
                    })?
                    .map_err(SupervisorOrBanditError::from)
            })
        } else {
            Box::pin(async move { Err(SupervisorError::BanditNotFound(msg.bandit_id).into()) })
        }
    }
}

impl Handler<GetBanditStats> for Supervisor {
    type Result = ResponseFuture<Result<PolicyStats, SupervisorOrBanditError>>;

    fn handle(&mut self, msg: GetBanditStats, _: &mut Context<Self>) -> Self::Result {
        if let Some(actor) = self.bandits.get(&msg.bandit_id).cloned() {
            Box::pin(async move {
                let stats = actor.send(GetStats).await.map_err(|_| {
                    SupervisorOrBanditError::from(SupervisorError::BanditNotAvailable(
                        msg.bandit_id,
                    ))
                })?;

                Ok(stats)
            })
        } else {
            Box::pin(async move { Err(SupervisorError::BanditNotFound(msg.bandit_id).into()) })
        }
    }
}

impl Handler<BanditCrashed> for Supervisor {
    type Result = ();

    fn handle(&mut self, msg: BanditCrashed, _: &mut Context<Self>) -> Self::Result {
        let _bandit_id = msg.bandit_id;
        todo!("load from storage");
        //let actor = Bandit::new(msg.bandit_id, policy, ctx.address()).start();
        //self.bandits.insert(msg.bandit_id, actor);
    }
}
