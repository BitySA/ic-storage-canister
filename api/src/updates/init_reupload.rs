use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, CandidType, Clone, Debug)]
pub struct Args {
    pub file_path: String,
    pub file_hash: String,
    pub file_size: u64,
    pub chunk_size: Option<u64>,
}

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub struct InitReuploadResp {}

pub type Response = Result<InitReuploadResp, InitReuploadError>;

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub enum InitReuploadError {
    FileNotFound,
    FileSizeMismatch,
    NotEnoughStorage,
    InvalidChunkSize,
    InvalidFilePath,
    TooManyChunks,
    TooManyFiles,
}
