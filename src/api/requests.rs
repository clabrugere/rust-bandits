use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct AddArmPayload {
    pub initial_reward: Option<f64>,
    pub initial_count: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ResetArmPayload {
    pub cumulative_reward: Option<f64>,
    pub count: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(super) struct UpdatePayload {
    pub timestamp: f64,
    pub arm_id: usize,
    pub reward: f64,
}

#[derive(Debug, Deserialize)]
pub(super) struct UpdateBatchPayload {
    pub updates: Vec<UpdatePayload>,
}
