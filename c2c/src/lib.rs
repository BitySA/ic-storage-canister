use bity_ic_storage_canister_api::cancel_upload;
use bity_ic_storage_canister_api::finalize_upload;
use bity_ic_storage_canister_api::get_storage_size::{
    Args as GetStorageSizeArgs, Response as GetStorageSizeResponse,
};
use bity_ic_storage_canister_api::init_upload;
use bity_ic_storage_canister_api::store_chunk;

pub mod get_storage_size {
    use super::*;
    pub type Args = GetStorageSizeArgs;
    pub type Response = GetStorageSizeResponse;
}

pub async fn get_storage_size(
    canister_id: candid::Principal,
    args: get_storage_size::Args,
) -> Result<get_storage_size::Response, String> {
    let response = ic_cdk::call::Call::unbounded_wait(canister_id, "get_storage_size")
        .with_arg(args)
        .await
        .map_err(|e| format!("Call failed: {:?}", e))?;

    response
        .candid::<get_storage_size::Response>()
        .map_err(|e| format!("Failed to decode response: {:?}", e))
}

pub async fn init_upload(
    canister_id: candid::Principal,
    args: init_upload::Args,
) -> Result<init_upload::Response, String> {
    let response = ic_cdk::call::Call::unbounded_wait(canister_id, "init_upload")
        .with_arg(args)
        .await
        .map_err(|e| format!("Call failed: {:?}", e))?;

    response
        .candid::<init_upload::Response>()
        .map_err(|e| format!("Failed to decode response: {:?}", e))
}

pub async fn store_chunk(
    canister_id: candid::Principal,
    args: store_chunk::Args,
) -> Result<store_chunk::Response, String> {
    let response = ic_cdk::call::Call::unbounded_wait(canister_id, "store_chunk")
        .with_arg(args)
        .await
        .map_err(|e| format!("Call failed: {:?}", e))?;

    response
        .candid::<store_chunk::Response>()
        .map_err(|e| format!("Failed to decode response: {:?}", e))
}

pub async fn finalize_upload(
    canister_id: candid::Principal,
    args: finalize_upload::Args,
) -> Result<finalize_upload::Response, String> {
    let response = ic_cdk::call::Call::unbounded_wait(canister_id, "finalize_upload")
        .with_arg(args)
        .await
        .map_err(|e| format!("Call failed: {:?}", e))?;

    response
        .candid::<finalize_upload::Response>()
        .map_err(|e| format!("Failed to decode response: {:?}", e))
}

pub async fn cancel_upload(
    canister_id: candid::Principal,
    args: cancel_upload::Args,
) -> Result<cancel_upload::Response, String> {
    let response = ic_cdk::call::Call::unbounded_wait(canister_id, "cancel_upload")
        .with_arg(args)
        .await
        .map_err(|e| format!("Call failed: {:?}", e))?;

    response
        .candid::<cancel_upload::Response>()
        .map_err(|e| format!("Failed to decode response: {:?}", e))
}
