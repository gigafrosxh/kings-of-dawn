//! F-01 scenario setup over the C ABI (`kod_scenario_*`).
//!
//! Each command is queued and executed immediately (one setup tick), so the
//! caller gets the validation result synchronously through `out_code`
//! (0 = applied, otherwise `CommandError::code()`). The UI reads a JSON
//! snapshot (`kod_scenario_view_json`) – no Rust containers cross the ABI.

use std::fmt::Write as _;

use kod_core::time::Era;
use kod_core::{ActorId, CommandEnvelope, CommandOutcome, SaveFile, SimDate, Simulation};
use kod_scenario::atlas::{AtlasSnapshot, DataQuality, FixtureAtlas, PolityId};
use kod_scenario::resolver::{DataStatus, ScenarioMode};
use kod_scenario::year::{label_de, to_display};
use kod_scenario::{BookmarkId, ScenarioSetup, SetupCommand, SetupQuery, SetupState};

use super::{KodStatus, check_abi, copy_out, guard, set_error, write_out};

/// Opaque scenario-setup handle.
pub struct KodScenario {
    sim: Simulation<ScenarioSetup>,
    sequence: u64,
    poisoned: bool,
}

/// # Safety
/// `h` null or live handle.
unsafe fn with_setup(
    h: *mut KodScenario,
    body: impl FnOnce(&mut KodScenario) -> KodStatus,
) -> KodStatus {
    if h.is_null() {
        set_error("null handle");
        return KodStatus::NullPointer;
    }
    // SAFETY: caller guarantees a live, exclusively used handle.
    let handle = unsafe { &mut *h };
    if handle.poisoned {
        set_error("handle poisoned by an earlier panic");
        return KodStatus::Poisoned;
    }
    let status = guard(|| body(handle));
    if status == KodStatus::Panic {
        // SAFETY: as above.
        unsafe { (*h).poisoned = true };
    }
    status
}

fn new_handle(sim: Simulation<ScenarioSetup>) -> *mut KodScenario {
    // Continue after the last sequence used by the setup actor (relevant after load).
    let sequence = sim.last_sequence(ActorId::new(1)).unwrap_or(0);
    Box::into_raw(Box::new(KodScenario {
        sim,
        sequence,
        poisoned: false,
    }))
}

/// Creates a setup against the M0 fixture atlas. `current_year` is the
/// host's calendar year (upper bound of the year picker).
///
/// # Safety
/// `out` valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_scenario_create(
    abi_version: u32,
    current_year: i32,
    out: *mut *mut KodScenario,
) -> KodStatus {
    guard(|| {
        if let Err(s) = check_abi(abi_version) {
            return s;
        }
        if out.is_null() {
            set_error("null output pointer");
            return KodStatus::NullPointer;
        }
        if !(1..=9999).contains(&current_year) {
            set_error("current_year out of range");
            return KodStatus::InvalidArgument;
        }
        let atlas = match AtlasSnapshot::capture(&FixtureAtlas) {
            Ok(a) => a,
            Err(e) => {
                set_error(e);
                return KodStatus::InvalidSave;
            }
        };
        let date = SimDate::new_year(current_year).expect("range checked");
        let sim = Simulation::new(0, date, SetupState::new(atlas, current_year));
        // SAFETY: checked non-null.
        unsafe { write_out(out, new_handle(sim)) }
    })
}

/// Releases a setup handle (null ignored).
///
/// # Safety
/// `h` null or a live handle, destroyed at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_scenario_destroy(h: *mut KodScenario) {
    if h.is_null() {
        return;
    }
    let _ = guard(|| {
        // SAFETY: ownership returns exactly once by contract.
        drop(unsafe { Box::from_raw(h) });
        KodStatus::Ok
    });
}

/// # Safety
/// `h` live handle; `out_code` null or valid.
unsafe fn run(h: *mut KodScenario, command: SetupCommand, out_code: *mut u32) -> KodStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        with_setup(h, |s| {
            s.sequence += 1;
            let env = CommandEnvelope {
                actor: ActorId::new(1),
                sequence: s.sequence,
                execute_at: s.sim.next_tick(),
                command,
            };
            if let Err(e) = s.sim.submit(env) {
                set_error(e.to_string());
                return KodStatus::SubmitRejected;
            }
            let report = s.sim.step();
            let code = match &report.commands[0].2 {
                CommandOutcome::Applied => 0,
                CommandOutcome::Rejected(e) => {
                    set_error(e.to_string());
                    e.code()
                }
            };
            if out_code.is_null() {
                KodStatus::Ok
            } else {
                write_out(out_code, code)
            }
        })
    }
}

/// Selects an astronomical year (1 BC = 0).
///
/// # Safety
/// `h` live handle; `out_code` null or valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_scenario_select_year(
    h: *mut KodScenario,
    year: i32,
    out_code: *mut u32,
) -> KodStatus {
    // SAFETY: forwarded.
    unsafe { run(h, SetupCommand::SelectYear(year), out_code) }
}

/// Moves the year by `delta` (clamped to the range).
///
/// # Safety
/// `h` live handle; `out_code` null or valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_scenario_step_year(
    h: *mut KodScenario,
    delta: i32,
    out_code: *mut u32,
) -> KodStatus {
    // SAFETY: forwarded.
    unsafe { run(h, SetupCommand::StepYear(delta), out_code) }
}

/// Selects the mode: 0 historical, 1 alternate history, 2 sandbox.
///
/// # Safety
/// `h` live handle; `out_code` null or valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_scenario_select_mode(
    h: *mut KodScenario,
    mode: u8,
    out_code: *mut u32,
) -> KodStatus {
    let Some(mode) = ScenarioMode::from_u8(mode) else {
        set_error("unknown mode");
        return KodStatus::InvalidArgument;
    };
    // SAFETY: forwarded.
    unsafe { run(h, SetupCommand::SelectMode(mode), out_code) }
}

/// Selects the player polity; `polity == 0` clears the selection.
///
/// # Safety
/// `h` live handle; `out_code` null or valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_scenario_select_polity(
    h: *mut KodScenario,
    polity: u32,
    out_code: *mut u32,
) -> KodStatus {
    let p = (polity != 0).then(|| PolityId::new(polity));
    // SAFETY: forwarded.
    unsafe { run(h, SetupCommand::SelectPolity(p), out_code) }
}

/// # Safety
/// `name` valid for `len` bytes.
unsafe fn read_name(name: *const u8, len: usize) -> Result<String, KodStatus> {
    if len == 0 {
        return Ok(String::new());
    }
    if name.is_null() {
        set_error("null name");
        return Err(KodStatus::NullPointer);
    }
    // SAFETY: caller guarantees `len` readable bytes.
    let bytes = unsafe { std::slice::from_raw_parts(name, len) };
    String::from_utf8(bytes.to_vec()).map_err(|_| {
        set_error("name is not UTF-8");
        KodStatus::InvalidArgument
    })
}

/// Creates a custom polity (sandbox only).
///
/// # Safety
/// `h` live handle; `name` valid for `len` bytes; `out_code` null or valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_scenario_create_custom_polity(
    h: *mut KodScenario,
    name: *const u8,
    len: usize,
    out_code: *mut u32,
) -> KodStatus {
    // SAFETY: forwarded.
    match unsafe { read_name(name, len) } {
        Ok(n) => unsafe { run(h, SetupCommand::CreateCustomPolity(n), out_code) },
        Err(s) => s,
    }
}

/// Bookmarks the current selection.
///
/// # Safety
/// `h` live handle; `name` valid for `len` bytes; `out_code` null or valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_scenario_add_bookmark(
    h: *mut KodScenario,
    name: *const u8,
    len: usize,
    out_code: *mut u32,
) -> KodStatus {
    // SAFETY: forwarded.
    match unsafe { read_name(name, len) } {
        Ok(n) => unsafe { run(h, SetupCommand::AddBookmark(n), out_code) },
        Err(s) => s,
    }
}

/// Restores a bookmark.
///
/// # Safety
/// `h` live handle; `out_code` null or valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_scenario_apply_bookmark(
    h: *mut KodScenario,
    bookmark: u32,
    out_code: *mut u32,
) -> KodStatus {
    // SAFETY: forwarded.
    unsafe {
        run(
            h,
            SetupCommand::ApplyBookmark(BookmarkId::new(bookmark)),
            out_code,
        )
    }
}

/// Removes a bookmark.
///
/// # Safety
/// `h` live handle; `out_code` null or valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_scenario_remove_bookmark(
    h: *mut KodScenario,
    bookmark: u32,
    out_code: *mut u32,
) -> KodStatus {
    // SAFETY: forwarded.
    unsafe {
        run(
            h,
            SetupCommand::RemoveBookmark(BookmarkId::new(bookmark)),
            out_code,
        )
    }
}

/// Locks the setup (requires a selected, available polity).
///
/// # Safety
/// `h` live handle; `out_code` null or valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_scenario_confirm(
    h: *mut KodScenario,
    out_code: *mut u32,
) -> KodStatus {
    // SAFETY: forwarded.
    unsafe { run(h, SetupCommand::Confirm, out_code) }
}

fn json_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn quality_str(q: DataQuality) -> &'static str {
    match q {
        DataQuality::Synthetic => "synthetic",
        DataQuality::Approximate => "approximate",
        DataQuality::Reconstructed => "reconstructed",
        DataQuality::Documented => "documented",
    }
}

fn mode_str(m: ScenarioMode) -> &'static str {
    match m {
        ScenarioMode::Historical => "historical",
        ScenarioMode::AlternateHistory => "alternate",
        ScenarioMode::Sandbox => "sandbox",
    }
}

fn year_json(out: &mut String, year: i32) {
    let d = to_display(year);
    let era = if d.era == Era::Bc { "bc" } else { "ad" };
    let _ = write!(
        out,
        "{{\"year\":{year},\"era\":\"{era}\",\"number\":{},\"label\":",
        d.number
    );
    json_str(out, &label_de(year));
    out.push('}');
}

/// JSON read model of the setup (UTF-8, schema documented in docs/features/F01-IMPLEMENTATION.md).
pub fn view_json(state: &SetupState) -> String {
    let v = ScenarioSetup::query_view(state);
    let mut o = String::with_capacity(1024);
    o.push_str("{\"schema\":1,\"selected\":");
    year_json(&mut o, v.scenario.year);
    let _ = write!(o, ",\"range\":[{},{}],\"presets\":[", v.range.0, v.range.1);
    for (i, y) in v.presets.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        year_json(&mut o, *y);
    }
    let _ = write!(
        o,
        "],\"mode\":\"{}\",\"polity\":{},",
        mode_str(v.scenario.mode),
        v.polity.map_or(0, |p| p.get())
    );
    o.push_str("\"dataset\":{\"id\":");
    json_str(&mut o, &v.scenario.dataset.id);
    o.push_str(",\"version\":");
    json_str(&mut o, &v.scenario.dataset.version);
    let _ = write!(
        o,
        ",\"checksum\":\"{}\"}},\"status\":",
        v.scenario.dataset.checksum
    );
    match &v.scenario.status {
        DataStatus::Complete => o.push_str("{\"kind\":\"complete\"}"),
        DataStatus::Missing => o.push_str("{\"kind\":\"missing\"}"),
        DataStatus::Partial(g) => {
            let _ = write!(
                o,
                "{{\"kind\":\"partial\",\"from\":{},\"to\":{},\"reason\":",
                g.from, g.to
            );
            json_str(&mut o, &g.reason);
            o.push('}');
        }
    }
    o.push_str(",\"min_quality\":");
    match v.scenario.min_quality {
        Some(q) => {
            let _ = write!(o, "\"{}\"", quality_str(q));
        }
        None => o.push_str("null"),
    }
    o.push_str(",\"polities\":[");
    for (i, p) in v.scenario.polities.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        let _ = write!(o, "{{\"id\":{},\"name\":", p.id.get());
        json_str(&mut o, &p.name);
        let _ = write!(
            o,
            ",\"from\":{},\"to\":{},\"quality\":\"{}\"}}",
            p.valid_from,
            p.valid_to,
            quality_str(p.quality)
        );
    }
    o.push_str("],\"bookmarks\":[");
    for (i, (id, b)) in v.bookmarks.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        let _ = write!(o, "{{\"id\":{},\"name\":", id.get());
        json_str(&mut o, &b.name);
        let _ = write!(
            o,
            ",\"year\":{},\"mode\":\"{}\",\"polity\":{}}}",
            b.year,
            mode_str(b.mode),
            b.polity.map_or(0, |p| p.get())
        );
    }
    let _ = write!(
        o,
        "],\"confirmed\":{},\"custom_polities_allowed\":{}}}",
        v.confirmed,
        v.scenario.mode.allows_custom_polities()
    );
    o
}

trait QueryView {
    fn query_view(state: &SetupState) -> kod_scenario::SetupView;
}

impl QueryView for ScenarioSetup {
    fn query_view(state: &SetupState) -> kod_scenario::SetupView {
        <ScenarioSetup as kod_core::Domain>::query(state, &SetupQuery::View)
    }
}

/// Copies the JSON view (two-call pattern).
///
/// # Safety
/// `h` live handle; `buf` null or valid for `cap`; `out_len` valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_scenario_view_json(
    h: *mut KodScenario,
    buf: *mut u8,
    cap: usize,
    out_len: *mut usize,
) -> KodStatus {
    // SAFETY: forwarded.
    unsafe {
        with_setup(h, |s| {
            copy_out(view_json(s.sim.state()).as_bytes(), buf, cap, out_len)
        })
    }
}

/// Serialises the setup (save container v1, schema `kod.scenario.setup`).
///
/// # Safety
/// `h` live handle; `buf` null or valid for `cap`; `out_len` valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_scenario_save(
    h: *mut KodScenario,
    buf: *mut u8,
    cap: usize,
    out_len: *mut usize,
) -> KodStatus {
    // SAFETY: forwarded.
    unsafe { with_setup(h, |s| copy_out(&SaveFile::write(&s.sim), buf, cap, out_len)) }
}

/// Restores a setup. The save keeps its own dataset; if the runtime fixture
/// differs, loading fails with `INVALID_SAVE` instead of switching sources.
///
/// # Safety
/// `data` valid for `len`; `out` valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kod_scenario_load(
    abi_version: u32,
    data: *const u8,
    len: usize,
    out: *mut *mut KodScenario,
) -> KodStatus {
    guard(|| {
        if let Err(s) = check_abi(abi_version) {
            return s;
        }
        if data.is_null() || out.is_null() {
            set_error("null pointer");
            return KodStatus::NullPointer;
        }
        // SAFETY: caller guarantees `len` bytes.
        let bytes = unsafe { std::slice::from_raw_parts(data, len) };
        let sim = match SaveFile::load::<ScenarioSetup>(bytes) {
            Ok(sim) => sim,
            Err(e) => {
                set_error(e.to_string());
                return KodStatus::InvalidSave;
            }
        };
        let available = kod_scenario::atlas::AtlasSource::dataset(&FixtureAtlas);
        if let Err(e) = kod_scenario::check_dataset(sim.state(), &available) {
            set_error(e);
            return KodStatus::InvalidSave;
        }
        // SAFETY: checked non-null.
        unsafe { write_out(out, new_handle(sim)) }
    })
}
