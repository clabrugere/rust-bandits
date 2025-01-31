use crate::policy::errors::PolicyError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BanditError {}

#[derive(Debug, Error)]
pub enum BanditOrPolicyError {
    #[error(transparent)]
    Bandit(#[from] BanditError),
    #[error(transparent)]
    PolicyError(#[from] PolicyError),
}
