use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UpdatePayload {
    pub ts: usize,
    pub arm_id: usize,
    pub reward: f64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBatchPayload {
    pub updates: Vec<UpdatePayload>,
}
