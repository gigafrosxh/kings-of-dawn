# ADR 0003 – Deterministischer Simulationskern und Persistenzverträge

- Status: akzeptiert (T03, Issue #8)
- Bereich: `rust/crates/core` (Crate `kod-core`)

## Kontext

Die Simulation muss headless, reproduzierbar und unabhängig von Render-FPS und Wandzeit laufen (docs/SIMULATION.md, docs/ARCHITECTURE.md). Replays, Desync-Erkennung, Saves und das FFI (#7) brauchen einen gemeinsamen, stabilen Vertrag, bevor fachliche Domänen (ab F-01, #11) entstehen.

## Entscheidungen

1. **Kernel/Domain-Trennung.** `Simulation<D: Domain>` besitzt Zeit, Command-Queue, RNG, Hash und Persistenz. Domänen implementieren `Domain` (State, Command, Event, Query) und kennen weder Threads noch Engine.
2. **Command-Vertrag.** `validate` (autorisieren + prüfen, seiteneffektfrei) und danach `apply` (darf nicht fehlschlagen). Abgelehnte Commands ändern den Domain-State nie; angenommene werden vollständig im selben Tick angewandt. Fehlercodes sind stabil (1 Unauthorized, 2 UnknownTarget, 3 InvalidArgument, 4 Conflict) und für das FFI gedacht.
3. **Stabile Reihenfolge.** Commands eines Ticks laufen sortiert nach `(execute_at, actor, sequence)`, unabhängig von Einreichungsreihenfolge oder Threads. `sequence` muss je Akteur streng steigen (Schutz gegen doppelte Einreichung). Danach laufen die Phasen in der festen Reihenfolge aus SIMULATION.md (`Phase::ORDER`).
4. **Zeit.** 1 Tick = 1 Tag. Datum astronomisch im proleptisch-gregorianischen Kalender (Jahr 0 = 1 v. Chr.), Anzeige nur über `SimDate::era_year`. Wochen-/Monats-/Jahreskadenzen kommen aus dem Kalender (`Cadence`), nicht aus Tick-Zählung.
5. **Feste IDs.** Typisierte `u32`-IDs (`define_id!`), monoton vergeben (`IdAllocator` ist Teil des States), nie wiederverwendet. Zustand nur in `BTreeMap`/`BTreeSet`; `HashMap` ist im Codec bewusst nicht kodierbar.
6. **Fixed-Point.** `Fixed` = `i128` mit Skala 10^9 (dezimal, für Geld exakt). Multiplikation/Division runden *half to even*; alle Operationen geprüft (Overflow/Division durch 0 als Fehler). Floats sind im Crate per Lint verboten.
7. **Zufall.** `DetRng` (xoshiro256**, Seed über SplitMix64), Zustand Teil des Saves; `fork(stream)` für unabhängige Teilströme.
8. **Kanonischer Codec + State-Hash.** Ein eigener, abhängigkeitsfreier Little-Endian-Codec dient Hash und Save zugleich. State-Hash = FNV-1a/128 über Schema-ID, Schema-Version und kanonische Bytes des gesamten Simulationszustands (inkl. Queue, RNG, Sequenzen). Er ist ein Reproduzierbarkeits-Fingerabdruck, kein kryptografischer Schutz.
9. **Save-Container v1.** `KODSAVE\0`, Container-Version, Schema-ID/-Version, State-Hash, Payload, Prüfsumme. Laden prüft Magic, Version, Prüfsumme, Schema und State-Hash und liefert nie einen Teilzustand. Ältere Schema-Versionen laufen über `Domain::migrate`, neuere werden abgelehnt. SQLite-Metadaten und Kompression (ARCHITECTURE.md) umhüllen diesen Container später.
10. **Replay.** `ReplayLog` = Start-Save + eingereichte Commands mit Tick + Hash-Checkpoints. `verify` spielt headless nach und meldet die erste Divergenz.
11. **Keine externen Abhängigkeiten** im Kern: minimiert Determinismusrisiken und hält das FFI-Staticlib-Linking (#7) einfach.

## Konsequenzen

- Jede inkompatible Änderung an Codec, RNG, Rundung oder State-Layout ändert die Golden-Hashes in `tests/determinism.rs` und erfordert eine Versionserhöhung (`KERNEL_STATE_VERSION`, `Domain::SCHEMA_VERSION`, Container- oder Replay-Format).
- `validate` sieht nur den State, nicht Tick/Datum; datumsabhängige Regeln gehören in Phasen. Eine Erweiterung wäre eine Vertragsänderung.
- Der Start-Save im Replay macht Logs bei großen Welten groß; später kann er durch Szenario-ID + Hash ersetzt werden.
- Das Fixture `kod_core::fixture::Ledger` (Feature `fixtures`) ist ausschließlich Testinfrastruktur für Kernel-Tests und den FFI-Roundtrip (#7), keine Spielmechanik.
