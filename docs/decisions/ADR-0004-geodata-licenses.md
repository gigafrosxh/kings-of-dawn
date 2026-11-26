# ADR-0004 – Lizenzentscheidung für Kartendaten

- Status: vorgeschlagen (Issue #57, Teil von #9) – **Annahme kommerzieller Vertrieb vom Projektinhaber zu bestätigen**
- Maschinenlesbar: [`data/sources/manifest.json`](../../data/sources/manifest.json), geprüft durch `scripts/validate-sources.py` (CI-Workflow `data`)
- Prüfstand: 2026-11-26 (Lizenzseiten der Quellen A–F abgerufen; G/H nicht erneut geprüft)

## Annahme

Kings of Dawn wird als kommerzielles Spiel vertrieben und liefert Geodaten im Build aus. Damit scheiden NonCommercial-Lizenzen aus, und Copyleft-Daten dürfen nicht mit den ausgelieferten Daten verschmelzen.

## Entscheidung je Quelle

| ID | Quelle | Version | Lizenz | Entscheidung |
|---|---|---|---|---|
| A | Cliopatria | v0.2.2 (2026-11-25) | CC BY 4.0 | **ausliefern** – Primärquelle; Attribution + DOI in Credits/Datenhinweis; Unterquellen im Archiv vor Import prüfen |
| B | historical-basemaps | HEAD | GPL-3.0 | **nur Werkzeug** – Vergleich in Offline-Validierung; keine Geometrie/Attribute in ausgelieferten Daten |
| C | OpenHistoricalMap | Live-DB | CC0 (Ausnahmen je Objekt) | **gefiltert ausliefern** – nur Objekte ohne bzw. mit CC0-`license`-Tag; CC BY-SA und OSM-Küstenlinien ausgeschlossen |
| D | CShapes 2.0 | 2.0 | CC BY-NC-SA 4.0 | **ausgeschlossen** – NonCommercial |
| E | Natural Earth Admin 0 | 5.1.1 | Public Domain | **ausliefern** – moderne Referenzebene; Drittanbieter-Ebenen mit eigenen Release Notes nicht importieren |
| F | geoBoundaries gbOpen | Release bei Download | CC BY 4.0 / ODbL je Grenze | **bedingt** – nur Grenzen mit CC-BY-Nachweis; ODbL nur als separate Datenbank mit Hinweis; CGAZ bis zur Klärung ausgeschlossen |
| G | Cliopatria-Fachartikel | – | Artikel | Referenz |
| H | Natural Earth Grenzpolitik | – | Richtlinie | Referenz |

## Konsequenzen

- Der Atlas-Import (F-02, #12) muss je Objekt die Quell-ID und Lizenz mitführen und Objekte mit Entscheidung `tooling-only`/`exclude` aus Build-Daten fernhalten.
- Credits-/Datenhinweis-Screen muss die Attributionspflichten aus dem Manifest anzeigen (A, F; empfohlen C, E).
- Prüfsummen (`sha256`) werden beim ersten Download eingetragen; bis dahin `null`. Downloads erfolgen erst im Import-Schritt.
- Bei nicht-kommerziellem Vertrieb wäre D als zusätzliche Validierungsquelle neu zu bewerten.
- Dies ist eine technische Lizenzeinordnung, keine Rechtsberatung; vor Veröffentlichung juristisch prüfen lassen.
