//! An upload that declares no hash finalizes on size and completeness alone;
//! one that declares a wrong hash still fails. This is what lets the gateway
//! encrypt private files in transit: it streams ciphertext chunks it cannot
//! hash ahead of time, so it has no digest to declare at `init_upload`.

use candid::Nat;
use ic_http_certification::{HttpRequest, StatusCode};

use bity_ic_storage_canister_api::{finalize_upload, init_upload, store_chunk};

use crate::client::storage::{finalize_upload, http_request, init_upload, store_chunk};
use crate::storage_suite::setup::default_test_setup;
use crate::storage_suite::setup::setup::TestEnv;
use crate::utils::{init_upload_legacy, LegacyInitUploadArgs};

#[test]
fn upload_without_hash_finalizes_and_serves() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        ..
    } = test_env;

    let file_path = "p/abc";
    // 300 KB in 200 KB chunks: two chunks, the second one short, so the test
    // also covers the received_size bookkeeping finalize now relies on alone.
    let chunk_size: u64 = 200_000;
    let bytes: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();

    let init_resp = init_upload(
        pic,
        controller,
        storage_canister_id,
        &(init_upload::Args {
            file_path: file_path.to_string(),
            file_hash: None,
            file_size: bytes.len() as u64,
            chunk_size: Some(chunk_size),
        }),
    );
    assert!(
        init_resp.is_ok(),
        "init_upload without a declared hash should be accepted, got {init_resp:?}"
    );

    for (index, chunk) in bytes.chunks(chunk_size as usize).enumerate() {
        let store_resp = store_chunk(
            pic,
            controller,
            storage_canister_id,
            &(store_chunk::Args {
                file_path: file_path.to_string(),
                chunk_id: Nat::from(index as u64),
                chunk_data: chunk.to_vec(),
            }),
        );
        assert!(
            store_resp.is_ok(),
            "store_chunk {index} should succeed, got {store_resp:?}"
        );
    }

    let finalize_resp = finalize_upload(
        pic,
        controller,
        storage_canister_id,
        &(finalize_upload::Args {
            file_path: file_path.to_string(),
        }),
    )
    .expect("finalize_upload should succeed when no hash was declared");
    assert!(
        finalize_resp.url.ends_with("/p/abc"),
        "unexpected url {}",
        finalize_resp.url
    );

    // Skipping the hash check must not skip storing: the bytes have to come
    // back byte-for-byte over the raw domain.
    let req = HttpRequest::get("/p/abc")
        .with_headers(vec![(
            "host".to_string(),
            format!("{}.raw.icp0.io", storage_canister_id),
        )])
        .build();
    let resp = http_request(pic, controller, storage_canister_id, &req);
    assert_eq!(resp.status_code(), StatusCode::OK);
    assert_eq!(
        resp.body(),
        &bytes,
        "served bytes differ from uploaded ones"
    );
}

#[test]
fn a_wrong_declared_hash_still_fails_at_finalize() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        ..
    } = test_env;

    let file_path = "h/x";

    init_upload(
        pic,
        controller,
        storage_canister_id,
        &(init_upload::Args {
            file_path: file_path.to_string(),
            file_hash: Some("00".repeat(32)),
            file_size: 3,
            chunk_size: None,
        }),
    )
    .expect("init_upload with a declared hash should be accepted");

    store_chunk(
        pic,
        controller,
        storage_canister_id,
        &(store_chunk::Args {
            file_path: file_path.to_string(),
            chunk_id: Nat::from(0u64),
            chunk_data: vec![1, 2, 3],
        }),
    )
    .expect("store_chunk should succeed");

    let finalize_resp = finalize_upload(
        pic,
        controller,
        storage_canister_id,
        &(finalize_upload::Args {
            file_path: file_path.to_string(),
        }),
    );
    assert!(
        matches!(
            finalize_resp,
            Err(finalize_upload::FinalizeUploadError::FileHashMismatch)
        ),
        "a declared hash must still be verified at finalize, got {finalize_resp:?}"
    );
}

#[test]
fn a_legacy_caller_declaring_a_bare_text_hash_is_still_honoured() {
    // Making `file_hash` optional must not break clients built against the
    // previous api crate: they encode the field as a bare `text`, which candid
    // promotes to `opt text` on the way in. A wrong hash is the sharp assertion
    // here, because it fails only if the promoted value really arrived as
    // `Some(hash)` and was compared. Had it been dropped to `None`, finalize
    // would have succeeded and the guarantee old callers rely on would be gone.
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        ..
    } = test_env;

    let file_path = "legacy/x";

    init_upload_legacy(
        pic,
        controller,
        storage_canister_id,
        LegacyInitUploadArgs {
            file_path: file_path.to_string(),
            file_hash: "00".repeat(32),
            file_size: 3,
            chunk_size: None,
        },
    )
    .expect("a pre-0.7.0 record shape must still be decodable");

    store_chunk(
        pic,
        controller,
        storage_canister_id,
        &(store_chunk::Args {
            file_path: file_path.to_string(),
            chunk_id: Nat::from(0u64),
            chunk_data: vec![1, 2, 3],
        }),
    )
    .expect("store_chunk should succeed");

    let finalize_resp = finalize_upload(
        pic,
        controller,
        storage_canister_id,
        &(finalize_upload::Args {
            file_path: file_path.to_string(),
        }),
    );
    assert!(
        matches!(
            finalize_resp,
            Err(finalize_upload::FinalizeUploadError::FileHashMismatch)
        ),
        "a hash declared by a legacy caller must still be verified, got {finalize_resp:?}"
    );
}
