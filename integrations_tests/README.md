# Integration Tests

A test suite that runs the real storage canister inside PocketIC and checks that it works correctly.

For installing the tools, see the [main README](../README.md#installation).

## Prerequisites

- Rust toolchain with the `wasm32-unknown-unknown` target
- `ic-wasm` 0.9.10 and `candid-extractor` 0.1.6 (used by the build script)
- PocketIC server (downloaded automatically on the first test run)

## Setting Up PocketIC

You do not need to do anything. On the first run, the `pocket-ic` crate downloads the matching PocketIC server into your temporary folder and reuses it after that.

If you do not want this download (for example, on a machine without internet access), download the server yourself from the [PocketIC releases](https://github.com/dfinity/pocketic/releases). Use the same version as the `pocket-ic` crate in the root `Cargo.toml` (12.0.0 right now). If the version does not fit, the tests stop with an error. Then:

1. Unzip it and make it executable:
```bash
gzip -d pocket-ic.gz
chmod +x pocket-ic
```

2. Set the environment variable:
```bash
export POCKET_IC_BIN=/path/to/pocket-ic
```

## Running Tests

### Build the canister

The tests read the canister from `wasm/storage_canister.wasm.gz` at the root of the repository, so build it first:
```bash
./scripts/build.sh
```

### Run all integration tests

This script builds the canister and then runs every test, one at a time:
```bash
./scripts/run_integrations_tests.sh
```

### Run one test

Give part of the test name to the script to run only that test:
```bash
./scripts/run_integrations_tests.sh test_storage_simple
```

The Cargo package is named `integration_tests`. If you use `cargo test` directly, build the canister first:
```bash
./scripts/build.sh
cargo test -p integration_tests test_storage_simple -- --test-threads=1
```

## Common issues and solutions

1. **"Failed to read storage_canister wasm"**
   - Run `./scripts/build.sh` first.

2. **PocketIC download fails**
   - Check your internet connection, or set `POCKET_IC_BIN` as described above.
   - If you set `POCKET_IC_BIN`, check that the file is executable and that its version is the same as the `pocket-ic` crate.

3. **"Too many open files"**
   - Run `ulimit -n 65536` in your terminal before the tests. The script does this for you.

4. **Build failures**
   - Check that `ic-wasm` is version 0.9.10. Version 0.11.0 and newer do not have the `optimize` command.

## Contributing

We welcome contributions to improve the test suite. Please:

1. Follow existing test patterns
2. Add comprehensive documentation
3. Ensure all tests pass
4. Submit pull requests with clear descriptions

## Resources

- [PocketIC](https://github.com/dfinity/pocketic)
- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
