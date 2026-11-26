# ADR-0001: Engine, Rust toolchain, and repository layout

- Status: accepted for the v0.1 foundation
- Date: 2026-11-26
- Issue: [#6](https://github.com/gigafrosxh/kings-of-dawn/issues/6)

## Decision

- Use Unreal Engine 5.8.4, the latest stable UE5 release at this decision date. Pin the project association to the 5.8 minor line in `unreal/KingsOfDawn.uproject` and target its UBT API via `Unreal5_8`.
- Use Rust 1.100.0, pinned in `rust-toolchain.toml`; keep the simulation as a standalone Cargo workspace under `rust/`.
- Keep the Unreal project under `unreal/` and the Rust workspace under `rust/`. The initial `kod-core` crate is headless and dependency-free.
- Unreal owns engine lifecycle, rendering, input, and UI in C++; Rust will own simulation behavior. The C ABI/plugin and simulation contracts are follow-up work in #7 and #8.

## Rationale

UE 5.9 is in Preview as of this decision; a preview is unsuitable as the project's reproducible foundation. UE 5.8 is the current stable line. Pinning a patch-level Rust compiler prevents silent toolchain drift and makes local and CI builds reproducible.

## Consequences

- Windows builds require Unreal Engine 5.8.4 and its supported Windows C++ toolchain. The current development environment is macOS and does not have Unreal installed, so the Windows editor/game build must be verified on a Windows UE runner before #6 can be accepted.
- Rust can be built and tested without Unreal. No Unreal or third-party Rust dependencies are introduced in this foundation step.
- Moving to a newer stable UE5 line or Rust toolchain requires updating these pins and validating the clean-checkout build.

## References

- [Unreal Engine 5.8 is now available](https://www.unrealengine.com/news/unreal-engine-5-8-is-now-available)
- [Unreal Engine 5.9 Preview 2 announcement](https://forums.unrealengine.com/t/preview-2-unreal-engine-5-9-preview/2852840)
- [Rust 1.100.0 release](https://blog.rust-lang.org/2026/11/12/Rust-1.100.0/)
