use super::arm::ArmStats;
use super::epsilon_greedy::EpsilonGreedy;
use super::errors::PolicyError;
use super::thomson_sampling::ThomsonSampling;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug)]
pub struct DrawResult {
    pub draw_id: Uuid,
    pub timestamp: u128,
    pub arm_id: usize,
}

pub type BatchUpdateElement = (u128, usize, f64);

#[derive(Clone, Debug, Serialize)]
pub struct PolicyStats {
    pub arms: HashMap<usize, ArmStats>,
}

#[derive(Debug, Deserialize)]
pub enum PolicyType {
    EpsilonGreedy {
        epsilon: f64,
        seed: Option<u64>,
    },
    ThomsonSampling {
        discount_factor: Option<f64>,
        seed: Option<u64>,
    },
}

impl PolicyType {
    pub fn into_inner(self) -> Box<dyn Policy + Send> {
        match self {
            PolicyType::EpsilonGreedy { epsilon, seed } => {
                Box::new(EpsilonGreedy::new(epsilon, seed))
            }
            PolicyType::ThomsonSampling {
                discount_factor,
                seed,
            } => Box::new(ThomsonSampling::new(discount_factor, seed)),
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
    fn reset(
        &mut self,
        arm_id: Option<usize>,
        reward: Option<f64>,
        count: Option<u64>,
    ) -> Result<(), PolicyError>;
    fn add_arm(&mut self, initial_reward: f64, initial_count: u64) -> usize;
    fn delete_arm(&mut self, arm_id: usize) -> Result<(), PolicyError>;
    fn draw(&mut self) -> Result<DrawResult, PolicyError>;
    fn update(&mut self, timestamp: u128, arm_id: usize, reward: f64) -> Result<(), PolicyError>;
    fn update_batch(&mut self, updates: &[BatchUpdateElement]) -> Result<(), PolicyError>;
    fn stats(&self) -> PolicyStats;
}
