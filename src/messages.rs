use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct Response {
    pub request_id: Uuid,
    pub result: ResponseResult,
    pub status: ReponseStatus,
}

#[derive(Debug, Deserialize)]
pub struct ResponseResult {
    pub data: Value,
}

#[derive(Debug, Deserialize)]
pub struct ReponseStatus {
    pub code: i16,
    pub message: String,
}