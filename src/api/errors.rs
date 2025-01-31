use super::routes::ApiResponse;
use crate::supervisor::errors::SupervisorOrBanditError;
use actix_web::{error::ResponseError, http::header::ContentType, http::StatusCode, HttpResponse};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiResponseError {
    #[error("Server Internal Error")]
    InternalError,
    #[error(transparent)]
    ErrorBadRequest(#[from] SupervisorOrBanditError),
    #[error(transparent)]
    ErrorBadUuid(#[from] uuid::Error),
}

impl ResponseError for ApiResponseError {
    fn error_response(&self) -> HttpResponse {
        let response = ApiResponse::with_data(Some(self.to_string()));
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(response)
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            ApiResponseError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            ApiResponseError::ErrorBadRequest(_) => StatusCode::BAD_REQUEST,
            ApiResponseError::ErrorBadUuid(_) => StatusCode::BAD_REQUEST,
        }
    }
}
