//! Regression guard for the abandoned-upload GC's interaction with upgrades.
//!
//! The `init_timestamp` stamped at `init_upload` must survive a canister upgrade.
//! State is persisted in `pre_upgrade` via `bity_ic_serializer`, which uses
//! `rmp_serde` with `.with_struct_map()`: structs are written as MessagePack maps
//! keyed by field name, so `#[serde(default)]` only fires for keys absent from the
//! serialized data (i.e. pre-S8 uploads that never recorded a timestamp). For any
//! upload created after the field was added, the stored value is read back
//! unchanged, so an upgrade does NOT grant the abandoned upload a fresh TTL.
//!
//! This test pins that behavior: if a future change resets `init_timestamp` on
//! upgrade (e.g. switching the serializer to a positional format, or adding a
//! migration `From` impl that drops the field), the GC would no longer fire at the
//! original deadline and this test would fail.
//!
//! Timeline (T0 = time of init_upload, GC TTL = 24h):
//!   T0          init_upload + one partial chunk for an abandoned file
//!   T0 + 12h    upgrade in place; assert the entry survived the upgrade
//!   T0 + 30h    fire GC; assert the entry was removed
//!
//! T0 + 30h is past the original deadline (T0 + 24h) but before the deadline a
//! reset timestamp would imply (upgrade time T0 + 12h, so T0 + 36h). The entry is
//! therefore removed only if the original T0 timestamp was preserved across the
//! upgrade. A 6h margin on each side keeps the test away from timer-tick edges.

use std::time::Duration;

use candid::Nat;

use bity_ic_storage_canister_api::init_upload;
use bity_ic_storage_canister_api::lifecycle::Args;
use bity_ic_storage_canister_api::post_upgrade::UpgradeArgs;
use bity_ic_storage_canister_api::store_chunk;
use bity_ic_types::BuildVersion;

use crate::client::storage::{init_upload, store_chunk};
use crate::storage_suite::setup::default_test_setup;
use crate::storage_suite::setup::setup::TestEnv;
use crate::storage_suite::setup::setup_storage::upgrade_storage_canister;
use crate::utils::tick_n_blocks;

const HOUR: Duration = Duration::from_secs(60 * 60);

#[test]
fn abandoned_upload_init_timestamp_survives_upgrade() {
    let mut test_env: TestEnv = default_test_setup();

    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        ..
    } = test_env;

    let file_path = "/abandoned.bin".to_string();
    // 2 MiB file => 2 chunks at the default 1 MiB chunk size. We store only the
    // first chunk and never finalize, leaving the upload in InProgress.
    let file_size: u64 = 2 * 1024 * 1024;

    // The hash is never checked: we deliberately never finalize this upload.
    let init_resp = init_upload(
        pic,
        controller,
        storage_canister_id,
        &(init_upload::Args {
            file_path: file_path.clone(),
            file_hash: "00".repeat(32),
            file_size,
            chunk_size: None,
        }),
    );
    assert!(
        init_resp.is_ok(),
        "initial init_upload should succeed, got {init_resp:?}"
    );

    let chunk = vec![0u8; 1024 * 1024];
    let store_resp = store_chunk(
        pic,
        controller,
        storage_canister_id,
        &(store_chunk::Args {
            file_path: file_path.clone(),
            chunk_id: Nat::from(0u64),
            chunk_data: chunk,
        }),
    );
    assert!(
        store_resp.is_ok(),
        "store_chunk should succeed, got {store_resp:?}"
    );

    // Advance 12h, then upgrade in place. If the timestamp were reset on upgrade,
    // it would become ~T0 + 12h here.
    pic.advance_time(12 * HOUR);
    tick_n_blocks(pic, 2);

    upgrade_storage_canister(
        pic,
        storage_canister_id,
        Args::Upgrade(UpgradeArgs {
            version: BuildVersion::min(),
            commit_hash: "gc-upgrade-test".to_string(),
        }),
        controller,
    );
    tick_n_blocks(pic, 2);

    // The in-flight upload must still occupy its path right after the upgrade
    // (it is neither finalized nor yet past its TTL).
    let reinit_after_upgrade = init_upload(
        pic,
        controller,
        storage_canister_id,
        &(init_upload::Args {
            file_path: file_path.clone(),
            file_hash: "00".repeat(32),
            file_size,
            chunk_size: None,
        }),
    );
    assert!(
        matches!(
            reinit_after_upgrade,
            Err(init_upload::InitUploadError::FileAlreadyExists)
        ),
        "right after upgrade the abandoned upload should still hold its path, got {reinit_after_upgrade:?}"
    );

    // Advance to ~T0 + 30h total and let the hourly GC timer fire. This is past
    // the original deadline (T0 + 24h) but before a reset deadline (T0 + 36h).
    pic.advance_time(18 * HOUR);
    tick_n_blocks(pic, 10);

    // The slot must now be free: the GC removed the abandoned upload because its
    // preserved T0 timestamp is older than the 24h TTL.
    let reinit_after_gc = init_upload(
        pic,
        controller,
        storage_canister_id,
        &(init_upload::Args {
            file_path: file_path.clone(),
            file_hash: "00".repeat(32),
            file_size,
            chunk_size: None,
        }),
    );
    assert!(
        reinit_after_gc.is_ok(),
        "after the TTL elapsed the GC should have freed the path (proves init_timestamp \
         was preserved across the upgrade, not reset to upgrade time), got {reinit_after_gc:?}"
    );
}
