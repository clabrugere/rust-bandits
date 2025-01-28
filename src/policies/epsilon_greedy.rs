use super::arm::{Arm, Arms};
use super::bandit::{Bandit, BanditError, BanditStats};
use rand::{rngs::SmallRng, seq::IteratorRandom, Rng, SeedableRng};

pub struct EpsilonGreedy {
    arms: Arms,
    epsilon: f64,
    rng: SmallRng,
}

impl EpsilonGreedy {
    pub fn new(epsilon: f64, seed: Option<u64>) -> Self {
        let rng = if let Some(seed) = seed {
            SmallRng::seed_from_u64(seed)
        } else {
            SmallRng::from_entropy()
        };

        Self {
            arms: Arms::new(),
            epsilon,
            rng,
        }
    }
}

impl Bandit for EpsilonGreedy {
    fn reset(&mut self) {
        self.arms.values_mut().for_each(|arm| arm.reset());
    }

    fn add_arm(&mut self) -> usize {
        let arm_id = self.arms.len();
        self.arms.insert(arm_id, Arm::default());
        arm_id
    }

    fn delete_arm(&mut self, arm_id: usize) -> Result<(), BanditError> {
        if self.arms.contains_key(&arm_id) {
            self.arms.remove(&arm_id);
            Ok(())
        } else {
            Err(BanditError::ArmNotFound(arm_id))
        }
    }

    fn draw(&mut self) -> Result<usize, BanditError> {
        if self.rng.gen::<f64>() < self.epsilon {
            self.arms
                .iter()
                .filter(|(_, arm)| arm.is_active)
                .map(|(&arm_id, _)| arm_id)
                .choose(&mut self.rng)
                .ok_or(BanditError::NoArmsAvailable)
        } else {
            self.arms
                .iter()
                .filter(|(_, arm)| arm.is_active)
                .max_by(|(_, a), (_, b)| a.cmp(b))
                .map(|(&k, _)| k)
                .ok_or(BanditError::NoArmsAvailable)
        }
    }

    fn update(&mut self, arm_id: usize, reward: f64) -> Result<(), BanditError> {
        if let Some(arm) = self.arms.get_mut(&arm_id) {
            arm.pulls += 1;
            arm.rewards += reward;
            arm.value += (1.0 / arm.pulls as f64) * (reward - arm.value);

            Ok(())
        } else {
            Err(BanditError::ArmNotFound(arm_id))
        }
    }

    fn stats(&self) -> BanditStats {
        BanditStats::from(&self.arms)
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
