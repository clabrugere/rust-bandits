use rand::Rng;
use serde::Serialize;

use crate::policies::errors::PolicyError;

pub trait Arm {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Result<f64, PolicyError>;
    fn reset(&mut self, cumulative_reward: Option<f64>, count: Option<u64>);
    fn update(&mut self, reward: f64, timestamp: f64);
    fn stats(&self) -> ArmStats;
}

#[derive(Clone, Debug, Serialize)]
pub struct ArmStats {
    pub pulls: u64,
    pub mean_reward: f64,
    pub is_active: bool,
}
