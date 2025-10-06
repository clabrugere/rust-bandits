use crate::policies::errors::PolicyError;

use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RepositoryError {
    //#[error("Lock poisoned")]
    //Poisoned,
    #[error("Experiment {0} not available")]
    ExperimentNotAvailable(Uuid),
    #[error("Experiment {0} not found")]
    ExperimentNotFound(Uuid),
    #[error("Storage not available: {0}")]
    StorageError(String),
}

#[derive(Debug, Error)]
pub enum RepositoryOrExperimentError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Experiment(#[from] ExperimentOrPolicyError),
}

#[derive(Debug, Error)]
pub enum ExperimentError {}

#[derive(Debug, Error)]
pub enum ExperimentOrPolicyError {
    #[error(transparent)]
    Experiment(#[from] ExperimentError),
    #[error(transparent)]
    PolicyError(#[from] PolicyError),
}

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("I/O error while writing cache: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to serialize cache to JSON: {0}")]
    Serialization(#[from] serde_json::Error),
}
