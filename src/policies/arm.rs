use serde::Serialize;

pub trait Arm: Default + Eq + PartialEq + Ord + PartialOrd {
    fn reset(&mut self);
    fn update(&mut self, reward: f64);
    fn stats(&self) -> ArmStats;
}

#[derive(Clone, Debug, Serialize)]
pub struct ArmStats {
    pub pulls: u64,
    pub mean_reward: f64,
    pub is_active: bool,
}
