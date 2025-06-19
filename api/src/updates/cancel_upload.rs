use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, CandidType, Clone, Debug)]
pub struct Args {
    pub file_path: String,
}

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub struct CancelUploadResp {}

pub type Response = Result<CancelUploadResp, CancelUploadError>;

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub enum CancelUploadError {
    UploadNotInitialized,
}
