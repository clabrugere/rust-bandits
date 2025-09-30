use serde::Serialize;

pub trait Arm: Default + Eq + PartialEq + Ord + PartialOrd {
    fn reset(&mut self, reward: Option<f64>, count: Option<u64>);
    fn update(&mut self, reward: f64, discount: Option<f64>);
    fn stats(&self) -> ArmStats;
}

#[derive(Clone, Debug, Serialize)]
pub struct ArmStats {
    pub pulls: u64,
    pub mean_reward: f64,
    pub is_active: bool,
}
