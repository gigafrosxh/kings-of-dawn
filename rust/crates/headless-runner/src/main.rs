//! `kod-headless` – headless runner for CI and local checks (issue #10).
//!
//! ```text
//! kod-headless bench  [--seed N] [--ticks N]   tick throughput + final state hash
//! kod-headless soak   [--seed N] [--years N]   long run with invariant checks
//! kod-headless verify-replay <file>            re-run a replay log, compare hashes
//! kod-headless verify-save <file>              load a save, print its state hash
//! kod-headless gen-fixtures <dir>              (re)write the regression fixtures
//! ```
//!
//! Wall-clock time is only used here for reporting; the simulation itself
//! never reads it.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use kod_core::fixture::{self, Ledger, LedgerCommand, LedgerState};
use kod_core::{Decode, Encode, Replay, ReplayLog, SaveFile, SimDate, Simulation, Tick};

/// Seed of all committed fixtures ("KOD_SEED").
pub const FIXTURE_SEED: u64 = 0x4B4F_445F_5345_4544;
/// Scenario seed of all committed fixtures.
pub const FIXTURE_SCENARIO: u64 = 11;
/// Recorded ticks of the replay fixture.
pub const FIXTURE_TICKS: u64 = 365;

fn start_date() -> SimDate {
    SimDate::new(1836, 1, 1).expect("valid date")
}

fn new_sim(seed: u64) -> Simulation<Ledger> {
    Simulation::new(seed, start_date(), LedgerState::default())
}

struct Args {
    seed: u64,
    ticks: u64,
    years: u64,
}

fn parse_flags(rest: &[String]) -> Result<Args, String> {
    let mut args = Args {
        seed: FIXTURE_SEED,
        ticks: 36_525,
        years: 100,
    };
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        let value = it.next().ok_or_else(|| format!("{flag} needs a value"))?;
        let number: u64 = value
            .parse()
            .map_err(|_| format!("{flag}: not a number: {value}"))?;
        match flag.as_str() {
            "--seed" => args.seed = number,
            "--ticks" => args.ticks = number,
            "--years" => args.years = number,
            _ => return Err(format!("unknown flag {flag}")),
        }
    }
    Ok(args)
}

/// Runs `ticks` ticks of the fixture scenario; returns the simulation and the
/// number of applied/rejected commands.
fn run_scenario(
    seed: u64,
    ticks: u64,
    mut each_tick: impl FnMut(&Simulation<Ledger>) -> Result<(), String>,
) -> Result<(Simulation<Ledger>, u64, u64), String> {
    let stream = fixture::scenario(FIXTURE_SCENARIO, ticks);
    let mut sim = new_sim(seed);
    let mut cmds = stream.into_iter().peekable();
    let (mut applied, mut rejected) = (0u64, 0u64);
    for _ in 0..ticks {
        let now = sim.next_tick();
        while let Some((_, env)) = cmds.next_if(|(t, _)| *t == now) {
            sim.submit(env).map_err(|e| e.to_string())?;
        }
        for (_, _, outcome) in sim.step().commands {
            match outcome {
                kod_core::CommandOutcome::Applied => applied += 1,
                kod_core::CommandOutcome::Rejected(_) => rejected += 1,
            }
        }
        each_tick(&sim)?;
    }
    Ok((sim, applied, rejected))
}

fn bench(args: &Args) -> Result<(), String> {
    let started = Instant::now();
    let (sim, applied, rejected) = run_scenario(args.seed, args.ticks, |_| Ok(()))?;
    let elapsed = started.elapsed();
    let hash_started = Instant::now();
    let hash = sim.state_hash();
    let hash_time = hash_started.elapsed();
    let save_started = Instant::now();
    let bytes = SaveFile::write(&sim);
    let save_time = save_started.elapsed();
    let load_started = Instant::now();
    let loaded = SaveFile::load::<Ledger>(&bytes).map_err(|e| e.to_string())?;
    let load_time = load_started.elapsed();
    if loaded.state_hash() != hash {
        return Err("save/load changed the state hash".into());
    }
    let tps = args.ticks as f64 / elapsed.as_secs_f64().max(1e-9);
    println!(
        "ticks={} seed={} accounts={} applied={applied} rejected={rejected}",
        args.ticks,
        args.seed,
        sim.state().accounts.len()
    );
    println!(
        "run_ms={:.3} ticks_per_s={tps:.0}",
        elapsed.as_secs_f64() * 1e3
    );
    println!(
        "hash_us={} save_us={} load_us={} save_bytes={}",
        hash_time.as_micros(),
        save_time.as_micros(),
        load_time.as_micros(),
        bytes.len()
    );
    println!("final_date={} state_hash={hash}", sim.current_date());
    Ok(())
}

fn soak(args: &Args) -> Result<(), String> {
    // 365.2425 days per Gregorian year.
    let ticks = args.years * 365_2425 / 10_000;
    let started = Instant::now();
    let (sim, applied, rejected) = run_scenario(args.seed, ticks, |s| {
        fixture::check_invariants(s.state()).map_err(|e| format!("{}: {e}", s.current_date()))
    })?;
    println!(
        "soak years={} ticks={ticks} applied={applied} rejected={rejected} invariants=ok",
        args.years
    );
    println!(
        "run_ms={:.1} final_date={} state_hash={}",
        started.elapsed().as_secs_f64() * 1e3,
        sim.current_date(),
        sim.state_hash()
    );
    Ok(())
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn verify_replay(path: &Path) -> Result<(), String> {
    let log = ReplayLog::<LedgerCommand>::from_canonical_bytes(&read(path)?)
        .map_err(|e| e.to_string())?;
    let sim = log.verify::<Ledger>().map_err(|e| e.to_string())?;
    println!(
        "replay ok: {} commands, {} checkpoints, end {} state_hash={}",
        log.commands.len(),
        log.checkpoints.len(),
        log.end_tick,
        sim.state_hash()
    );
    Ok(())
}

fn verify_save(path: &Path) -> Result<(), String> {
    let sim = SaveFile::load::<Ledger>(&read(path)?).map_err(|e| e.to_string())?;
    println!(
        "save ok: next {} date {} state_hash={}",
        sim.next_tick(),
        sim.current_date(),
        sim.state_hash()
    );
    Ok(())
}

/// Builds the replay fixture: the scenario recorded with checkpoints every 30 ticks.
fn record_fixture_replay() -> Result<(Simulation<Ledger>, ReplayLog<LedgerCommand>), String> {
    let stream = fixture::scenario(FIXTURE_SCENARIO, FIXTURE_TICKS);
    let mut rec = Replay::new(new_sim(FIXTURE_SEED), 30);
    let mut cmds = stream.into_iter().peekable();
    for _ in 0..FIXTURE_TICKS {
        let now = rec.sim().next_tick();
        while let Some((_, env)) = cmds.next_if(|(t, _)| *t == now) {
            rec.submit(env).map_err(|e| e.to_string())?;
        }
        rec.step();
    }
    Ok(rec.finish())
}

fn gen_fixtures(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let (sim, log) = record_fixture_replay()?;
    let save = SaveFile::write(&sim);
    let replay = log.to_canonical_bytes();
    let write = |name: &str, bytes: &[u8]| -> Result<PathBuf, String> {
        let path = dir.join(name);
        std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(path)
    };
    let save_path = write("ledger-v1-365.kodsave", &save)?;
    let replay_path = write("ledger-r2-365.kodreplay", &replay)?;
    println!(
        "wrote {} ({} bytes) and {} ({} bytes), state_hash={}",
        save_path.display(),
        save.len(),
        replay_path.display(),
        replay.len(),
        sim.state_hash()
    );
    debug_assert_eq!(sim.next_tick(), Tick(FIXTURE_TICKS));
    Ok(())
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let result = match argv.first().map(String::as_str) {
        Some("bench") => parse_flags(&argv[1..]).and_then(|a| bench(&a)),
        Some("soak") => parse_flags(&argv[1..]).and_then(|a| soak(&a)),
        Some("verify-replay") if argv.len() == 2 => verify_replay(Path::new(&argv[1])),
        Some("verify-save") if argv.len() == 2 => verify_save(Path::new(&argv[1])),
        Some("gen-fixtures") if argv.len() == 2 => gen_fixtures(Path::new(&argv[1])),
        _ => Err("usage: kod-headless <bench|soak> [--seed N] [--ticks N] [--years N] | verify-replay <file> | verify-save <file> | gen-fixtures <dir>".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
