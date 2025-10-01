use super::arm::{Arm, ArmStats};
use super::errors::PolicyError;
use super::policy::{BatchUpdateElement, CloneBoxedPolicy, DrawResult, Policy, PolicyStats};
use super::rng::MaybeSeededRng;

use rand::{seq::IteratorRandom, Rng};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpsilonGreedyArm {
    reward: f64,
    count: u64,
    is_active: bool,
}

impl Default for EpsilonGreedyArm {
    fn default() -> Self {
        Self {
            reward: 0.0,
            count: 0,
            is_active: true,
        }
    }
}

impl Eq for EpsilonGreedyArm {}

impl PartialEq for EpsilonGreedyArm {
    fn eq(&self, other: &Self) -> bool {
        self.reward == other.reward
    }
}

impl PartialOrd for EpsilonGreedyArm {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EpsilonGreedyArm {
    fn cmp(&self, other: &Self) -> Ordering {
        self.reward
            .partial_cmp(&other.reward)
            .unwrap_or(Ordering::Equal)
    }
}

impl Arm for EpsilonGreedyArm {
    fn reset(&mut self, reward: Option<f64>, count: Option<u64>) {
        self.reward = reward.unwrap_or_default();
        self.count = count.unwrap_or_default();
    }

    fn update(&mut self, reward: f64, _: Option<f64>) {
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

type DrawHistoryElement = (Uuid, usize);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpsilonGreedy {
    draw_history: HashMap<DrawHistoryElement, u128>,
    arms: HashMap<usize, EpsilonGreedyArm>,
    epsilon: f64,
    rng: MaybeSeededRng,
}

impl EpsilonGreedy {
    pub fn new(epsilon: f64, seed: Option<u64>) -> Self {
        Self {
            draw_history: HashMap::new(),
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
    fn reset(
        &mut self,
        arm_id: Option<usize>,
        reward: Option<f64>,
        count: Option<u64>,
    ) -> Result<(), PolicyError> {
        if let Some(arm_id) = arm_id {
            self.arms
                .get_mut(&arm_id)
                .map(|arm| arm.reset(reward, count))
                .ok_or(PolicyError::ArmNotFound(arm_id))?;
        } else {
            self.arms.values_mut().for_each(|arm| arm.reset(None, None));
        }
        Ok(())
    }

    fn add_arm(&mut self, initial_reward: f64, initial_count: u64) -> usize {
        let arm_id = self.arms.len();
        self.arms.insert(
            arm_id,
            EpsilonGreedyArm {
                reward: initial_reward,
                count: initial_count,
                is_active: true,
            },
        );
        arm_id
    }

    fn delete_arm(&mut self, arm_id: usize) -> Result<(), PolicyError> {
        self.arms
            .remove(&arm_id)
            .ok_or(PolicyError::ArmNotFound(arm_id))?;
        Ok(())
    }

    fn draw(&mut self) -> Result<DrawResult, PolicyError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let draw_id = Uuid::new_v4();

        let result = if self.rng.get_rng().gen::<f64>() < self.epsilon {
            self.arms
                .iter()
                .filter(|(_, arm)| arm.is_active)
                .map(|(&arm_id, _)| arm_id)
                .choose(&mut self.rng.get_rng())
                .map(|arm_id| DrawResult {
                    timestamp,
                    draw_id,
                    arm_id,
                })
                .ok_or(PolicyError::NoArmsAvailable)
        } else {
            self.arms
                .iter()
                .filter(|(_, arm)| arm.is_active)
                .max_by(|(_, a), (_, b)| a.cmp(b))
                .map(|(&arm_id, _)| DrawResult {
                    timestamp,
                    draw_id,
                    arm_id,
                })
                .ok_or(PolicyError::NoArmsAvailable)
        }?;

        // store draw for future update
        self.draw_history
            .insert((result.draw_id, result.arm_id), timestamp);

        Ok(result)
    }

    fn update(
        &mut self,
        draw_id: Uuid,
        _: u128,
        arm_id: usize,
        reward: f64,
    ) -> Result<(), PolicyError> {
        // check if we can match the update with a previous draw and pop it if found
        self.draw_history
            .remove(&(draw_id, arm_id))
            .ok_or(PolicyError::DrawNotFound(draw_id, arm_id))?;

        // update the arm statistics
        self.arms
            .get_mut(&arm_id)
            .ok_or(PolicyError::ArmNotFound(arm_id))?
            .update(reward, None);

        Ok(())
    }

    fn update_batch(&mut self, updates: &[BatchUpdateElement]) -> Result<(), PolicyError> {
        updates
            .iter()
            .try_for_each(|&(draw_id, timestamp, arm_id, reward)| {
                self.update(draw_id, timestamp, arm_id, reward)
            })
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

        let arm_id = policy.add_arm(0.0, 0);
        assert!(policy.arms.contains_key(&arm_id))
    }

    #[test]
    fn delete_arm() {
        let mut policy = EpsilonGreedy::new(0.15, Some(SEED));
        let arm_id = policy.add_arm(0.0, 0);
        assert!(policy.delete_arm(arm_id).is_ok());
        assert!(!policy.arms.contains_key(&arm_id));
        assert!(policy.delete_arm(arm_id).is_err());
    }

    #[test]
    fn draw() {
        let mut policy = EpsilonGreedy::new(0.15, Some(SEED));
        let arm_id = policy.add_arm(0.0, 0);
        let result = policy.draw().ok().map(|DrawResult { arm_id, .. }| arm_id);
        assert_eq!(result, Some(arm_id));
    }

    #[test]
    fn draw_best() {
        let mut policy = EpsilonGreedy::new(0.0, Some(SEED));
        let arm_1 = policy.add_arm(0.0, 0);
        let _ = policy.add_arm(0.0, 0);

        policy.arms.get_mut(&arm_1).map(|arm| arm.reward = 1.0);
        let result = policy.draw().ok().map(|DrawResult { arm_id, .. }| arm_id);
        assert_eq!(result, Some(arm_1));
    }

    #[test]
    fn draw_empty() {
        let mut policy = EpsilonGreedy::new(0.15, Some(SEED));
        assert!(policy.draw().is_err());
    }

    #[test]
    fn update() {
        let mut policy = EpsilonGreedy::new(0.0, Some(SEED));
        let arm_1 = policy.add_arm(0.0, 0);
        let arm_2 = policy.add_arm(0.0, 0);

        let DrawResult {
            draw_id,
            timestamp,
            arm_id,
        } = policy.draw().unwrap();

        assert!(policy.update(draw_id, timestamp + 1, arm_id, 1.0).is_ok());
        assert_eq!(policy.arms.get(&arm_1).map(|arm| arm.reward), Some(1.0));
        assert_eq!(policy.arms.get(&arm_2).map(|arm| arm.reward), Some(0.0));
    }

    #[test]
    fn update_batch() {
        let mut policy = EpsilonGreedy::new(0.0, Some(SEED));
        let _ = policy.add_arm(0.0, 0);
        let _ = policy.add_arm(0.0, 0);

        let draws = (0..3)
            .map(|_| policy.draw().unwrap())
            .collect::<Vec<DrawResult>>();
        let updates = draws
            .iter()
            .map(|draw| (draw.draw_id, draw.timestamp + 1, draw.arm_id, 1.0))
            .collect::<Vec<(Uuid, u128, usize, f64)>>();

        assert!(policy.update_batch(&updates).is_ok());
        updates.iter().for_each(|(_, _, arm_id, reward)| {
            assert_eq!(policy.arms.get(&arm_id).unwrap().reward, *reward);
        });
    }

    #[test]
    fn debug() {
        let mut rng = SmallRng::seed_from_u64(SEED);
        let mut policy = EpsilonGreedy::new(0.2, Some(SEED));

        let mut true_values = vec![0.05, 0.2, 0.5];
        let mut arm_ids = true_values
            .iter()
            .map(|_| policy.add_arm(0.0, 0))
            .collect::<Vec<usize>>();

        for i in 0..1000 {
            let DrawResult {
                draw_id,
                timestamp,
                arm_id,
            } = policy.draw().unwrap();
            let reward = (rng.gen::<f64>() < true_values[arm_id]) as i32 as f64;
            let _ = policy.update(draw_id, timestamp + 1, arm_id, reward);

            if i == 250 {
                true_values.push(0.8);
                arm_ids.push(policy.add_arm(0.0, 0));
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
