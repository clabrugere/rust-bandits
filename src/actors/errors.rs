use crate::policies::errors::PolicyError;

use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum SupervisorError {
    #[error("Experiment {0} not available")]
    ExperimentNotAvailable(Uuid),
    #[error("Experiment {0} not found")]
    ExperimentNotFound(Uuid),
}

#[derive(Debug, Error)]
pub enum SupervisorOrExperimentError {
    #[error(transparent)]
    Supervisor(#[from] SupervisorError),
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
