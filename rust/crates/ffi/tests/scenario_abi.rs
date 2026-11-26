//! F-01 scenario setup through the C ABI.

use std::ptr;

use kod_ffi::scenario::*;
use kod_ffi::{KOD_ABI_VERSION, KodStatus};

fn create() -> *mut KodScenario {
    let mut h = ptr::null_mut();
    assert_eq!(
        unsafe { kod_scenario_create(KOD_ABI_VERSION, 2026, &mut h) },
        KodStatus::Ok
    );
    h
}

fn view(h: *mut KodScenario) -> String {
    let mut len = 0;
    assert_eq!(
        unsafe { kod_scenario_view_json(h, ptr::null_mut(), 0, &mut len) },
        KodStatus::BufferTooSmall
    );
    let mut buf = vec![0u8; len];
    assert_eq!(
        unsafe { kod_scenario_view_json(h, buf.as_mut_ptr(), len, &mut len) },
        KodStatus::Ok
    );
    String::from_utf8(buf).unwrap()
}

#[test]
fn year_selection_status_and_confirm() {
    let h = create();
    let mut code = u32::MAX;
    unsafe {
        assert_eq!(kod_scenario_select_year(h, 0, &mut code), KodStatus::Ok);
        assert_eq!(code, 0);
        assert!(view(h).contains("\"label\":\"1 v. Chr.\""));
        kod_scenario_step_year(h, 1, &mut code);
        assert!(view(h).contains(
            "\"selected\":{\"year\":1,\"era\":\"ad\",\"number\":1,\"label\":\"1 n. Chr.\"}"
        ));

        kod_scenario_select_year(h, 650, &mut code);
        let v = view(h);
        assert!(v.contains("\"kind\":\"partial\""), "{v}");
        assert!(v.contains("\"min_quality\":\"synthetic\""));

        kod_scenario_select_year(h, 1936, &mut code);
        assert_eq!(kod_scenario_confirm(h, &mut code), KodStatus::Ok);
        assert_eq!(code, 4); // no polity selected
        kod_scenario_select_polity(h, 5, &mut code);
        assert_eq!(code, 0);
        kod_scenario_select_polity(h, 1, &mut code);
        assert_eq!(code, 2); // polity 1 does not exist in 1936
        kod_scenario_confirm(h, &mut code);
        assert_eq!(code, 0);
        assert!(view(h).contains("\"confirmed\":true"));

        let name = "Sandbox";
        kod_scenario_create_custom_polity(h, name.as_ptr(), name.len(), &mut code);
        assert_eq!(code, 4); // confirmed -> read-only
        assert_eq!(
            kod_scenario_select_mode(h, 9, &mut code),
            KodStatus::InvalidArgument
        );
        kod_scenario_destroy(h);
    }
}

#[test]
fn save_keeps_dataset_and_continues_sequences() {
    let h = create();
    let mut code = 0;
    unsafe {
        kod_scenario_select_year(h, 1492, &mut code);
        let bm = "Testpunkt";
        kod_scenario_add_bookmark(h, bm.as_ptr(), bm.len(), &mut code);
        assert_eq!(code, 0);
        let mut len = 0;
        kod_scenario_save(h, ptr::null_mut(), 0, &mut len);
        let mut bytes = vec![0u8; len];
        assert_eq!(
            kod_scenario_save(h, bytes.as_mut_ptr(), len, &mut len),
            KodStatus::Ok
        );
        let mut loaded = ptr::null_mut();
        assert_eq!(
            kod_scenario_load(KOD_ABI_VERSION, bytes.as_ptr(), len, &mut loaded),
            KodStatus::Ok
        );
        assert_eq!(view(h), view(loaded));
        assert!(view(loaded).contains("\"dataset\":{\"id\":\"kod.fixture.m0\",\"version\":\"1\""));
        // Commands keep working after load (sequence continues).
        assert_eq!(
            kod_scenario_select_year(loaded, 1914, &mut code),
            KodStatus::Ok
        );
        assert_eq!(code, 0);
        bytes[len / 2] ^= 1;
        let mut broken = ptr::null_mut();
        assert_eq!(
            kod_scenario_load(KOD_ABI_VERSION, bytes.as_ptr(), len, &mut broken),
            KodStatus::InvalidSave
        );
        kod_scenario_destroy(h);
        kod_scenario_destroy(loaded);
        kod_scenario_destroy(ptr::null_mut());
    }
}
