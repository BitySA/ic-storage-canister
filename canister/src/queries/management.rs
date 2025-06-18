use crate::state::read_state;

pub use bity_ic_storage_canister_api::queries::get_storage_size::{Args, Response};

use ic_cdk::query;

#[query]
async fn get_storage_size(_: Args) -> Response {
    read_state(|s| s.data.storage.get_storage_size_bytes())
}
