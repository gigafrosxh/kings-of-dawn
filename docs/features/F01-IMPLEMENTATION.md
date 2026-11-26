# F-01 – Umsetzung (v0.1.0, Issue #11)

## Aufbau
| Schicht | Ort | Inhalt |
|---|---|---|
| Rust-Fachlogik | `rust/crates/scenario` | `year::YearPicker` (100 v. Chr. … aktuelles Jahr, kein Jahr 0), `atlas` (Atlasvertrag `AtlasSource`, `AtlasSnapshot`, `DatasetVersion`, synthetischer `FixtureAtlas`), `resolver::resolve` (Jahr + Modus → initialisierbarer Zustand mit `DataStatus`), Domain `ScenarioSetup` (Commands/Events/Query, Save fixiert Datensatz) |
| C-ABI | `rust/crates/ffi/src/scenario.rs` | `kod_scenario_*`: create/destroy, select_year, step_year, select_mode, select_polity, create_custom_polity, add/apply/remove_bookmark, confirm, view_json, save/load |
| Unreal | `unreal/Plugins/KingsOfDawnSimulation` | `FKodScenarioViewModel` (JSON-Read-Model + Commands), `SKodScenarioPanel` (Slate, Mockup f01), `UKodScenarioScreenSubsystem` (zeigt Panel beim Spielstart, speichert bestätigtes Setup nach `Saved/KodScenario.kodsave`) |

Commands: `SelectYear`, `StepYear`, `SelectMode`, `SelectPolity`, `CreateCustomPolity` (nur Sandbox), `AddBookmark`, `ApplyBookmark`, `RemoveBookmark`, `Confirm`. Events: `YearChanged`, `ModeChanged`, `PolityChanged` (auch automatisches Abwählen, wenn der Staat im neuen Jahr/Modus nicht existiert), `CustomPolityCreated`, `BookmarkAdded/Removed`, `Confirmed`. Fehlercodes wie ADR-0003 (1 unberechtigt, 2 unbekannt, 3 ungültig, 4 Konflikt).

## JSON-Read-Model (`kod_scenario_view_json`, schema 1)
`selected{year,era,number,label}`, `range[min,max]`, `presets[]`, `mode`, `polity` (0 = keiner), `dataset{id,version,checksum}`, `status{kind: complete|partial|missing, from?, to?, reason?}`, `min_quality`, `polities[{id,name,from,to,quality}]`, `bookmarks[{id,name,year,mode,polity}]`, `confirmed`, `custom_polities_allowed`.

## Abnahme
| Kriterium | Nachweis |
|---|---|
| 1 v. Chr. springt auf 1 n. Chr. | `year::tests::one_bc_steps_to_one_ad`, `acceptance::one_bc_steps_to_one_ad`, UE `KingsOfDawn.Scenario.UiSmoke` |
| Jedes erlaubte Jahr liefert einen initialisierbaren Zustand | `every_allowed_year_is_initialisable_and_missing_data_is_visible` (alle Jahre −99…2026 × 3 Modi), Testmatrix 100 v. Chr./1 v. Chr./1 n. Chr./476/1492/1914/1936/heute |
| Fehlende Daten sichtbar | `DataStatus::Partial/Missing`, Panel-Zeile „Datenstand“ (Warnfarbe), UI-Smoke prüft „Lückenhaft“ |
| Ausgewählte Quelle bleibt im Save fixiert | Atlas-Snapshot inkl. `DatasetVersion` im State; `chosen_source_stays_fixed_in_save`, `tampered_atlas_in_save_is_rejected`, ABI `save_keeps_dataset_and_continues_sequences`, UI-Smoke Save/Load |

Screenshot (macOS, UE 5.8.4, Development): `docs/screenshots/f01-scenario-panel-v0.1.0-dev.png`.

## Verbleibende Einschränkungen
- **Daten:** nur der synthetische M0-Fixture-Atlas (`kod.fixture.m0`, „Testreich Alpha“ …, Güte „Synthetisch“). Keine historischen Aussagen; echte Daten liefert F-02 (#12) über `AtlasSource`.
- **Karte:** Kartenansicht und Länderwahl auf der Karte folgen mit #9/F-03; die Länderwahl erfolgt in v0.1.0 über die Liste im Panel.
- **HUD:** obere Statusleiste, Outliner und Minikarte des Mockups gehören zu späteren Features und sind nicht Teil von F-01.
- **Startdatum:** immer 1. Januar (dokumentierte Konvention, SIMULATION.md); tagesgenaue Startdaten sind nicht Teil von F-01.
- **Übergang ins Spiel:** „Szenario starten“ bestätigt und speichert das Setup; die Initialisierung der Weltsimulation aus dem Setup folgt mit den Domänen ab F-02/F-05.
- **Plattform:** UE-Teil auf macOS verifiziert; Windows offen (siehe #6/#7/#10).
- **Lokalisierung:** Texte deutsch über `LOCTEXT`; weitere Sprachen mit F-42.
