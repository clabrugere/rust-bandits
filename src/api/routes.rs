use tokio::sync::RwLock;

use super::errors::ApiResponseError;
use super::requests::{AddArmPayload, UpdateBatchPayload, UpdatePayload};
use super::responses::{
    AddExperimentArmResponse, CreateExperimentResponse, DrawResponse, ListExperimentsResponse,
};
use crate::api::requests::ResetArmPayload;
use crate::errors::RepositoryOrExperimentError;
use crate::policies::PolicyType;
use crate::repository::Repository;

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
async fn list(repository: Data<RwLock<Repository>>) -> Result<impl Responder> {
    let response = repository
        .read()
        .await
        .list_experiments()
        .map(|experiments| Json(ListExperimentsResponse { experiments }))
        .map_err(|_| ApiResponseError::InternalError)?;

    Ok(response)
}

#[delete("clear")]
async fn clear(repository: Data<RwLock<Repository>>) -> Result<impl Responder> {
    repository.write().await.clear();

    Ok(HttpResponse::Ok())
}

#[post("create")]
async fn create(
    repository: Data<RwLock<Repository>>,
    policy_type: Json<PolicyType>,
) -> Result<impl Responder> {
    let policy_type = policy_type.into_inner();
    let experiment_id = repository
        .write()
        .await
        .create_experiment(None, policy_type.into_inner());

    Ok(Json(CreateExperimentResponse { experiment_id }))
}

#[get("{experiment_id}/ping")]
async fn ping_experiment(
    repository: Data<RwLock<Repository>>,
    path: Path<String>,
) -> Result<impl Responder> {
    let experiment_id =
        Uuid::try_parse(&path.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let response = repository
        .read()
        .await
        .ping_experiment(experiment_id)
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(response)
}

#[put("{experiment_id}/reset")]
async fn reset(repository: Data<RwLock<Repository>>, path: Path<String>) -> Result<impl Responder> {
    let experiment_id =
        Uuid::try_parse(&path.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let response = repository
        .read()
        .await
        .reset_experiment(experiment_id, None, None, None)
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(response)
}

#[post("{experiment_id}/{arm_id}/reset")]
async fn reset_arm(
    repository: Data<RwLock<Repository>>,
    path: Path<(String, usize)>,
    payload: Json<ResetArmPayload>,
) -> Result<impl Responder> {
    let (experiment_id, arm_id) = path.into_inner();
    let experiment_id = Uuid::try_parse(&experiment_id).map_err(ApiResponseError::ErrorBadUuid)?;
    let ResetArmPayload {
        cumulative_reward,
        count,
    } = payload.into_inner();
    let response = repository
        .read()
        .await
        .reset_experiment(experiment_id, Some(arm_id), cumulative_reward, count)
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(response)
}

#[delete("{experiment_id}/delete")]
async fn delete_experiment(
    repository: Data<RwLock<Repository>>,
    path: Path<String>,
) -> Result<impl Responder> {
    let experiment_id =
        Uuid::try_parse(&path.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let response = repository
        .write()
        .await
        .delete_experiment(experiment_id)
        .map(|_| HttpResponse::Ok())
        .map_err(RepositoryOrExperimentError::Repository)
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(response)
}

#[post("{experiment_id}/add_arm")]
async fn add_arm(
    repository: Data<RwLock<Repository>>,
    path: Path<String>,
    payload: Json<AddArmPayload>,
) -> Result<impl Responder> {
    let experiment_id =
        Uuid::try_parse(&path.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let AddArmPayload {
        initial_reward,
        initial_count,
    } = payload.into_inner();
    let arm_id = repository
        .read()
        .await
        .add_experiment_arm(experiment_id, initial_reward, initial_count)
        .await
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(Json(AddExperimentArmResponse { arm_id }))
}

#[put("{experiment_id}/{arm_id}/disable")]
async fn disable_arm(
    repository: Data<RwLock<Repository>>,
    path: Path<(String, usize)>,
) -> Result<impl Responder> {
    let (experiment_id, arm_id) = path.into_inner();
    let experiment_id = Uuid::try_parse(&experiment_id).map_err(ApiResponseError::ErrorBadUuid)?;
    let response = repository
        .read()
        .await
        .disable_experiment_arm(experiment_id, arm_id)
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(response)
}

#[put("{experiment_id}/{arm_id}/enable")]
async fn enable_arm(
    repository: Data<RwLock<Repository>>,
    path: Path<(String, usize)>,
) -> Result<impl Responder> {
    let (experiment_id, arm_id) = path.into_inner();
    let experiment_id = Uuid::try_parse(&experiment_id).map_err(ApiResponseError::ErrorBadUuid)?;
    let response = repository
        .read()
        .await
        .enable_experiment_arm(experiment_id, arm_id)
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(response)
}

#[delete("{experiment_id}/{arm_id}/delete")]
async fn delete_arm(
    repository: Data<RwLock<Repository>>,
    path: Path<(String, usize)>,
) -> Result<impl Responder> {
    let (experiment_id, arm_id) = path.into_inner();
    let experiment_id = Uuid::try_parse(&experiment_id).map_err(ApiResponseError::ErrorBadUuid)?;
    let response = repository
        .read()
        .await
        .delete_experiment_arm(experiment_id, arm_id)
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(response)
}

#[get("{experiment_id}/draw")]
async fn draw(repository: Data<RwLock<Repository>>, path: Path<String>) -> Result<impl Responder> {
    let experiment_id =
        Uuid::try_parse(&path.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let draw_result = repository
        .read()
        .await
        .draw_experiment(experiment_id)
        .await
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(Json(DrawResponse::from(draw_result)))
}

#[put("{experiment_id}/update")]
async fn update(
    repository: Data<RwLock<Repository>>,
    path: Path<String>,
    payload: Json<UpdatePayload>,
) -> Result<impl Responder> {
    let experiment_id =
        Uuid::try_parse(&path.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let UpdatePayload {
        timestamp,
        arm_id,
        reward,
    } = payload.into_inner();
    let response = repository
        .read()
        .await
        .update_experiment(experiment_id, timestamp, arm_id, reward)
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(response)
}

#[put("{experiment_id}/update_batch")]
async fn update_batch(
    repository: Data<RwLock<Repository>>,
    path: Path<String>,
    payload: Json<UpdateBatchPayload>,
) -> Result<impl Responder> {
    let experiment_id =
        Uuid::try_parse(&path.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let updates = payload
        .into_inner()
        .updates
        .iter()
        .map(|u| (u.timestamp, u.arm_id, u.reward))
        .collect();

    let response = repository
        .read()
        .await
        .batch_update_experiment(experiment_id, updates)
        .await
        .map(|_| HttpResponse::Ok())
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(response)
}

#[get("{experiment_id}/stats")]
async fn stats(repository: Data<RwLock<Repository>>, path: Path<String>) -> Result<impl Responder> {
    let experiment_id =
        Uuid::try_parse(&path.into_inner()).map_err(ApiResponseError::ErrorBadUuid)?;
    let response = repository
        .read()
        .await
        .get_experiment_stats(experiment_id)
        .await
        .map(Json)
        .map_err(ApiResponseError::ErrorBadRequest)?;

    Ok(response)
}
