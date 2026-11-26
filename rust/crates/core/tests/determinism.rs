//! Acceptance tests of #8: identical command streams replay to the same state
//! hash, and invalid commands never change state.

use kod_core::fixture::{AccountId, Ledger, LedgerCommand, LedgerQuery, LedgerState};
use kod_core::{
    ActorId, CommandEnvelope, CommandError, CommandOutcome, DetRng, Fixed, Replay, ReplayError,
    SaveError, SaveFile, SimDate, Simulation, StateHash, SubmitError, Tick,
};

const SEED: u64 = 0x4B4F_445F_5345_4544; // "KOD_SEED"

fn start() -> SimDate {
    SimDate::new(1836, 1, 1).unwrap()
}

fn sim(seed: u64) -> Simulation<Ledger> {
    Simulation::new(seed, start(), LedgerState::default())
}

fn env(
    actor: u32,
    sequence: u64,
    at: u64,
    command: LedgerCommand,
) -> CommandEnvelope<LedgerCommand> {
    CommandEnvelope {
        actor: ActorId::new(actor),
        sequence,
        execute_at: Tick(at),
        command,
    }
}

fn fx(s: &str) -> Fixed {
    Fixed::parse(s).unwrap()
}

/// Deterministic pseudo-random command stream (valid and invalid mixed),
/// as `(submit before tick, envelope)`.
fn command_stream(seed: u64, ticks: u64) -> Vec<(u64, CommandEnvelope<LedgerCommand>)> {
    let mut rng = DetRng::from_seed(seed);
    let mut out = Vec::new();
    let mut seq = [0u64; 4];
    for tick in 0..ticks {
        for _ in 0..rng.below(4) {
            let actor = 1 + rng.below(3) as u32;
            seq[actor as usize] += 1;
            let command = match rng.below(10) {
                0..=2 => LedgerCommand::Open {
                    name: format!("acct-{}", rng.below(40)),
                    deposit: Fixed::from_raw(
                        i128::from(rng.below(5_000_000_000_000)) - 500_000_000_000,
                    ),
                },
                3..=8 => LedgerCommand::Transfer {
                    from: AccountId::new(rng.below(30) as u32),
                    to: AccountId::new(rng.below(30) as u32),
                    amount: Fixed::from_raw(i128::from(rng.below(900_000_000_000))),
                },
                _ => LedgerCommand::Close {
                    account: AccountId::new(rng.below(30) as u32),
                },
            };
            out.push((
                tick,
                env(actor, seq[actor as usize], tick + rng.below(3), command),
            ));
        }
    }
    out
}

fn run(
    seed: u64,
    stream: &[(u64, CommandEnvelope<LedgerCommand>)],
    ticks: u64,
) -> (Simulation<Ledger>, usize, usize) {
    let mut s = sim(seed);
    let (mut applied, mut rejected) = (0, 0);
    let mut it = stream.iter().peekable();
    for tick in 0..ticks {
        while let Some((_, e)) = it.next_if(|(t, _)| *t == tick) {
            s.submit(e.clone()).unwrap();
        }
        for (_, _, outcome) in s.step().commands {
            match outcome {
                CommandOutcome::Applied => applied += 1,
                CommandOutcome::Rejected(_) => rejected += 1,
            }
        }
    }
    (s, applied, rejected)
}

#[test]
fn identical_commands_yield_identical_state_hash() {
    let stream = command_stream(11, 730);
    let (a, applied, rejected) = run(SEED, &stream, 730);
    let (b, ..) = run(SEED, &stream, 730);
    assert!(
        applied > 100 && rejected > 100,
        "stream must exercise both paths: {applied}/{rejected}"
    );
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.state(), b.state());
    assert_eq!(a.current_date(), SimDate::new(1837, 12, 31).unwrap()); // 1836 is a leap year
}

#[test]
fn different_seed_or_commands_change_the_hash() {
    let stream = command_stream(11, 200);
    let (base, ..) = run(SEED, &stream, 200);
    let (other_seed, ..) = run(SEED + 1, &stream, 200);
    let (other_cmds, ..) = run(SEED, &command_stream(12, 200), 200);
    assert_ne!(base.state_hash(), other_seed.state_hash());
    assert_ne!(base.state_hash(), other_cmds.state_hash());
}

#[test]
fn execution_order_is_independent_of_submission_order() {
    // Per-actor sequences must be submitted in order, but interleaving across
    // actors (e.g. network or thread timing) must not matter.
    let a = [
        env(
            1,
            1,
            0,
            LedgerCommand::Open {
                name: "a".into(),
                deposit: fx("10"),
            },
        ),
        env(
            1,
            2,
            0,
            LedgerCommand::Transfer {
                from: AccountId::new(1),
                to: AccountId::new(2),
                amount: fx("1"),
            },
        ),
    ];
    let b = [
        env(
            2,
            1,
            0,
            LedgerCommand::Open {
                name: "b".into(),
                deposit: fx("5"),
            },
        ),
        env(
            2,
            2,
            0,
            LedgerCommand::Transfer {
                from: AccountId::new(2),
                to: AccountId::new(1),
                amount: fx("3"),
            },
        ),
    ];
    let quiet = || {
        let state = LedgerState {
            windfall_chance: Fixed::ZERO,
            monthly_interest: Fixed::ZERO,
            ..LedgerState::default()
        };
        Simulation::<Ledger>::new(SEED, start(), state)
    };
    let mut forward = quiet();
    let mut interleaved = quiet();
    for c in a.iter().chain(b.iter()) {
        forward.submit(c.clone()).unwrap();
    }
    for c in [&b[0], &a[0], &b[1], &a[1]] {
        interleaved.submit(c.clone()).unwrap();
    }
    let rf = forward.step();
    let ri = interleaved.step();
    assert_eq!(rf.commands, ri.commands);
    assert_eq!(rf.events, ri.events);
    assert_eq!(forward.state_hash(), interleaved.state_hash());
    // Order is (actor, sequence): actor 1 opens "a" (id 1), its transfer to the
    // not yet existing id 2 is rejected, then actor 2 opens "b" (id 2) and
    // transfers 3 to "a".
    assert!(matches!(
        rf.commands[1].2,
        CommandOutcome::Rejected(CommandError::UnknownTarget(_))
    ));
    assert_eq!(
        forward.query(&LedgerQuery::Balance(AccountId::new(1))),
        Some(fx("13"))
    );
    assert_eq!(
        forward.query(&LedgerQuery::Balance(AccountId::new(2))),
        Some(fx("2"))
    );
}

#[test]
fn invalid_commands_do_not_change_state() {
    let mut s = sim(SEED);
    s.submit(env(
        1,
        1,
        0,
        LedgerCommand::Open {
            name: "treasury".into(),
            deposit: fx("100"),
        },
    ))
    .unwrap();
    s.submit(env(
        2,
        1,
        0,
        LedgerCommand::Open {
            name: "rival".into(),
            deposit: fx("1"),
        },
    ))
    .unwrap();
    s.step();
    let mut control = s.clone();

    let invalid = [
        (
            1,
            LedgerCommand::Open {
                name: "treasury".into(),
                deposit: fx("1"),
            },
            4,
        ), // duplicate name
        (
            1,
            LedgerCommand::Open {
                name: String::new(),
                deposit: fx("1"),
            },
            3,
        ),
        (
            1,
            LedgerCommand::Open {
                name: "neg".into(),
                deposit: fx("-1"),
            },
            3,
        ),
        (
            2,
            LedgerCommand::Transfer {
                from: AccountId::new(1),
                to: AccountId::new(2),
                amount: fx("1"),
            },
            1,
        ), // not owner
        (
            1,
            LedgerCommand::Transfer {
                from: AccountId::new(1),
                to: AccountId::new(2),
                amount: fx("1000"),
            },
            4,
        ),
        (
            1,
            LedgerCommand::Transfer {
                from: AccountId::new(1),
                to: AccountId::new(1),
                amount: fx("1"),
            },
            3,
        ),
        (
            1,
            LedgerCommand::Transfer {
                from: AccountId::new(1),
                to: AccountId::new(9),
                amount: fx("1"),
            },
            2,
        ),
        (
            1,
            LedgerCommand::Transfer {
                from: AccountId::new(1),
                to: AccountId::new(2),
                amount: Fixed::ZERO,
            },
            3,
        ),
        (
            1,
            LedgerCommand::Close {
                account: AccountId::new(1),
            },
            4,
        ), // not empty
        (
            1,
            LedgerCommand::Close {
                account: AccountId::new(2),
            },
            1,
        ), // not owner
    ];
    for (i, (actor, cmd, _)) in invalid.iter().enumerate() {
        s.submit(env(*actor, 10 + i as u64, 1, cmd.clone()))
            .unwrap();
    }
    let report = s.step();
    control.step();

    assert_eq!(report.commands.len(), invalid.len());
    for (_, sequence, outcome) in &report.commands {
        let code = invalid[(*sequence - 10) as usize].2;
        match outcome {
            CommandOutcome::Rejected(err) => assert_eq!(err.code(), code, "{err}"),
            CommandOutcome::Applied => panic!("invalid command applied"),
        }
    }
    assert!(
        report
            .events
            .iter()
            .all(|e| !matches!(e, kod_core::fixture::LedgerEvent::Opened(_)))
    );
    // Apart from the consumed sequence numbers, the state is identical to a
    // simulation that never received the commands.
    assert_eq!(s.state(), control.state());
    assert_ne!(
        s.state_hash(),
        control.state_hash(),
        "sequence bookkeeping is part of the state"
    );
    let mut s_tail = s.clone();
    let mut c_tail = control.clone();
    s_tail.run_until(Tick(90));
    c_tail.run_until(Tick(90));
    assert_eq!(s_tail.state(), c_tail.state());
}

#[test]
fn rejected_command_matches_validation_error() {
    let mut s = sim(SEED);
    s.submit(env(
        1,
        1,
        0,
        LedgerCommand::Close {
            account: AccountId::new(42),
        },
    ))
    .unwrap();
    let report = s.step();
    assert_eq!(
        report.commands[0].2,
        CommandOutcome::Rejected(CommandError::UnknownTarget("acct42".into()))
    );
}

#[test]
fn scheduler_refuses_past_ticks_and_replayed_sequences() {
    let mut s = sim(SEED);
    s.run_until(Tick(5));
    assert_eq!(
        s.submit(env(
            1,
            1,
            4,
            LedgerCommand::Close {
                account: AccountId::new(1)
            }
        )),
        Err(SubmitError::InPast {
            execute_at: Tick(4),
            next_tick: Tick(5)
        })
    );
    s.submit(env(
        1,
        7,
        5,
        LedgerCommand::Close {
            account: AccountId::new(1),
        },
    ))
    .unwrap();
    assert_eq!(
        s.submit(env(
            1,
            7,
            6,
            LedgerCommand::Close {
                account: AccountId::new(1)
            }
        )),
        Err(SubmitError::SequenceNotIncreasing {
            actor: ActorId::new(1),
            last: 7
        })
    );
    assert_eq!(s.queued(), 1);
}

#[test]
fn monthly_interest_uses_calendar_cadence_and_defined_rounding() {
    let state = LedgerState {
        windfall_chance: Fixed::ZERO,
        ..LedgerState::default()
    };
    let mut s = Simulation::<Ledger>::new(SEED, SimDate::new(1900, 1, 31).unwrap(), state);
    s.submit(env(
        1,
        1,
        0,
        LedgerCommand::Open {
            name: "a".into(),
            deposit: fx("0.000000002"),
        },
    ))
    .unwrap();
    s.step(); // 31 Jan: no interest
    let report = s.step(); // 1 Feb: interest 0.000000002 * 0.0025 = 5e-12 -> rounds to 0
    assert!(
        report
            .events
            .iter()
            .any(|e| matches!(e, kod_core::fixture::LedgerEvent::InterestPaid { .. }))
    );
    assert_eq!(
        s.query(&LedgerQuery::Balance(AccountId::new(1))),
        Some(fx("0.000000002"))
    );
}

#[test]
fn recorded_replay_verifies_and_detects_divergence() {
    let stream = command_stream(21, 365);
    let mut rec = Replay::new(sim(SEED), 30);
    let mut it = stream.iter().peekable();
    for tick in 0..365 {
        while let Some((_, e)) = it.next_if(|(t, _)| *t == tick) {
            rec.submit(e.clone()).unwrap();
        }
        rec.step();
    }
    let (live, log) = rec.finish();
    assert_eq!(log.checkpoints.len(), 1 + 12 + 1); // start, every 30 ticks, end
    assert_eq!(log.final_hash(), Some(live.state_hash()));

    // Roundtrip through bytes, then replay headless.
    let bytes = kod_core::Encode::to_canonical_bytes(&log);
    let decoded: kod_core::ReplayLog<LedgerCommand> =
        kod_core::Decode::from_canonical_bytes(&bytes).unwrap();
    let replayed = decoded.verify::<Ledger>().unwrap();
    assert_eq!(replayed.state_hash(), live.state_hash());

    // Tampering with one command is detected at the next checkpoint.
    let mut tampered = log.clone();
    let idx = tampered
        .commands
        .iter()
        .position(|(t, e)| t.0 > 100 && matches!(e.command, LedgerCommand::Open { .. }))
        .unwrap();
    if let LedgerCommand::Open { deposit, .. } = &mut tampered.commands[idx].1.command {
        *deposit = fx("123456");
    }
    match tampered.verify::<Ledger>() {
        Err(ReplayError::Divergence { tick, .. }) => assert!(tick.0 > 100),
        other => panic!("expected divergence, got {other:?}"),
    }
}

#[test]
fn save_load_roundtrip_continues_identically() {
    let stream = command_stream(31, 400);
    let (mut original, ..) = run(
        SEED,
        &stream[..stream.iter().position(|(t, _)| *t >= 200).unwrap()],
        200,
    );
    // Queue a future command so the queue is part of the save.
    original
        .submit(env(
            9,
            1,
            250,
            LedgerCommand::Open {
                name: "future".into(),
                deposit: fx("1"),
            },
        ))
        .unwrap();

    let bytes = SaveFile::write(&original);
    let mut loaded = SaveFile::load::<Ledger>(&bytes).unwrap();
    assert_eq!(loaded.state_hash(), original.state_hash());
    assert_eq!(
        SaveFile::write(&loaded),
        bytes,
        "save encoding is canonical"
    );

    original.run_until(Tick(400));
    loaded.run_until(Tick(400));
    assert_eq!(loaded.state_hash(), original.state_hash());
}

#[test]
fn corrupted_or_foreign_saves_are_rejected() {
    let (s, ..) = run(SEED, &command_stream(41, 50), 50);
    let bytes = SaveFile::write(&s);

    for i in [10, bytes.len() / 2, bytes.len() - 20] {
        let mut broken = bytes.clone();
        broken[i] ^= 0x40;
        assert_eq!(
            SaveFile::load::<Ledger>(&broken).unwrap_err(),
            SaveError::Corrupted,
            "byte {i}"
        );
    }
    assert_eq!(
        SaveFile::load::<Ledger>(&bytes[..bytes.len() - 1]).unwrap_err(),
        SaveError::Corrupted
    );
    assert_eq!(
        SaveFile::load::<Ledger>(b"not a save at all").unwrap_err(),
        SaveError::BadMagic
    );

    let mut future = bytes.clone();
    future[8] = 2; // container version 2
    assert_eq!(
        SaveFile::load::<Ledger>(&future).unwrap_err(),
        SaveError::UnsupportedContainer(2)
    );
}

/// Golden hashes freeze the canonical encoding, RNG and rounding. If one of
/// them changes intentionally, bump the affected schema/format version and
/// re-record; an unintended change breaks existing saves and replays.
#[test]
fn golden_state_hash() {
    let (s, ..) = run(SEED, &command_stream(11, 365), 365);
    assert_eq!(s.state_hash(), StateHash(GOLDEN_365));
    let empty = sim(SEED);
    assert_eq!(empty.state_hash(), StateHash(GOLDEN_EMPTY));
}

const GOLDEN_365: u128 = 97271787111575379684882249859281977556;
const GOLDEN_EMPTY: u128 = 12555637202742734912531531974392262754;
