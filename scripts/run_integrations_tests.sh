#!/bin/bash

# Build the canister, then run the integration tests one at a time.
# Extra arguments are passed to the test binary, so you can run a single test:
#   scripts/run_integrations_tests.sh test_storage_simple

set -euo pipefail

cd "$(dirname "$0")/.."

ulimit -n 65536 || echo "Warning: could not raise the open file limit to 65536"

./scripts/build.sh

cargo test -p integration_tests -- --test-threads=1 "$@"
