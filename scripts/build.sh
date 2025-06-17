#!/bin/bash

cargo rustc --crate-type=cdylib --target wasm32-unknown-unknown --target-dir "canister/src/target" --release --locked -p storage_canister &&
ic-wasm "canister/src/target/wasm32-unknown-unknown/release/storage_canister.wasm" -o "canister/src/target/wasm32-unknown-unknown/release/storage_canister.wasm" shrink &&
ic-wasm "canister/src/target/wasm32-unknown-unknown/release/storage_canister.wasm" -o "canister/src/target/wasm32-unknown-unknown/release/storage_canister.wasm" optimize --inline-functions-with-loops O3 &&
gzip --no-name -9 -v -c "canister/src/target/wasm32-unknown-unknown/release/storage_canister.wasm" > "canister/src/target/wasm32-unknown-unknown/release/storage_canister.wasm.gz" &&
gzip -v -t "canister/src/target/wasm32-unknown-unknown/release/storage_canister.wasm.gz" &&
cp "canister/src/target/wasm32-unknown-unknown/release/storage_canister.wasm" "./wasm/storage_canister.wasm" &&
candid-extractor "./wasm/storage_canister.wasm" > "./wasm/can.did" &&
cp "canister/src/target/wasm32-unknown-unknown/release/storage_canister.wasm.gz" "./integrations_tests/wasm" &&
mv "canister/src/target/wasm32-unknown-unknown/release/storage_canister.wasm.gz" "./wasm/storage_canister.wasm.gz"
