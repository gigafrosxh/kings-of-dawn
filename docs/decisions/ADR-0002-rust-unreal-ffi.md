# ADR-0002 – Rust–Unreal C-ABI und Plugin KingsOfDawnSimulation

- Status: akzeptiert; Unreal-Seite auf macOS verifiziert, Windows offen (T02, Issue #7)
- Bereich: `rust/crates/ffi` (`kod-ffi`), `unreal/Plugins/KingsOfDawnSimulation`

## Entscheidungen

1. **Staticlib + versionierte C-ABI.** `kod-ffi` wird als `staticlib` gebaut und über das External-Modul `KodFfi` (Build.cs) gelinkt (ARCHITECTURE.md: staticlib bevorzugt). `KOD_ABI_VERSION` (aktuell 1) steht im Header; `kod_sim_create`/`kod_sim_load` verlangen sie als Argument, das Plugin prüft sie beim Modulstart und bricht bei Abweichung ab.
2. **Header aus cbindgen.** `kod_ffi.h` wird mit `scripts/gen-ffi-header.sh` erzeugt und eingecheckt (UE baut ohne Rust-Werkzeuge im Include-Pfad); CI prüft mit `--check`, dass er aktuell ist.
3. **Opaque Handles, klare Eigentümerschaft.** `KodSim*` entsteht nur in `kod_sim_create`/`kod_sim_load` und wird genau einmal mit `kod_sim_destroy` freigegeben (null erlaubt). Fehlgeschlagene Erzeugung liefert nie einen Handle. Keine Rust-Container, Strings oder UObject-Zeiger über die Grenze; Puffer gehören dem Aufrufer (Zwei-Aufruf-Muster mit `BUFFER_TOO_SMALL`).
4. **Fehlercodes statt Exceptions.** Jede Funktion liefert `KodStatus`; Details liefert `kod_last_error` (thread-lokal, UTF-8). Abgelehnte Commands bei der Ausführung erscheinen über `kod_sim_rejections` mit den stabilen Codes aus ADR-0003.
5. **Keine Panics über die Grenze.** Jeder Einstieg läuft in `catch_unwind`; eine Panic ergibt `KOD_STATUS_PANIC` und vergiftet nur diesen Handle (danach `POISONED`, nur `destroy` erlaubt). Der Workspace bleibt bei `panic = "unwind"`, damit das Abfangen wirkt.
6. **Threading.** Ein Handle ist nicht thread-sicher. `FKodSimulationWorker` (FRunnable) besitzt den Handle exklusiv auf einem eigenen Thread und führt Jobs in Einreichungsreihenfolge aus (MPSC-Queue). `UKodSimulationSubsystem` (GameInstanceSubsystem) reicht nur Jobs ein; Ergebnisse gehen per `AsyncTask(GameThread)` mit `TWeakObjectPtr` zurück. `Deinitialize` stoppt den Worker, arbeitet Restjobs ab, gibt den Handle auf dem Worker-Thread frei und joint.
7. **Zahlen über die ABI.** Fixed-Point-Beträge werden als `int64` in 10^-9-Einheiten übertragen (Bereich ±9,2 × 10^9); größere Werte liefern `OVERFLOW`. Ein 128-Bit-Transportformat folgt, wenn echte Domänen es brauchen (ABI-Version erhöhen).
8. **Fixture-Domäne.** Bis zu echten Domänen exportiert die ABI die Ledger-Testdomäne aus `kod-core` (`kod_ledger_*`). Neue Domänen kommen als zusätzliche Funktionen; Entfernen/Ändern bestehender Signaturen erhöht `KOD_ABI_VERSION`.

## Verifikation

- Rust: `rust/crates/ffi/tests/abi.rs` (Roundtrip, Save/Load, Fehlerpfade, Panic-Vergiftung, parallele Handles auf Threads).
- C: `scripts/ffi-smoke.sh` kompiliert `rust/crates/ffi/tests/c/smoke.c` gegen Header und Staticlib (Linux/macOS mit cc, Windows mit MSVC `cl`) – in CI auf allen drei Plattformen.
- Unreal (2026-11-26, macOS arm64, UE 5.8.4, Xcode 27.0): `KingsOfDawnEditor Mac Development` baut; Automation-Tests `KingsOfDawn.Simulation.FfiRoundtrip` und `.WorkerLifecycle` bestanden (`UnrealEditor-Cmd … -ExecCmds="Automation RunTests KingsOfDawn.Simulation; Quit" -nullrhi`).
- Verpackter Mac-Build (UAT `BuildCookRun -platform=Mac -clientconfig=Development -build -cook -stage -pak`): `KingsOfDawn -KodFfiSmoke -nullrhi` → `KodFfiSmoke: PASS` (Create, Commands, 365 Ticks, Save/Load, identischer Hash), Worker und GameInstance-Subsystem sauber beendet, Exit-Code 0.
- **Offen:** verpackter **Windows**-Build mit demselben Smoke-Test (Abnahme #7) – kein Windows-Rechner verfügbar.

## Nächste Schritte

Auf einem Windows-Rechner mit UE 5.8.4: `scripts/build-ffi.ps1`, Editor-Target bauen, Automation-Tests ausführen, Windows-Package erstellen und `KingsOfDawn.exe -KodFfiSmoke -nullrhi` prüfen (Exit-Code 0, Log `KodFfiSmoke: PASS`); Befunde als Bugs gemäß INFO.md erfassen.
