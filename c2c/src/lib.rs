use bity_ic_canister_client::generate_candid_c2c_call;
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

generate_candid_c2c_call!(get_storage_size);
generate_candid_c2c_call!(init_upload);
generate_candid_c2c_call!(store_chunk);
generate_candid_c2c_call!(finalize_upload);
generate_candid_c2c_call!(cancel_upload);
