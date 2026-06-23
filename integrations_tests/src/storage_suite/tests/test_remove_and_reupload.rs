use crate::client::storage::get_storage_size;
use crate::client::storage::{
    finalize_upload, get_memory, http_request, init_reupload, init_upload, remove_file, store_chunk,
};
use bity_ic_storage_canister_api::updates::{
    finalize_upload, init_reupload, init_upload, remove_file, store_chunk,
};
use candid::Nat;
use ic_http_certification::{HttpRequest, StatusCode};
use sha2::{Digest, Sha256};

use crate::storage_suite::setup::default_test_setup;
use crate::storage_suite::setup::setup::TestEnv;

fn upload_custom_file(
    pic: &mut pocket_ic::PocketIc,
    controller: candid::Principal,
    storage_canister_id: candid::Principal,
    upload_path: &str,
    content: &[u8],
) {
    let file_size = content.len() as u64;
    let mut hasher = Sha256::new();
    hasher.update(content);
    let file_hash = format!("{:x}", hasher.finalize());

    init_upload(
        pic,
        controller,
        storage_canister_id,
        &(init_upload::Args {
            file_path: upload_path.to_string(),
            file_hash,
            file_size,
            chunk_size: None,
        }),
    )
    .expect("init_upload failed");

    store_chunk(
        pic,
        controller,
        storage_canister_id,
        &(store_chunk::Args {
            file_path: upload_path.to_string(),
            chunk_id: Nat::from(0u64),
            chunk_data: content.to_vec(),
        }),
    )
    .expect("store_chunk failed");

    finalize_upload(
        pic,
        controller,
        storage_canister_id,
        &(finalize_upload::Args {
            file_path: upload_path.to_string(),
        }),
    )
    .expect("finalize_upload failed");
}

#[test]
fn test_init_reupload() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        ..
    } = test_env;

    let upload_path = "/reupload_test.txt";

    // 1. Reupload file that does not exist should fail
    let reupload_nonexistent_resp = init_reupload(
        pic,
        controller,
        storage_canister_id,
        &(init_reupload::Args {
            file_path: upload_path.to_string(),
            file_hash: "00".repeat(32),
            file_size: 100,
            chunk_size: None,
        }),
    );
    match reupload_nonexistent_resp {
        Err(init_reupload::InitReuploadError::FileNotFound) => {}
        other => panic!("Expected FileNotFound, got {:?}", other),
    }

    // 2. Create the file first
    let original_content = b"Original Content".to_vec();
    upload_custom_file(
        pic,
        controller,
        storage_canister_id,
        upload_path,
        &original_content,
    );

    // 3. Reupload with incorrect size should fail
    let incorrect_size = (original_content.len() + 5) as u64;
    let reupload_incorrect_size_resp = init_reupload(
        pic,
        controller,
        storage_canister_id,
        &(init_reupload::Args {
            file_path: upload_path.to_string(),
            file_hash: "00".repeat(32),
            file_size: incorrect_size,
            chunk_size: None,
        }),
    );
    match reupload_incorrect_size_resp {
        Err(init_reupload::InitReuploadError::FileSizeMismatch) => {}
        other => panic!("Expected FileSizeMismatch, got {:?}", other),
    }

    // 4. Reupload with correct size should succeed
    let new_content = b"Modified Content".to_vec(); // Same size (16 bytes)
    assert_eq!(original_content.len(), new_content.len());

    let mut hasher = Sha256::new();
    hasher.update(&new_content);
    let new_hash = format!("{:x}", hasher.finalize());

    let reupload_success_resp = init_reupload(
        pic,
        controller,
        storage_canister_id,
        &(init_reupload::Args {
            file_path: upload_path.to_string(),
            file_hash: new_hash,
            file_size: new_content.len() as u64,
            chunk_size: None,
        }),
    );
    assert!(
        reupload_success_resp.is_ok(),
        "Expected Ok response for reupload initialization"
    );

    // 5. Store the new chunk and finalize the reupload
    store_chunk(
        pic,
        controller,
        storage_canister_id,
        &(store_chunk::Args {
            file_path: upload_path.to_string(),
            chunk_id: Nat::from(0u64),
            chunk_data: new_content.clone(),
        }),
    )
    .expect("store_chunk for reupload failed");

    finalize_upload(
        pic,
        controller,
        storage_canister_id,
        &(finalize_upload::Args {
            file_path: upload_path.to_string(),
        }),
    )
    .expect("finalize_upload for reupload failed");

    // 6. Verify that the file content was updated to the new content
    let req = HttpRequest::get(upload_path)
        .with_headers(vec![(
            "host".to_string(),
            format!("{}.raw.icp0.io", storage_canister_id),
        )])
        .build();
    let resp = http_request(pic, controller, storage_canister_id, &req);
    assert_eq!(resp.status_code(), StatusCode::OK);
    assert_eq!(resp.body(), &new_content);
}

#[test]
fn test_remove_file() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        ..
    } = test_env;

    let upload_path = "/remove_test.txt";

    // 1. Removing a file that does not exist should fail
    let remove_nonexistent_resp = remove_file(
        pic,
        controller,
        storage_canister_id,
        &(remove_file::Args {
            file_path: upload_path.to_string(),
        }),
    );
    match remove_nonexistent_resp {
        Err(remove_file::RemoveFileError::UploadNotInitialized) => {}
        other => panic!("Expected UploadNotInitialized, got {:?}", other),
    }

    // 2. Get initial memory usage
    let initial_memory = get_memory(pic, controller, storage_canister_id, &());

    // 3. Create the file (1000 bytes)
    let content = vec![65u8; 1000];
    upload_custom_file(pic, controller, storage_canister_id, upload_path, &content);

    // 4. Verify memory usage increased
    let memory_after_upload = get_memory(pic, controller, storage_canister_id, &());
    assert!(
        memory_after_upload > initial_memory,
        "Memory usage should increase after uploading a file. Before: {}, After: {}",
        initial_memory,
        memory_after_upload
    );

    // 5. Remove the file
    let remove_resp = remove_file(
        pic,
        controller,
        storage_canister_id,
        &(remove_file::Args {
            file_path: upload_path.to_string(),
        }),
    );
    assert!(remove_resp.is_ok(), "remove_file failed");

    // 6. Verify memory usage is lower than after upload
    let memory_after_remove = get_memory(pic, controller, storage_canister_id, &());
    assert!(
        memory_after_remove < memory_after_upload,
        "Memory usage should decrease after removing a file. Before remove: {}, After remove: {}",
        memory_after_upload,
        memory_after_remove
    );
    assert_eq!(
        memory_after_remove, initial_memory,
        "Memory usage should return to initial state"
    );

    // 7. Try to retrieve the removed file, it should return 404
    let req = HttpRequest::get(upload_path)
        .with_headers(vec![(
            "host".to_string(),
            format!("{}.raw.icp0.io", storage_canister_id),
        )])
        .build();
    let resp = http_request(pic, controller, storage_canister_id, &req);
    assert_eq!(resp.status_code(), StatusCode::NOT_FOUND);
}

#[test]
fn test_remove_file_and_upload_another() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        ..
    } = test_env;

    let upload_path_1 = "/remove_test.txt";

    // 2. Get initial memory usage
    let initial_memory = get_memory(pic, controller, storage_canister_id, &());

    // 3. Create the file (1000 bytes)
    let content_1 = vec![65u8; 1000];
    upload_custom_file(
        pic,
        controller,
        storage_canister_id,
        upload_path_1,
        &content_1,
    );

    // 4. Verify memory usage increased
    let memory_after_upload = get_memory(pic, controller, storage_canister_id, &());
    let stable_memory_after_upload = get_storage_size(pic, controller, storage_canister_id, &());
    assert!(
        memory_after_upload > initial_memory,
        "Memory usage should increase after uploading a file. Before: {}, After: {}",
        initial_memory,
        memory_after_upload
    );

    // 5. Remove the file
    let remove_resp = remove_file(
        pic,
        controller,
        storage_canister_id,
        &(remove_file::Args {
            file_path: upload_path_1.to_string(),
        }),
    );
    assert!(remove_resp.is_ok(), "remove_file failed");

    // 6. Verify memory usage is lower than after upload
    let memory_after_remove = get_memory(pic, controller, storage_canister_id, &());
    let stable_memory_after_remove = get_storage_size(pic, controller, storage_canister_id, &());
    assert!(
        memory_after_remove < memory_after_upload,
        "Memory usage should decrease after removing a file. Before remove: {}, After remove: {}",
        memory_after_upload,
        memory_after_remove
    );
    assert_eq!(
        memory_after_remove, initial_memory,
        "Memory usage should return to initial state"
    );

    assert!(
        stable_memory_after_remove == stable_memory_after_upload,
        "Stable Memory usage should stay the same after removing a file. Before remove: {}, After remove: {}",
        stable_memory_after_upload,
        stable_memory_after_remove
    );

    let content_2 = vec![66u8; 1000];
    let upload_path_2 = "/remove_test_2.txt";
    upload_custom_file(
        pic,
        controller,
        storage_canister_id,
        upload_path_2,
        &content_2,
    );

    let memory_after_new_upload = get_memory(pic, controller, storage_canister_id, &());
    let stable_memory_after_new_upload =
        get_storage_size(pic, controller, storage_canister_id, &());
    assert!(
        memory_after_upload == memory_after_new_upload,
        "Memory usage should stay the same for bothfiles. After upload: {}, After new upload (with prev file removal): {}",
        memory_after_upload,
        memory_after_new_upload,
    );
    assert!(
        stable_memory_after_remove == stable_memory_after_new_upload,
        "Stable Memory usage should stay the same after uploading a new file. After new upload: {}, Before new upload: {}",
        stable_memory_after_remove, stable_memory_after_new_upload
    );
}
