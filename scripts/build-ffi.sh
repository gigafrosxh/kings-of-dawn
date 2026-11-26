#!/usr/bin/env bash
# Builds the kod_ffi static library for Unreal (macOS/Linux) at
# rust/target/<triple>/release/libkod_ffi.a, where KodFfi.Build.cs expects it.
# Usage: scripts/build-ffi.sh [<rust target triple>]
set -euo pipefail
cd "$(dirname "$0")/.."
triple=${1:-$(rustc -vV | sed -n 's/^host: //p')}
rustup target add "$triple" >/dev/null
cargo build --manifest-path rust/Cargo.toml -p kod-ffi --release --locked --target "$triple"
echo "built rust/target/$triple/release/libkod_ffi.a"
