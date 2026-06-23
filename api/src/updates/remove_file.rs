use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, CandidType, Clone, Debug)]
pub struct Args {
    pub file_path: String,
}

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub struct RemoveFileResp {}

pub type Response = Result<RemoveFileResp, RemoveFileError>;

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub enum RemoveFileError {
    UploadNotInitialized,
    InvalidFilePath,
}
