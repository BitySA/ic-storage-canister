use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum UploadState {
    Init,
    ReuploadInit,
    InProgress,
    Finalized,
}
