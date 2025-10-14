use actix::MailboxError;
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
    #[error("No policy defined for experiment")]
    NoPolicy,
    #[error("Policy error: {0}")]
    PolicyError(#[from] PolicyError),
}

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Experiment {0} not found")]
    ExperimentNotFound(Uuid),
    #[error("Experiment error: {0}")]
    Experiment(#[from] ExperimentError),
}

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("I/O error while writing state store: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to serialize state store: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Mailbox error from {actor}: {source}")]
    Mailbox {
        actor: &'static str,
        #[source]
        source: MailboxError,
    },
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Persitence error: {0}")]
    Persistence(#[from] PersistenceError),
    #[error("No accountant defined")]
    Accountant,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Invalid UUID: {0}")]
    InvalidUuid(#[from] uuid::Error),
    #[error("Service error: {0}")]
    Service(#[from] ServiceError),
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
            ApiError::Service(err) => match err {
                ServiceError::Mailbox { .. } => "MailboxError",
                ServiceError::Repository(_) => "RepositoryError",
                ServiceError::Persistence(_) => "PersistenceError",
                ServiceError::Accountant => "AccountantError",
            },
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::InvalidUuid(_) => StatusCode::BAD_REQUEST,
            ApiError::Service(service_err) => match service_err {
                ServiceError::Mailbox { .. } => StatusCode::SERVICE_UNAVAILABLE,
                ServiceError::Repository(repo_err) => match repo_err {
                    RepositoryError::ExperimentNotFound(_) => StatusCode::NOT_FOUND,
                    RepositoryError::Experiment(_) => StatusCode::BAD_REQUEST,
                },
                ServiceError::Persistence(_) => StatusCode::INTERNAL_SERVER_ERROR,
                ServiceError::Accountant => StatusCode::SERVICE_UNAVAILABLE,
            },
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
