use super::policy::{
    get_timestamp, ArmStats, BatchUpdateElement, CloneBoxedPolicy, DrawResult, Policy, PolicyStats,
    PolicyType,
};
use super::rng::MaybeSeededRng;

use crate::errors::PolicyError;

use rand::Rng;
use rand_distr::Beta;
use rand_distr::Distribution;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

const EPS: f64 = 1e-6;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ThomsonSamplingArm {
    alpha: f64,
    beta: f64,
    count: u64,
    halflife_seconds: Option<f64>,
    last_ts: f64,
    is_active: bool,
}

impl ThomsonSamplingArm {
    fn new(initial_reward: f64, initial_count: u64, halflife_seconds: Option<f64>) -> Self {
        Self {
            alpha: 1.0 + initial_reward,
            beta: 1.0 + (initial_count as f64) - initial_reward,
            count: initial_count,
            halflife_seconds,
            last_ts: get_timestamp(),
            is_active: true,
        }
    }

    fn reset(&mut self, cumulative_reward: Option<f64>, count: Option<u64>) {
        if let (Some(cumulative_reward), Some(count)) = (cumulative_reward, count) {
            self.alpha = cumulative_reward + 1.0;
            self.beta = (count as f64) - cumulative_reward + 1.0;
            self.count = count;
        } else {
            self.alpha = 1.0;
            self.beta = 1.0;
            self.count = 0;
        }
        self.last_ts = get_timestamp();
    }

    // apply an exponential decay d = exp(dt * ln2 / h)
    fn decay_weight(&self, timestamp: f64) -> f64 {
        if let Some(h) = self.halflife_seconds {
            let dt = timestamp - self.last_ts;
            (-dt * std::f64::consts::LN_2 / h).exp()
        } else {
            1.0
        }
    }

    fn apply_discount(&mut self, timestamp: f64) {
        let decay = self.decay_weight(timestamp);
        self.alpha = (self.alpha * decay).max(EPS);
        self.beta = (self.beta * decay).max(EPS);
        self.last_ts = timestamp;
    }

    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Result<f64, PolicyError> {
        let s = Beta::new(self.alpha, self.beta)
            .map_err(|e| PolicyError::SamplingError(e.to_string()))?
            .sample(rng);

        Ok(s)
    }

    fn update(&mut self, reward: f64, timestamp: f64) {
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
    halflife_seconds: Option<f64>,
    arms: HashMap<usize, ThomsonSamplingArm>,
    rng: MaybeSeededRng,
}

impl ThomsonSampling {
    // Decayed Thomson Sampling using halflife, after which past evidence is halved
    pub fn new(halflife_seconds: Option<f64>, seed: Option<u64>) -> Self {
        Self {
            halflife_seconds,
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
    fn policy_type(&self) -> PolicyType {
        PolicyType::ThomsonSampling {
            halflife_seconds: self.halflife_seconds,
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

    fn add_arm(&mut self, initial_reward: f64, initial_count: u64) -> usize {
        let arm_id = self.arms.len();
        self.arms.insert(
            arm_id,
            ThomsonSamplingArm::new(initial_reward, initial_count, self.halflife_seconds),
        );

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

        // apply discount to all arms
        self.arms
            .values_mut()
            .filter(|arm| arm.is_active)
            .for_each(|arm| arm.apply_discount(timestamp));

        // sample from the beta distribution for each arm and select the arm with the best statistic
        let arm_id = self
            .arms
            .iter()
            .filter(|(_, arm)| arm.is_active)
            .filter_map(|(arm_id, arm)| match arm.sample(self.rng.get_rng()) {
                Ok(sample) => Some((arm_id, sample)),
                Err(_) => None,
            })
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .map(|(&arm_id, _)| arm_id)
            .ok_or(PolicyError::NoArmsAvailable)?;

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
    const DEFAULT_SEED: Option<u64> = Some(1234);

    fn make_policy() -> ThomsonSampling {
        ThomsonSampling::new(None, DEFAULT_SEED)
    }

    #[test]
    fn create_arm() {
        let mut policy = make_policy();
        assert!(policy.arms.len() == 0);

        let arm_id = policy.add_arm(0.0, 0);
        assert!(policy.arms.contains_key(&arm_id))
    }

    #[test]
    fn disable_arm() {
        let mut policy = make_policy();
        let arm_id = policy.add_arm(0.0, 0);

        assert!(policy.disable_arm(arm_id).is_ok());
        assert_eq!(
            policy.arms.iter().filter(|(_, arm)| arm.is_active).count(),
            0
        );
    }

    #[test]
    fn enable_arm() {
        let mut policy = make_policy();
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
        let mut policy = make_policy();
        let arm_id = policy.add_arm(0.0, 0);
        assert!(policy.delete_arm(arm_id).is_ok());
        assert!(!policy.arms.contains_key(&arm_id));
        assert!(policy.delete_arm(arm_id).is_err());
    }

    #[test]
    fn draw() {
        let mut policy = make_policy();
        let arm_id = policy.add_arm(0.0, 0);
        let result = policy.draw().ok().map(|DrawResult { arm_id, .. }| arm_id);
        assert_eq!(result, Some(arm_id));
    }

    #[test]
    fn draw_best() {
        let mut policy = make_policy();
        let arm_1 = policy.add_arm(0.0, 0);
        let _ = policy.add_arm(0.0, 0);

        policy.arms.get_mut(&arm_1).map(|arm| arm.alpha += 100.0);
        let result = policy.draw().ok().map(|DrawResult { arm_id, .. }| arm_id);
        assert_eq!(result, Some(arm_1));
    }

    #[test]
    fn draw_empty() {
        let mut policy = make_policy();
        assert!(policy.draw().is_err());
    }

    #[test]
    fn update() {
        let mut policy = make_policy();
        let _ = policy.add_arm(0.0, 0);
        let _ = policy.add_arm(0.0, 0);

        let DrawResult {
            timestamp, arm_id, ..
        } = policy.draw().unwrap();

        assert!(policy.update(timestamp + 1.0, arm_id, 1.0).is_ok());
        println!("{:?}", policy.arms);
        assert_eq!(policy.arms.get(&arm_id).map(|arm| arm.alpha), Some(2.0));
        assert_eq!(policy.arms.get(&arm_id).map(|arm| arm.beta), Some(1.0));
    }

    #[test]
    fn update_batch() {
        let mut policy = make_policy();
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
    }

    #[test]
    fn no_discount() {
        let mut arm = ThomsonSamplingArm {
            alpha: 1.0,
            beta: 1.0,
            count: 0,
            halflife_seconds: None,
            last_ts: 0.0,
            is_active: true,
        };
        arm.apply_discount(1.0); // dt = 1s
        assert!((arm.alpha - 1.0).abs() < EPS);
        assert!((arm.beta - 1.0).abs() < EPS);
    }

    #[test]
    fn no_time_discount() {
        let mut arm = ThomsonSamplingArm {
            alpha: 1.0,
            beta: 1.0,
            count: 0,
            halflife_seconds: Some(60.0),
            last_ts: 0.0,
            is_active: true,
        };
        arm.apply_discount(0.0); // dt = 0s
        assert!((arm.alpha - 1.0).abs() < EPS);
        assert!((arm.beta - 1.0).abs() < EPS);
    }

    #[test]
    fn discount() {
        let mut arm = ThomsonSamplingArm {
            alpha: 1.0,
            beta: 1.0,
            count: 0,
            halflife_seconds: Some(60.0),
            last_ts: 0.0,
            is_active: true,
        };
        arm.apply_discount(60.0); // dt = 60s
        assert!((arm.alpha - 0.5).abs() < EPS);
        assert!((arm.beta - 0.5).abs() < EPS);
    }
}
