use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, CandidType, Clone, Debug)]
pub struct Args {
    pub file_path: String,
    pub chunk_id: Nat,
    pub chunk_data: Vec<u8>,
}

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub struct StoreChunkResp {}

pub type Response = Result<StoreChunkResp, StoreChunkError>;

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub enum StoreChunkError {
    UploadNotInitialized,
    UploadAlreadyFinalized,
    InvalidChunkId,
    InvalidChunkData,
    InvalidFilePath,
    InvalidFileSize,
    InvalidFileHash,
    InvalidFileFormat,
}
