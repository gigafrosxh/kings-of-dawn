#!/usr/bin/env bash
# Runs the same Rust checks as .github/workflows/rust.yml on the local machine.
set -euo pipefail
cd "$(dirname "$0")/.."

expected_hash=990d8ecc950c75c2be9922a133e491fc
manifest=rust/Cargo.toml
bin=rust/target/release/kod-headless

cargo fmt --manifest-path "$manifest" --all --check
cargo clippy --manifest-path "$manifest" --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path "$manifest" --workspace --locked
cargo test --manifest-path "$manifest" --workspace --locked --release
cargo build --manifest-path "$manifest" -p kod-headless --locked --release
"$bin" verify-save rust/fixtures/ledger-v1-365.kodsave
"$bin" verify-replay rust/fixtures/ledger-v1-365.kodreplay
"$bin" soak --years 100
out=$("$bin" bench --ticks 36525)
echo "$out"
hash=$(sed -n 's/.*state_hash=\([0-9a-f]*\).*/\1/p' <<<"$out")
if [ "$hash" != "$expected_hash" ]; then
  echo "state hash $hash differs from expected $expected_hash" >&2
  exit 1
fi
echo "ci-local: all checks passed"
