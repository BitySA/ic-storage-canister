use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, CandidType, Clone, Debug)]
pub struct Args {
    pub file_path: String,
    /// Expected SHA-256 of the whole file, hex-encoded, verified at
    /// `finalize_upload`.
    ///
    /// `None`: the canister does not verify a hash at finalize; the caller
    /// vouches for integrity by other means (the minting gateway uses
    /// per-chunk authenticated encryption, so it cannot know the ciphertext
    /// digest before streaming the file).
    pub file_hash: Option<String>,
    pub file_size: u64,
    pub chunk_size: Option<u64>,
}

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub struct InitUploadResp {}

pub type Response = Result<InitUploadResp, InitUploadError>;

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub enum InitUploadError {
    ConcurrentManagementCall,
    FileAlreadyExists,
    NotEnoughStorage,
    InvalidChunkSize,
    InvalidFilePath,
    TooManyChunks,
    TooManyFiles,
}
