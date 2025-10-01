use crate::{
    actors::accountant::{Accountant, LogResponse},
    policies::DrawResult,
};

use actix::Addr;
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
    web::Data,
    Error, Result,
};
use serde::Serialize;
use std::{
    fmt::Debug,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::info;
use uuid::Uuid;

pub async fn log_response(
    request: ServiceRequest,
    next: Next<impl actix_web::body::MessageBody>,
) -> Result<ServiceResponse<impl actix_web::body::MessageBody>, Error> {
    let method = request.method().to_string();
    let path = request.path().to_string();
    let request_id = Uuid::new_v4();

    let accountant = request.app_data::<Data<Addr<Accountant>>>().cloned();
    let response = next.call(request).await?;
    let status = response.status();

    info!(
        method = %method,
        path = %path,
        request_id = %request_id,
        status = %status.as_u16(),
        "Request"
    );

    if let Some(accountant) = accountant {
        accountant.do_send(LogResponse {
            response: LoggedResponse::new(request_id, &path, status.as_u16()),
        });
    }
    Ok(response)
}

#[derive(Debug, Serialize)]
pub struct LoggedResponse {
    pub id: Uuid,
    pub timestamp: u128,
    pub route: String,
    pub status: u16,
}

impl LoggedResponse {
    pub fn new(id: Uuid, route: &str, status: u16) -> Self {
        Self {
            id,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            route: route.to_string(),
            status,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListExperimentsResponse {
    pub experiment_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct CreateExperimentResponse {
    pub experiment_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct AddExperimentArmResponse {
    pub arm_id: usize,
}

#[derive(Debug, Serialize)]
pub struct DrawResponse {
    pub timestamp: u128,
    pub arm_id: usize,
}

impl From<DrawResult> for DrawResponse {
    fn from(draw_result: DrawResult) -> Self {
        Self {
            timestamp: draw_result.timestamp,
            arm_id: draw_result.arm_id,
        }
    }
}
