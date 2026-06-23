use crate::{generate_pocket_query_call, generate_pocket_update_call};

use bity_ic_storage_canister_api::queries::{get_storage_size, http_request, get_memory};
use bity_ic_storage_canister_api::updates::{
    cancel_upload, finalize_upload, init_reupload, init_upload, remove_file, store_chunk,
};

generate_pocket_query_call!(get_storage_size);
generate_pocket_query_call!(http_request);
generate_pocket_query_call!(get_memory);

generate_pocket_update_call!(init_upload);
generate_pocket_update_call!(init_reupload);
generate_pocket_update_call!(store_chunk);
generate_pocket_update_call!(finalize_upload);
generate_pocket_update_call!(cancel_upload);
generate_pocket_update_call!(remove_file);
