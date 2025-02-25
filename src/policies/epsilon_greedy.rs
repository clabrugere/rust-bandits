use super::arm::{Arm, ArmStats};
use super::errors::PolicyError;
use super::policy::{CloneBoxedPolicy, Policy, PolicyStats};
use super::rng::MaybeSeededRng;

use rand::{seq::IteratorRandom, Rng};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpsilonGreedyArm {
    value: f64,
    pulls: u64,
    is_active: bool,
}

impl Default for EpsilonGreedyArm {
    fn default() -> Self {
        Self {
            value: 0.0,
            pulls: 0,
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
    }

    fn update(&mut self, reward: f64) {
        self.pulls += 1;
        self.value += (reward - self.value) / (self.pulls as f64);
    }

    fn stats(&self) -> ArmStats {
        ArmStats {
            pulls: self.pulls,
            mean_reward: self.value,
            is_active: self.is_active,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpsilonGreedy {
    arms: HashMap<usize, EpsilonGreedyArm>,
    epsilon: f64,
    rng: MaybeSeededRng,
}

impl EpsilonGreedy {
    pub fn new(epsilon: f64, seed: Option<u64>) -> Self {
        Self {
            arms: HashMap::new(),
            epsilon,
            rng: MaybeSeededRng::new(seed),
        }
    }
}

impl CloneBoxedPolicy for EpsilonGreedy {
    fn clone_box(&self) -> Box<dyn Policy + Send> {
        Box::new(self.clone())
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
            arm.update(reward);
            Ok(())
        } else {
            Err(PolicyError::ArmNotFound(arm_id))
        }
    }

    fn update_batch(&mut self, updates: &[(u64, usize, f64)]) -> Result<(), PolicyError> {
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

    const SEED: u64 = 1234;

    #[test]
    fn create_arm() {
        let mut policy = EpsilonGreedy::new(0.15, Some(SEED));
        assert!(policy.arms.len() == 0);

        let arm_id = policy.add_arm();
        assert!(policy.arms.contains_key(&arm_id))
    }

    #[test]
    fn delete_arm() {
        let mut policy = EpsilonGreedy::new(0.15, Some(SEED));
        let arm_id = policy.add_arm();
        assert!(policy.delete_arm(arm_id).is_ok());
        assert!(!policy.arms.contains_key(&arm_id));
        assert!(policy.delete_arm(arm_id).is_err());
    }

    #[test]
    fn draw() {
        let mut policy = EpsilonGreedy::new(0.15, Some(SEED));
        let arm_id = policy.add_arm();
        assert_eq!(policy.draw().ok(), Some(arm_id));
    }

    #[test]
    fn draw_best() {
        let mut policy = EpsilonGreedy::new(0.0, Some(SEED));
        let arm_1 = policy.add_arm();
        let _ = policy.add_arm();

        policy.arms.get_mut(&arm_1).map(|arm| arm.value = 1.0);
        assert_eq!(policy.draw().ok(), Some(arm_1));
    }

    #[test]
    fn draw_empty() {
        let mut policy = EpsilonGreedy::new(0.15, Some(SEED));
        assert!(policy.draw().is_err());
    }

    #[test]
    fn update() {
        let mut policy = EpsilonGreedy::new(0.0, Some(SEED));
        let arm_1 = policy.add_arm();
        let arm_2 = policy.add_arm();

        assert!(policy.update(arm_1, 1.0).is_ok());
        assert_eq!(policy.arms.get(&arm_1).map(|arm| arm.value), Some(1.0));
        assert_eq!(policy.arms.get(&arm_2).map(|arm| arm.value), Some(0.0));
    }

    #[test]
    fn update_batch() {
        let mut policy = EpsilonGreedy::new(0.0, Some(SEED));
        let arm_1 = policy.add_arm();
        let arm_2 = policy.add_arm();
        let batch = vec![(0, arm_2, 0.0), (1, arm_1, 1.0), (2, arm_2, 0.0)];

        assert!(policy.update_batch(&batch).is_ok());

        assert_eq!(policy.arms.get(&arm_1).map(|arm| arm.pulls), Some(1));
        assert_eq!(policy.arms.get(&arm_1).map(|arm| arm.value), Some(1.0));
        assert_eq!(policy.arms.get(&arm_2).map(|arm| arm.pulls), Some(2));
        assert_eq!(policy.arms.get(&arm_2).map(|arm| arm.value), Some(0.0));
    }

    #[test]
    fn debug() {
        let mut rng = SmallRng::seed_from_u64(SEED);
        let mut policy = EpsilonGreedy::new(0.2, Some(SEED));

        let mut true_values = vec![0.05, 0.2, 0.5];
        let mut arm_ids = true_values
            .iter()
            .map(|_| policy.add_arm())
            .collect::<Vec<usize>>();

        for i in 0..1000 {
            let arm_id = policy.draw().unwrap();
            let reward = (rng.gen::<f64>() < true_values[arm_id]) as i32 as f64;
            let _ = policy.update(arm_id, reward);

            if i == 250 {
                true_values.push(0.8);
                arm_ids.push(policy.add_arm());
            }
        }

        let stats = policy.stats();
        let mut rewards = stats
            .arms
            .iter()
            .map(|(arm_id, arm_stats)| (arm_id, arm_stats.mean_reward))
            .collect::<Vec<(&usize, f64)>>();
        rewards.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));

        println!(
            "arms: {:?}",
            arm_ids
                .iter()
                .zip(&true_values)
                .collect::<Vec<(&usize, &f64)>>()
        );
        println!("{rewards:?}");

        assert_eq!(
            rewards
                .iter()
                .map(|(&arm_id, _)| arm_id)
                .collect::<Vec<usize>>(),
            arm_ids
        );
    }
}
