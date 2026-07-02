use crate::state::read_state;

pub use bity_ic_storage_canister_api::queries::get_storage_size::{
    Args as GetStorageSizeArgs, Response as GetStorageSizeResponse,
};
pub use bity_ic_storage_canister_api::queries::get_stored_files_size_bytes::{
    Args as GetStoredFilesSizeBytesArgs, Response as GetStoredFilesSizeBytesResponse,
};

use ic_cdk::query;

#[query]
async fn get_storage_size(_: GetStorageSizeArgs) -> GetStorageSizeResponse {
    read_state(|s| s.data.storage.get_storage_size_bytes())
}

#[query]
async fn get_stored_files_size_bytes(
    _: GetStoredFilesSizeBytesArgs,
) -> GetStoredFilesSizeBytesResponse {
    read_state(|s| s.data.storage.get_stored_files_size_bytes())
}
