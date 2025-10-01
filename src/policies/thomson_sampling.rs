use super::arm::{Arm, ArmStats};
use super::errors::PolicyError;
use super::policy::{BatchUpdateElement, CloneBoxedPolicy, DrawResult, Policy, PolicyStats};
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
    discount_factor: f64,
    last_ts: Option<u128>,
    is_active: bool,
}

impl ThomsonSamplingArm {
    fn new(reward: f64, count: u64, discount_factor: f64) -> Self {
        Self {
            alpha: reward + 1.0,
            beta: (count as f64) - reward + 1.0,
            count,
            discount_factor,
            last_ts: None,
            is_active: true,
        }
    }

    fn apply_discount(&mut self, timestamp: u128) {
        if let Some(last_ts) = self.last_ts {
            let dt = (timestamp - last_ts) as f64;
            if dt != 0.0 {
                let discount = (self.discount_factor.ln() * dt).exp();
                self.alpha *= discount;
                self.beta *= discount;
            }
        }
        self.last_ts = Some(timestamp);
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
        self.last_ts = None;
    }

    fn update(&mut self, reward: f64, timestamp: u128) {
        self.apply_discount(timestamp);

        // update alpha and beta
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
    arms: HashMap<usize, ThomsonSamplingArm>,
    rng: MaybeSeededRng,
}

impl ThomsonSampling {
    pub fn new(discount_factor: Option<f64>, seed: Option<u64>) -> Self {
        Self {
            discount_factor: discount_factor.unwrap_or(1.0),
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
        self.arms.insert(
            arm_id,
            ThomsonSamplingArm::new(initial_reward, initial_count, self.discount_factor),
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
        let draw_id = Uuid::new_v4();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        // apply discount to all arms
        self.arms
            .values_mut()
            .for_each(|arm| arm.apply_discount(timestamp));

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

        Ok(DrawResult {
            timestamp,
            draw_id,
            arm_id,
        })
    }

    fn update(&mut self, timestamp: u128, arm_id: usize, reward: f64) -> Result<(), PolicyError> {
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

        policy.arms.get_mut(&arm_1).map(|arm| arm.alpha += 100.0);
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
            timestamp, arm_id, ..
        } = policy.draw().unwrap();

        assert!(policy.update(timestamp + 1, arm_id, 1.0).is_ok());
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
            .map(|draw| (draw.timestamp + 1, draw.arm_id, 1.0))
            .collect::<Vec<(u128, usize, f64)>>();

        assert!(policy.update_batch(&updates).is_ok());
    }
}
