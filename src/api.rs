use crate::policies::bandit::BanditType;
use crate::supervisor::{
    AddArmBandit, CreateBandit, DeleteArmBandit, DeleteBandit, DrawBandit, GetBanditStats,
    ListBandits, ResetBandit, Supervisor, UpdateBandit,
};
use actix::prelude::*;
use actix_web::{
    error::{ErrorBadRequest, ErrorInternalServerError},
    get, post,
    web::{Data, Json, Path},
    Responder, Result,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
struct ListBanditsResponse {
    bandit_ids: Vec<Uuid>,
}

#[derive(Serialize)]
struct CreateResponse {
    bandit_id: Uuid,
}

#[derive(Serialize)]
struct ResetResponse {
    bandit_id: Uuid,
}

#[derive(Serialize)]
struct DeleteResponse {
    bandit_id: Uuid,
}

#[derive(Serialize)]
struct AddArmResponse {
    bandit_id: Uuid,
    arm_id: usize,
}

#[derive(Serialize)]
struct DeleteArmResponse {
    bandit_id: Uuid,
    arm_id: usize,
}

#[derive(Serialize)]
struct DrawResponse {
    bandit_id: Uuid,
    arm_id: usize,
}

#[derive(Serialize)]
struct UpdateResponse {
    bandit_id: Uuid,
    arm_id: usize,
    reward: f64,
}

#[derive(Debug, Deserialize)]
struct UpdatePayload {
    arm_id: usize,
    reward: f64,
}

#[get("bandit/list")]
async fn list_bandits(supervisor: Data<Addr<Supervisor>>) -> Result<impl Responder> {
    let bandit_ids = supervisor
        .send(ListBandits)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(Json(ListBanditsResponse { bandit_ids }))
}

#[post("bandit/create")]
async fn create_bandit(
    supervisor: Data<Addr<Supervisor>>,
    bandit_type: Json<BanditType>,
) -> Result<impl Responder> {
    let bandit_type = bandit_type.into_inner();
    let bandit_id = supervisor
        .send(CreateBandit { bandit_type })
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(Json(CreateResponse { bandit_id }))
}

#[post("bandit/{bandit_id}/reset")]
async fn reset_bandit(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
) -> Result<impl Responder> {
    let bandit_id = Uuid::try_parse(&bandit_id.into_inner()).map_err(ErrorBadRequest)?;
    supervisor
        .send(ResetBandit { bandit_id })
        .await
        .map_err(ErrorInternalServerError)?
        .map_err(ErrorBadRequest)?;

    Ok(Json(ResetResponse { bandit_id }))
}

#[post("bandit/{bandit_id}/delete")]
async fn delete_bandit(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
) -> Result<impl Responder> {
    let bandit_id = Uuid::try_parse(&bandit_id.into_inner()).map_err(ErrorBadRequest)?;
    supervisor
        .send(DeleteBandit { bandit_id })
        .await
        .map_err(ErrorInternalServerError)?
        .map_err(ErrorBadRequest)?;

    Ok(Json(DeleteResponse { bandit_id }))
}

#[post("bandit/{bandit_id}/add_arm")]
async fn add_arm_bandit(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
) -> Result<impl Responder> {
    let bandit_id = Uuid::try_parse(&bandit_id.into_inner()).map_err(ErrorBadRequest)?;
    let arm_id = supervisor
        .send(AddArmBandit { bandit_id })
        .await
        .map_err(ErrorInternalServerError)?
        .map_err(ErrorBadRequest)?;

    Ok(Json(AddArmResponse { bandit_id, arm_id }))
}

#[post("bandit/{bandit_id}/delete_arm/{arm_id}")]
async fn delete_arm_bandit(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
    arm_id: Path<usize>,
) -> Result<impl Responder> {
    let bandit_id = Uuid::try_parse(&bandit_id.into_inner()).map_err(ErrorBadRequest)?;
    let arm_id = arm_id.into_inner();
    supervisor
        .send(DeleteArmBandit { bandit_id, arm_id })
        .await
        .map_err(ErrorInternalServerError)?
        .map_err(ErrorBadRequest)?;

    Ok(Json(DeleteArmResponse { bandit_id, arm_id }))
}

#[get("bandit/{bandit_id}/draw")]
async fn draw_bandit(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
) -> Result<impl Responder> {
    let bandit_id = Uuid::try_parse(&bandit_id.into_inner()).map_err(ErrorBadRequest)?;
    let arm_id = supervisor
        .send(DrawBandit { bandit_id })
        .await
        .map_err(ErrorInternalServerError)?
        .map_err(ErrorBadRequest)?;

    Ok(Json(DrawResponse { bandit_id, arm_id }))
}

#[post("bandit/{bandit_id}/update")]
async fn update_bandit(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
    payload: Json<UpdatePayload>,
) -> Result<impl Responder> {
    let bandit_id = Uuid::try_parse(&bandit_id).map_err(ErrorBadRequest)?;
    let UpdatePayload { arm_id, reward } = payload.into_inner();
    supervisor
        .send(UpdateBandit {
            bandit_id,
            arm_id,
            reward,
        })
        .await
        .map_err(ErrorInternalServerError)?
        .map_err(ErrorBadRequest)?;

    Ok(Json(UpdateResponse {
        bandit_id,
        arm_id,
        reward,
    }))
}

#[get("bandit/{bandit_id}/stats")]
async fn bandit_stats(
    supervisor: Data<Addr<Supervisor>>,
    bandit_id: Path<String>,
) -> Result<impl Responder> {
    let bandit_id = Uuid::try_parse(&bandit_id.into_inner()).map_err(ErrorBadRequest)?;
    let stats = supervisor
        .send(GetBanditStats { bandit_id })
        .await
        .map_err(ErrorInternalServerError)?
        .map_err(ErrorBadRequest)?;

    Ok(Json(stats))
}
