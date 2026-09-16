#!/usr/bin/env bash

set -euo pipefail

show_help() {
  cat << EOF
Generate the candid file for a built canister wasm.
Must be run from the repository's root folder.

Note: scripts/build.sh already does this for the storage canister, so you only
need this script to regenerate the candid file on its own.

Usage:
  scripts/generate_did.sh [options] [wasm]

Arguments:
  wasm              The wasm file to read (default: wasm/storage_canister.wasm)

Options:
  -h, --help        Show this message and exit
  -o, --output FILE The candid file to write (default: api/can.did)
  -d, --dry-run     Only print the result, without writing on disk
EOF
}

wasm_path="wasm/storage_canister.wasm"
output_path="api/can.did"
dryrun=0

while [[ $# -gt 0 ]]; do
  case $1 in
    -h | --help )
      show_help
      exit
      ;;
    -o | --output )
      if [[ $# -lt 2 ]]; then
        echo "Error: $1 needs a file path."
        exit 1
      fi
      output_path=$2
      shift 2
      ;;
    -d | --dry-run )
      dryrun=1
      shift
      ;;
    -- )
      shift
      break
      ;;
    -* )
      echo "Error: unknown option $1"
      show_help
      exit 1
      ;;
    * )
      break
      ;;
  esac
done

if [[ $# -gt 1 ]]; then
  echo "Error: expected at most one wasm file."
  show_help
  exit 1
fi
if [[ $# -eq 1 ]]; then
  wasm_path=$1
fi

if [[ ! -f "$wasm_path" ]]; then
  echo "Error: wasm file not found: $wasm_path. Run ./scripts/build.sh first."
  exit 1
fi

if [[ $dryrun -eq 1 ]]; then
  echo -e "This would be written to ${output_path}:\n"
  candid-extractor "$wasm_path"
else
  candid-extractor "$wasm_path" > "$output_path"
  echo "Wrote ${output_path}"
fi
