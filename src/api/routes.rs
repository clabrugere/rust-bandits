use super::errors::ApiResponseError;
use super::requests::{UpdateBatchPayload, UpdatePayload};
use super::responses::{AddArmResponse, CreateResponse, DrawResponse, ListBanditsResponse};

use crate::actors::supervisor::{
    AddArmBandit, Clear, CreateBandit, DeleteArmBandit, DeleteBandit, DrawBandit, GetBanditStats,
    ListBandits, ResetBandit, Supervisor, UpdateBandit, UpdateBatchBandit,
};
use crate::policies::PolicyType;

use actix::prelude::*;
use actix_web::{
    get, post,
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
        .send(ListBandits)
        .await
        .map(|bandit_ids| Json(ListBanditsResponse { bandit_ids }))
        .map_err(|_| ApiResponseError::InternalError)?;

    Ok(response)
}

#[post("clear")]
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
        .send(CreateBandit {
            bandit_id: None,
            policy_type,
        })
        .await
        .map(|bandit_id| Json(CreateResponse { bandit_id }))
        .map_err(|_| ApiResponseError::InternalError)?;

    Ok(response)
}

#[post("{bandit_id}/reset")]
async fn reset(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
) -> Result<impl Responder> {
    let bandit_id =
        Uuid::try_parse(&bandit_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let response = supervisor
        .send(ResetBandit { bandit_id })
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(|_| ApiResponseError::InternalError)?;

    Ok(response)
}

#[post("{bandit_id}/delete")]
async fn delete(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
) -> Result<impl Responder> {
    let bandit_id =
        Uuid::try_parse(&bandit_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let response = supervisor
        .send(DeleteBandit { bandit_id })
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(|_| ApiResponseError::InternalError)?;

    Ok(response)
}

#[post("{bandit_id}/add_arm")]
async fn add_arm(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
) -> Result<impl Responder> {
    let bandit_id =
        Uuid::try_parse(&bandit_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let response = supervisor
        .send(AddArmBandit { bandit_id })
        .await
        .map_err(|_| ApiResponseError::InternalError)?
        .map(|arm_id| Json(AddArmResponse { arm_id }))
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(response)
}

#[post("{bandit_id}/delete_arm/{arm_id}")]
async fn delete_arm(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
    arm_id: Path<usize>,
) -> Result<impl Responder> {
    let bandit_id =
        Uuid::try_parse(&bandit_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let arm_id = arm_id.into_inner();
    let response = supervisor
        .send(DeleteArmBandit { bandit_id, arm_id })
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(|_| ApiResponseError::InternalError)?;

    Ok(response)
}

#[get("{bandit_id}/draw")]
async fn draw(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
) -> Result<impl Responder> {
    let bandit_id =
        Uuid::try_parse(&bandit_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let response = supervisor
        .send(DrawBandit { bandit_id })
        .await
        .map_err(|_| ApiResponseError::InternalError)?
        .map(|arm_id| Json(DrawResponse { arm_id }))
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(response)
}

#[post("{bandit_id}/update")]
async fn update(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
    payload: Json<UpdatePayload>,
) -> Result<impl Responder> {
    let bandit_id =
        Uuid::try_parse(&bandit_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let UpdatePayload { arm_id, reward, .. } = payload.into_inner();
    let response = supervisor
        .send(UpdateBandit {
            bandit_id,
            arm_id,
            reward,
        })
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(|_| ApiResponseError::InternalError)?;

    Ok(response)
}

#[post("{bandit_id}/update_batch")]
async fn update_batch(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
    payload: Json<UpdateBatchPayload>,
) -> Result<impl Responder> {
    let bandit_id =
        Uuid::try_parse(&bandit_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let UpdateBatchPayload { updates } = payload.into_inner();
    let response = supervisor
        .send(UpdateBatchBandit {
            bandit_id,
            updates: updates.iter().map(|u| (u.ts, u.arm_id, u.reward)).collect(),
        })
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(|_| ApiResponseError::InternalError)?;

    Ok(response)
}

#[get("{bandit_id}/stats")]
async fn stats(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
) -> Result<impl Responder> {
    let bandit_id =
        Uuid::try_parse(&bandit_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let response = supervisor
        .send(GetBanditStats { bandit_id })
        .await
        .map_err(|_| ApiResponseError::InternalError)?
        .map(Json)
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(response)
}
