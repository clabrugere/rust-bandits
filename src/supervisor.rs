use crate::{
    actor::{AddArm, BanditActor, DeleteArm, Draw, GetStats, Reset, Update},
    policies::bandit::{create_bandit, BanditError, BanditStats, BanditType},
};
use actix::prelude::*;
use std::{collections::HashMap, error::Error, fmt};
use uuid::Uuid;

#[derive(Debug)]
pub enum SupervisorError {
    BanditNotAvailable(Uuid),
    BanditNotFound(Uuid),
}

impl Error for SupervisorError {}

impl fmt::Display for SupervisorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SupervisorError::BanditNotAvailable(bandit_id) => {
                write!(f, "Bandit {} not available", bandit_id)
            }
            SupervisorError::BanditNotFound(bandit_id) => {
                write!(f, "Bandit {} not found", bandit_id)
            }
        }
    }
}

#[derive(Debug)]
pub enum SupervisorOrBanditError {
    Supervisor(SupervisorError),
    Bandit(BanditError),
}

impl fmt::Display for SupervisorOrBanditError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SupervisorOrBanditError::Supervisor(err) => write!(f, "Supervisor error: {}", err),
            SupervisorOrBanditError::Bandit(err) => write!(f, "Bandit error: {}", err),
        }
    }
}

impl From<SupervisorError> for SupervisorOrBanditError {
    fn from(err: SupervisorError) -> Self {
        SupervisorOrBanditError::Supervisor(err)
    }
}

impl From<BanditError> for SupervisorOrBanditError {
    fn from(err: BanditError) -> Self {
        SupervisorOrBanditError::Bandit(err)
    }
}

pub struct Supervisor {
    bandits: HashMap<Uuid, Addr<BanditActor>>,
}

impl Supervisor {
    pub fn new() -> Self {
        Self {
            bandits: HashMap::new(),
        }
    }

    pub fn create_bandit(&mut self, bandit_type: BanditType) -> Uuid {
        let bandit_id = Uuid::new_v4();
        let bandit = create_bandit(bandit_type);
        let actor = BanditActor::new(bandit).start();

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
    pub bandit_type: BanditType,
}

#[derive(Message)]
#[rtype(result = "Result<(), SupervisorError>")]
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
#[rtype(result = "Result<BanditStats, SupervisorOrBanditError>")]
pub struct GetBanditStats {
    pub bandit_id: Uuid,
}

impl Handler<ListBandits> for Supervisor {
    type Result = MessageResult<ListBandits>;

    fn handle(&mut self, _: ListBandits, _: &mut Self::Context) -> Self::Result {
        MessageResult(self.bandits.keys().cloned().collect())
    }
}

impl Handler<CreateBandit> for Supervisor {
    type Result = MessageResult<CreateBandit>;

    fn handle(&mut self, msg: CreateBandit, _: &mut Self::Context) -> Self::Result {
        MessageResult(self.create_bandit(msg.bandit_type))
    }
}

impl Handler<DeleteBandit> for Supervisor {
    type Result = Result<(), SupervisorError>;

    fn handle(&mut self, msg: DeleteBandit, _: &mut Self::Context) -> Self::Result {
        self.delete_bandit(&msg.bandit_id)
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

impl Handler<GetBanditStats> for Supervisor {
    type Result = ResponseFuture<Result<BanditStats, SupervisorOrBanditError>>;

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
