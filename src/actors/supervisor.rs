use super::errors::{SupervisorError, SupervisorOrExperimentError};
use super::experiment::{
    AddArm, DeleteArm, Draw, Experiment, GetStats, Ping, Pong, Reset, Update, UpdateBatch,
};
use super::experiment_cache::{ExperimentCache, ReadFullExperimentCache, RemoveExperimentCache};

use crate::actors::experiment_cache::ReadExperimentCache;
use crate::config::{ExperimentConfig, SupervisorConfig};
use crate::policies::{DrawResult, Policy, PolicyStats, PolicyType};

use actix::prelude::*;
use futures_util::future::join_all;
use log::{info, warn};
use std::{collections::HashMap, time::Duration};
use uuid::Uuid;

pub struct Supervisor {
    config: SupervisorConfig,
    experiments: HashMap<Uuid, Addr<Experiment>>,
    experiment_config: ExperimentConfig,
    cache: Addr<ExperimentCache>,
}

impl Supervisor {
    pub fn new(
        config: SupervisorConfig,
        experiment_config: ExperimentConfig,
        cache: Addr<ExperimentCache>,
    ) -> Self {
        Self {
            config,
            experiments: HashMap::new(),
            experiment_config,
            cache,
        }
    }

    fn initialize_from_storage(&self, ctx: &mut Context<Self>) {
        self.cache
            .send(ReadFullExperimentCache)
            .into_actor(self)
            .then(|storage, supervisor, _| {
                match storage {
                    Ok(experiments) => {
                        experiments.iter().for_each(|(&experiment_id, policy)| {
                            supervisor.create_experiment(Some(experiment_id), policy.clone_box());
                        });
                    }
                    Err(err) => warn!("Storage not available: {}", err),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    pub fn create_experiment(
        &mut self,
        experiment_id: Option<Uuid>,
        policy: Box<dyn Policy + Send>,
    ) -> Uuid {
        let experiment_id = experiment_id.unwrap_or(Uuid::new_v4());
        let actor = Experiment::new(
            experiment_id,
            policy,
            self.cache.clone(),
            self.experiment_config.cache_every,
        )
        .start();
        self.experiments.insert(experiment_id, actor);

        experiment_id
    }

    pub fn delete_experiment(&mut self, experiment_id: Uuid) -> Result<(), SupervisorError> {
        if self.experiments.contains_key(&experiment_id) {
            self.experiments.remove(&experiment_id);
            Ok(())
        } else {
            Err(SupervisorError::ExperimentNotFound(experiment_id))
        }
    }
}

impl Actor for Supervisor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Starting supervisor actor");
        self.initialize_from_storage(ctx);
        ctx.run_interval(Duration::from_secs(self.config.ping_every), |_, ctx| {
            ctx.address().do_send(PingExperiments)
        });
    }
}

// Messages
#[derive(Message)]
#[rtype(result = "()")]
struct PingExperiments;

#[derive(Message)]
#[rtype(result = "Vec<Uuid>")]
pub struct ListExperiments;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Clear;

#[derive(Message)]
#[rtype(result = "Uuid")]
pub struct CreateExperiment {
    pub experiment_id: Option<Uuid>,
    pub policy_type: PolicyType,
}

#[derive(Message)]
#[rtype(result = "Result<(), SupervisorOrExperimentError>")]
pub struct DeleteExperiment {
    pub experiment_id: Uuid,
}

#[derive(Message)]
#[rtype(result = "Result<(), SupervisorOrExperimentError>")]
pub struct ResetExperiment {
    pub experiment_id: Uuid,
}

#[derive(Message)]
#[rtype(result = "Result<usize, SupervisorOrExperimentError>")]
pub struct AddExperimentArm {
    pub experiment_id: Uuid,
    pub initial_reward: Option<f64>,
    pub initial_count: Option<u64>,
}

#[derive(Message)]
#[rtype(result = "Result<(), SupervisorOrExperimentError>")]
pub struct DeleteExperimentArm {
    pub experiment_id: Uuid,
    pub arm_id: usize,
}

#[derive(Message)]
#[rtype(result = "Result<DrawResult, SupervisorOrExperimentError>")]
pub struct DrawExperiment {
    pub experiment_id: Uuid,
}

#[derive(Message)]
#[rtype(result = "Result<(), SupervisorOrExperimentError>")]
pub struct UpdateExperiment {
    pub experiment_id: Uuid,
    pub draw_id: Uuid,
    pub timestamp: u128,
    pub arm_id: usize,
    pub reward: f64,
}

#[derive(Message)]
#[rtype(result = "Result<(), SupervisorOrExperimentError>")]
pub struct UpdateBatchExperiment {
    pub experiment_id: Uuid,
    pub updates: Vec<(Uuid, u128, usize, f64)>,
}

#[derive(Message)]
#[rtype(result = "Result<PolicyStats, SupervisorOrExperimentError>")]
pub struct GetExperimentStats {
    pub experiment_id: Uuid,
}

impl Handler<PingExperiments> for Supervisor {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, _: PingExperiments, _: &mut Self::Context) -> Self::Result {
        info!("Checking experiment actors health");
        let experiments = self.experiments.clone();
        let _cache = self.cache.clone();

        Box::pin(async move {
            let futures = experiments
                .iter()
                .map(|(&experiment_id, address)| async move {
                    let future = address.send(Ping).await;
                    (experiment_id, future)
                });

            join_all(futures)
                .await
                .iter()
                .for_each(|(experiment_id, result)| match result {
                    Ok(Pong) => (),
                    Err(err) => {
                        warn!("Experiment {} not available: {}", experiment_id, err);
                        // TODO: message to Supervisor to restart the experiment actor
                    }
                });
        })
    }
}

impl Handler<Clear> for Supervisor {
    type Result = ();

    fn handle(&mut self, _: Clear, _: &mut Self::Context) -> Self::Result {
        self.experiments.clear();
    }
}

impl Handler<ListExperiments> for Supervisor {
    type Result = MessageResult<ListExperiments>;

    fn handle(&mut self, _: ListExperiments, _: &mut Self::Context) -> Self::Result {
        MessageResult(self.experiments.keys().cloned().collect())
    }
}

impl Handler<CreateExperiment> for Supervisor {
    type Result = MessageResult<CreateExperiment>;

    fn handle(&mut self, msg: CreateExperiment, _: &mut Self::Context) -> Self::Result {
        MessageResult(self.create_experiment(msg.experiment_id, msg.policy_type.into_inner()))
    }
}

impl Handler<DeleteExperiment> for Supervisor {
    type Result = Result<(), SupervisorOrExperimentError>;

    fn handle(&mut self, msg: DeleteExperiment, _: &mut Self::Context) -> Self::Result {
        self.delete_experiment(msg.experiment_id)
            .map_err(SupervisorOrExperimentError::from)
            .map(|_| {
                self.cache.do_send(RemoveExperimentCache {
                    experiment_id: msg.experiment_id,
                });
            })
    }
}

impl Handler<ResetExperiment> for Supervisor {
    type Result = ResponseFuture<Result<(), SupervisorOrExperimentError>>;

    fn handle(&mut self, msg: ResetExperiment, _: &mut Self::Context) -> Self::Result {
        if let Some(actor) = self.experiments.get(&msg.experiment_id).cloned() {
            Box::pin(async move {
                actor
                    .send(Reset)
                    .await
                    .map_err(|_| SupervisorError::ExperimentNotAvailable(msg.experiment_id).into())
            })
        } else {
            Box::pin(
                async move { Err(SupervisorError::ExperimentNotFound(msg.experiment_id).into()) },
            )
        }
    }
}

impl Handler<AddExperimentArm> for Supervisor {
    type Result = ResponseFuture<Result<usize, SupervisorOrExperimentError>>;

    fn handle(&mut self, msg: AddExperimentArm, _: &mut Self::Context) -> Self::Result {
        if let Some(actor) = self.experiments.get(&msg.experiment_id).cloned() {
            Box::pin(async move {
                actor
                    .send(AddArm {
                        initial_reward: msg.initial_reward,
                        initial_count: msg.initial_count,
                    })
                    .await
                    .map_err(|_| SupervisorError::ExperimentNotAvailable(msg.experiment_id).into())
            })
        } else {
            Box::pin(
                async move { Err(SupervisorError::ExperimentNotFound(msg.experiment_id).into()) },
            )
        }
    }
}

impl Handler<DeleteExperimentArm> for Supervisor {
    type Result = ResponseFuture<Result<(), SupervisorOrExperimentError>>;

    fn handle(&mut self, msg: DeleteExperimentArm, _: &mut Self::Context) -> Self::Result {
        if let Some(actor) = self.experiments.get(&msg.experiment_id).cloned() {
            Box::pin(async move {
                actor
                    .send(DeleteArm { arm_id: msg.arm_id })
                    .await
                    .map_err(|_| {
                        SupervisorOrExperimentError::from(SupervisorError::ExperimentNotAvailable(
                            msg.experiment_id,
                        ))
                    })?
                    .map_err(SupervisorOrExperimentError::from)
            })
        } else {
            Box::pin(
                async move { Err(SupervisorError::ExperimentNotFound(msg.experiment_id).into()) },
            )
        }
    }
}

impl Handler<DrawExperiment> for Supervisor {
    type Result = ResponseFuture<Result<DrawResult, SupervisorOrExperimentError>>;

    fn handle(&mut self, msg: DrawExperiment, _: &mut Self::Context) -> Self::Result {
        if let Some(actor) = self.experiments.get(&msg.experiment_id).cloned() {
            Box::pin(async move {
                actor
                    .send(Draw)
                    .await
                    .map_err(|_| {
                        SupervisorOrExperimentError::from(SupervisorError::ExperimentNotAvailable(
                            msg.experiment_id,
                        ))
                    })?
                    .map_err(SupervisorOrExperimentError::from)
            })
        } else {
            Box::pin(
                async move { Err(SupervisorError::ExperimentNotFound(msg.experiment_id).into()) },
            )
        }
    }
}

// FIX: doesn't return an error when the experiment is not found?
impl Handler<UpdateExperiment> for Supervisor {
    type Result = ResponseFuture<Result<(), SupervisorOrExperimentError>>;

    fn handle(&mut self, msg: UpdateExperiment, _: &mut Self::Context) -> Self::Result {
        if let Some(actor) = self.experiments.get(&msg.experiment_id).cloned() {
            Box::pin(async move {
                actor
                    .send(Update {
                        draw_id: msg.draw_id,
                        timestamp: msg.timestamp,
                        arm_id: msg.arm_id,
                        reward: msg.reward,
                    })
                    .await
                    .map_err(|_| {
                        SupervisorOrExperimentError::from(SupervisorError::ExperimentNotAvailable(
                            msg.experiment_id,
                        ))
                    })?
                    .map_err(SupervisorOrExperimentError::from)
            })
        } else {
            Box::pin(
                async move { Err(SupervisorError::ExperimentNotFound(msg.experiment_id).into()) },
            )
        }
    }
}

impl Handler<UpdateBatchExperiment> for Supervisor {
    type Result = ResponseFuture<Result<(), SupervisorOrExperimentError>>;

    fn handle(&mut self, msg: UpdateBatchExperiment, _: &mut Self::Context) -> Self::Result {
        if let Some(actor) = self.experiments.get(&msg.experiment_id).cloned() {
            Box::pin(async move {
                actor
                    .send(UpdateBatch {
                        updates: msg.updates,
                    })
                    .await
                    .map_err(|_| {
                        SupervisorOrExperimentError::from(SupervisorError::ExperimentNotAvailable(
                            msg.experiment_id,
                        ))
                    })?
                    .map_err(SupervisorOrExperimentError::from)
            })
        } else {
            Box::pin(
                async move { Err(SupervisorError::ExperimentNotFound(msg.experiment_id).into()) },
            )
        }
    }
}

impl Handler<GetExperimentStats> for Supervisor {
    type Result = ResponseFuture<Result<PolicyStats, SupervisorOrExperimentError>>;

    fn handle(&mut self, msg: GetExperimentStats, _: &mut Self::Context) -> Self::Result {
        if let Some(actor) = self.experiments.get(&msg.experiment_id).cloned() {
            Box::pin(async move {
                let stats = actor.send(GetStats).await.map_err(|_| {
                    SupervisorOrExperimentError::from(SupervisorError::ExperimentNotAvailable(
                        msg.experiment_id,
                    ))
                })?;

                Ok(stats)
            })
        } else {
            Box::pin(
                async move { Err(SupervisorError::ExperimentNotFound(msg.experiment_id).into()) },
            )
        }
    }
}
