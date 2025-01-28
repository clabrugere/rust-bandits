use super::arm::Arms;
use super::epsilon_greedy::EpsilonGreedy;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, fmt};

#[derive(Debug, Deserialize, Serialize)]
pub enum BanditType {
    EpsilonGreedy { epsilon: f64, seed: Option<u64> },
}

pub fn create_bandit(bandit_type: BanditType) -> Box<dyn Bandit + Send> {
    match bandit_type {
        BanditType::EpsilonGreedy { epsilon, seed } => Box::new(EpsilonGreedy::new(epsilon, seed)),
    }
}

pub trait Bandit {
    fn reset(&mut self);
    fn add_arm(&mut self) -> usize;
    fn delete_arm(&mut self, arm_id: usize) -> Result<(), BanditError>;
    fn draw(&mut self) -> Result<usize, BanditError>;
    fn update(&mut self, arm_id: usize, reward: f64) -> Result<(), BanditError>;
    fn stats(&self) -> BanditStats;
}

#[derive(Debug, Serialize)]
pub struct BanditStats {
    pub arms: Vec<usize>,
    pub pulls: HashMap<usize, usize>,
    pub rewards: HashMap<usize, f64>,
    pub active: HashMap<usize, bool>,
}

impl From<&Arms> for BanditStats {
    fn from(arms: &Arms) -> Self {
        Self {
            arms: arms.keys().cloned().collect(),
            pulls: arms
                .iter()
                .map(|(&arm_id, arm)| (arm_id, arm.pulls))
                .collect(),
            rewards: arms
                .iter()
                .map(|(&arm_id, arm)| (arm_id, arm.rewards))
                .collect(),
            active: arms
                .iter()
                .map(|(&arm_id, arm)| (arm_id, arm.is_active))
                .collect(),
        }
    }
}

#[derive(Debug)]
pub enum BanditError {
    ArmNotFound(usize),
    NoArmsAvailable,
}

impl Error for BanditError {}

impl fmt::Display for BanditError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BanditError::ArmNotFound(arm) => write!(f, "Arm {} not found", arm),
            BanditError::NoArmsAvailable => write!(f, "No arms to draw from"),
        }
    }
}
