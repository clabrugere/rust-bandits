use crate::actors::errors::SupervisorOrExperimentError;

use actix_web::{error::ResponseError, http::StatusCode};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiResponseError {
    #[error("Server Internal Error")]
    InternalError,
    #[error(transparent)]
    ErrorBadRequest(#[from] SupervisorOrExperimentError),
    #[error(transparent)]
    ErrorBadUuid(#[from] uuid::Error),
}

impl ResponseError for ApiResponseError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiResponseError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            ApiResponseError::ErrorBadRequest(_) => StatusCode::BAD_REQUEST,
            ApiResponseError::ErrorBadUuid(_) => StatusCode::BAD_REQUEST,
        }
    }
}
