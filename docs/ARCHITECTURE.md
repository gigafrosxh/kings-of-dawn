# Architektur: Unreal Engine 5 und Rust

## Verbindliche technische Architektur · Unreal Engine 5 + Rust

**Verbindliche Engine und Sprachwahl**

Unreal Engine 5 (UE5) ist die verbindliche Hauptengine für KINGS OF DAWN. Eine stabile UE5-Version wird in M0 ausgewählt und festgeschrieben. Rust ist die bevorzugte und primäre Sprache für den deterministischen Simulationskern mit getrennten Crates (world, economy, society, government, diplomacy, military, history, persistence, ai). Unreal-Integration, Rendering, Eingaben, Engine-Lifecycle, UI und plattformspezifische Funktionen werden in C++ mit Unreal APIs umgesetzt; UMG/Slate für das moderne HOI4-inspirierte Interface, Blueprints nur für Präsentation und geeignete Editor-Workflows. Keine doppelte Fachlogik in C++ oder Blueprints. Rust ist keine nativ unterstützte Unreal-Gameplay-Sprache; keine Abhängigkeit von einem ungewarteten Rust-Gameplay-Plugin voraussetzen.

**Rust–Unreal-Integration**

Ein eigenes Unreal-Plugin KingsOfDawnSimulation bindet die Rust-Bibliothek über eine versionierte C-ABI (extern "C", repr(C), generierte Header via cbindgen) in UnrealBuildTool/Build.cs ein. Bevorzugt Cargo staticlib, cdylib nur bei begründetem Bedarf. Opaque Handles, explizite Eigentums- und Freigaberegeln, keine Rust-Container oder UObject-Zeiger über die ABI, keine Panics oder C++-Exceptions über Sprachgrenzen. Explizite Fehlercodes und ABI-Version prüfen. Ein C++-Subsystem verwaltet den Lebenszyklus. Simulation auf einem dedizierten Worker mit atomarer Command-Queue; immutable, gebündelte Snapshots/Deltas an den Game Thread, keine UObject-Zugriffe aus Rust oder Hintergrundthreads. UI und Renderer sind Konsumenten, der Rust-Core ist einzige autoritative Zustandsquelle.

**Kartenrendering und Assets**

UE5 rendert die interaktive politische Weltkarte aus offline vorbereiteten Geodaten: georeferenzierte, gekachelte Meshes, LOD, Culling, GPU-basierte politische Einfärbung und Auswahl über stabile Zell-IDs; sichtbare Grenzen, Gelände, Straßen, Symbole und Kartenmodi als separate Layer. GeoJSON/GeoPackage dienen als Importformate; Vektortiles können als internes Import-/Cacheformat dienen, nicht als WebGL-Frontend. Keine Actor-Instanz pro Zelle im Vollausbau. Geeignete UE-Materialien, Texturen, Instancing und Streaming nach Benchmark. Slate/UMG-Widgets nutzen virtualisierte Listen und ereignisbasierte Updates. Die HTML-Mockups sind reine Designreferenzen, keine Laufzeittechnologie.

**Build, Datenhaltung und Abnahme**

Repository: Unreal-Projekt (.uproject, Source/, Content/, Config/, Plugins/KingsOfDawnSimulation/) sowie rust/ als Cargo-Workspace mit core, ffi, atlas-cli und headless-runner. SQLite für lokale Metadaten/Saves, komprimierte binäre Snapshots, versionsgebundene Datenschemata. Rust-Core muss ohne UE oder Remote-LLM ausführbar und testbar sein. M0-Spike: Cargo-Build je Zielplattform, UE-Build.cs-Linking, verpackter Windows-Build mit FFI-Roundtrip und reproduzierbarem Save/Load, Worker-Lifecycle, 10.000-Zellen-Kartenprototyp, Headless-Replay-Hash und Fehlerpfade; Linux/macOS-Builds separat verifizieren. Unreal Engine bleibt verbindlich; Linkage, Kartenpipeline und konkrete Engine-Version nach Messung festlegen. Rust bleibt bevorzugt; C++-Ausnahmen für den Core nur nach dokumentiertem technischen Blocker und Architekturentscheidung. Unreal-Lizenzbedingungen und Drittanbieter-Lizenzen vor Vertrieb prüfen.

Technische Referenzen: [Epic: Third-Party Libraries in Unreal Engine](https://dev.epicgames.com/documentation/en-us/unreal-engine/integrating-third-party-libraries-into-unreal-engine) · [Epic: Slate in Unreal Engine](https://dev.epicgames.com/documentation/en-us/unreal-engine/using-slate-in-a-project-in-unreal-engine) · [Rustonomicon: FFI](https://doc.rust-lang.org/stable/nomicon/ffi.html) · [Rust Reference: Linkage](https://doc.rust-lang.org/stable/reference/linkage.html)

## Datenhaltung & Schnittstellen

Stabile UUIDs/IDs; EntityRef statt Namensschlüssel; Schema-Version und Migration pro Save; source_id, validity, confidence, geometry_hash für historische Daten. Commands z. B. FoundPolity, EnactLaw, BuildFacility, SetMonetaryPolicy. Queries liefern read-only ViewModels; Domain-Events wie LawEnacted, TradeSettled, BorderChanged. Kein UI-Schreibzugriff direkt auf Datenbank. Speichern atomar, Prüfsummen, rotierende Backups, Fehlerdialog ohne Datenverlust.

Weitere Regeln: [Simulation](SIMULATION.md), [QA](QUALITY.md), [UI](DESIGN.md).
