use crate::policies::errors::PolicyError;

use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum SupervisorError {
    #[error("Bandit {0} not available")]
    BanditNotAvailable(Uuid),
    #[error("Bandit {0} not found")]
    BanditNotFound(Uuid),
}

#[derive(Debug, Error)]
pub enum SupervisorOrBanditError {
    #[error(transparent)]
    Supervisor(#[from] SupervisorError),
    #[error(transparent)]
    Bandit(#[from] BanditOrPolicyError),
}

#[derive(Debug, Error)]
pub enum BanditError {}

#[derive(Debug, Error)]
pub enum BanditOrPolicyError {
    #[error(transparent)]
    Bandit(#[from] BanditError),
    #[error(transparent)]
    PolicyError(#[from] PolicyError),
}
