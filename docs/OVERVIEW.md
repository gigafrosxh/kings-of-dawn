# Projektziel und Umfang

## Projektziel

Eine global spielbare, fortlaufende Simulation von 100 v. Chr. bis zum aktuellen Kalenderjahr als Startdatum, danach offene alternative Entwicklung. Spieler übernehmen Staaten oder definierte Rollen, Beobachter sehen einer autonomen Welt zu. Zielgruppe: Strategie- und Geschichtsfans, Modder, Sandbox-Spieler. Kein vorgegebenes Siegesziel; optional eigene Missionsziele.

## Auftrag & Abgrenzung

Auftraggeber: noch festzulegen. Projektleitung, Teamgröße, Budget, Fixtermine und Lizenz-/Vertriebsmodell: offen. Dieser Auftrag ist ein Soll-Konzept, kein bereits implementiertes Spiel. Desktop zuerst (Windows/Linux/macOS als Ziel); Singleplayer offline zuerst. Keine vollständige 3D-Stadt- oder Schlachtfeldsimulation, keine historischen Tagesdaten für jede Region versprochen, kein echtes Geld oder reale Finanztransaktionen. Startdatum ist begrenzt, Enddatum nicht. Raumfahrt/außerirdische Kolonien sind eine optionale spätere Erweiterung.

## Kernablauf

Startjahr → Datenqualität und Weltzustand prüfen → Land/Rolle wählen oder Staat gründen → Ziele, Haushalt und Gesetze bearbeiten → Zeit laufen lassen → Ereignisse und Folgen analysieren → reagieren → speichern. Der Beobachtermodus nutzt dieselbe Engine ohne Spielercommands. Sandbox erlaubt direkte Änderungen, Realismusmodus verlangt institutionelle und wirtschaftliche Voraussetzungen.

## Risiken und Gegenmaßnahmen

Historische Quellenlücken → datierte Näherung, regionale Unsicherheiten, Quellenregister; Kartenlizenz → allowlist und gesonderte Datenpakete; Systemkomplexität → vertikaler MVP, Featureflags, keine ungetesteten Massenmodule; Performance → Kohorten, inkrementelle Updates, LOD und Benchmarks; Balancing → deterministische Szenarien und Telemetrie lokal/opt-in; Multiplayer → erst nach stabiler Replay-Engine. Historische Darstellungen müssen zwischen Fakten, Schätzungen und alternativer Spielgeschichte unterscheiden.

Release-Plan und Freigaben: [INFO.md](../INFO.md). Alle 42 Features: [ROADMAP.md](ROADMAP.md).
