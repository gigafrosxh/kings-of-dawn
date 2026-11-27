# ADR-0004 – Lizenzentscheidung für Kartendaten

- Status: angenommen (Issue #57, Teil von #9). Vorgabe des Projektinhabers vom 2026-11-27: Das Vertriebsmodell ist **noch offen**. Bis zu einer Entscheidung werden ausschließlich Kartendaten verwendet, deren Lizenz kommerzielle Nutzung erlaubt, damit ein späterer Verkauf möglich bleibt.
- Maschinenlesbar: [`data/sources/manifest.json`](../../data/sources/manifest.json), geprüft durch `scripts/validate-sources.py` (CI-Workflow `data`)
- Prüfstand: 2026-11-26 (Lizenzseiten der Quellen A–F abgerufen; G/H nicht erneut geprüft)

## Annahme

Kings of Dawn muss kommerziell vertreibbar bleiben (Vertriebsmodell noch offen, siehe Status) und liefert Geodaten im Build aus. Damit scheiden NonCommercial-Lizenzen aus, und Copyleft-Daten dürfen nicht mit den ausgelieferten Daten verschmelzen.

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
- Erst wenn der Projektinhaber ausdrücklich einen nicht-kommerziellen Vertrieb festlegt, wäre D als zusätzliche Validierungsquelle neu zu bewerten. Bis dahin bleibt D ausgeschlossen.
- Dies ist eine technische Lizenzeinordnung, keine Rechtsberatung; vor Veröffentlichung juristisch prüfen lassen.
