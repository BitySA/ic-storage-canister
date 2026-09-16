#!/usr/bin/env bash

# Deploy the storage canister to the local replica in test mode.
#
# Usage: scripts/deploy/local/deploy_storage.sh [principal...]
#
# The current dfx identity is always authorized to upload and delete files.
# Pass more principals to authorize them too.
#
# dfx deploy runs scripts/build.sh, which also regenerates api/can.did.
# If the canister already has code, it is reinstalled, which deletes its files.
# dfx asks for confirmation before it does that.

set -euo pipefail

cd "$(dirname "$0")/../../.."

if ! dfx ping local > /dev/null 2>&1; then
    echo "Error: the local replica is not running. Start it with: dfx start --background --clean"
    exit 1
fi

principals="principal \"$(dfx identity get-principal)\";"
for principal in "$@"; do
    principals+=" principal \"$principal\";"
done

# The init argument only works on a fresh install. A canister that already has
# code needs a reinstall, because an upgrade expects the Upgrade argument.
mode_args=()
if dfx canister info storage --network local 2> /dev/null | grep -q "Module hash: 0x"; then
    mode_args=(--mode reinstall)
fi

dfx deploy --network local storage ${mode_args[@]+"${mode_args[@]}"} --argument "(variant { Init = record {
    test_mode = true;
    version = record {
     major = 0:nat32;
     minor = 0:nat32;
     patch = 0:nat32;
    };
    commit_hash = \"stagingcommit\";
    authorized_principals = vec { $principals };
}})"
