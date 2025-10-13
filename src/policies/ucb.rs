use super::policy::{
    get_timestamp, ArmStats, BatchUpdateElement, CloneBoxedPolicy, DrawResult, Policy, PolicyStats,
    PolicyType,
};
use super::rng::MaybeSeededRng;

use crate::errors::PolicyError;

use rand::seq::IteratorRandom;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UcbArm {
    reward: f64,
    count: u64,
    is_active: bool,
}

impl UcbArm {
    fn new(initial_reward: f64, initial_count: u64) -> Self {
        Self {
            reward: initial_reward,
            count: initial_count,
            is_active: true,
        }
    }

    fn sample(&self, alpha: f64, total_count: u64) -> Result<f64, PolicyError> {
        Ok(self.reward + (alpha * (total_count as f64).ln() / (2.0 * (self.count as f64))))
    }
    fn reset(&mut self, cumulative_reward: Option<f64>, count: Option<u64>) {
        self.reward = cumulative_reward.unwrap_or_default();
        self.count = count.unwrap_or_default();
    }

    fn update(&mut self, reward: f64, _: f64) {
        self.count += 1;
        self.reward += (reward - self.reward) / (self.count as f64);
    }

    fn stats(&self) -> ArmStats {
        ArmStats {
            pulls: self.count,
            mean_reward: self.reward,
            is_active: self.is_active,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ucb {
    arms: HashMap<usize, UcbArm>,
    alpha: f64,
    rng: MaybeSeededRng,
}

impl Ucb {
    pub fn new(alpha: f64, seed: Option<u64>) -> Self {
        Self {
            arms: HashMap::new(),
            alpha,
            rng: MaybeSeededRng::new(seed),
        }
    }

    fn total_count(&self) -> u64 {
        self.arms
            .values()
            .filter(|arm| arm.is_active)
            .map(|arm| arm.count)
            .sum::<u64>()
    }
}

impl CloneBoxedPolicy for Ucb {
    fn clone_box(&self) -> Box<dyn Policy + Send> {
        Box::new(self.clone())
    }
}

#[typetag::serde]
impl Policy for Ucb {
    fn policy_type(&self) -> PolicyType {
        PolicyType::Ucb {
            alpha: self.alpha,
            seed: self.rng.seed,
        }
    }

    fn reset(
        &mut self,
        arm_id: Option<usize>,
        cumulative_reward: Option<f64>,
        count: Option<u64>,
    ) -> Result<(), PolicyError> {
        if let Some(arm_id) = arm_id {
            self.arms
                .get_mut(&arm_id)
                .map(|arm| arm.reset(cumulative_reward, count))
                .ok_or(PolicyError::ArmNotFound(arm_id))?;
        } else {
            self.arms.values_mut().for_each(|arm| arm.reset(None, None));
        }
        Ok(())
    }

    fn add_arm(&mut self, cumulative_reward: f64, count: u64) -> usize {
        let arm_id = self.arms.len();
        self.arms
            .insert(arm_id, UcbArm::new(cumulative_reward, count));

        arm_id
    }

    fn disable_arm(&mut self, arm_id: usize) -> Result<(), PolicyError> {
        self.arms
            .get_mut(&arm_id)
            .map(|arm| arm.is_active = false)
            .ok_or(PolicyError::ArmNotFound(arm_id))
    }

    fn enable_arm(&mut self, arm_id: usize) -> Result<(), PolicyError> {
        self.arms
            .get_mut(&arm_id)
            .map(|arm| arm.is_active = true)
            .ok_or(PolicyError::ArmNotFound(arm_id))
    }

    fn delete_arm(&mut self, arm_id: usize) -> Result<(), PolicyError> {
        self.arms
            .remove(&arm_id)
            .ok_or(PolicyError::ArmNotFound(arm_id))?;
        Ok(())
    }

    fn draw(&mut self) -> Result<DrawResult, PolicyError> {
        let timestamp = get_timestamp();

        // sample random arms while no feedback has been observed for every one, and then the one with the best statistic
        let arm_id = if let Some(arm_id) = self
            .arms
            .iter()
            .filter(|(_, arm)| arm.is_active && (arm.count == 0))
            .map(|(&arm_id, _)| arm_id)
            .choose(&mut self.rng.get_rng())
        {
            Ok(arm_id)
        } else {
            self.arms
                .iter()
                .filter(|(_, arm)| arm.is_active)
                .filter_map(
                    |(arm_id, arm)| match arm.sample(self.alpha, self.total_count()) {
                        Ok(sample) => Some((arm_id, sample)),
                        Err(_) => None,
                    },
                )
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                .map(|(&arm_id, _)| arm_id)
                .ok_or(PolicyError::NoArmsAvailable)
        }?;

        Ok(DrawResult { timestamp, arm_id })
    }

    fn update(&mut self, timestamp: f64, arm_id: usize, reward: f64) -> Result<(), PolicyError> {
        // update the arm statistics
        self.arms
            .get_mut(&arm_id)
            .ok_or(PolicyError::ArmNotFound(arm_id))?
            .update(reward, timestamp);

        Ok(())
    }

    fn update_batch(&mut self, updates: &[BatchUpdateElement]) -> Result<(), PolicyError> {
        updates
            .iter()
            .try_for_each(|&(timestamp, arm_id, reward)| self.update(timestamp, arm_id, reward))
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

    const SEED: u64 = 1234;

    #[test]
    fn create_arm() {
        let mut policy = Ucb::new(1.0, Some(SEED));
        assert!(policy.arms.len() == 0);

        let arm_id = policy.add_arm(0.0, 0);
        assert!(policy.arms.contains_key(&arm_id))
    }

    #[test]
    fn disable_arm() {
        let mut policy = Ucb::new(1.0, Some(SEED));
        let arm_id = policy.add_arm(0.0, 0);

        assert!(policy.disable_arm(arm_id).is_ok());
        assert_eq!(
            policy.arms.iter().filter(|(_, arm)| arm.is_active).count(),
            0
        );
    }

    #[test]
    fn enable_arm() {
        let mut policy = Ucb::new(1.0, Some(SEED));
        let arm_id = policy.add_arm(0.0, 0);

        assert!(policy.disable_arm(arm_id).is_ok());
        assert_eq!(
            policy.arms.iter().filter(|(_, arm)| arm.is_active).count(),
            0
        );
        assert!(policy.enable_arm(arm_id).is_ok());
        assert_eq!(
            policy.arms.iter().filter(|(_, arm)| arm.is_active).count(),
            1
        );
    }

    #[test]
    fn delete_arm() {
        let mut policy = Ucb::new(1.0, Some(SEED));
        let arm_id = policy.add_arm(0.0, 0);
        assert!(policy.delete_arm(arm_id).is_ok());
        assert!(!policy.arms.contains_key(&arm_id));
        assert!(policy.delete_arm(arm_id).is_err());
    }

    #[test]
    fn draw() {
        let mut policy = Ucb::new(1.0, Some(SEED));
        let arm_id = policy.add_arm(0.0, 0);
        let result = policy.draw().ok().map(|DrawResult { arm_id, .. }| arm_id);
        assert_eq!(result, Some(arm_id));
    }

    #[test]
    fn draw_best() {
        let mut policy = Ucb::new(1.0, Some(SEED));
        let arm_1 = policy.add_arm(0.0, 0);
        let arm_2 = policy.add_arm(0.0, 0);

        policy.arms.get_mut(&arm_1).map(|arm| {
            arm.reward = 1.0;
            arm.count += 1
        });
        policy.arms.get_mut(&arm_2).map(|arm| {
            arm.reward = 0.0;
            arm.count += 1
        });

        let result = policy.draw().ok().map(|DrawResult { arm_id, .. }| arm_id);
        assert_eq!(result, Some(arm_1));
    }

    #[test]
    fn draw_empty() {
        let mut policy = Ucb::new(1.0, Some(SEED));
        assert!(policy.draw().is_err());
    }

    #[test]
    fn update() {
        let mut policy = Ucb::new(1.0, Some(SEED));
        let _ = policy.add_arm(0.0, 0);
        let _ = policy.add_arm(0.0, 0);

        let DrawResult {
            timestamp, arm_id, ..
        } = policy.draw().unwrap();

        assert!(policy.update(timestamp + 1.0, arm_id, 1.0).is_ok());
        assert_eq!(policy.arms.get(&arm_id).map(|arm| arm.reward), Some(1.0));
    }

    #[test]
    fn update_batch() {
        let mut policy = Ucb::new(1.0, Some(SEED));
        let _ = policy.add_arm(0.0, 0);
        let _ = policy.add_arm(0.0, 0);

        let draws = (0..3)
            .map(|_| policy.draw().unwrap())
            .collect::<Vec<DrawResult>>();
        let updates = draws
            .iter()
            .map(|draw| (draw.timestamp + 1.0, draw.arm_id, 1.0))
            .collect::<Vec<BatchUpdateElement>>();

        assert!(policy.update_batch(&updates).is_ok());
        updates.iter().for_each(|(_, arm_id, reward)| {
            assert_eq!(policy.arms.get(&arm_id).unwrap().reward, *reward);
        });
    }
}
