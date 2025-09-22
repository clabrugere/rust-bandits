use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("No active arms to draw from")]
    NoArmsAvailable,
    #[error("Arm {0} not found")]
    ArmNotFound(usize),
    #[error("Draw {0} for arm {1} not found")]
    DrawNotFound(Uuid, usize),
}
