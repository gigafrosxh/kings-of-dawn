# Kings of Dawn – verbindlicher Entwicklungs- und Release-Workflow

Diese Datei ist die maßgebliche Prozess- und Versionsvorgabe für Menschen, Codex und Claude Code. Der [Projektauftrag](PROJECT_SPEC.md) bleibt die fachliche und technische Spezifikation (Unreal Engine 5, Rust-Core); die bisherigen M0–M4-Phasen sind nur Orientierung, keine Release-Milestones mehr. Bei Widersprüchen zum bisherigen Prozess gilt INFO.md.

## 1. Versionsmodell

- Genau ein neues Feature-Modul je Minor-Version: v0.1.0 bis v0.42.0. Versionsnummern sind numerisch, nicht lexikografisch zu sortieren.
- Ein nativer GitHub-Milestone `v0.N` umfasst das Feature, seine erste Veröffentlichung und sämtliche anschließenden Patches bis zur Stabilisierung.
- `v0.N.0` ist das Feature-Release. `v0.N.1` bis `v0.N.5` sind die fünf festen Prioritäts-Patches. Danach folgen bei Bedarf `v0.N.6`, `v0.N.7` usw. Keine neue Feature-Entwicklung während der Stabilisierung.
- Die technischen Grundlagen #1 und #6–#10 sind Voraussetzungen der ersten Version; kein zusätzliches Feature und keine zusätzliche Minor-Version. Die alten Tracking-Issues #2–#5 bleiben als historische Übersicht erhalten.
- Jedes Release erhält einen eindeutigen Git-Tag, Release Notes und einen reproduzierbaren Build. Eine Veröffentlichung ist keine Freigabe für das nächste Feature.

## 2. Verbindlicher Ablauf jeder Version (Beispiel v0.1)

1. Feature aus dem zugeordneten Issue vollständig implementieren, Abnahmekriterien und notwendige Build-/Smoke-Tests erfüllen.
2. Release `v0.1.0` veröffentlichen.
3. Den veröffentlichten Build testen: Feature, betroffene Schnittstellen und Regressionen bereits stabilisierter Features.
4. Für jeden reproduzierbaren neu gefundenen Fehler ein GitHub-Issue erstellen (Label `bug`, genau eines von `P0`–`P4`, Milestone `v0.1`, betroffene Version und geplante Fix-Version). Duplikate verknüpfen statt erneut anlegen.
5. Bugs aus dem initialen Release-Test nach Priorität in den fünf vorgesehenen Patches abarbeiten.
6. Nach jedem Patch-Release den veröffentlichten Build erneut testen, einschließlich der korrigierten Fehler und Regressionen. Neu entdeckte Bugs oder fehlgeschlagene Fixes werden sofort als Issues erfasst bzw. wieder geöffnet und im **direkt nächsten Patch** abgearbeitet, unabhängig von ihrer Priorität. Die ursprünglich für diesen Patch vorgesehenen Bugs bleiben ebenfalls Bestandteil des Patch-Umfangs.
7. Nach dem P4-Patch v0.1.5 alle verbleibenden und neu gefundenen Bugs in v0.1.6 beheben, unabhängig von der Priorität. Wieder veröffentlichen, testen und neue Issues erfassen. Das für v0.1.7, v0.1.8 usw. wiederholen, bis die Abnahme erfolgreich ist.
8. Erst nach einem vollständig bestandenen Testlauf ohne neue gefundene Fehler und ohne offene bestätigte Bug-Issues in dieser Version sowie erfüllter Feature-Abnahme den Feature-Issue und Milestone schließen. Erst dann Entwicklung von v0.2.0 beginnen.

## 3. Patch-Zuordnung

| Release | Pflichtumfang |
|---|---|
| v0.N.0 | Ein neues Feature |
| v0.N.1 | P0 aus dem initialen Test |
| v0.N.2 | P1 aus dem initialen Test + sämtliche neuen Bugs aus .1 |
| v0.N.3 | P2 aus dem initialen Test + sämtliche neuen Bugs aus .2 |
| v0.N.4 | P3 aus dem initialen Test + sämtliche neuen Bugs aus .3 |
| v0.N.5 | P4 aus dem initialen Test + sämtliche neuen Bugs aus .4 |
| v0.N.6 und höher | Alle noch offenen und neu gefundenen Bugs, unabhängig von P0–P4 |

Die fünf Patch-Stufen werden auch bei leeren Prioritätsgruppen durchlaufen: ein unveränderter, verifizierter Build darf veröffentlicht werden; keine künstlichen Codeänderungen oder erfundenen Bugs. Vorhandene Bugs dürfen bei dringendem Bedarf früher behoben werden; der nächste reguläre Patch darf P0/P1 aus neuen Tests nicht auf eine spätere Stufe verschieben. Fehlgeschlagene Tests ohne reproduzierbaren Befund werden dokumentiert und erneut geprüft, nicht ignoriert. Keine neuen Features in Patch-Releases. Kein behaupteter absoluter Bugfreiheitsbeweis: Freigabe bedeutet null bekannte offene bestätigte Bugs und bestandener dokumentierter Testumfang.

## 4. Bug-Prioritäten (GitHub-Labels)

| Label | Bedeutung | Beispiele |
|---|---|---|
| P0 | Blocker / kritisch | Absturz beim Start, Datenverlust, nicht ladbare Saves, Sicherheitsproblem |
| P1 | Schwerwiegend | Kernfunktion unbrauchbar, starke Regression, kein praktikabler Workaround |
| P2 | Mittel | Funktionsfehler mit Workaround, falsche Simulationswerte ohne Totalausfall |
| P3 | Gering | Lokaler UI-/UX- oder Darstellungsfehler, geringe Auswirkung |
| P4 | Kosmetisch | Tippfehler, kleine visuelle Inkonsistenz ohne Funktionseinbuße |

Genau eine Priorität pro Bug, nach Auswirkung, nicht nach geschätzter Implementierungsdauer. Prioritäten bei neuen Erkenntnissen korrigieren. Sicherheitslücken nicht mit ausnutzbaren Details in öffentlichen Issues melden; vertraulich behandeln.

### Bug-Issue-Vorlage

```markdown
Titel: [v0.N.P][P#] Kurze Fehlerbeschreibung
Labels: bug, P#
Milestone: v0.N
Betroffene Version:
Geplanter Fix-Release:
Plattform / Build / Commit:
Reproduktionsschritte:
Erwartetes Verhalten:
Tatsächliches Verhalten:
Auswirkung / Prioritätsbegründung:
Logs / Screenshots / Save / Seed (nur relevante Ausschnitte, keine Geheimnisse):
Abnahmekriterium / Regressionstest:
```

## 5. Freigabe-Checkliste

- [ ] Feature-Abnahme gemäß Issue erfüllt.
- [ ] Build und Release Notes für den aktuellen Tag vorhanden.
- [ ] Rust-Unit-/Integrations-/Replay-Tests und relevante Unreal-Automation-/UI-Tests bestanden.
- [ ] Manuelle Prüfung des veröffentlichten Builds und Regressionstests dokumentiert.
- [ ] Alle gefundenen Fehler erfasst, behoben und im veröffentlichten Patch nachgetestet.
- [ ] Mindestens .5 veröffentlicht und getestet; falls notwendig weitere Patches veröffentlicht und getestet.
- [ ] Keine offenen bestätigten Bugs in diesem Versions-Milestone; letzter Testlauf ohne neue Befunde.
- [ ] Feature-Issue und Versions-Milestone geschlossen; erst jetzt nächstes Feature freigeben.

## 6. Roadmap v0.1–v0.42

Reihenfolge nach Abhängigkeiten, nicht nach F-ID. Vorwärtsabhängigkeiten werden ausschließlich über klar begrenzte Schnittstellen und Test-Fixtures entkoppelt (kein vorgezogener Vollausbau eines späteren Features). Dazu zählen insbesondere F-01/F-02, F-12/F-15, F-19/F-16 und F-16/F-21. M0 stellt außerdem die minimale Simulationsuhr/Persistenz und UI-Basis bereit; der vollständige Funktionsumfang folgt in der jeweiligen Feature-Version. Beim späteren Anschluss muss der echte Adapter die Fixtures ersetzen und Regressionstests bestehen. Ein nicht erfülltes Abnahmekriterium darf nicht als bestanden deklariert werden; falls ein Feature ohne spätere Abhängigkeit nicht vollständig abnehmbar ist, vor Release den Umfang/Vertrag transparent dokumentieren und klären.

| Version | Feature | Issue |
|---|---|---|
| v0.1.0 | F-01 – Startjahr und Szenarien | [#11](https://github.com/gigafrosxh/kings-of-dawn/issues/11) |
| v0.2.0 | F-02 – Historischer Atlas und Kartenimport | [#12](https://github.com/gigafrosxh/kings-of-dawn/issues/12) |
| v0.3.0 | F-03 – Weltkarte und Kartenmodi | [#13](https://github.com/gigafrosxh/kings-of-dawn/issues/13) |
| v0.4.0 | F-04 – Geografie- und Provinzeditor | [#14](https://github.com/gigafrosxh/kings-of-dawn/issues/14) |
| v0.5.0 | F-05 – Staaten, Territorien und nationale Identität | [#15](https://github.com/gigafrosxh/kings-of-dawn/issues/15) |
| v0.6.0 | F-06 – Flaggen, Wappen, Hymnen und Symbolik | [#16](https://github.com/gigafrosxh/kings-of-dawn/issues/16) |
| v0.7.0 | F-08 – Personen, Ämter und Charaktereditor | [#18](https://github.com/gigafrosxh/kings-of-dawn/issues/18) |
| v0.8.0 | F-07 – Verfassung und Regierungsform | [#17](https://github.com/gigafrosxh/kings-of-dawn/issues/17) |
| v0.9.0 | F-12 – Verwaltung, Ministerien und öffentliche Dienste | [#22](https://github.com/gigafrosxh/kings-of-dawn/issues/22) |
| v0.10.0 | F-10 – Gesetzgebung und Gesetzeseditor | [#20](https://github.com/gigafrosxh/kings-of-dawn/issues/20) |
| v0.11.0 | F-15 – Staatshaushalt, Steuern und Staatsverschuldung | [#25](https://github.com/gigafrosxh/kings-of-dawn/issues/25) |
| v0.12.0 | F-19 – Bevölkerung, Demografie und Migration | [#29](https://github.com/gigafrosxh/kings-of-dawn/issues/29) |
| v0.13.0 | F-16 – Waren, Märkte und Preissimulation | [#26](https://github.com/gigafrosxh/kings-of-dawn/issues/26) |
| v0.14.0 | F-21 – Städtebau, Wohnraum und Gebäude | [#31](https://github.com/gigafrosxh/kings-of-dawn/issues/31) |
| v0.15.0 | F-17 – Unternehmen, Produktion und Lieferketten | [#27](https://github.com/gigafrosxh/kings-of-dawn/issues/27) |
| v0.16.0 | F-14 – Währungen und Zentralbank | [#24](https://github.com/gigafrosxh/kings-of-dawn/issues/24) |
| v0.17.0 | F-22 – Verkehr, Energie und Versorgung | [#32](https://github.com/gigafrosxh/kings-of-dawn/issues/32) |
| v0.18.0 | F-27 – Beziehungen, Anerkennung und Verhandlungen | [#37](https://github.com/gigafrosxh/kings-of-dawn/issues/37) |
| v0.19.0 | F-35 – Autonome KI und Entscheidungssystem | [#45](https://github.com/gigafrosxh/kings-of-dawn/issues/45) |
| v0.20.0 | F-39 – Spielmodi, Zeitsteuerung und Speichern | [#49](https://github.com/gigafrosxh/kings-of-dawn/issues/49) |
| v0.21.0 | F-42 – UX, Einstellungen und Onboarding | [#52](https://github.com/gigafrosxh/kings-of-dawn/issues/52) |
| v0.22.0 | F-38 – Statistiken, Ranglisten und Analyse | [#48](https://github.com/gigafrosxh/kings-of-dawn/issues/48) |
| v0.23.0 | F-09 – Dynastien, Erbfolge und Biografien | [#19](https://github.com/gigafrosxh/kings-of-dawn/issues/19) |
| v0.24.0 | F-11 – Parteien, Ideologien und Wahlen | [#21](https://github.com/gigafrosxh/kings-of-dawn/issues/21) |
| v0.25.0 | F-13 – Justiz, Sicherheit und Bürgerrechte | [#23](https://github.com/gigafrosxh/kings-of-dawn/issues/23) |
| v0.26.0 | F-20 – Kultur, Religion und Sprache | [#30](https://github.com/gigafrosxh/kings-of-dawn/issues/30) |
| v0.27.0 | F-23 – Bildung, Gesundheit und Sozialstaat | [#33](https://github.com/gigafrosxh/kings-of-dawn/issues/33) |
| v0.28.0 | F-24 – Öffentliche Meinung, Medien und Unruhen | [#34](https://github.com/gigafrosxh/kings-of-dawn/issues/34) |
| v0.29.0 | F-25 – Umwelt, Klima und Landwirtschaft | [#35](https://github.com/gigafrosxh/kings-of-dawn/issues/35) |
| v0.30.0 | F-26 – Forschung, Erfindungen und Wissenstransfer | [#36](https://github.com/gigafrosxh/kings-of-dawn/issues/36) |
| v0.31.0 | F-28 – Verträge, Bündnisse und internationale Organisationen | [#38](https://github.com/gigafrosxh/kings-of-dawn/issues/38) |
| v0.32.0 | F-18 – Handel, Banken und Kapitalmarkt | [#28](https://github.com/gigafrosxh/kings-of-dawn/issues/28) |
| v0.33.0 | F-29 – Streitkräfte, Organisation und Personal | [#39](https://github.com/gigafrosxh/kings-of-dawn/issues/39) |
| v0.34.0 | F-30 – Ausrüstung, Rüstung und Militärdoktrin | [#40](https://github.com/gigafrosxh/kings-of-dawn/issues/40) |
| v0.35.0 | F-32 – Nachschub, Transport und Mobilisierung | [#42](https://github.com/gigafrosxh/kings-of-dawn/issues/42) |
| v0.36.0 | F-31 – Krieg, Fronten und Konfliktsimulation | [#41](https://github.com/gigafrosxh/kings-of-dawn/issues/41) |
| v0.37.0 | F-33 – Besatzung, Friedenskonferenz und Wiederaufbau | [#43](https://github.com/gigafrosxh/kings-of-dawn/issues/43) |
| v0.38.0 | F-34 – Nachrichtendienste und Informationslage | [#44](https://github.com/gigafrosxh/kings-of-dawn/issues/44) |
| v0.39.0 | F-36 – Ereignisse, Missionen und Szenarioeditor | [#46](https://github.com/gigafrosxh/kings-of-dawn/issues/46) |
| v0.40.0 | F-37 – Zeitung, Chronik und Geschichtsarchiv | [#47](https://github.com/gigafrosxh/kings-of-dawn/issues/47) |
| v0.41.0 | F-40 – Modding und benutzerdefinierte Inhalte | [#50](https://github.com/gigafrosxh/kings-of-dawn/issues/50) |
| v0.42.0 | F-41 – Multiplayer und Rollenverwaltung | [#51](https://github.com/gigafrosxh/kings-of-dawn/issues/51) |

## 7. Arbeitsregeln für Coding-Agenten

- Zu Beginn dieser Datei folgen und den aktuell freigegebenen Versions-Milestone prüfen. Nicht einfach das nächste Feature anfangen.
- Nur aktuelle Issues, relevante Abhängigkeiten und betroffene Dateien lesen. Kein vollständiges HTML mit Base64-Bildern und keine unnötigen Build-Logs in den Kontext laden.
- Bugfixes zuerst reproduzieren, Regressionstest ergänzen, minimal korrigieren, bauen und testen. Ergebnisse, Commit und Fix-Version im Issue dokumentieren.
- Issues nach verifiziertem Fix schließen; Feature-Issue erst nach kompletter Stabilisierung. Keine erfundenen Tests, Release-Ergebnisse oder Bugs.
- Tags nicht überschreiben. Keine künstlichen Patch- oder Feature-Änderungen; Releases mit nachvollziehbaren Versionsständen und Testnachweisen.
- Milestones/Labels initialisieren bzw. bestehende Feature-Issues zuordnen: `scripts/setup-milestones.ps1` (GitHub CLI, authentifizierter Account). Bestehende Bugs werden nicht automatisch umgehängt, außer ihre Zuordnung ist ausdrücklich bekannt.

[Issues](https://github.com/gigafrosxh/kings-of-dawn/issues) · [Milestones](https://github.com/gigafrosxh/kings-of-dawn/milestones) · [Releases](https://github.com/gigafrosxh/kings-of-dawn/releases)
