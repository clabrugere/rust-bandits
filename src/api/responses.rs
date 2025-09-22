use crate::actors::accountant::{Accountant, LogResponse};

use actix::Addr;
use actix_http::h1;
use actix_web::{
    body::{to_bytes, MessageBody},
    dev::{Payload, ServiceRequest, ServiceResponse},
    middleware::Next,
    web::{Bytes, Data},
    Error, HttpResponseBuilder, Result,
};
use log::warn;
use serde::Serialize;
use std::{
    fmt::Debug,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

fn bytes_to_payload(buf: Bytes) -> Payload {
    let (_, mut pl) = h1::Payload::create(true);
    pl.unread_data(buf);
    Payload::from(pl)
}

pub async fn log_response(
    mut request: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let accountant = request.app_data::<Data<Addr<Accountant>>>().cloned();

    // deconstruct request
    let request_body = request.extract::<Bytes>().await.unwrap_or_default();
    let request_body_str = String::from_utf8_lossy(&request_body).to_string();

    // reconstruct request
    request.set_payload(bytes_to_payload(request_body));

    // execute wrapped service
    let response = next.call(request).await?;

    // deconstruct the response
    let status = response.status();
    let header = response.headers().clone();
    let (request, http_response) = response.into_parts();
    let response_body = to_bytes(http_response.into_body())
        .await
        .unwrap_or_default();

    // send to logs
    match accountant {
        Some(accountant) => {
            let route = request.path();
            let body_str = String::from_utf8_lossy(&response_body).to_string();
            accountant.do_send(LogResponse {
                response: LoggedResponse::new(route, status.as_u16(), &request_body_str, &body_str),
            });
        }
        None => {
            warn!("No accountant to log response to");
        }
    }

    // reconstruct the response
    let mut http_response = HttpResponseBuilder::new(status).body(response_body);
    for (name, val) in header.iter() {
        http_response
            .headers_mut()
            .insert(name.clone(), val.clone());
    }

    let new_response = ServiceResponse::new(request, http_response);
    Ok(new_response)
}

#[derive(Debug, Serialize)]
pub struct LoggedResponse {
    pub id: Uuid,
    pub timestamp: u128,
    pub route: String,
    pub status: u16,
    pub request_body: String,
    pub response_body: String,
}

impl LoggedResponse {
    pub fn new(route: &str, status: u16, request_body: &str, response_body: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            route: route.to_string(),
            status,
            request_body: request_body.to_string(),
            response_body: response_body.to_string(),
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
    pub draw_id: Uuid,
    pub arm_id: usize,
}
