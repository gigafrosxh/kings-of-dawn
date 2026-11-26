#!/usr/bin/env bash
# Builds kod_ffi as a static library and runs the C smoke test against the
# generated header (Linux/macOS with cc, Windows with MSVC cl from Git Bash).
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
    # Requires an MSVC developer environment (cl.exe on PATH).
    lib=rust/target/release/kod_ffi.lib
    export MSYS2_ARG_CONV_EXCL='*'
    # shellcheck disable=SC2086
    cl /nologo /W3 /WX /I "$include" rust/crates/ffi/tests/c/smoke.c /Fe"$out/smoke.exe" /Fo"$out/" /link "$lib" $native_libs
    "$out/smoke.exe"
    ;;
  *)
    lib=rust/target/release/libkod_ffi.a
    # shellcheck disable=SC2086
    ${CC:-cc} -std=c11 -Wall -Wextra -Werror -I "$include" rust/crates/ffi/tests/c/smoke.c "$lib" $native_libs -lpthread -o "$out/smoke"
    "$out/smoke"
    ;;
esac
