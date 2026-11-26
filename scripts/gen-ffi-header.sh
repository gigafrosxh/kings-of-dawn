#!/usr/bin/env bash
# Regenerates the C header of the Rust FFI (requires `cargo install cbindgen --locked`).
# Usage: scripts/gen-ffi-header.sh [--check]
set -euo pipefail
cd "$(dirname "$0")/.."
header=unreal/Plugins/KingsOfDawnSimulation/Source/ThirdParty/KodFfi/include/kod_ffi.h
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
cbindgen --config rust/crates/ffi/cbindgen.toml --crate kod-ffi --output "$tmp" rust/crates/ffi
if [ "${1:-}" = "--check" ]; then
  if ! diff -u "$header" "$tmp"; then
    echo "kod_ffi.h is out of date; run scripts/gen-ffi-header.sh" >&2
    exit 1
  fi
  echo "kod_ffi.h up to date"
else
  cp "$tmp" "$header"
  echo "wrote $header"
fi
