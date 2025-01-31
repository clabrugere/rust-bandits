use super::errors::ApiResponseError;
use crate::policy::PolicyType;
use crate::supervisor::{
    AddArmBandit, CreateBandit, DeleteArmBandit, DeleteBandit, DrawBandit, GetBanditStats,
    ListBandits, ResetBandit, Supervisor, UpdateBandit, UpdateBatchBandit,
};
use actix::prelude::*;
use actix_web::{
    get, post,
    web::{Data, Json, Path},
    Responder, Result,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Serialize)]
pub struct ApiResponse<T> {
    request_id: Uuid,
    ts: u128,
    body: Option<T>,
}

impl<T> Default for ApiResponse<T> {
    fn default() -> Self {
        Self {
            request_id: Uuid::new_v4(),
            ts: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            body: None,
        }
    }
}

impl<T: Serialize> ApiResponse<T> {
    pub fn with_data(data: Option<T>) -> Self {
        Self {
            request_id: Uuid::new_v4(),
            ts: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            body: data,
        }
    }
}

#[derive(Serialize)]
struct ListBanditsResponse {
    bandit_ids: Vec<Uuid>,
}

#[derive(Serialize)]
struct CreateResponse {
    bandit_id: Uuid,
}

#[derive(Serialize)]
struct AddArmResponse {
    arm_id: usize,
}

#[derive(Serialize)]
struct DrawResponse {
    arm_id: usize,
}

#[derive(Debug, Deserialize)]
struct UpdatePayload {
    arm_id: usize,
    reward: f64,
}

#[derive(Debug, Deserialize)]
struct UpdateBatchPayload {
    updates: Vec<(usize, usize, f64)>,
}

#[get("bandit/list")]
async fn list_bandits(supervisor: Data<Addr<Supervisor>>) -> Result<impl Responder> {
    let bandit_ids = supervisor
        .send(ListBandits)
        .await
        .map_err(|_| ApiResponseError::InternalError)?;

    let response = ApiResponse::with_data(Some(ListBanditsResponse { bandit_ids }));

    Ok(Json(response))
}

#[post("bandit/create")]
async fn create_bandit(
    supervisor: Data<Addr<Supervisor>>,
    bandit_type: Json<PolicyType>,
) -> Result<impl Responder> {
    let bandit_type = bandit_type.into_inner();
    let bandit_id = supervisor
        .send(CreateBandit { bandit_type })
        .await
        .map_err(|_| ApiResponseError::InternalError)?;

    let response = ApiResponse::with_data(Some(CreateResponse { bandit_id }));

    Ok(Json(response))
}

#[post("bandit/{bandit_id}/reset")]
async fn reset_bandit(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
) -> Result<impl Responder> {
    let bandit_id =
        Uuid::try_parse(&bandit_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    supervisor
        .send(ResetBandit { bandit_id })
        .await
        .map_err(|_| ApiResponseError::InternalError)?
        .map_err(ApiResponseError::ErrorBadRequest)?;

    let response = ApiResponse::<()>::default();

    Ok(Json(response))
}

#[post("bandit/{bandit_id}/delete")]
async fn delete_bandit(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
) -> Result<impl Responder> {
    let bandit_id =
        Uuid::try_parse(&bandit_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    supervisor
        .send(DeleteBandit { bandit_id })
        .await
        .map_err(|_| ApiResponseError::InternalError)?
        .map_err(ApiResponseError::ErrorBadRequest)?;

    let response = ApiResponse::<()>::default();

    Ok(Json(response))
}

#[post("bandit/{bandit_id}/add_arm")]
async fn add_arm_bandit(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
) -> Result<impl Responder> {
    let bandit_id =
        Uuid::try_parse(&bandit_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let arm_id = supervisor
        .send(AddArmBandit { bandit_id })
        .await
        .map_err(|_| ApiResponseError::InternalError)?
        .map_err(ApiResponseError::ErrorBadRequest)?;

    let response = ApiResponse::with_data(Some(AddArmResponse { arm_id }));

    Ok(Json(response))
}

#[post("bandit/{bandit_id}/delete_arm/{arm_id}")]
async fn delete_arm_bandit(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
    arm_id: Path<usize>,
) -> Result<impl Responder> {
    let bandit_id =
        Uuid::try_parse(&bandit_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let arm_id = arm_id.into_inner();
    supervisor
        .send(DeleteArmBandit { bandit_id, arm_id })
        .await
        .map_err(|_| ApiResponseError::InternalError)?
        .map_err(ApiResponseError::ErrorBadRequest)?;

    let response = ApiResponse::<()>::default();

    Ok(Json(response))
}

#[get("bandit/{bandit_id}/draw")]
async fn draw_bandit(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
) -> Result<impl Responder> {
    let bandit_id =
        Uuid::try_parse(&bandit_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let arm_id = supervisor
        .send(DrawBandit { bandit_id })
        .await
        .map_err(|_| ApiResponseError::InternalError)?
        .map_err(ApiResponseError::ErrorBadRequest)?;

    let response = ApiResponse::with_data(Some(DrawResponse { arm_id }));

    Ok(Json(response))
}

#[post("bandit/{bandit_id}/update")]
async fn update_bandit(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
    payload: Json<UpdatePayload>,
) -> Result<impl Responder> {
    let bandit_id = Uuid::try_parse(&bandit_id).map_err(ApiResponseError::ErrorBadUuid)?;
    let UpdatePayload { arm_id, reward } = payload.into_inner();
    supervisor
        .send(UpdateBandit {
            bandit_id,
            arm_id,
            reward,
        })
        .await
        .map_err(|_| ApiResponseError::InternalError)?
        .map_err(ApiResponseError::ErrorBadRequest)?;

    let response = ApiResponse::<()>::default();

    Ok(Json(response))
}

#[post("bandit/{bandit_id}/update_batch")]
async fn update_batch_bandit(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
    payload: Json<UpdateBatchPayload>,
) -> Result<impl Responder> {
    let bandit_id = Uuid::try_parse(&bandit_id).map_err(ApiResponseError::ErrorBadUuid)?;
    let UpdateBatchPayload { updates } = payload.into_inner();
    supervisor
        .send(UpdateBatchBandit { bandit_id, updates })
        .await
        .map_err(|_| ApiResponseError::InternalError)?
        .map_err(ApiResponseError::ErrorBadRequest)?;

    let response = ApiResponse::<()>::default();

    Ok(Json(response))
}

#[get("bandit/{bandit_id}/stats")]
async fn bandit_stats(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
) -> Result<impl Responder> {
    let bandit_id =
        Uuid::try_parse(&bandit_id.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let stats = supervisor
        .send(GetBanditStats { bandit_id })
        .await
        .map_err(|_| ApiResponseError::InternalError)?
        .map_err(ApiResponseError::ErrorBadRequest)?;

    let response = ApiResponse::with_data(Some(stats));

    Ok(Json(response))
}
