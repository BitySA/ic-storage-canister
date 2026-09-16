# Storage Canister

A storage canister for the Internet Computer. It keeps files (images, videos, any bytes) and serves them over HTTP with certified responses.

A **canister** is a program that runs on the Internet Computer. It holds both code and data, and people can reach it with normal HTTP requests.

## Overview

The Storage Canister stores and serves files on the Internet Computer. Unlike DFINITY's asset canister, it keeps files in **stable memory**. Stable memory is the part of a canister's memory that survives upgrades, so files are not lost when you install a new version. The canister also keeps a small cache of files in heap memory, because reading from the heap is faster.

## Key Features

- **Files survive upgrades**: files live in stable memory, so a code upgrade does not delete them.
- **Heap cache**: recently served files are kept in heap memory so the next request is fast.
- **Certified HTTP responses**: the Internet Computer can prove that a response really came from this canister.
- **Automatic cache cleanup**: when the cache is full, the canister removes the files that were cached first to make space.
- **Large files**: files are uploaded in pieces ("chunks"), and large files are streamed back in pieces too.

## How It Works

1. **Storage**: files are saved in stable memory.
2. **Caching**: files that were recently served are kept in heap memory.
3. **Request flow**:
   - When a request comes in, the canister first looks in the heap cache.
   - If the file is in the cache (a cache hit), the canister sends it right away.
   - If the file is not in the cache (a cache miss), the canister:
     1. Turns the query call into an update call
     2. Frees space in the heap if needed
     3. Reads the file from stable memory
     4. Sends the file with a certified HTTP response
4. **Errors**: if the file does not exist, the canister returns an error response.

## Project Layout

| Folder | What is inside |
| --- | --- |
| `canister/` | The canister code itself (Rust). |
| `api/` | The public types and the Candid interface file `api/can.did`. Published as the `bity-ic-storage-canister-api` crate. |
| `c2c/` | A small client that other canisters use to call this canister. Published as the `bity-ic-storage-canister-c2c` crate. |
| `integrations_tests/` | End to end tests that run the real canister inside PocketIC. |
| `scripts/` | Helper scripts to build, test, and upload files. |
| `test_assets/` | Sample files (an SVG, a PNG, and an MP4) you can upload by hand. |

## Installation

These steps work on macOS and Linux. You only need to do them once.

### 1. Install Rust

Install Rust with rustup. A recent stable version is enough.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

The canister is compiled to WebAssembly, so you also need the WebAssembly target:

```bash
rustup target add wasm32-unknown-unknown
```

### 2. Install the build tools

The build script uses two extra tools. `ic-wasm` makes the WebAssembly file smaller and faster. `candid-extractor` reads the finished file and writes the Candid interface (`api/can.did`).

```bash
cargo install ic-wasm --version 0.9.10 --locked
cargo install candid-extractor --version 0.1.6 --locked
```

Please use these exact versions. Version 0.11.0 of `ic-wasm` removed the `optimize` command that the build script needs, so a newer version fails with `error: unexpected argument 'optimize' found`.

### 3. Install dfx

`dfx` is the command line tool for the Internet Computer. You need it to run a local replica (a local copy of the Internet Computer) and to deploy the canister. This project uses dfx `0.31.0`, as written in `dfx.json`.

```bash
sh -ci "$(curl -fsSL https://internetcomputer.org/install.sh)"
dfxvm default 0.31.0
```

Open a new terminal after the install, then check that everything is ready:

```bash
cargo --version
ic-wasm --version     # should print ic-wasm 0.9.10
candid-extractor --version
dfx --version         # should print dfx 0.31.0
```

### 4. Get the code and build it

```bash
git clone https://github.com/BitySA/ic-storage-canister.git
cd ic-storage-canister
./scripts/build.sh
```

The build writes the finished canister to `wasm/storage_canister.wasm.gz`. The `wasm/` folder is ignored by git. The build also updates two files that git does track: `api/can.did` and `integrations_tests/wasm/storage_canister.wasm.gz`. If you change the canister code, you will see these two files in `git status` after a build.

## Testing

There are three ways to test the canister. Unit tests are the fastest. Integration tests are slower but check the real canister. The manual test lets you see it work with your own eyes.

### Unit tests

Unit tests check small pieces of logic, such as file path validation. They do not need a build or a replica.

```bash
cargo test -p storage_canister
```

### Integration tests

Integration tests install the real canister WebAssembly file into **PocketIC**. PocketIC is a small local Internet Computer made for tests. Each test uploads files, downloads them over HTTP, upgrades the canister, and checks the results.

The easiest way to run them is the script. It builds the canister first and then runs all the tests:

```bash
./scripts/run_integrations_tests.sh
```

The first run downloads the PocketIC server into your temporary folder. Later runs reuse it. If you already have a PocketIC server binary and do not want the download, point to it before running the tests:

```bash
export POCKET_IC_BIN=/path/to/pocket-ic
```

A few things are good to know:

- The tests read the canister from `wasm/storage_canister.wasm.gz`. The script builds it for you. If you run `cargo test` yourself, run `./scripts/build.sh` first. Otherwise the tests fail with a message that asks you to build.
- The Cargo package is named `integration_tests` (no "s" after "integration"), even though the folder is named `integrations_tests`.
- Each test starts its own PocketIC instance with several subnets, and this uses a lot of memory. This is why the script runs the tests one at a time with `--test-threads=1`.
- The script raises the open file limit with `ulimit -n 65536`. Do the same if you run the tests by hand and see "too many open files".

To run a single test, give part of the test name to the script:

```bash
./scripts/run_integrations_tests.sh test_storage_simple
```

See [`integrations_tests/README.md`](integrations_tests/README.md) for more detail about the test suite.

### Manual test on a local replica

This part shows the whole flow by hand: deploy the canister, upload a file, open it in a browser, and delete it.

**1. Start a local replica.** It runs in the background.

```bash
dfx start --background --clean
```

**2. Deploy the canister.** Only principals in `authorized_principals` can upload or delete files. A **principal** is an identity on the Internet Computer. The deploy script builds the canister and puts your own dfx identity in that list.

```bash
./scripts/deploy/local/deploy_storage.sh
```

To let other principals upload too, add them at the end: `./scripts/deploy/local/deploy_storage.sh <principal> <principal>`. If the canister is already deployed, the script reinstalls it. This deletes all its files, so dfx asks you to confirm first. If nobody can answer the question (for example, when the script runs in CI), dfx stops and the files are kept.

The script deploys in test mode (`test_mode = true`), which is for local work and staging. In test mode, the canister accepts at most 500 MB and uses a small heap cache. Without test mode the limit is 500 GB. The limit is checked against the stable memory the canister has taken, which is more than the size of the files.

**3. Upload a file.** The script sends the file in three steps and prints a link to it.

```bash
./scripts/upload.sh test_assets/logo.svg
```

The script uses the file name as the path in the canister, so this file is stored as `logo.svg`. It only talks to the local replica.

**4. Open the file.** Open the link from step 3 in a browser. It looks like `http://<canister-id>.raw.localhost:4943/logo.svg`. The script also prints the `icp0.io` link that the canister returns. That link is for the main Internet Computer network, so it does not work locally.

You can also download the file with `curl` and compare it with the original:

```bash
curl -s "http://$(dfx canister id storage --network local).raw.localhost:4943/logo.svg" | shasum -a 256
shasum -a 256 test_assets/logo.svg
```

The two hashes should be the same.

**5. Look at the storage numbers.**

```bash
dfx canister call storage get_stored_files_size_bytes '(null)'   # total size of the stored files, in bytes
dfx canister call storage get_storage_size '(null)'              # stable memory the canister has taken, in bytes
```

**6. Delete the file.** After this, the address from step 4 returns 404.

```bash
dfx canister call storage remove_file '(record { file_path = "logo.svg" })'
```

**7. Stop the replica** when you are done.

```bash
dfx stop
```

## Uploading Files (API Summary)

Files are uploaded in pieces called **chunks**. A normal upload has three calls:

1. `init_upload`: tell the canister the file path, the file size, and (optionally) the SHA-256 hash of the file.
2. `store_chunk`: send each chunk with its number, starting from 0.
3. `finalize_upload`: tell the canister you are done. If you gave a hash, the canister checks it now. The reply contains the public URL of the file.

Other calls:

- `cancel_upload` stops an upload that is not finished.
- `remove_file` deletes a finished file.
- `init_reupload` replaces the content of a file that already exists. Unlike `init_upload`, it requires the file hash.

Limits:

- The default chunk size is 1 MiB. You can choose a smaller chunk size, but not a bigger one.
- A file can have at most 10,000 chunks.
- A canister can hold at most 100,000 files.
- Uploads that are never finished are removed automatically about 24 hours after they started.

All of these calls need an authorized principal. The full interface is in [`api/can.did`](api/can.did). Rust canisters can call this canister with the `bity-ic-storage-canister-c2c` crate.

## Common Problems

**"Caller is not a governance principal"**: the identity you are using is not in `authorized_principals`. Check which identity you use with `dfx identity whoami`. Then deploy again and pass its principal to the deploy script, or switch back to the identity that deployed the canister. In test mode, the identity that deploys the canister is always authorized.

**"error: unexpected argument 'optimize' found"**: your `ic-wasm` is too new. Install version 0.9.10 (see step 2 of Installation).

**Integration tests say "Failed to read storage_canister wasm"**: run `./scripts/build.sh` first.

**The `icp0.io` link does not work locally**: use `http://<canister-id>.raw.localhost:4943/<file-path>` instead. The upload script prints this link for you.

**"The local replica is not running"**: start it with `dfx start --background --clean`.

## Contributing

Contributions are welcome. Before you open a pull request, please run `cargo clippy`, the unit tests, and the integration tests.
