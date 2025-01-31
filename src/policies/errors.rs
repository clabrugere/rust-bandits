use thiserror::Error;

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("No active arms to draw from")]
    NoArmsAvailable,
    #[error("Arm {0} not found")]
    ArmNotFound(usize),
}
