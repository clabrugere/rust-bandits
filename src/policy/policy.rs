use super::arm::Arms;
use super::epsilon_greedy::EpsilonGreedy;
use super::errors::PolicyError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum PolicyType {
    EpsilonGreedy { epsilon: f64, seed: Option<u64> },
}

pub fn create_policy(policy_type: &PolicyType) -> Box<dyn Policy + Send> {
    match policy_type {
        PolicyType::EpsilonGreedy { epsilon, seed } => {
            Box::new(EpsilonGreedy::new(*epsilon, *seed))
        }
    }
}

pub trait Policy {
    fn reset(&mut self);
    fn add_arm(&mut self) -> usize;
    fn delete_arm(&mut self, arm_id: usize) -> Result<(), PolicyError>;
    fn draw(&mut self) -> Result<usize, PolicyError>;
    fn update(&mut self, arm_id: usize, reward: f64) -> Result<(), PolicyError>;
    fn update_batch(&mut self, updates: &[(usize, usize, f64)]) -> Result<(), PolicyError>;
    fn stats(&self) -> PolicyStats;
}

#[derive(Debug, Serialize)]
pub struct PolicyStats {
    pub arms: Arms,
}

impl From<&Arms> for PolicyStats {
    fn from(arms: &Arms) -> Self {
        Self { arms: arms.clone() }
    }
}
