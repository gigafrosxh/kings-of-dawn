//! Versioned C ABI of the Kings of Dawn simulation (issue #7, T02).
//!
//! Contract (see `docs/decisions/ADR-0002-rust-unreal-ffi.md`):
//! - Every function returns a [`KodStatus`]; results go through out-pointers.
//! - [`KodSim`] is an opaque handle created by [`kod_sim_create`] /
//!   [`kod_sim_load`] and released exactly once by [`kod_sim_destroy`].
//! - A handle is not thread-safe: one thread at a time (the Unreal worker).
//!   Different handles may be used concurrently on different threads.
//! - No Rust panic crosses the boundary. A panic is caught, reported as
//!   [`KodStatus::Panic`] and poisons the handle; afterwards only
//!   [`kod_sim_destroy`] is valid.
//! - Buffers are owned by the caller. Variable-size output uses the
//!   two-call pattern: on [`KodStatus::BufferTooSmall`] `out_len` holds the
//!   required size.
//! - [`kod_abi_version`] must equal [`KOD_ABI_VERSION`] of the header the
//!   caller was compiled against; creating a handle checks it.
//!
//! Until real domains exist the ABI exposes the kernel fixture domain
//! `kod_core::fixture::Ledger`, which exercises commands, validation errors,
//! events, saves and hashes end to end.

pub mod scenario;

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

use kod_core::fixture::{AccountId, Ledger, LedgerCommand, LedgerQuery, LedgerState};
use kod_core::{
    ActorId, CommandEnvelope, CommandOutcome, Fixed, SaveFile, SimDate, Simulation, Tick,
};

/// ABI version of this library. Bump on every incompatible change.
pub const KOD_ABI_VERSION: u32 = 1;

/// Maximum number of rejections retained from the last `kod_sim_step` call.
pub const KOD_MAX_REJECTIONS: usize = 4096;

/// Status code of every ABI function.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KodStatus {
    /// Success.
    Ok = 0,
    /// A required pointer was null.
    NullPointer = 1,
    /// An argument was out of range or malformed (e.g. invalid date, UTF-8).
    InvalidArgument = 2,
    /// Output buffer too small; `out_len` holds the required size.
    BufferTooSmall = 3,
    /// Caller and library ABI versions differ.
    AbiMismatch = 4,
    /// The command could not be queued (past tick or non-increasing sequence).
    SubmitRejected = 5,
    /// Save data is corrupted, foreign or of an unsupported version.
    InvalidSave = 6,
    /// The requested entity does not exist.
    NotFound = 7,
    /// A value does not fit the ABI type.
    Overflow = 8,
    /// An internal panic was caught; the handle is poisoned.
    Panic = 9,
    /// The handle was poisoned by an earlier panic.
    Poisoned = 10,
}

/// Opaque simulation handle.
pub struct KodSim {
    sim: Simulation<Ledger>,
    rejections: Vec<KodRejection>,
    poisoned: bool,
}

/// 128-bit state hash, little-endian halves.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KodHash128 {
    /// Low 64 bits.
    pub lo: u64,
    /// High 64 bits.
    pub hi: u64,
}

/// Calendar date, astronomical year (1 BC = 0).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KodDate {
    /// Astronomical year.
    pub year: i32,
    /// 1..=12.
    pub month: u8,
    /// 1..=31.
    pub day: u8,
}

/// Summary of one `kod_sim_step` call.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KodStepSummary {
    /// Next tick after stepping.
    pub next_tick: u64,
    /// Commands applied.
    pub applied: u32,
    /// Commands rejected by validation (details: `kod_sim_rejections`).
    pub rejected: u32,
    /// Events emitted.
    pub events: u32,
}

/// A command rejected during the last `kod_sim_step` call.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KodRejection {
    /// Issuer.
    pub actor: u32,
    /// Issuer's sequence number.
    pub sequence: u64,
    /// `CommandError::code()`: 1 unauthorized, 2 unknown target,
    /// 3 invalid argument, 4 conflict.
    pub code: u32,
}

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

fn set_error(message: impl Into<String>) {
    LAST_ERROR.with(|e| *e.borrow_mut() = message.into());
}

/// Runs `body` with panic protection.
fn guard(body: impl FnOnce() -> KodStatus) -> KodStatus {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(status) => status,
        Err(payload) => {
            let msg = payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".into());
            set_error(format!("panic: {msg}"));
            KodStatus::Panic
        }
    }
}

/// Runs `body` on a live handle; poisons it if `body` panics.
///
/// # Safety
/// `sim` must be null or a live handle from this library.
unsafe fn with_sim(sim: *mut KodSim, body: impl FnOnce(&mut KodSim) -> KodStatus) -> KodStatus {
    if sim.is_null() {
        set_error("null handle");
        return KodStatus::NullPointer;
    }
    // SAFETY: caller guarantees a live, exclusively used handle.
    let handle = unsafe { &mut *sim };
    if handle.poisoned {
        set_error("handle poisoned by an earlier panic");
        return KodStatus::Poisoned;
    }
    let status = guard(|| body(handle));
    if status == KodStatus::Panic {
        // SAFETY: as above; `body` has unwound and no borrow is alive.
        unsafe { (*sim).poisoned = true };
    }
    status
}

/// Writes `value` to `out` if non-null.
///
/// # Safety
/// `out` must be null or valid for writes.
unsafe fn write_out<T>(out: *mut T, value: T) -> KodStatus {
    if out.is_null() {
        set_error("null output pointer");
        return KodStatus::NullPointer;
    }
    // SAFETY: checked non-null; caller guarantees validity.
    unsafe { out.write(value) };
    KodStatus::Ok
}

/// Copies `bytes` into a caller buffer using the two-call pattern.
///
/// # Safety
/// `buf` must be null or valid for `cap` bytes; `out_len` valid for writes.
unsafe fn copy_out(bytes: &[u8], buf: *mut u8, cap: usize, out_len: *mut usize) -> KodStatus {
    if out_len.is_null() {
        set_error("null out_len");
        return KodStatus::NullPointer;
    }
    // SAFETY: checked non-null.
    unsafe { out_len.write(bytes.len()) };
    if buf.is_null() || cap < bytes.len() {
        set_error(format!("buffer too small: need {} bytes", bytes.len()));
        return KodStatus::BufferTooSmall;
    }
    // SAFETY: buffer holds at least `bytes.len()` bytes and cannot overlap
    // our freshly allocated source.
    unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), buf, bytes.len()) };
    KodStatus::Ok
}

fn new_handle(sim: Simulation<Ledger>) -> *mut KodSim {
    Box::into_raw(Box::new(KodSim {
        sim,
        rejections: Vec::new(),
        poisoned: false,
    }))
}

fn check_abi(abi_version: u32) -> Result<(), KodStatus> {
    if abi_version == KOD_ABI_VERSION {
        Ok(())
    } else {
        set_error(format!(
            "ABI version {abi_version} requested, library implements {KOD_ABI_VERSION}"
        ));
        Err(KodStatus::AbiMismatch)
    }
}

/// ABI version implemented by the library.
#[unsafe(no_mangle)]
pub extern "C" fn kod_abi_version() -> u32 {
    KOD_ABI_VERSION
}

/// Copies the last error message of the calling thread (UTF-8, not
/// NUL-terminated) using the two-call pattern.
///
/// # Safety
/// `buf` null or valid for `cap` bytes; `out_len` valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_last_error(
    buf: *mut u8,
    cap: usize,
    out_len: *mut usize,
) -> KodStatus {
    let message = LAST_ERROR.with(|e| e.borrow().clone());
    // SAFETY: forwarded caller contract.
    let status = unsafe { copy_out(message.as_bytes(), buf, cap, out_len) };
    // Reading the message must not replace it (two-call pattern).
    LAST_ERROR.with(|e| *e.borrow_mut() = message);
    status
}

/// Creates a new session with the fixture domain.
///
/// # Safety
/// `out` must be valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_sim_create(
    abi_version: u32,
    seed: u64,
    start: KodDate,
    out: *mut *mut KodSim,
) -> KodStatus {
    guard(|| {
        if let Err(status) = check_abi(abi_version) {
            return status;
        }
        if out.is_null() {
            set_error("null output pointer");
            return KodStatus::NullPointer;
        }
        let Some(date) = SimDate::new(start.year, start.month, start.day) else {
            set_error("invalid start date");
            return KodStatus::InvalidArgument;
        };
        let handle = new_handle(Simulation::new(seed, date, LedgerState::default()));
        // SAFETY: checked non-null.
        unsafe { write_out(out, handle) }
    })
}

/// Releases a handle. Null is ignored. The handle must not be used afterwards.
///
/// # Safety
/// `sim` must be null or a handle from this library not yet destroyed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_sim_destroy(sim: *mut KodSim) {
    if sim.is_null() {
        return;
    }
    let _ = guard(|| {
        // SAFETY: ownership is transferred back exactly once by contract.
        drop(unsafe { Box::from_raw(sim) });
        KodStatus::Ok
    });
}

/// Processes `ticks` ticks. Rejections of this call are available through
/// `kod_sim_rejections` until the next step.
///
/// # Safety
/// `sim` live handle; `out` null or valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_sim_step(
    sim: *mut KodSim,
    ticks: u32,
    out: *mut KodStepSummary,
) -> KodStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        with_sim(sim, |h| {
            h.rejections.clear();
            let mut summary = KodStepSummary::default();
            for _ in 0..ticks {
                let report = h.sim.step();
                summary.events = summary.events.saturating_add(report.events.len() as u32);
                for (actor, sequence, outcome) in report.commands {
                    match outcome {
                        CommandOutcome::Applied => {
                            summary.applied = summary.applied.saturating_add(1)
                        }
                        CommandOutcome::Rejected(err) => {
                            summary.rejected = summary.rejected.saturating_add(1);
                            if h.rejections.len() < KOD_MAX_REJECTIONS {
                                h.rejections.push(KodRejection {
                                    actor: actor.get(),
                                    sequence,
                                    code: err.code(),
                                });
                            }
                        }
                    }
                }
            }
            summary.next_tick = h.sim.next_tick().0;
            if out.is_null() {
                KodStatus::Ok
            } else {
                write_out(out, summary)
            }
        })
    }
}

/// Copies up to `cap` rejections of the last step into `buf`; `out_len`
/// receives the total number retained (at most `KOD_MAX_REJECTIONS`).
///
/// # Safety
/// `sim` live handle; `buf` null or valid for `cap` elements; `out_len` valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_sim_rejections(
    sim: *mut KodSim,
    buf: *mut KodRejection,
    cap: usize,
    out_len: *mut usize,
) -> KodStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        with_sim(sim, |h| {
            if out_len.is_null() {
                set_error("null out_len");
                return KodStatus::NullPointer;
            }
            out_len.write(h.rejections.len());
            if h.rejections.len() > cap || (buf.is_null() && !h.rejections.is_empty()) {
                set_error("buffer too small");
                return KodStatus::BufferTooSmall;
            }
            if !h.rejections.is_empty() {
                ptr::copy_nonoverlapping(h.rejections.as_ptr(), buf, h.rejections.len());
            }
            KodStatus::Ok
        })
    }
}

/// Next tick to be processed.
///
/// # Safety
/// `sim` live handle; `out` valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_sim_next_tick(sim: *mut KodSim, out: *mut u64) -> KodStatus {
    // SAFETY: forwarded caller contract.
    unsafe { with_sim(sim, |h| write_out(out, h.sim.next_tick().0)) }
}

/// Date of the next tick.
///
/// # Safety
/// `sim` live handle; `out` valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_sim_date(sim: *mut KodSim, out: *mut KodDate) -> KodStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        with_sim(sim, |h| {
            let d = h.sim.current_date();
            write_out(
                out,
                KodDate {
                    year: d.year(),
                    month: d.month(),
                    day: d.day(),
                },
            )
        })
    }
}

/// State hash of the complete simulation.
///
/// # Safety
/// `sim` live handle; `out` valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_sim_state_hash(sim: *mut KodSim, out: *mut KodHash128) -> KodStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        with_sim(sim, |h| {
            let v = h.sim.state_hash().0;
            write_out(
                out,
                KodHash128 {
                    lo: v as u64,
                    hi: (v >> 64) as u64,
                },
            )
        })
    }
}

/// Serialises the session (save container v1) with the two-call pattern.
///
/// # Safety
/// `sim` live handle; `buf` null or valid for `cap` bytes; `out_len` valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_sim_save(
    sim: *mut KodSim,
    buf: *mut u8,
    cap: usize,
    out_len: *mut usize,
) -> KodStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        with_sim(sim, |h| {
            copy_out(&SaveFile::write(&h.sim), buf, cap, out_len)
        })
    }
}

/// Restores a session from save bytes. Never yields a partial state.
///
/// # Safety
/// `data` valid for `len` bytes; `out` valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_sim_load(
    abi_version: u32,
    data: *const u8,
    len: usize,
    out: *mut *mut KodSim,
) -> KodStatus {
    guard(|| {
        if let Err(status) = check_abi(abi_version) {
            return status;
        }
        if data.is_null() || out.is_null() {
            set_error("null pointer");
            return KodStatus::NullPointer;
        }
        // SAFETY: caller guarantees `len` readable bytes.
        let bytes = unsafe { std::slice::from_raw_parts(data, len) };
        match SaveFile::load::<Ledger>(bytes) {
            Ok(sim) => unsafe { write_out(out, new_handle(sim)) },
            Err(err) => {
                set_error(err.to_string());
                KodStatus::InvalidSave
            }
        }
    })
}

fn fixed_from_raw(raw_nanos: i64) -> Fixed {
    Fixed::from_raw(i128::from(raw_nanos))
}

/// # Safety
/// `sim` live handle.
unsafe fn submit(
    sim: *mut KodSim,
    actor: u32,
    sequence: u64,
    execute_at: u64,
    command: LedgerCommand,
) -> KodStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        with_sim(sim, |h| {
            let env = CommandEnvelope {
                actor: ActorId::new(actor),
                sequence,
                execute_at: Tick(execute_at),
                command,
            };
            match h.sim.submit(env) {
                Ok(()) => KodStatus::Ok,
                Err(err) => {
                    set_error(err.to_string());
                    KodStatus::SubmitRejected
                }
            }
        })
    }
}

/// Queues "open account" (fixture domain). `deposit_nanos` is the amount in
/// units of 10^-9. Validation happens when the command executes.
///
/// # Safety
/// `sim` live handle; `name` valid for `name_len` bytes of UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_ledger_open(
    sim: *mut KodSim,
    actor: u32,
    sequence: u64,
    execute_at: u64,
    name: *const u8,
    name_len: usize,
    deposit_nanos: i64,
) -> KodStatus {
    let status = guard(|| {
        if name.is_null() && name_len > 0 {
            set_error("null name");
            return KodStatus::NullPointer;
        }
        let bytes = if name_len == 0 {
            &[][..]
        } else {
            // SAFETY: caller guarantees `name_len` readable bytes.
            unsafe { std::slice::from_raw_parts(name, name_len) }
        };
        match std::str::from_utf8(bytes) {
            Ok(_) => KodStatus::Ok,
            Err(_) => {
                set_error("name is not UTF-8");
                KodStatus::InvalidArgument
            }
        }
    });
    if status != KodStatus::Ok {
        return status;
    }
    let name = if name_len == 0 {
        String::new()
    } else {
        // SAFETY: validated above.
        String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(name, name_len) }).into_owned()
    };
    // SAFETY: forwarded caller contract.
    unsafe {
        submit(
            sim,
            actor,
            sequence,
            execute_at,
            LedgerCommand::Open {
                name,
                deposit: fixed_from_raw(deposit_nanos),
            },
        )
    }
}

/// Queues a transfer (fixture domain); `amount_nanos` in units of 10^-9.
///
/// # Safety
/// `sim` live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_ledger_transfer(
    sim: *mut KodSim,
    actor: u32,
    sequence: u64,
    execute_at: u64,
    from: u32,
    to: u32,
    amount_nanos: i64,
) -> KodStatus {
    let command = LedgerCommand::Transfer {
        from: AccountId::new(from),
        to: AccountId::new(to),
        amount: fixed_from_raw(amount_nanos),
    };
    // SAFETY: forwarded caller contract.
    unsafe { submit(sim, actor, sequence, execute_at, command) }
}

/// Queues "close account" (fixture domain).
///
/// # Safety
/// `sim` live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_ledger_close(
    sim: *mut KodSim,
    actor: u32,
    sequence: u64,
    execute_at: u64,
    account: u32,
) -> KodStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        submit(
            sim,
            actor,
            sequence,
            execute_at,
            LedgerCommand::Close {
                account: AccountId::new(account),
            },
        )
    }
}

/// Balance of an account in units of 10^-9.
///
/// # Safety
/// `sim` live handle; `out` valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_ledger_balance(
    sim: *mut KodSim,
    account: u32,
    out: *mut i64,
) -> KodStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        with_sim(sim, |h| {
            match h.sim.query(&LedgerQuery::Balance(AccountId::new(account))) {
                None => {
                    set_error(format!("account {account} not found"));
                    KodStatus::NotFound
                }
                Some(balance) => match i64::try_from(balance.raw()) {
                    Ok(raw) => write_out(out, raw),
                    Err(_) => {
                        set_error("balance exceeds i64 nanos");
                        KodStatus::Overflow
                    }
                },
            }
        })
    }
}

/// Test hook: panics inside the guarded region to exercise panic safety.
/// Only compiled for tests and the C smoke test (`ffi-test-hooks`).
///
/// # Safety
/// `sim` live handle.
#[cfg(any(test, feature = "test-hooks"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_test_panic(sim: *mut KodSim) -> KodStatus {
    // SAFETY: forwarded caller contract.
    unsafe { with_sim(sim, |_| panic!("deliberate test panic")) }
}
