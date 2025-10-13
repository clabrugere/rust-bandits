use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("No active arms to draw from")]
    NoArmsAvailable,
    #[error("Arm {0} not found")]
    ArmNotFound(usize),
    #[error("Sampling error: {0}")]
    SamplingError(String),
}

#[derive(Debug, Error)]
pub enum ExperimentError {
    #[error("Policy error: {0}")]
    PolicyError(#[from] PolicyError),
    #[error("No policy defined for experiment")]
    NoPolicy,
}

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Experiment error: {0}")]
    Experiment(#[from] ExperimentError),
    #[error("Experiment {0} not available")]
    ExperimentUnavailable(Uuid),
    #[error("Experiment {0} not found")]
    ExperimentNotFound(Uuid),
    #[error("Storage not available")]
    StorageUnavailable,
}

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("I/O error while writing state store: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to serialize state store: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Invalid UUID format: {0}")]
    InvalidUuid(#[from] uuid::Error),
    #[error("Invalid experiment or policy request: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Persistence failure: {0}")]
    Persistence(#[from] PersistenceError),
    #[error("Accountant service unavailable")]
    AccountantUnavailable,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: u16,
    kind: &'static str,
    message: String,
}

impl ApiError {
    fn kind(&self) -> &'static str {
        match self {
            ApiError::InvalidUuid(_) => "InvalidUuid",
            ApiError::Repository(_) => "InvalidRequest",
            ApiError::Persistence(_) => "PersistenceError",
            ApiError::AccountantUnavailable => "AccountantUnavailable",
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::InvalidUuid(_) => StatusCode::BAD_REQUEST,
            ApiError::Repository(error) => match error {
                RepositoryError::ExperimentNotFound(_) => StatusCode::NOT_FOUND,
                RepositoryError::ExperimentUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
                RepositoryError::StorageUnavailable => StatusCode::SERVICE_UNAVAILABLE,
                RepositoryError::Experiment(_) => StatusCode::BAD_REQUEST,
            },
            ApiError::Persistence(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::AccountantUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        let body = ErrorBody {
            code: status.as_u16(),
            kind: self.kind(),
            message: self.to_string(),
        };

        HttpResponse::build(status).json(body)
    }
}
