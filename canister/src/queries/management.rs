use crate::state::read_state;

pub use bity_ic_storage_canister_api::queries::get_storage_size::{
    Args as GetStorageSizeArgs, Response as GetStorageSizeResponse,
};
pub use bity_ic_storage_canister_api::queries::get_memory::{
    Args as GetMemoryArgs, Response as GetMemoryResponse,
};

use ic_cdk::query;

#[query]
async fn get_storage_size(_: GetStorageSizeArgs) -> GetStorageSizeResponse {
    read_state(|s| s.data.storage.get_storage_size_bytes())
}

#[query]
async fn get_memory(_: GetMemoryArgs) -> GetMemoryResponse {
    read_state(|s| s.data.storage.get_memory())
}

