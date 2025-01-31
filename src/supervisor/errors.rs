use crate::bandit::errors::BanditOrPolicyError;
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
