use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AddArmPayload {
    pub initial_reward: Option<f64>,
    pub initial_count: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ResetArmPayload {
    pub reward: Option<f64>,
    pub count: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePayload {
    pub timestamp: u128,
    pub arm_id: usize,
    pub reward: f64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBatchPayload {
    pub updates: Vec<UpdatePayload>,
}
