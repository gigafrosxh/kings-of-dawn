#!/usr/bin/env bash
# Builds kod_ffi as a static library and runs the C smoke test against the
# generated header (Linux, macOS, Windows/MSYS with clang).
set -euo pipefail
cd "$(dirname "$0")/.."

manifest=rust/Cargo.toml
include=unreal/Plugins/KingsOfDawnSimulation/Source/ThirdParty/KodFfi/include
out=rust/target/ffi-smoke
mkdir -p "$out"

cargo build --manifest-path "$manifest" -p kod-ffi --release --locked --features test-hooks
native_libs=$(cargo rustc --manifest-path "$manifest" -p kod-ffi --release --locked --features test-hooks --crate-type staticlib -- --print native-static-libs 2>&1 \
  | sed -n 's/.*native-static-libs: //p' | tail -1)

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    lib=rust/target/release/kod_ffi.lib
    # MSVC-style flags (/defaultlib:foo.lib) -> clang-cl style link libraries.
    libs=$(sed -e 's#/defaultlib:#-l#g' -e 's#\.lib##g' <<<"$native_libs")
    exe="$out/smoke.exe"
    ;;
  *)
    lib=rust/target/release/libkod_ffi.a
    libs="$native_libs -lpthread"
    exe="$out/smoke"
    ;;
esac

# shellcheck disable=SC2086
${CC:-clang} -std=c11 -Wall -Wextra -Werror -I "$include" rust/crates/ffi/tests/c/smoke.c "$lib" $libs -o "$exe"
"$exe"
