//! F-01 acceptance (#11): 1 BC steps to 1 AD; every allowed year yields an
//! initialisable state; missing data is visible; the chosen source stays
//! fixed in the save. Test matrix: 100 BC, 1 BC, 1 AD, 476, 1492, 1914, 1936,
//! current year.

use kod_core::{ActorId, CommandEnvelope, CommandOutcome, SaveFile, SimDate, Simulation, Tick};
use kod_scenario::atlas::{AtlasSnapshot, AtlasSource, FixtureAtlas, PolityId};
use kod_scenario::resolver::{DataStatus, ScenarioMode, resolve};
use kod_scenario::year::{YearPicker, label_de};
use kod_scenario::{
    ScenarioSetup, SetupCommand, SetupEvent, SetupQuery, SetupState, check_dataset,
};

const CURRENT_YEAR: i32 = 2026;
const MATRIX: [i32; 8] = [-99, 0, 1, 476, 1492, 1914, 1936, CURRENT_YEAR];

fn atlas() -> AtlasSnapshot {
    AtlasSnapshot::capture(&FixtureAtlas).unwrap()
}

fn setup() -> Simulation<ScenarioSetup> {
    Simulation::new(
        7,
        SimDate::new_year(CURRENT_YEAR).unwrap(),
        SetupState::new(atlas(), CURRENT_YEAR),
    )
}

struct Driver {
    sim: Simulation<ScenarioSetup>,
    seq: u64,
}

impl Driver {
    fn new() -> Self {
        Self {
            sim: setup(),
            seq: 0,
        }
    }

    fn run(&mut self, command: SetupCommand) -> (CommandOutcome, Vec<SetupEvent>) {
        self.seq += 1;
        let at = self.sim.next_tick();
        self.sim
            .submit(CommandEnvelope {
                actor: ActorId::new(1),
                sequence: self.seq,
                execute_at: at,
                command,
            })
            .unwrap();
        let mut report = self.sim.step();
        (report.commands.remove(0).2, report.events)
    }

    fn ok(&mut self, command: SetupCommand) -> Vec<SetupEvent> {
        let (outcome, events) = self.run(command.clone());
        assert_eq!(outcome, CommandOutcome::Applied, "{command:?}");
        events
    }

    fn rejected(&mut self, command: SetupCommand) -> u32 {
        let before = self.sim.state().clone();
        let (outcome, events) = self.run(command.clone());
        assert!(events.is_empty());
        assert_eq!(
            self.sim.state(),
            &before,
            "rejected {command:?} changed state"
        );
        match outcome {
            CommandOutcome::Rejected(e) => e.code(),
            CommandOutcome::Applied => panic!("{command:?} unexpectedly applied"),
        }
    }
}

#[test]
fn one_bc_steps_to_one_ad() {
    let mut d = Driver::new();
    d.ok(SetupCommand::SelectYear(0));
    assert_eq!(label_de(d.sim.state().year), "1 v. Chr.");
    let events = d.ok(SetupCommand::StepYear(1));
    assert_eq!(events, vec![SetupEvent::YearChanged(1)]);
    assert_eq!(label_de(d.sim.state().year), "1 n. Chr.");
}

#[test]
fn every_allowed_year_is_initialisable_and_missing_data_is_visible() {
    let atlas = atlas();
    let picker = YearPicker::new(CURRENT_YEAR);
    let (min, max) = picker.range();
    let mut partial = 0;
    for year in min..=max {
        for mode in [
            ScenarioMode::Historical,
            ScenarioMode::AlternateHistory,
            ScenarioMode::Sandbox,
        ] {
            let r = resolve(&atlas, year, mode);
            assert_eq!(r.year, year);
            assert_eq!(r.dataset, atlas.dataset);
            match &r.status {
                DataStatus::Complete => assert!(!r.polities.is_empty()),
                DataStatus::Partial(gap) => {
                    partial += 1;
                    assert!(gap.from <= year && year < gap.to);
                }
                DataStatus::Missing => assert!(r.polities.is_empty()),
            }
            assert!(r.polities.iter().all(|p| p.valid_in(year)));
        }
    }
    assert_eq!(partial, 3 * 100, "fixture gap 600..700 must be reported");
}

#[test]
fn test_matrix_initialises_and_confirms() {
    for year in MATRIX {
        let mut d = Driver::new();
        d.ok(SetupCommand::SelectYear(year));
        let view = d.sim.query(&SetupQuery::View);
        assert_eq!(view.scenario.year, year, "{}", label_de(year));
        let first = view
            .scenario
            .polities
            .first()
            .unwrap_or_else(|| panic!("no polity in {}", label_de(year)))
            .id;
        d.ok(SetupCommand::SelectPolity(Some(first)));
        d.ok(SetupCommand::Confirm);
        assert!(d.sim.query(&SetupQuery::View).confirmed);
        assert_eq!(
            d.rejected(SetupCommand::SelectYear(1500)),
            4,
            "confirmed setup is read-only"
        );
    }
}

#[test]
fn invalid_selections_are_rejected_without_state_change() {
    let mut d = Driver::new();
    assert_eq!(d.rejected(SetupCommand::SelectYear(-100)), 3); // before 100 BC
    assert_eq!(d.rejected(SetupCommand::SelectYear(CURRENT_YEAR + 1)), 3);
    assert_eq!(
        d.rejected(SetupCommand::SelectPolity(Some(PolityId::new(5)))),
        2
    ); // not in 100 BC
    assert_eq!(
        d.rejected(SetupCommand::CreateCustomPolity("Utopia".into())),
        1
    ); // not sandbox
    assert_eq!(d.rejected(SetupCommand::Confirm), 4); // no polity
    assert_eq!(d.rejected(SetupCommand::AddBookmark(" ".into())), 3);
}

#[test]
fn year_change_clears_unavailable_polity() {
    let mut d = Driver::new();
    d.ok(SetupCommand::SelectYear(100));
    d.ok(SetupCommand::SelectPolity(Some(PolityId::new(1)))); // valid -99..500
    let events = d.ok(SetupCommand::SelectYear(1492));
    assert_eq!(
        events,
        vec![
            SetupEvent::YearChanged(1492),
            SetupEvent::PolityChanged(None)
        ]
    );
    assert_eq!(d.sim.state().polity, None);
}

#[test]
fn sandbox_allows_custom_polities_and_modes_are_enforced() {
    let mut d = Driver::new();
    d.ok(SetupCommand::SelectMode(ScenarioMode::Sandbox));
    d.ok(SetupCommand::SelectYear(650)); // inside the fixture gap
    let created = d.ok(SetupCommand::CreateCustomPolity("Freistaat Omega".into()));
    let SetupEvent::CustomPolityCreated(id) = created[0] else {
        panic!("{created:?}")
    };
    d.ok(SetupCommand::SelectPolity(Some(id)));
    assert!(
        d.sim
            .query(&SetupQuery::View)
            .scenario
            .polities
            .iter()
            .any(|p| p.id == id)
    );
    let events = d.ok(SetupCommand::SelectMode(ScenarioMode::Historical));
    assert!(
        events.contains(&SetupEvent::PolityChanged(None)),
        "custom polity invalid in historical mode"
    );
}

#[test]
fn bookmarks_restore_selection() {
    let mut d = Driver::new();
    d.ok(SetupCommand::SelectYear(1914));
    d.ok(SetupCommand::SelectPolity(Some(PolityId::new(6))));
    let added = d.ok(SetupCommand::AddBookmark("Juli-Krise (Test)".into()));
    let SetupEvent::BookmarkAdded(bm) = added[0] else {
        panic!()
    };
    assert_eq!(
        d.rejected(SetupCommand::AddBookmark("Juli-Krise (Test)".into())),
        4
    );
    d.ok(SetupCommand::SelectYear(-99));
    d.ok(SetupCommand::ApplyBookmark(bm));
    assert_eq!(
        (d.sim.state().year, d.sim.state().polity),
        (1914, Some(PolityId::new(6)))
    );
    d.ok(SetupCommand::RemoveBookmark(bm));
    assert_eq!(d.rejected(SetupCommand::ApplyBookmark(bm)), 2);
}

#[test]
fn chosen_source_stays_fixed_in_save() {
    let mut d = Driver::new();
    d.ok(SetupCommand::SelectYear(1936));
    d.ok(SetupCommand::SelectPolity(Some(PolityId::new(5))));
    let bytes = SaveFile::write(&d.sim);
    let loaded = SaveFile::load::<ScenarioSetup>(&bytes).unwrap();
    assert_eq!(loaded.state(), d.sim.state());
    assert_eq!(loaded.state().atlas.dataset, FixtureAtlas.dataset());
    check_dataset(loaded.state(), &FixtureAtlas.dataset()).unwrap();

    // A runtime offering another dataset version must be detected, not silently used.
    let mut other = FixtureAtlas.dataset();
    other.version = "2".into();
    assert!(
        check_dataset(loaded.state(), &other)
            .unwrap_err()
            .contains("kod.fixture.m0 1")
    );
    assert_eq!(loaded.next_tick(), Tick(2));
}

#[test]
fn tampered_atlas_in_save_is_rejected() {
    let d = Driver::new();
    let mut state = d.sim.state().clone();
    state.atlas.polities[0].name = "manipuliert".into();
    let sim = Simulation::<ScenarioSetup>::new(1, SimDate::new_year(CURRENT_YEAR).unwrap(), state);
    let bytes = SaveFile::write(&sim);
    assert!(SaveFile::load::<ScenarioSetup>(&bytes).is_err());
}

#[test]
fn identical_setup_commands_replay_to_identical_hash() {
    let script = [
        SetupCommand::SelectMode(ScenarioMode::AlternateHistory),
        SetupCommand::SelectYear(1492),
        SetupCommand::SelectPolity(Some(PolityId::new(3))),
        SetupCommand::AddBookmark("A".into()),
        SetupCommand::StepYear(-1000),
        SetupCommand::Confirm,
    ];
    let run = || {
        let mut d = Driver::new();
        for c in script.iter().cloned() {
            let _ = d.run(c);
        }
        d.sim.state_hash()
    };
    assert_eq!(run(), run());
}
