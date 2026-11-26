//! Save/replay regression fixtures (issue #10).
//!
//! `rust/fixtures/*` were written by `kod-headless gen-fixtures` with save
//! container v1, replay format v2, kernel state v1 and ledger schema v1. They
//! must keep loading unchanged; if a format version is bumped intentionally,
//! keep the old files, add new ones and test the migration path.

use kod_core::fixture::{self, Ledger, LedgerCommand, LedgerState};
use kod_core::{Decode, Encode, Replay, ReplayLog, SaveFile, SimDate, Simulation, StateHash, Tick};

const SAVE_V1: &[u8] = include_bytes!("../../../fixtures/ledger-v1-365.kodsave");
const REPLAY_R2: &[u8] = include_bytes!("../../../fixtures/ledger-r2-365.kodreplay");

/// Same value as `GOLDEN_365` in `determinism.rs`.
const HASH_365: StateHash = StateHash(0x492d_dd60_8868_a007_20ee_bd74_66b4_00d4);
const SEED: u64 = 0x4B4F_445F_5345_4544;

fn record() -> (Simulation<Ledger>, ReplayLog<LedgerCommand>) {
    let sim = Simulation::<Ledger>::new(
        SEED,
        SimDate::new(1836, 1, 1).unwrap(),
        LedgerState::default(),
    );
    let mut rec = Replay::new(sim, 30);
    let mut cmds = fixture::scenario(11, 365).into_iter().peekable();
    for _ in 0..365 {
        let now = rec.sim().next_tick();
        while let Some((_, env)) = cmds.next_if(|(t, _)| *t == now) {
            rec.submit(env).unwrap();
        }
        rec.step();
    }
    rec.finish()
}

#[test]
fn committed_save_v1_loads_with_known_hash() {
    let sim = SaveFile::load::<Ledger>(SAVE_V1).unwrap();
    assert_eq!(sim.state_hash(), HASH_365);
    assert_eq!(sim.next_tick(), Tick(365));
    fixture::check_invariants(sim.state()).unwrap();
}

#[test]
fn committed_replay_v1_verifies() {
    let log = ReplayLog::<LedgerCommand>::from_canonical_bytes(REPLAY_R2).unwrap();
    let sim = log.verify::<Ledger>().unwrap();
    assert_eq!(sim.state_hash(), HASH_365);
    assert_eq!(log.final_hash(), Some(HASH_365));
}

#[test]
fn current_writer_reproduces_fixtures_byte_for_byte() {
    let (sim, log) = record();
    assert_eq!(
        SaveFile::write(&sim),
        SAVE_V1,
        "save encoding drifted; bump a format version"
    );
    assert_eq!(
        log.to_canonical_bytes(),
        REPLAY_R2,
        "replay encoding drifted; bump a format version"
    );
}

#[test]
fn ten_year_soak_keeps_invariants() {
    let mut sim = Simulation::<Ledger>::new(
        SEED,
        SimDate::new(1836, 1, 1).unwrap(),
        LedgerState::default(),
    );
    let ticks = 3653;
    let mut cmds = fixture::scenario(11, ticks).into_iter().peekable();
    for _ in 0..ticks {
        let now = sim.next_tick();
        while let Some((_, env)) = cmds.next_if(|(t, _)| *t == now) {
            sim.submit(env).unwrap();
        }
        sim.step();
        if let Err(err) = fixture::check_invariants(sim.state()) {
            panic!("{}: {err}", sim.current_date());
        }
    }
    assert_eq!(sim.current_date(), SimDate::new(1846, 1, 1).unwrap());
}
