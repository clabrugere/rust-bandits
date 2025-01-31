use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Serialize)]
pub struct ApiResponse<T> {
    request_id: Uuid,
    ts: u128,
    body: Option<T>,
}

impl<T> Default for ApiResponse<T> {
    fn default() -> Self {
        Self {
            request_id: Uuid::new_v4(),
            ts: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            body: None,
        }
    }
}

impl<T: Serialize> ApiResponse<T> {
    pub fn with_data(data: Option<T>) -> Self {
        Self {
            request_id: Uuid::new_v4(),
            ts: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            body: data,
        }
    }
}

#[derive(Serialize)]
pub struct ListBanditsResponse {
    pub bandit_ids: Vec<Uuid>,
}

#[derive(Serialize)]
pub struct CreateResponse {
    pub bandit_id: Uuid,
}

#[derive(Serialize)]
pub struct AddArmResponse {
    pub arm_id: usize,
}

#[derive(Serialize)]
pub struct DrawResponse {
    pub arm_id: usize,
}
