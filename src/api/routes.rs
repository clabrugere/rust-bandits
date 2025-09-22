use super::errors::ApiResponseError;
use super::requests::{AddArmPayload, UpdateBatchPayload, UpdatePayload};
use super::responses::{
    AddExperimentArmResponse, CreateExperimentResponse, DrawResponse, ListExperimentsResponse,
};

use crate::actors::supervisor::{
    AddExperimentArm, Clear, CreateExperiment, DeleteExperiment, DeleteExperimentArm,
    DrawExperiment, GetExperimentStats, ListExperiments, ResetExperiment, Supervisor,
    UpdateBatchExperiment, UpdateExperiment,
};
use crate::policies::{DrawResult, PolicyType};

use actix::prelude::*;
use actix_web::{
    delete, get, post, put,
    web::{Data, Json, Path},
    HttpResponse, Responder, Result,
};
use uuid::Uuid;

#[get("ping")]
async fn ping() -> Result<impl Responder> {
    Ok(HttpResponse::Ok().finish())
}

#[get("list")]
async fn list(supervisor: Data<Addr<Supervisor>>) -> Result<impl Responder> {
    let response = supervisor
        .send(ListExperiments)
        .await
        .map(|experiment_ids| Json(ListExperimentsResponse { experiment_ids }))
        .map_err(|_| ApiResponseError::InternalError)?;

    Ok(response)
}

#[delete("clear")]
async fn clear(supervisor: Data<Addr<Supervisor>>) -> Result<impl Responder> {
    let response = supervisor
        .send(Clear)
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(|_| ApiResponseError::InternalError)?;

    Ok(response)
}

#[post("create")]
async fn create(
    supervisor: Data<Addr<Supervisor>>,
    policy_type: Json<PolicyType>,
) -> Result<impl Responder> {
    let policy_type = policy_type.into_inner();
    let response = supervisor
        .send(CreateExperiment {
            experiment_id: None,
            policy_type,
        })
        .await
        .map(|experiment_id| Json(CreateExperimentResponse { experiment_id }))
        .map_err(|_| ApiResponseError::InternalError)?;

    Ok(response)
}

#[put("{experiment_id}/reset")]
async fn reset(
    supervisor: Data<Addr<Supervisor>>,
    experiment_id: Path<String>,
) -> Result<impl Responder> {
    let experiment_id =
        Uuid::try_parse(&experiment_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let response = supervisor
        .send(ResetExperiment { experiment_id })
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(|_| ApiResponseError::InternalError)?;

    Ok(response)
}

#[delete("{experiment_id}/delete")]
async fn delete(
    supervisor: Data<Addr<Supervisor>>,
    experiment_id: Path<String>,
) -> Result<impl Responder> {
    let experiment_id =
        Uuid::try_parse(&experiment_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let response = supervisor
        .send(DeleteExperiment { experiment_id })
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(|_| ApiResponseError::InternalError)?;

    Ok(response)
}

#[post("{experiment_id}/add_arm")]
async fn add_arm(
    supervisor: Data<Addr<Supervisor>>,
    experiment_id: Path<String>,
    initial_state: Json<AddArmPayload>,
) -> Result<impl Responder> {
    let experiment_id =
        Uuid::try_parse(&experiment_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let AddArmPayload {
        initial_reward,
        initial_count,
    } = initial_state.into_inner();
    let response = supervisor
        .send(AddExperimentArm {
            experiment_id,
            initial_reward,
            initial_count,
        })
        .await
        .map_err(|_| ApiResponseError::InternalError)?
        .map(|arm_id| Json(AddExperimentArmResponse { arm_id }))
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(response)
}

#[delete("{experiment_id}/delete_arm/{arm_id}")]
async fn delete_arm(
    supervisor: Data<Addr<Supervisor>>,
    experiment_id: Path<String>,
    arm_id: Path<usize>,
) -> Result<impl Responder> {
    let experiment_id =
        Uuid::try_parse(&experiment_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let arm_id = arm_id.into_inner();
    let response = supervisor
        .send(DeleteExperimentArm {
            experiment_id,
            arm_id,
        })
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(|_| ApiResponseError::InternalError)?;

    Ok(response)
}

#[get("{experiment_id}/draw")]
async fn draw(
    supervisor: Data<Addr<Supervisor>>,
    experiment_id: Path<String>,
) -> Result<impl Responder> {
    let experiment_id =
        Uuid::try_parse(&experiment_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let response = supervisor
        .send(DrawExperiment { experiment_id })
        .await
        .map_err(|_| ApiResponseError::InternalError)?
        .map(
            |DrawResult {
                 timestamp,
                 draw_id,
                 arm_id,
             }| {
                Json(DrawResponse {
                    timestamp,
                    draw_id,
                    arm_id,
                })
            },
        )
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(response)
}

#[put("{experiment_id}/update")]
async fn update(
    supervisor: Data<Addr<Supervisor>>,
    experiment_id: Path<String>,
    payload: Json<UpdatePayload>,
) -> Result<impl Responder> {
    let experiment_id =
        Uuid::try_parse(&experiment_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let UpdatePayload {
        draw_id,
        timestamp,
        arm_id,
        reward,
    } = payload.into_inner();
    let response = supervisor
        .send(UpdateExperiment {
            experiment_id,
            draw_id,
            timestamp,
            arm_id,
            reward,
        })
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(|_| ApiResponseError::InternalError)?;

    Ok(response)
}

#[put("{experiment_id}/update_batch")]
async fn update_batch(
    supervisor: Data<Addr<Supervisor>>,
    experiment_id: Path<String>,
    payload: Json<UpdateBatchPayload>,
) -> Result<impl Responder> {
    let experiment_id =
        Uuid::try_parse(&experiment_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let UpdateBatchPayload { updates } = payload.into_inner();
    let response = supervisor
        .send(UpdateBatchExperiment {
            experiment_id,
            updates: updates
                .iter()
                .map(|u| (u.draw_id, u.timestamp, u.arm_id, u.reward))
                .collect(),
        })
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(|_| ApiResponseError::InternalError)?;

    Ok(response)
}

#[get("{experiment_id}/stats")]
async fn stats(
    supervisor: Data<Addr<Supervisor>>,
    experiment_id: Path<String>,
) -> Result<impl Responder> {
    let experiment_id =
        Uuid::try_parse(&experiment_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let response = supervisor
        .send(GetExperimentStats { experiment_id })
        .await
        .map_err(|_| ApiResponseError::InternalError)?
        .map(Json)
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(response)
}
