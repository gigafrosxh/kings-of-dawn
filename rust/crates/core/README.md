# kod-core

Deterministischer, headless Simulationskern von Kings of Dawn (Issue #8, T03). Keine Unreal-, Render-, Wandzeit- oder Float-Abhängigkeiten. Entscheidungen: [ADR 0003](../../../docs/decisions/ADR-0003-deterministic-simulation-core.md).

## Bauen und testen

```bash
cd rust
cargo test -p kod-core
cargo clippy -p kod-core --all-targets -- -D warnings
cargo fmt --check
```

## Domain anbinden

```rust
use kod_core::{CommandEnvelope, ActorId, SimDate, Simulation, Tick, SaveFile};
use kod_core::fixture::{Ledger, LedgerCommand, LedgerState}; // Feature "fixtures"

let mut sim = Simulation::<Ledger>::new(42, SimDate::new_year(1836).unwrap(), LedgerState::default());
sim.submit(CommandEnvelope {
    actor: ActorId::new(1),
    sequence: 1,
    execute_at: Tick(0),
    command: LedgerCommand::Open { name: "Staatskasse".into(), deposit: kod_core::Fixed::from_int(100) },
})?;
let report = sim.step();             // Commands (stabil sortiert) + Phasen
let hash = sim.state_hash();         // Reproduzierbarkeits-Fingerabdruck
let bytes = SaveFile::write(&sim);   // versionierter, geprüfter Save
let again = SaveFile::load::<Ledger>(&bytes)?;
```

Neue Domänen implementieren `kod_core::Domain`; Zustände verwenden nur `BTreeMap`/`BTreeSet`, IDs über `define_id!`, Zahlen über `Fixed`, Zufall nur über `TickContext::rng`.
