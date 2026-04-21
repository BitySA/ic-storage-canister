#!/bin/bash

# http://uzt4z-lp777-77774-qaabq-cai.raw.localhost:4943/logs

set -euo pipefail

CANISTER_ID="uzt4z-lp777-77774-qaabq-cai"
NETWORK="local"
CHUNK_SIZE=1048576  # 1MiB — must match DEFAULT_CHUNK_SIZE in canister/src/types/storage.rs

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <local_file_path>"
    exit 1
fi

FILE_PATH="$1"
FILE_NAME=$(basename "$FILE_PATH")

if [ ! -f "$FILE_PATH" ]; then
    echo "Error: file not found: $FILE_PATH"
    exit 1
fi

if [[ "$OSTYPE" == "darwin"* ]]; then
    FILE_SIZE=$(stat -f%z "$FILE_PATH")
else
    FILE_SIZE=$(stat -c%s "$FILE_PATH")
fi
FILE_HASH=$(shasum -a 256 "$FILE_PATH" | cut -d ' ' -f 1)

echo "--- Storage Canister Upload ---"
echo "Target:  $CANISTER_ID ($NETWORK)"
echo "File:    $FILE_NAME ($FILE_SIZE bytes)"
echo "Hash:    $FILE_HASH"

# Step 1: init_upload
echo ""
echo "[1/3] Initializing upload..."
INIT_RES=$(dfx canister --network "$NETWORK" call "$CANISTER_ID" init_upload "(record {
    file_path = \"$FILE_NAME\";
    file_size = $FILE_SIZE : nat64;
    file_hash = \"$FILE_HASH\";
    chunk_size = null;
})")

if [[ "$INIT_RES" == *"variant { Err"* ]]; then
    echo "Error during init_upload: $INIT_RES"
    exit 1
fi
echo "Initialized: $INIT_RES"

# Step 2: store_chunk
TOTAL_CHUNKS=$(( (FILE_SIZE + CHUNK_SIZE - 1) / CHUNK_SIZE ))
echo ""
echo "[2/3] Uploading $TOTAL_CHUNKS chunk(s)..."

for (( i=0; i<TOTAL_CHUNKS; i++ )); do
    OFFSET=$(( i * CHUNK_SIZE ))
    echo "  Chunk $i / $((TOTAL_CHUNKS - 1)) (offset $OFFSET)..."

    ARG_FILE=$(mktemp /tmp/chunk_XXXXXX)
    HEX_DATA=$(dd if="$FILE_PATH" bs=1 skip="$OFFSET" count="$CHUNK_SIZE" 2>/dev/null \
        | hexdump -ve '1/1 "\\%02x"')
    printf '(record { file_path = "%s"; chunk_id = %d : nat; chunk_data = blob "%s"; })' \
        "$FILE_NAME" "$i" "$HEX_DATA" > "$ARG_FILE"

    CHUNK_RES=$(dfx canister --network "$NETWORK" call "$CANISTER_ID" store_chunk \
        --argument-file "$ARG_FILE")
    rm -f "$ARG_FILE"

    if [[ "$CHUNK_RES" == *"variant { Err"* ]]; then
        echo "Error at chunk $i: $CHUNK_RES"
        echo "Cancelling upload..."
        dfx canister --network "$NETWORK" call "$CANISTER_ID" cancel_upload "(record {
            file_path = \"$FILE_NAME\";
        })" || true
        exit 1
    fi
done

# Step 3: finalize_upload
echo ""
echo "[3/3] Finalizing upload..."
FINALIZE_RES=$(dfx canister --network "$NETWORK" call "$CANISTER_ID" finalize_upload "(record {
    file_path = \"$FILE_NAME\";
})")

if [[ "$FINALIZE_RES" == *"variant { Ok"* ]]; then
    URL=$(echo "$FINALIZE_RES" | sed -n 's/.*url = "\([^"]*\)".*/\1/p')
    echo ""
    echo "Upload complete! File available at:"
    echo "$URL"
else
    echo "Error during finalize_upload: $FINALIZE_RES"
    exit 1
fi
