use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, CandidType, Clone, Debug)]
pub struct Args {
    pub file_path: String,
}

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub struct FinalizeUploadResp {
    pub url: String,
}

pub type Response = Result<FinalizeUploadResp, FinalizeUploadError>;

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub enum FinalizeUploadError {
    UploadNotStarted,
    UploadAlreadyFinalized,
    IncompleteUpload,
    FileSizeMismatch,
    FileHashMismatch,
}
