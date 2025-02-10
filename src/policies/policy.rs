use super::arm::ArmStats;
use super::epsilon_greedy::EpsilonGreedy;
use super::errors::PolicyError;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize)]
pub struct PolicyStats {
    pub arms: HashMap<usize, ArmStats>,
}

#[derive(Debug, Deserialize)]
pub enum PolicyType {
    EpsilonGreedy { epsilon: f64, seed: Option<u64> },
}

impl PolicyType {
    pub fn into_inner(self) -> Box<dyn Policy + Send> {
        match self {
            PolicyType::EpsilonGreedy { epsilon, seed } => {
                Box::new(EpsilonGreedy::new(epsilon, seed))
            }
        }
    }
}

impl Clone for Box<dyn Policy + Send> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub trait CloneBoxedPolicy {
    fn clone_box(&self) -> Box<dyn Policy + Send>;
}

#[typetag::serde(tag = "type")]
pub trait Policy: Send + CloneBoxedPolicy {
    fn reset(&mut self);
    fn add_arm(&mut self) -> usize;
    fn delete_arm(&mut self, arm_id: usize) -> Result<(), PolicyError>;
    fn draw(&mut self) -> Result<usize, PolicyError>;
    fn update(&mut self, arm_id: usize, reward: f64) -> Result<(), PolicyError>;
    fn update_batch(&mut self, updates: &[(u64, usize, f64)]) -> Result<(), PolicyError>;
    fn stats(&self) -> PolicyStats;
}
