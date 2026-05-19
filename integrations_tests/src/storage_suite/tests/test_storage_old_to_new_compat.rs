//! Version-pair migration tests.
//!
//! Each `#[test]` here installs a historical storage canister WASM, uploads a
//! file via its API, upgrades in-place to the WASM built from the current
//! working tree, and asserts the file is still served byte-for-byte by
//! calling the canister's `http_request` query directly.
//!
//! We deliberately call `http_request` via PocketIC rather than going through
//! `ic-http-gateway`: the gateway adds boundary-node certification validation
//! that's flaky on locally-installed pocket-ic, and the property we actually
//! want to test is "the upgraded canister still serves the original bytes",
//! not "ic-http-gateway can validate a 206 partial response."
//!
//! If any of these fails, state written by that historical version cannot be
//! deserialized by the current code — that's the signal to write a real
//! `From<StorageDataV0> for StorageData` migration.
//!
//! To add a new version pair: drop the fixture into `<root>/wasm/`, expose it
//! via a `STORAGE_WASM_V<...>` lazy_static in `crate::wasms`, and add a new
//! `#[test]` below that calls `assert_upgrade_preserves_files(...)`.

use crate::client::storage::{finalize_upload, http_request, init_upload, store_chunk};
use crate::storage_suite::setup::historical_test_setup;
use crate::storage_suite::setup::setup::TestEnv;
use crate::storage_suite::setup::setup_storage::upgrade_storage_canister;
use crate::wasms::{STORAGE_WASM_V0_2_0, STORAGE_WASM_V0_2_1};
use bity_ic_storage_canister_api::finalize_upload;
use bity_ic_storage_canister_api::init_upload;
use bity_ic_storage_canister_api::lifecycle::Args;
use bity_ic_storage_canister_api::post_upgrade::UpgradeArgs;
use bity_ic_storage_canister_api::store_chunk;
use bity_ic_types::{BuildVersion, CanisterWasm};
use candid::Nat;
use ic_http_certification::{HttpRequest, StatusCode};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[test]
fn test_v0_2_0_to_current_upgrade_preserves_files() {
    assert_upgrade_preserves_files(STORAGE_WASM_V0_2_0.clone(), "v0.2.0");
}

#[test]
fn test_v0_2_1_to_current_upgrade_preserves_files() {
    assert_upgrade_preserves_files(STORAGE_WASM_V0_2_1.clone(), "v0.2.1");
}

/// Generic harness: install `historical_wasm`, upload a file via its API,
/// upgrade in-place to the WASM built from the current working tree, then call
/// the canister's `http_request` query directly and assert the full body
/// matches the original buffer. Stitches together multiple range requests if
/// the canister returns 206 partial content.
fn assert_upgrade_preserves_files(historical_wasm: CanisterWasm, label: &str) {
    let mut test_env: TestEnv = historical_test_setup(historical_wasm);

    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        ..
    } = test_env;

    // --- 1. Upload a file using the historical WASM's API. ---
    let file_path_local = Path::new("./src/storage_suite/assets/test.png");
    let mut file = File::open(&file_path_local).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");
    let file_size = buffer.len() as u64;

    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let file_hash = format!("{:x}", hasher.finalize());

    let target_path = "/test.png".to_string();

    init_upload(
        pic,
        controller,
        storage_canister_id,
        &(init_upload::Args {
            file_path: target_path.clone(),
            file_hash: file_hash.clone(),
            file_size,
            chunk_size: None,
        }),
    )
    .expect("init_upload on historical wasm failed");

    let mut offset = 0;
    let chunk_size = 1024 * 1024;
    let mut chunk_index = 0;
    while offset < buffer.len() {
        let chunk = &buffer[offset..(offset + chunk_size as usize).min(buffer.len())];
        store_chunk(
            pic,
            controller,
            storage_canister_id,
            &(store_chunk::Args {
                file_path: target_path.clone(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            }),
        )
        .expect("store_chunk on historical wasm failed");
        offset += chunk_size as usize;
        chunk_index += 1;
    }

    finalize_upload(
        pic,
        controller,
        storage_canister_id,
        &(finalize_upload::Args {
            file_path: target_path.clone(),
        }),
    )
    .expect("finalize_upload on historical wasm failed");

    // --- 2. Upgrade in-place to the WASM built from the current working tree. ---
    let upgrade_args = Args::Upgrade(UpgradeArgs {
        version: BuildVersion::min(),
        commit_hash: format!("upgrade-from-{label}"),
    });
    upgrade_storage_canister(pic, storage_canister_id, upgrade_args, controller);

    // Tick a few times so any post-upgrade timers register.
    for _ in 0..3 {
        pic.tick();
    }

    // --- 3. Stitch range-requests together to read the full file, then assert
    //        byte equality. IC queries cap responses at ~3MB so we always go
    //        through the Range-served path. ---
    let full_body = stitch_range_requests(
        pic,
        controller,
        storage_canister_id,
        &target_path,
        file_size as usize,
    );

    assert_eq!(
        full_body.len() as u64,
        file_size,
        "{label}->current: served body length {} != expected {}",
        full_body.len(),
        file_size,
    );
    assert_eq!(
        full_body, buffer,
        "{label}->current upgrade lost or corrupted file bytes",
    );
}

fn stitch_range_requests(
    pic: &mut pocket_ic::PocketIc,
    controller: candid::Principal,
    canister_id: candid::Principal,
    target_path: &str,
    total: usize,
) -> Vec<u8> {
    let canister_id_text = canister_id.to_string();
    let mut buf: Vec<u8> = Vec::with_capacity(total);
    let mut iterations = 0;
    while buf.len() < total {
        iterations += 1;
        assert!(
            iterations < 1000,
            "stitch_range_requests: gave up after 1000 iterations at offset {}",
            buf.len()
        );
        let start = buf.len();
        let req = HttpRequest::get(target_path)
            .with_headers(vec![
                ("host".to_string(), format!("{canister_id_text}.raw.icp0.io")),
                ("range".to_string(), format!("bytes={start}-")),
            ])
            .build();
        let resp = http_request(pic, controller, canister_id, &req);
        let status: u16 = resp.status_code().into();
        assert!(
            status == StatusCode::OK || status == StatusCode::PARTIAL_CONTENT,
            "range request at offset {start}: expected 200 or 206, got {status}",
        );
        let body = resp.body();
        assert!(
            !body.is_empty(),
            "empty body at offset {start} after {iterations} iterations",
        );
        buf.extend_from_slice(body);
        if status == StatusCode::OK {
            break;
        }
    }
    buf
}
