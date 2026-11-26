# CI, Tests und Performance-Baseline (T05, Issue #10)

## Automatisierte Prüfungen

Workflow: [`.github/workflows/rust.yml`](../.github/workflows/rust.yml), lokal identisch über `scripts/ci-local.sh`.

| Prüfung | Befehl | Plattformen |
|---|---|---|
| Formatierung | `cargo fmt --all --check` | Linux |
| Lints | `cargo clippy --workspace --all-targets --locked -- -D warnings` | Linux |
| Unit-/Integrations-/Replay-/Doc-Tests (Debug + Release) | `cargo test --workspace --locked [--release]` | Windows, Linux, macOS |
| Save-/Replay-Fixtures | `kod-headless verify-save` / `verify-replay` auf `rust/fixtures/*` | Windows, Linux, macOS |
| 100-Jahre-Headless-Lauf mit Invarianten | `kod-headless soak --years 100` | Windows, Linux, macOS |
| FFI-Header aktuell | `scripts/gen-ffi-header.sh --check` (cbindgen 0.29.4) | Linux |
| FFI-C-Smoke-Test (Staticlib + Header, Roundtrip, Fehler, Panic, Threads) | `scripts/ffi-smoke.sh` | Windows (MSVC), Linux, macOS |
| Benchmark + plattformübergreifender State-Hash | `kod-headless bench --ticks 36525`, Hash muss `BENCH_100Y_HASH` entsprechen | Windows, Linux, macOS |

Plattformübergreifender Determinismus wird doppelt geprüft: durch die Golden-Hashes in `rust/crates/core/tests/determinism.rs` und durch den erwarteten Bench-Hash im Workflow. Abweichung auf einer Plattform = Determinismusfehler (P0).

## Reproduzierbare Builds

- Toolchain fest über `rust-toolchain.toml` (Rust 1.100.0, rustfmt, clippy); CI installiert sie daraus.
- `Cargo.lock` eingecheckt, alle Cargo-Aufrufe mit `--locked`; der Kern hat keine externen Crates.
- Fixtures (`*.kodsave`, `*.kodreplay`) sind über `.gitattributes` als binär markiert (kein CRLF-Umbau unter Windows).
- `current_writer_reproduces_fixtures_byte_for_byte` erkennt jede Formatdrift; bewusste Änderungen erfordern eine Versionserhöhung und neue Fixture-Dateien (`kod-headless gen-fixtures rust/fixtures`), alte bleiben als Migrationstest erhalten.
- Bit-identische Binärartefakte sind nicht Ziel dieser Stufe; reproduzierbar ist das Simulationsergebnis.

## Plattformmatrix

| Bereich | Windows | Linux | macOS |
|---|---|---|---|
| Rust-Kern + Headless-Runner | CI (windows-latest) | CI (ubuntu-latest) | CI (macos-latest), lokal Apple M4 |
| Rust-FFI (Staticlib + C-ABI) | CI (MSVC) | CI | CI |
| UE5-Editor-/Game-Build (5.8.4) | **nicht verifiziert** | nicht vorgesehen | **nicht verifiziert** |
| UE-Automation-Tests | **nicht verifiziert** | – | – |
| Verpacktes Windows-Package | **nicht verifiziert** | – | – |

UE-Build, UE-Automation und Windows-Package laufen nicht auf GitHub-Hosted-Runnern (keine Engine-Installation, Lizenz- und Größenbedingungen) und sind lokal derzeit nicht prüfbar (keine Unreal-Installation; `/Users/Shared/Epic Games/UE_5.6` ist leer). Sie benötigen einen Windows-Rechner bzw. self-hosted Runner mit UE 5.8.4 und folgen mit #7 (FFI-Plugin) und #9 (Kartenprototyp). Bis dahin gilt dieser Teil der Abnahme von #10 als offen.

## Performance-Baseline (Headless)

Gemessen 2026-11-26, Apple M4, 16 GB RAM, macOS, Rust 1.100.0, Release-Build, `kod-headless bench --ticks 36525` (100 Jahre ab 1836-01-01, Seed `0x4B4F445F53454544`), drei Läufe:

| Kennzahl | Wert |
|---|---|
| Laufzeit 36 525 Ticks | 22,2–23,1 ms (≈ 1,58–1,64 Mio. Ticks/s) |
| Commands | 9 490 angewandt, 45 590 abgelehnt |
| State-Hash | 11–23 µs |
| Save schreiben / laden | 19–43 µs / 28–40 µs bei 1 657 Byte |
| 100-Jahre-Soak mit Invariantenprüfung je Tick | ≈ 104 ms |

Einordnung: Die Baseline misst ausschließlich den Kernel-Overhead (Scheduler, Validierung, Codec, Hash, Save/Load) mit der Test-Domäne `fixture::Ledger` (≤ 40 Konten). Sie ist **keine** Aussage über die Zielwerte aus [QUALITY.md](QUALITY.md) (60 FPS, 10 000 Zellen, 20 Staaten); diese werden mit den echten Domänen und dem Kartenprototyp (#9) gemessen. Die CI-Werte je Plattform stehen in der Job-Zusammenfassung des Workflows.
