use super::bandit::{
    AddArm, Bandit, DeleteArm, Draw, GetStats, Ping, Pong, Reset, Update, UpdateBatch,
};
use super::errors::{SupervisorError, SupervisorOrBanditError};
use super::policy_cache::{PolicyCache, ReadFullPolicyCache, RemovePolicyCache};

use crate::config::{BanditConfig, SupervisorConfig};
use crate::policies::{Policy, PolicyStats, PolicyType};

use actix::prelude::*;
use futures_util::future::join_all;
use log::{info, warn};
use std::{collections::HashMap, time::Duration};
use uuid::Uuid;

pub struct Supervisor {
    config: SupervisorConfig,
    bandits: HashMap<Uuid, Addr<Bandit>>,
    bandit_config: BanditConfig,
    cache: Addr<PolicyCache>,
}

impl Supervisor {
    pub fn new(
        config: SupervisorConfig,
        bandit_config: BanditConfig,
        cache: Addr<PolicyCache>,
    ) -> Self {
        Self {
            config,
            bandits: HashMap::new(),
            bandit_config,
            cache,
        }
    }

    fn initialize_from_storage(&self, ctx: &mut Context<Self>) {
        self.cache
            .send(ReadFullPolicyCache)
            .into_actor(self)
            .then(|storage, supervisor, _| {
                match storage {
                    Ok(policy_states) => {
                        policy_states.iter().for_each(|(bandit_id, policy)| {
                            supervisor.create_bandit(Some(*bandit_id), policy.clone_box());
                        });
                    }
                    Err(err) => warn!("Storage not available: {}", err),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    pub fn create_bandit(
        &mut self,
        bandit_id: Option<Uuid>,
        policy: Box<dyn Policy + Send>,
    ) -> Uuid {
        let bandit_id = bandit_id.unwrap_or(Uuid::new_v4());
        let actor = Bandit::new(
            bandit_id,
            policy,
            self.cache.clone(),
            self.bandit_config.cache_every,
        )
        .start();
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

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Starting supervisor");
        self.initialize_from_storage(ctx);
        ctx.run_interval(Duration::from_secs(self.config.ping_every), |_, ctx| {
            ctx.address().do_send(PingBandits)
        });
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct PingBandits;

#[derive(Message)]
#[rtype(result = "Vec<Uuid>")]
pub struct ListBandits;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Clear;

#[derive(Message)]
#[rtype(result = "Uuid")]
pub struct CreateBandit {
    pub bandit_id: Option<Uuid>,
    pub policy_type: PolicyType,
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
    pub updates: Vec<(u64, usize, f64)>,
}

#[derive(Message)]
#[rtype(result = "Result<PolicyStats, SupervisorOrBanditError>")]
pub struct GetBanditStats {
    pub bandit_id: Uuid,
}

impl Handler<PingBandits> for Supervisor {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, _: PingBandits, _: &mut Self::Context) -> Self::Result {
        info!("Checking bandits health");
        let bandits = self.bandits.clone();

        Box::pin(async move {
            let futures = bandits.iter().map(|(&bandit_id, address)| async move {
                let future = address.send(Ping).await;
                (bandit_id, future)
            });

            join_all(futures)
                .await
                .iter()
                .for_each(|(bandit_id, result)| match result {
                    Ok(Pong) => (),
                    Err(err) => {
                        warn!("Bandit {} not available: {}", bandit_id, err);
                        // TODO: message to Supervisor to restart the bandit
                    }
                });
        })
    }
}

impl Handler<Clear> for Supervisor {
    type Result = ();

    fn handle(&mut self, _: Clear, _: &mut Self::Context) -> Self::Result {
        self.bandits.clear();
    }
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
        MessageResult(self.create_bandit(msg.bandit_id, msg.policy_type.into_inner()))
    }
}

impl Handler<DeleteBandit> for Supervisor {
    type Result = Result<(), SupervisorOrBanditError>;

    fn handle(&mut self, msg: DeleteBandit, _: &mut Self::Context) -> Self::Result {
        self.delete_bandit(&msg.bandit_id)
            .map_err(SupervisorOrBanditError::from)
            .map(|_| {
                self.cache.do_send(RemovePolicyCache {
                    bandit_id: msg.bandit_id,
                });
            })
    }
}

impl Handler<ResetBandit> for Supervisor {
    type Result = ResponseFuture<Result<(), SupervisorOrBanditError>>;

    fn handle(&mut self, msg: ResetBandit, _: &mut Self::Context) -> Self::Result {
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

    fn handle(&mut self, msg: AddArmBandit, _: &mut Self::Context) -> Self::Result {
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

    fn handle(&mut self, msg: DeleteArmBandit, _: &mut Self::Context) -> Self::Result {
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

    fn handle(&mut self, msg: DrawBandit, _: &mut Self::Context) -> Self::Result {
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

    fn handle(&mut self, msg: UpdateBandit, _: &mut Self::Context) -> Self::Result {
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

    fn handle(&mut self, msg: UpdateBatchBandit, _: &mut Self::Context) -> Self::Result {
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

    fn handle(&mut self, msg: GetBanditStats, _: &mut Self::Context) -> Self::Result {
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
