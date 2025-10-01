use super::arm::{Arm, ArmStats};
use super::errors::PolicyError;
use super::policy::{
    BatchUpdateElement, CloneBoxedPolicy, DrawHistoryElement, DrawResult, Policy, PolicyStats,
};
use super::rng::MaybeSeededRng;

use rand_distr::Beta;
use rand_distr::Distribution;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThomsonSamplingArm {
    alpha: f64,
    beta: f64,
    count: u64,
    is_active: bool,
}

impl Default for ThomsonSamplingArm {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            beta: 1.0,
            count: 0,
            is_active: true,
        }
    }
}

impl Arm for ThomsonSamplingArm {
    fn reset(&mut self, reward: Option<f64>, count: Option<u64>) {
        if let (Some(reward), Some(count)) = (reward, count) {
            self.alpha = reward + 1.0;
            self.beta = (count as f64) - reward + 1.0;
            self.count = count;
        } else {
            self.alpha = 1.0;
            self.beta = 1.0;
            self.count = 0;
        }
    }

    fn update(&mut self, reward: f64, _: Option<f64>) {
        self.alpha += reward;
        self.beta += 1.0 - reward;
        self.count += 1;
    }

    fn stats(&self) -> ArmStats {
        ArmStats {
            pulls: self.count,
            mean_reward: self.alpha / (self.alpha + self.beta),
            is_active: self.is_active,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThomsonSampling {
    discount_factor: f64,
    draw_history: HashMap<DrawHistoryElement, u128>,
    arms: HashMap<usize, ThomsonSamplingArm>,
    rng: MaybeSeededRng,
}

impl ThomsonSampling {
    pub fn new(discount_factor: Option<f64>, seed: Option<u64>) -> Self {
        Self {
            discount_factor: discount_factor.unwrap_or(1.0),
            draw_history: HashMap::new(),
            arms: HashMap::new(),
            rng: MaybeSeededRng::new(seed),
        }
    }
}

impl CloneBoxedPolicy for ThomsonSampling {
    fn clone_box(&self) -> Box<dyn Policy + Send> {
        Box::new(self.clone())
    }
}

#[typetag::serde]
impl Policy for ThomsonSampling {
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
        let mut arm = ThomsonSamplingArm::default();
        arm.reset(Some(initial_reward), Some(initial_count));
        self.arms.insert(arm_id, arm);

        arm_id
    }

    fn delete_arm(&mut self, arm_id: usize) -> Result<(), PolicyError> {
        self.arms
            .remove(&arm_id)
            .ok_or(PolicyError::ArmNotFound(arm_id))?;
        Ok(())
    }

    fn draw(&mut self) -> Result<DrawResult, PolicyError> {
        let draw_id = Uuid::new_v4();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        // sample from the beta distribution for each arm and select the arm with the best statistic
        let arm_id = self
            .arms
            .iter()
            .filter(|(_, arm)| arm.is_active)
            .flat_map(|(&arm_id, arm)| {
                let sample = Beta::new(arm.alpha, arm.beta)
                    .map_err(|_| PolicyError::SamplingError(arm_id))?
                    .sample(&mut self.rng.get_rng());
                Ok::<(usize, f64), PolicyError>((arm_id, sample))
            })
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .map(|(arm_id, _)| arm_id)
            .ok_or(PolicyError::NoArmsAvailable)?;

        self.draw_history.insert((draw_id, arm_id), timestamp);

        Ok(DrawResult {
            timestamp,
            draw_id,
            arm_id,
        })
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

    const SEED: u64 = 1234;

    #[test]
    fn create_arm() {
        let mut policy = ThomsonSampling::new(None, Some(SEED));
        assert!(policy.arms.len() == 0);

        let arm_id = policy.add_arm(0.0, 0);
        assert!(policy.arms.contains_key(&arm_id))
    }

    #[test]
    fn delete_arm() {
        let mut policy = ThomsonSampling::new(None, Some(SEED));
        let arm_id = policy.add_arm(0.0, 0);
        assert!(policy.delete_arm(arm_id).is_ok());
        assert!(!policy.arms.contains_key(&arm_id));
        assert!(policy.delete_arm(arm_id).is_err());
    }

    #[test]
    fn draw() {
        let mut policy = ThomsonSampling::new(None, Some(SEED));
        let arm_id = policy.add_arm(0.0, 0);
        let result = policy.draw().ok().map(|DrawResult { arm_id, .. }| arm_id);
        assert_eq!(result, Some(arm_id));
    }

    #[test]
    fn draw_best() {
        let mut policy = ThomsonSampling::new(None, Some(SEED));
        let arm_1 = policy.add_arm(0.0, 0);
        let _ = policy.add_arm(0.0, 0);

        policy.arms.get_mut(&arm_1).map(|arm| arm.alpha += 1.0);
        let result = policy.draw().ok().map(|DrawResult { arm_id, .. }| arm_id);
        assert_eq!(result, Some(arm_1));
    }

    #[test]
    fn draw_empty() {
        let mut policy = ThomsonSampling::new(None, Some(SEED));
        assert!(policy.draw().is_err());
    }

    #[test]
    fn update() {
        let mut policy = ThomsonSampling::new(None, Some(SEED));
        let _ = policy.add_arm(0.0, 0);
        let _ = policy.add_arm(0.0, 0);

        let DrawResult {
            draw_id,
            timestamp,
            arm_id,
        } = policy.draw().unwrap();

        assert!(policy.update(draw_id, timestamp + 1, arm_id, 1.0).is_ok());
        assert_eq!(policy.arms.get(&arm_id).map(|arm| arm.alpha), Some(2.0));
        assert_eq!(policy.arms.get(&arm_id).map(|arm| arm.beta), Some(1.0));
    }

    #[test]
    fn update_batch() {
        let mut policy = ThomsonSampling::new(None, Some(SEED));
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
    }
}
