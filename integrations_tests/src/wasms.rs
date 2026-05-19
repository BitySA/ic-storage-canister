use bity_ic_types::CanisterWasm;
use lazy_static::lazy_static;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

lazy_static! {
    // Wasms in wasms folder
    // pub static ref IC_ICRC1_LEDGER: CanisterWasm = get_canister_wasm("ic_icrc1_ledger");
    // pub static ref IC_ICRC2_LEDGER: CanisterWasm = get_canister_wasm_gz("icrc_ledger");
    // pub static ref SNS_GOVERNANCE: CanisterWasm = get_canister_wasm("sns_governance");
    // pub static ref SNS_ROOT: CanisterWasm = get_canister_wasm("sns_root");
    // pub static ref ICP_LEDGER: CanisterWasm = get_canister_wasm("ledger");
    // pub static ref REGISTRY_WASM: CanisterWasm = get_canister_wasm("registry");

    // Wasms in particular canister folder.
    // STORAGE_WASM is whatever the working tree builds — the current/under-development
    // version. Historical wasms are pinned by published version below.
    pub static ref STORAGE_WASM: CanisterWasm = get_canister_wasm_from_bin("storage_canister");

    // Historical storage canister WASMs, kept as fixtures for version-pair migration tests.
    // Version refers to `canister/Cargo.toml`'s `version` (the canister cdylib itself),
    // NOT the published api/c2c crate versions — those are independent.
    //
    // To add another historical version: drop the .wasm.gz into integrations_tests/wasm/
    // as `storage_canister_v<MAJOR>_<MINOR>_<PATCH>.wasm.gz` and add a matching
    // lazy_static below. Never repurpose an existing fixture name.

    // v0.2.0: pre-fix master (commit cdeb4d6). The "videos don't load" build.
    pub static ref STORAGE_WASM_V0_2_0: CanisterWasm =
        get_canister_wasm_from_local_fixture("storage_canister_v0_2_0");

    // v0.2.1: post-fix master (commit ed9e765). The build origyn-nft's
    // 4-storage-canister-migration branch currently bundles.
    pub static ref STORAGE_WASM_V0_2_1: CanisterWasm =
        get_canister_wasm_from_local_fixture("storage_canister_v0_2_1");
}

/// Read a historical-version fixture committed to `integrations_tests/wasm/`.
/// Distinct from `get_canister_wasm_from_bin`, which reads the under-development
/// WASM produced by `scripts/build.sh` into the gitignored top-level `wasm/`.
fn get_canister_wasm_from_local_fixture(canister_name: &str) -> CanisterWasm {
    match read_file_from_relative_bin(&format!("wasm/{canister_name}.wasm.gz")) {
        Ok(wasm) => wasm,
        Err(err) => {
            println!(
                "Failed to read {canister_name} fixture wasm from integrations_tests/wasm/: {err}"
            );
            panic!()
        }
    }
}

fn get_canister_wasm_from_bin(canister_name: &str) -> CanisterWasm {
    match read_file_from_relative_bin(&format!("../wasm/{canister_name}.wasm.gz")) {
        Ok(wasm) => wasm,
        Err(err) => {
            println!(
                "Failed to read {canister_name} wasm: {err}. \n\x1b[31mRun \"./scripts/build.sh \"\x1b[0m"
            );
            panic!()
        }
    }
}

fn get_canister_wasm(canister_name: &str) -> CanisterWasm {
    read_file_from_local_bin(&format!("{canister_name}_canister.wasm"))
}

fn get_canister_wasm_gz(canister_name: &str) -> CanisterWasm {
    read_file_from_local_bin(&format!("{canister_name}_canister.wasm.gz"))
}

fn read_file_from_local_bin(file_name: &str) -> Vec<u8> {
    let mut file_path = local_bin();
    file_path.push(file_name);

    let mut file = File::open(&file_path)
        .unwrap_or_else(|_| panic!("Failed to open file: {}", file_path.to_str().unwrap()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("Failed to read file");
    bytes
}

pub fn local_bin() -> PathBuf {
    let mut file_path = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("Failed to read CARGO_MANIFEST_DIR env variable"),
    );
    file_path.push("wasm");
    file_path
}

fn read_file_from_relative_bin(file_path: &str) -> Result<Vec<u8>, std::io::Error> {
    // Open the wasm file
    let mut file = File::open(file_path)?;

    // Read the contents of the file into a vector
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    Ok(buffer)
}
