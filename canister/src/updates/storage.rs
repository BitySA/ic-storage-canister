use crate::guards::caller_is_governance_principal;
use crate::state::mutate_state;
pub use bity_ic_storage_canister_api::cancel_upload;
pub use bity_ic_storage_canister_api::finalize_upload;
pub use bity_ic_storage_canister_api::init_upload;
pub use bity_ic_storage_canister_api::store_chunk;
use ic_cdk::update;

#[update(guard = "caller_is_governance_principal")]
pub fn init_upload(data: init_upload::Args) -> init_upload::Response {
    match mutate_state(|state| state.data.init_upload(data)) {
        Ok(_) => Ok(init_upload::InitUploadResp {}),
        Err(e) => Err(e),
    }
}

#[update(guard = "caller_is_governance_principal")]
pub fn store_chunk(data: store_chunk::Args) -> store_chunk::Response {
    match mutate_state(|state| state.data.store_chunk(data)) {
        Ok(_) => Ok(store_chunk::StoreChunkResp {}),
        Err(e) => Err(e),
    }
}

#[update(guard = "caller_is_governance_principal")]
pub fn finalize_upload(data: finalize_upload::Args) -> finalize_upload::Response {
    match mutate_state(|state| state.data.finalize_upload(data)) {
        Ok(resp) => Ok(resp),
        Err(e) => Err(e),
    }
}

#[update(guard = "caller_is_governance_principal")]
pub fn cancel_upload(data: cancel_upload::Args) -> cancel_upload::Response {
    match mutate_state(|state| state.data.cancel_upload(data.file_path)) {
        Ok(_) => Ok(cancel_upload::CancelUploadResp {}),
        Err(e) => Err(e),
    }
}
