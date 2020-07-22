use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) enum Response {
    Close(String),
}

// #[derive(Debug, Deserialize)]
// pub struct Response {
//     pub sequence: Uuid,
//     pub result: ResponseResult,
//     pub status: ReponseStatus,
// }

// #[derive(Debug, Deserialize)]
// pub struct ResponseResult {
//     pub data: Value,
// }

// #[derive(Debug, Deserialize)]
// pub struct ReponseStatus {
//     pub code: i16,
//     pub message: String,
// }
