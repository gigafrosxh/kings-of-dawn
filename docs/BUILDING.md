# Build and smoke checks

## Rust (headless)

Install Rustup, then run from the repository root:

```sh
cargo test --manifest-path rust/Cargo.toml --locked
cargo clippy --manifest-path rust/Cargo.toml --all-targets --locked -- -D warnings
```

`rust-toolchain.toml` selects Rust 1.100.0 and the `rustfmt`/`clippy` components.

## Unreal Engine

Install Unreal Engine 5.8.4 and the matching supported Windows C++ toolchain. Open `unreal/KingsOfDawn.uproject` in the editor once to generate project files, or build the editor target from a Visual Studio Developer Command Prompt:

```bat
"%UE_5_8_ROOT%\Engine\Build\BatchFiles\Build.bat" KingsOfDawnEditor Win64 Development -Project="%CD%\unreal\KingsOfDawn.uproject" -WaitMutex
```

The same project file can be opened on other UE-supported platforms; Windows is the required release build target. Generated Unreal output and Rust `target/` files are excluded from Git.
