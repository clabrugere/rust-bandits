use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashMap};

pub type Arms = HashMap<usize, Arm>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Arm {
    pub(super) value: f64,
    pub(super) pulls: usize,
    pub(super) rewards: f64,
    pub(super) is_active: bool,
}

impl Default for Arm {
    fn default() -> Self {
        Self {
            value: 0.0,
            pulls: 0,
            rewards: 0.0,
            is_active: true,
        }
    }
}

impl Eq for Arm {}

impl PartialEq for Arm {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl PartialOrd for Arm {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Arm {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value
            .partial_cmp(&other.value)
            .unwrap_or(Ordering::Equal)
    }
}

impl Arm {
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.pulls = 0;
        self.rewards = 0.0;
    }
}
