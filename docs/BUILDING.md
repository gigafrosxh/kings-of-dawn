# Build and smoke checks

## Rust (headless)

Install Rustup, then run from the repository root:

```sh
cargo test --manifest-path rust/Cargo.toml --locked
cargo clippy --manifest-path rust/Cargo.toml --all-targets --locked -- -D warnings
```

`rust-toolchain.toml` selects Rust 1.100.0 and the `rustfmt`/`clippy` components.

All CI checks (tests, fixtures, 100-year soak, benchmark hash) run locally with `scripts/ci-local.sh`; see [CI.md](CI.md).

## Rust FFI for Unreal

Unreal links the static library `kod_ffi` (see [ADR-0002](decisions/ADR-0002-rust-unreal-ffi.md)). Build it before the editor target:

```sh
scripts/build-ffi.sh          # macOS/Linux -> rust/target/<host triple>/release/libkod_ffi.a
```

```bat
powershell -File scripts\build-ffi.ps1   # Windows -> rust\target\x86_64-pc-windows-msvc\release\kod_ffi.lib
```

After changing the ABI, regenerate the header with `scripts/gen-ffi-header.sh` (needs `cargo install cbindgen --version 0.29.4 --locked`). `scripts/ffi-smoke.sh` runs the C smoke test.

## Unreal Engine

Install Unreal Engine 5.8.4 and the matching supported Windows C++ toolchain. Open `unreal/KingsOfDawn.uproject` in the editor once to generate project files, or build the editor target from a Visual Studio Developer Command Prompt:

```bat
"%UE_5_8_ROOT%\Engine\Build\BatchFiles\Build.bat" KingsOfDawnEditor Win64 Development -Project="%CD%\unreal\KingsOfDawn.uproject" -WaitMutex
```

The same project file can be opened on other UE-supported platforms; Windows is the required release build target. Generated Unreal output and Rust `target/` files are excluded from Git.
