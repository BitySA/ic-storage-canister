use candid::Nat;

use self::setup::{TestEnv, TestEnvBuilder};

pub mod setup;
pub mod setup_storage;

pub fn default_test_setup() -> TestEnv {
    TestEnvBuilder::new().build()
}

/// Build a TestEnv that has a specific historical storage canister WASM installed.
/// Pass a fixture from `crate::wasms` (e.g. `STORAGE_WASM_V0_2_0`). Used by
/// version-pair migration tests.
pub fn historical_test_setup(wasm: bity_ic_types::CanisterWasm) -> TestEnv {
    TestEnvBuilder::new().build_with_historical_wasm(wasm)
}
