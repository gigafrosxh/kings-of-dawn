//! ABI contract tests through the exported `extern "C"` functions.

use std::ptr;

use kod_ffi::*;

const START: KodDate = KodDate {
    year: 1836,
    month: 1,
    day: 1,
};

fn create(seed: u64) -> *mut KodSim {
    let mut sim = ptr::null_mut();
    assert_eq!(
        unsafe { kod_sim_create(KOD_ABI_VERSION, seed, START, &mut sim) },
        KodStatus::Ok
    );
    assert!(!sim.is_null());
    sim
}

fn hash(sim: *mut KodSim) -> KodHash128 {
    let mut h = KodHash128::default();
    assert_eq!(unsafe { kod_sim_state_hash(sim, &mut h) }, KodStatus::Ok);
    h
}

fn last_error() -> String {
    let mut len = 0usize;
    assert_eq!(
        unsafe { kod_last_error(ptr::null_mut(), 0, &mut len) },
        if len == 0 {
            KodStatus::Ok
        } else {
            KodStatus::BufferTooSmall
        }
    );
    let mut buf = vec![0u8; len];
    assert_eq!(
        unsafe { kod_last_error(buf.as_mut_ptr(), buf.len(), &mut len) },
        KodStatus::Ok
    );
    String::from_utf8(buf).unwrap()
}

fn open(sim: *mut KodSim, actor: u32, seq: u64, at: u64, name: &str, deposit: i64) -> KodStatus {
    unsafe { kod_ledger_open(sim, actor, seq, at, name.as_ptr(), name.len(), deposit) }
}

#[test]
fn abi_version_is_checked() {
    assert_eq!(kod_abi_version(), KOD_ABI_VERSION);
    let mut sim = ptr::null_mut();
    assert_eq!(
        unsafe { kod_sim_create(KOD_ABI_VERSION + 1, 1, START, &mut sim) },
        KodStatus::AbiMismatch
    );
    assert!(sim.is_null());
    assert!(last_error().contains("ABI version"));
}

#[test]
fn roundtrip_commands_queries_hash_and_save() {
    let sim = create(42);
    assert_eq!(
        open(sim, 1, 1, 0, "Staatskasse", 100_000_000_000),
        KodStatus::Ok
    );
    assert_eq!(open(sim, 2, 1, 0, "Händler", 5_000_000_000), KodStatus::Ok);
    assert_eq!(
        unsafe { kod_ledger_transfer(sim, 1, 2, 1, 1, 2, 25_000_000_000) },
        KodStatus::Ok
    );
    // invalid: actor 2 debits actor 1's account
    assert_eq!(
        unsafe { kod_ledger_transfer(sim, 2, 2, 1, 1, 2, 1) },
        KodStatus::Ok
    );

    let mut summary = KodStepSummary::default();
    assert_eq!(unsafe { kod_sim_step(sim, 2, &mut summary) }, KodStatus::Ok);
    assert_eq!(
        (summary.next_tick, summary.applied, summary.rejected),
        (2, 3, 1)
    );
    let mut rej = [KodRejection::default(); 4];
    let mut n = 0;
    assert_eq!(
        unsafe { kod_sim_rejections(sim, rej.as_mut_ptr(), rej.len(), &mut n) },
        KodStatus::Ok
    );
    assert_eq!(n, 1);
    assert_eq!(
        rej[0],
        KodRejection {
            actor: 2,
            sequence: 2,
            code: 1
        }
    );

    let mut balance = 0i64;
    assert_eq!(
        unsafe { kod_ledger_balance(sim, 2, &mut balance) },
        KodStatus::Ok
    );
    assert!(balance >= 30_000_000_000, "{balance}"); // 5 + 25 (+ possible windfall)
    assert_eq!(
        unsafe { kod_ledger_balance(sim, 99, &mut balance) },
        KodStatus::NotFound
    );

    let mut date = KodDate::default();
    assert_eq!(unsafe { kod_sim_date(sim, &mut date) }, KodStatus::Ok);
    assert_eq!(
        date,
        KodDate {
            year: 1836,
            month: 1,
            day: 3
        }
    );

    // Save with the two-call pattern, load, compare hashes, continue identically.
    let mut len = 0;
    assert_eq!(
        unsafe { kod_sim_save(sim, ptr::null_mut(), 0, &mut len) },
        KodStatus::BufferTooSmall
    );
    let mut bytes = vec![0u8; len];
    assert_eq!(
        unsafe { kod_sim_save(sim, bytes.as_mut_ptr(), bytes.len(), &mut len) },
        KodStatus::Ok
    );
    let mut loaded = ptr::null_mut();
    assert_eq!(
        unsafe { kod_sim_load(KOD_ABI_VERSION, bytes.as_ptr(), bytes.len(), &mut loaded) },
        KodStatus::Ok
    );
    assert_eq!(hash(sim), hash(loaded));
    unsafe {
        kod_sim_step(sim, 400, ptr::null_mut());
        kod_sim_step(loaded, 400, ptr::null_mut());
    }
    assert_eq!(hash(sim), hash(loaded));

    unsafe {
        kod_sim_destroy(sim);
        kod_sim_destroy(loaded);
    }
}

#[test]
fn error_paths_never_crash() {
    unsafe {
        kod_sim_destroy(ptr::null_mut());
        assert_eq!(
            kod_sim_step(ptr::null_mut(), 1, ptr::null_mut()),
            KodStatus::NullPointer
        );
        assert_eq!(
            kod_sim_state_hash(ptr::null_mut(), ptr::null_mut()),
            KodStatus::NullPointer
        );
        let mut out = ptr::null_mut();
        assert_eq!(
            kod_sim_create(
                KOD_ABI_VERSION,
                1,
                KodDate {
                    year: 1900,
                    month: 2,
                    day: 29
                },
                &mut out
            ),
            KodStatus::InvalidArgument
        );
        assert_eq!(
            kod_sim_create(KOD_ABI_VERSION, 1, START, ptr::null_mut()),
            KodStatus::NullPointer
        );
        let junk = [1u8, 2, 3];
        assert_eq!(
            kod_sim_load(KOD_ABI_VERSION, junk.as_ptr(), junk.len(), &mut out),
            KodStatus::InvalidSave
        );
        assert!(out.is_null());
    }
    let sim = create(1);
    unsafe {
        assert_eq!(
            kod_sim_state_hash(sim, ptr::null_mut()),
            KodStatus::NullPointer
        );
        let bad_utf8 = [0xffu8, 0xfe];
        assert_eq!(
            kod_ledger_open(sim, 1, 1, 0, bad_utf8.as_ptr(), 2, 0),
            KodStatus::InvalidArgument
        );
        assert_eq!(
            kod_ledger_open(sim, 1, 1, 0, ptr::null(), 3, 0),
            KodStatus::NullPointer
        );
        assert_eq!(open(sim, 1, 5, 0, "a", 0), KodStatus::Ok);
        assert_eq!(open(sim, 1, 5, 0, "b", 0), KodStatus::SubmitRejected); // sequence reused
        kod_sim_step(sim, 3, ptr::null_mut());
        assert_eq!(open(sim, 1, 6, 1, "c", 0), KodStatus::SubmitRejected); // tick in the past
        assert!(last_error().contains("submitted at"));
        kod_sim_destroy(sim);
    }
}

#[test]
fn panic_is_caught_and_poisons_only_that_handle() {
    let sim = create(3);
    let other = create(3);
    unsafe {
        assert_eq!(kod_test_panic(sim), KodStatus::Panic);
        assert!(last_error().contains("deliberate test panic"));
        assert_eq!(kod_sim_step(sim, 1, ptr::null_mut()), KodStatus::Poisoned);
        assert_eq!(kod_sim_step(other, 1, ptr::null_mut()), KodStatus::Ok);
        kod_sim_destroy(sim);
        kod_sim_destroy(other);
    }
}

#[test]
fn independent_handles_on_worker_threads_are_deterministic() {
    struct Send(*mut KodSim);
    // SAFETY: each handle is moved to and used by exactly one thread.
    unsafe impl std::marker::Send for Send {}

    let workers: Vec<_> = (0..4)
        .map(|_| {
            let handle = Send(create(77));
            std::thread::spawn(move || {
                let h = handle;
                for seq in 1..=50u64 {
                    assert_eq!(
                        open(
                            h.0,
                            1,
                            seq,
                            seq,
                            &format!("acct-{}", seq % 7),
                            1_000_000_000
                        ),
                        KodStatus::Ok
                    );
                }
                unsafe { kod_sim_step(h.0, 365, ptr::null_mut()) };
                let result = hash(h.0);
                unsafe { kod_sim_destroy(h.0) };
                result
            })
        })
        .collect();
    let hashes: Vec<KodHash128> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert!(hashes.windows(2).all(|w| w[0] == w[1]));
}
