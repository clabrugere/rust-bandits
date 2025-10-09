use super::errors::PolicyError;
use super::policy::{
    get_timestamp, ArmStats, BatchUpdateElement, CloneBoxedPolicy, DrawResult, Policy, PolicyStats,
    PolicyType,
};
use super::rng::MaybeSeededRng;

use rand::{seq::IteratorRandom, Rng};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpsilonGreedyArm {
    reward: f64,
    count: u64,
    is_active: bool,
}

impl EpsilonGreedyArm {
    fn new(initial_reward: f64, initial_count: u64) -> Self {
        Self {
            reward: initial_reward,
            count: initial_count,
            is_active: true,
        }
    }

    fn reset(&mut self, cumulative_reward: Option<f64>, count: Option<u64>) {
        self.reward = cumulative_reward.unwrap_or_default();
        self.count = count.unwrap_or_default();
    }

    fn sample<R: Rng + ?Sized>(&self, _: &mut R) -> Result<f64, PolicyError> {
        Ok(self.reward)
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

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum DecayType {
    Exponential { decay: f64 },
    Inverse { decay: f64 },
    Linear { decay: f64, min_epsilon: f64 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpsilonGreedy {
    arms: HashMap<usize, EpsilonGreedyArm>,
    epsilon: f64,
    epsilon_decay: Option<DecayType>,
    rng: MaybeSeededRng,
}

impl EpsilonGreedy {
    pub fn new(epsilon: f64, epsilon_decay: Option<DecayType>, seed: Option<u64>) -> Self {
        Self {
            arms: HashMap::new(),
            epsilon,
            epsilon_decay,
            rng: MaybeSeededRng::new(seed),
        }
    }

    fn total_count(&self) -> u64 {
        self.arms
            .values()
            .filter(|arm| arm.is_active)
            .map(|arm| arm.count)
            .sum()
    }

    fn epsilon_with_decay(&self) -> f64 {
        let total_count = self.total_count() as f64;
        match self.epsilon_decay {
            Some(DecayType::Exponential { decay }) => self.epsilon * (-decay * total_count).exp(),
            Some(DecayType::Inverse { decay }) => self.epsilon / (1.0 + decay * total_count),
            Some(DecayType::Linear { decay, min_epsilon }) => {
                (self.epsilon - decay * total_count).max(min_epsilon)
            }
            None => self.epsilon,
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
    fn policy_type(&self) -> PolicyType {
        PolicyType::EpsilonGreedy {
            epsilon: self.epsilon,
            epsilon_decay: self.epsilon_decay,
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
            .insert(arm_id, EpsilonGreedyArm::new(cumulative_reward, count));

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
        let arm_iterator = self.arms.iter().filter(|(_, arm)| arm.is_active);
        let epsilon = self.epsilon_with_decay();

        // either sample a random arm or return the one with the highest reward so far
        let arm_id = if self.rng.get_rng().random::<f64>() < epsilon {
            arm_iterator
                .map(|(&arm_id, _)| arm_id)
                .choose(&mut self.rng.get_rng())
                .ok_or(PolicyError::NoArmsAvailable)
        } else {
            arm_iterator
                .filter_map(|(arm_id, arm)| match arm.sample(self.rng.get_rng()) {
                    Ok(sample) => Some((arm_id, sample)),
                    Err(_) => None,
                })
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
        let mut policy = EpsilonGreedy::new(0.1, None, Some(SEED));
        assert!(policy.arms.len() == 0);

        let arm_id = policy.add_arm(0.0, 0);
        assert!(policy.arms.contains_key(&arm_id))
    }

    #[test]
    fn disable_arm() {
        let mut policy = EpsilonGreedy::new(0.1, None, Some(SEED));
        let arm_id = policy.add_arm(0.0, 0);

        assert!(policy.disable_arm(arm_id).is_ok());
        assert_eq!(
            policy.arms.iter().filter(|(_, arm)| arm.is_active).count(),
            0
        );
    }

    #[test]
    fn enable_arm() {
        let mut policy = EpsilonGreedy::new(0.1, None, Some(SEED));
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
        let mut policy = EpsilonGreedy::new(0.1, None, Some(SEED));
        let arm_id = policy.add_arm(0.0, 0);
        assert!(policy.delete_arm(arm_id).is_ok());
        assert!(!policy.arms.contains_key(&arm_id));
        assert!(policy.delete_arm(arm_id).is_err());
    }

    #[test]
    fn draw() {
        let mut policy = EpsilonGreedy::new(0.1, None, Some(SEED));
        let arm_id = policy.add_arm(0.0, 0);
        let result = policy.draw().ok().map(|DrawResult { arm_id, .. }| arm_id);
        assert_eq!(result, Some(arm_id));
    }

    #[test]
    fn draw_best() {
        let mut policy = EpsilonGreedy::new(0.1, None, Some(SEED));
        let arm_1 = policy.add_arm(0.0, 0);
        let _ = policy.add_arm(0.0, 0);

        policy.arms.get_mut(&arm_1).map(|arm| arm.reward = 1.0);
        let result = policy.draw().ok().map(|DrawResult { arm_id, .. }| arm_id);
        assert_eq!(result, Some(arm_1));
    }

    #[test]
    fn draw_empty() {
        let mut policy = EpsilonGreedy::new(0.1, None, Some(SEED));
        assert!(policy.draw().is_err());
    }

    #[test]
    fn update() {
        let mut policy = EpsilonGreedy::new(0.1, None, Some(SEED));
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
        let mut policy = EpsilonGreedy::new(0.1, None, Some(SEED));
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

    #[test]
    fn decay_none() {
        let mut policy = EpsilonGreedy::new(0.1, None, Some(SEED));
        let _ = policy.add_arm(0.0, 0);
        let _ = policy.add_arm(0.0, 0);

        let draws = (0..3)
            .map(|_| policy.draw().unwrap())
            .collect::<Vec<DrawResult>>();
        let updates = draws
            .iter()
            .map(|draw| (draw.timestamp + 1.0, draw.arm_id, 1.0))
            .collect::<Vec<BatchUpdateElement>>();

        _ = policy.update_batch(&updates);

        assert_eq!(policy.epsilon_with_decay(), 0.1);
    }

    #[test]
    fn decay_exponential() {
        let mut policy = EpsilonGreedy::new(
            0.1,
            Some(DecayType::Exponential { decay: 0.01 }),
            Some(SEED),
        );
        let _ = policy.add_arm(0.0, 0);
        let _ = policy.add_arm(0.0, 0);

        let draws = (0..3)
            .map(|_| policy.draw().unwrap())
            .collect::<Vec<DrawResult>>();
        let updates = draws
            .iter()
            .map(|draw| (draw.timestamp + 1.0, draw.arm_id, 1.0))
            .collect::<Vec<BatchUpdateElement>>();

        _ = policy.update_batch(&updates);

        assert!((policy.epsilon_with_decay() - 0.097045).abs() < 1e-6);
    }

    #[test]
    fn decay_inverse() {
        let mut policy =
            EpsilonGreedy::new(0.1, Some(DecayType::Inverse { decay: 0.01 }), Some(SEED));
        let _ = policy.add_arm(0.0, 0);
        let _ = policy.add_arm(0.0, 0);

        let draws = (0..3)
            .map(|_| policy.draw().unwrap())
            .collect::<Vec<DrawResult>>();
        let updates = draws
            .iter()
            .map(|draw| (draw.timestamp + 1.0, draw.arm_id, 1.0))
            .collect::<Vec<BatchUpdateElement>>();

        _ = policy.update_batch(&updates);

        assert!((policy.epsilon_with_decay() - 0.097087).abs() < 1e-6);
    }

    #[test]
    fn decay_linear() {
        let mut policy = EpsilonGreedy::new(
            0.1,
            Some(DecayType::Linear {
                decay: 0.01,
                min_epsilon: 0.01,
            }),
            Some(SEED),
        );
        let _ = policy.add_arm(0.0, 0);
        let _ = policy.add_arm(0.0, 0);

        let draws = (0..3)
            .map(|_| policy.draw().unwrap())
            .collect::<Vec<DrawResult>>();
        let updates = draws
            .iter()
            .map(|draw| (draw.timestamp + 1.0, draw.arm_id, 1.0))
            .collect::<Vec<BatchUpdateElement>>();

        _ = policy.update_batch(&updates);

        assert!((policy.epsilon_with_decay() - 0.07).abs() < 1e-6);

        let draws = (0..10)
            .map(|_| policy.draw().unwrap())
            .collect::<Vec<DrawResult>>();
        let updates = draws
            .iter()
            .map(|draw| (draw.timestamp + 1.0, draw.arm_id, 1.0))
            .collect::<Vec<BatchUpdateElement>>();

        _ = policy.update_batch(&updates);

        assert_eq!(policy.epsilon_with_decay(), 0.01);
    }
}
