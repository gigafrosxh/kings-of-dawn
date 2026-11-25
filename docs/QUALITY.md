# Qualität, Tests und Abnahme

## Qualität & Performance

Zielwerte, noch durch Benchmarks zu bestätigen: 60 FPS bei Karteninteraktion auf Referenz-PC, p95 UI-Antwort <100 ms, MVP mit mindestens 20 autonomen Teststaaten und 10.000 Zellen; Vollausbau-Ziel 100.000+ Zellen, mehrere hundert Polities, Kohorten aggregiert. Benchmarkprofil und Hardware dokumentieren. 100 Jahre Headless-Lauf ohne negative Bestände, ungültige Eigentümer, NaN oder Buchungsfehler; Speicher- und Ladezeit messen. Unit-, Property-, Integration-, Snapshot-, Replay-, Karten- und End-to-End-Tests; feste Testseeds und CI.

## Release-Abnahme

[INFO.md](../INFO.md) definiert verbindlich den Release-/Patch-Zyklus. Tests pro Feature, reproduzierbare Builds, dokumentierte Risiken, Demo und Quellenmanifest wo relevant. Alte M0–M4-Phasen dienen nur der historischen Orientierung.
