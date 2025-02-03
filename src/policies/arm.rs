use serde::Serialize;
use std::collections::HashMap;

pub type Arms<A: Arm> = HashMap<usize, A>;

pub trait Arm: Default + Eq + PartialEq + Ord + PartialOrd {
    fn reset(&mut self);
    fn stats(&self) -> ArmStats;
}

#[derive(Debug, Serialize)]
pub struct ArmStats {
    pub pulls: u64,
    pub rewards: f64,
    pub is_active: bool,
}
