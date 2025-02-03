use super::arm::{Arm, ArmStats, Arms};
use super::errors::PolicyError;
use super::policy::{Policy, PolicyStats};
use super::rng::MaybeSeededRng;

use rand::{seq::IteratorRandom, Rng};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Serialize, Deserialize)]
pub struct EpsilonGreedyArm {
    pub(super) value: f64,
    pub(super) pulls: usize,
    pub(super) rewards: f64,
    pub(super) is_active: bool,
}

impl Default for EpsilonGreedyArm {
    fn default() -> Self {
        Self {
            value: 0.0,
            pulls: 0,
            rewards: 0.0,
            is_active: true,
        }
    }
}

impl Eq for EpsilonGreedyArm {}

impl PartialEq for EpsilonGreedyArm {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl PartialOrd for EpsilonGreedyArm {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EpsilonGreedyArm {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value
            .partial_cmp(&other.value)
            .unwrap_or(Ordering::Equal)
    }
}

impl Arm for EpsilonGreedyArm {
    fn reset(&mut self) {
        self.value = 0.0;
        self.pulls = 0;
        self.rewards = 0.0;
    }

    fn stats(&self) -> ArmStats {
        ArmStats {
            pulls: self.pulls,
            rewards: self.rewards,
            is_active: self.is_active,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EpsilonGreedy {
    arms: Arms<EpsilonGreedyArm>,
    epsilon: f64,
    rng: MaybeSeededRng,
}

impl EpsilonGreedy {
    pub fn new(epsilon: f64, seed: Option<u64>) -> Self {
        Self {
            arms: Arms::new(),
            epsilon,
            rng: MaybeSeededRng::new(seed),
        }
    }
}

#[typetag::serde]
impl Policy for EpsilonGreedy {
    fn reset(&mut self) {
        self.arms.values_mut().for_each(|arm| arm.reset());
    }

    fn add_arm(&mut self) -> usize {
        let arm_id = self.arms.len();
        self.arms.insert(arm_id, EpsilonGreedyArm::default());
        arm_id
    }

    fn delete_arm(&mut self, arm_id: usize) -> Result<(), PolicyError> {
        if self.arms.contains_key(&arm_id) {
            self.arms.remove(&arm_id);
            Ok(())
        } else {
            Err(PolicyError::ArmNotFound(arm_id))
        }
    }

    fn draw(&mut self) -> Result<usize, PolicyError> {
        if self.rng.get_rng().gen::<f64>() < self.epsilon {
            self.arms
                .iter()
                .filter(|(_, arm)| arm.is_active)
                .map(|(&arm_id, _)| arm_id)
                .choose(&mut self.rng.get_rng())
                .ok_or(PolicyError::NoArmsAvailable)
        } else {
            self.arms
                .iter()
                .filter(|(_, arm)| arm.is_active)
                .max_by(|(_, a), (_, b)| a.cmp(b))
                .map(|(&k, _)| k)
                .ok_or(PolicyError::NoArmsAvailable)
        }
    }

    fn update(&mut self, arm_id: usize, reward: f64) -> Result<(), PolicyError> {
        if let Some(arm) = self.arms.get_mut(&arm_id) {
            arm.pulls += 1;
            arm.rewards += reward;
            arm.value += (reward - arm.value) / (arm.pulls as f64);

            Ok(())
        } else {
            Err(PolicyError::ArmNotFound(arm_id))
        }
    }

    fn update_batch(&mut self, updates: &[(usize, usize, f64)]) -> Result<(), PolicyError> {
        let mut updates = updates.to_vec();
        updates.sort_unstable_by_key(|(ts, _, _)| *ts);
        updates
            .iter()
            .try_for_each(|&(_, arm_id, reward)| self.update(arm_id, reward))
    }

    fn stats(&self) -> PolicyStats {
        PolicyStats {
            arms: self
                .arms
                .iter()
                .map(|(&id, arm)| (id, arm.stats()))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::SmallRng, Rng, SeedableRng};
    use std::collections::HashMap;

    const SEED: u64 = 1234;

    #[test]
    fn create_arm() {
        let mut bandit = EpsilonGreedy::new(0.15, Some(SEED));
        assert!(bandit.arms.len() == 0);

        let arm_id = bandit.add_arm();
        assert!(bandit.arms.contains_key(&arm_id))
    }

    #[test]
    fn delete_arm() {
        let mut bandit = EpsilonGreedy::new(0.15, Some(SEED));
        let arm_id = bandit.add_arm();
        assert!(bandit.delete_arm(arm_id).is_ok());
        assert!(!bandit.arms.contains_key(&arm_id));
        assert!(bandit.delete_arm(arm_id).is_err());
    }

    #[test]
    fn draw() {
        let mut bandit = EpsilonGreedy::new(0.15, Some(SEED));
        let arm_id = bandit.add_arm();
        assert_eq!(bandit.draw().ok(), Some(arm_id));
    }

    #[test]
    fn draw_best() {
        let mut bandit = EpsilonGreedy::new(0.0, Some(SEED));
        let arm_1 = bandit.add_arm();
        let _ = bandit.add_arm();

        bandit.arms.get_mut(&arm_1).map(|arm| arm.value = 1.0);
        assert_eq!(bandit.draw().ok(), Some(arm_1));
    }

    #[test]
    fn draw_empty() {
        let mut bandit = EpsilonGreedy::new(0.15, Some(SEED));
        assert!(bandit.draw().is_err());
    }

    #[test]
    fn update() {
        let mut bandit = EpsilonGreedy::new(0.0, Some(SEED));
        let arm_1 = bandit.add_arm();
        let arm_2 = bandit.add_arm();

        assert!(bandit.update(arm_1, 1.0).is_ok());
        assert_eq!(bandit.arms.get(&arm_1).map(|arm| arm.value), Some(1.0));
        assert_eq!(bandit.arms.get(&arm_2).map(|arm| arm.value), Some(0.0));
    }

    #[test]
    fn update_batch() {
        let mut bandit = EpsilonGreedy::new(0.0, Some(SEED));
        let arm_1 = bandit.add_arm();
        let arm_2 = bandit.add_arm();
        let batch = vec![(0, arm_2, 0.0), (1, arm_1, 1.0), (2, arm_2, 0.0)];

        assert!(bandit.update_batch(&batch).is_ok());

        assert_eq!(bandit.arms.get(&arm_1).map(|arm| arm.pulls), Some(1));
        assert_eq!(bandit.arms.get(&arm_1).map(|arm| arm.rewards), Some(1.0));
        assert_eq!(bandit.arms.get(&arm_2).map(|arm| arm.pulls), Some(2));
        assert_eq!(bandit.arms.get(&arm_2).map(|arm| arm.rewards), Some(0.0));
    }

    #[test]
    fn debug() {
        let mut rng = SmallRng::seed_from_u64(SEED);
        let mut bandit = EpsilonGreedy::new(0.15, Some(SEED));

        println!("{:?}", bandit.draw());

        let mut arms: HashMap<_, _> = (0..4)
            .map(|_| (bandit.add_arm(), rng.gen::<f64>()))
            .collect();

        println!("arms: {:?}", arms);

        for i in 0..1000 {
            let arm_id = bandit.draw().unwrap();
            let reward = (rng.gen::<f64>() < *arms.get(&arm_id).unwrap()) as i32 as f64;
            let _ = bandit.update(arm_id, reward);

            if i == 250 {
                arms.insert(bandit.add_arm(), 0.8);
            }
        }

        println!("arms: {:?}", arms);
        println!("{:?}", bandit.stats());
    }
}
