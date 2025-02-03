use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UpdatePayload {
    pub ts: u64,
    pub arm_id: usize,
    pub reward: f64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBatchPayload {
    pub updates: Vec<UpdatePayload>,
}
