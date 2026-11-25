# Historischer Atlas: Quellen und Import

## Historische Karten

Für jedes Jahrhundert: automatisiert die aktuellste lizenzgeeignete Version der globalen Geodaten und zusätzliche regionale Quellen recherchieren; Quellenstand, Abrufdatum, Datenzeitstand, Abdeckung, Auflösung, Lizenz, Prüfsumme und Konflikte erfassen. Einen Weltkarten-Referenzstand am Jahrhundertbeginn erzeugen; zusätzliche relevante Grenzwechsel und alle verfügbaren Jahresstände importieren. Für jedes gewünschte Jahr gültige territoriale Aussagen auswählen; wenn nur benachbarte Stände existieren, letzten belegten Stand nur als Näherung anzeigen, niemals Grenzen linear interpolieren oder exakte Jahresgenauigkeit vortäuschen. Regionale datierte Belege priorisieren, widersprüchliche Aussagen parallel speichern, tatsächliche Kontrolle vs. Anspruch getrennt visualisieren. Für 2025 bis zum jeweils aktuellen Kalenderjahr aktuelle Karten mit datierten amtlichen/seriösen Quellen für Veränderungen ergänzen. Import in Quarantäne → Lizenzprüfung → Zeit-/Koordinatennormalisierung (EPSG:4326) → Geometrievalidierung/Antimeridian → stabile IDs/Topologie → Quellen-/Konfliktprüfung → Offline-Tiles/Index → visuelle und automatisierte QA → versionierte Veröffentlichung. Updates nie still in laufende Saves einspielen. Die Matrix ist ein Recherche- und Integrationsplan, keine bereits erzeugte oder verifizierte Sammlung von 22 Weltkarten. Ein tatsächlicher Download/Import ist Implementierungsarbeit.

## Quellen

- A: [Cliopatria v0.2.2](https://zenodo.org/records/24518977) – 25.11.2026; weltweit historische Polities, Referenzreichweite 3400 v. Chr.–2024 n. Chr.; neueste gefundene Version, frame years bleiben Einschränkung. CC BY 4.0 für Basissatz; konkrete Release-Dateien/Unterquellen vor Import prüfen.

- B: [Historical Basemaps](https://github.com/aourednik/historical-basemaps) – Historische GeoJSON-Zeitstände, Sekundärvergleich; GPL-3.0 und Ursprungsquellen prüfen, Datenpaket getrennt halten oder bei inkompatibler Distribution ausschließen.

- C: [OpenHistoricalMap](https://www.openhistoricalmap.org/copyright) – Datierte regionale Geometrien, Straßen, Städte und Gebäude; grundsätzlich CC0, Ausnahmen pro Objekt; keine lückenlose globale Jahresabdeckung.

- D: [CShapes 2.0](https://icr.ethz.ch/data/cshapes/) – 1886–2019 weltweit; Europa ab 1816; wissenschaftlicher Vergleich, nicht kommerziell ohne passende Rechte; nicht als Standardasset ausliefern.

- E: [Natural Earth Admin 0](https://www.naturalearthdata.com/downloads/10m-cultural-vectors/10m-admin-0-countries/) – Moderne politische Basiskarte, Länderdatei Version 5.1.1 laut Seite; Public Domain. Nicht automatisch ein tagesaktueller Grenzstand. Umstrittene Gebiete gesondert.

- F: [geoBoundaries](https://www.geoboundaries.org/) – Aktuelle administrative Ebenen; gbOpen und einzelne Metadaten/Lizenzen prüfen, CC BY 4.0 als allgemeine Angabe; nicht auf frühere Jahrhunderte übertragen.

- G: [Cliopatria Fachartikel](https://www.nature.com/articles/s41597-025-04516-9) – Methodik, Grenzen der historischen Rekonstruktion und Validierung; kein Ersatz für Primärquellen.

- H: [Natural Earth Grenzpolitik](https://www.naturalearthdata.com/about/disputed-boundaries-policy/) – De facto, de jure und unterschiedliche Sichtweisen; kartografische Entscheidung dokumentieren.

## Jahrhundertmatrix

| Zeitfenster | Referenzjahr | Quellen | Status |

|---|---|---|---|

| 100–1 v. Chr. | 100 v. Chr. | A, B, C, G | Import & QA offen |

| 1–100 n. Chr. | 1 n. Chr. | A, B, C, G | Import & QA offen |

| 101–200 n. Chr. | 101 n. Chr. | A, B, C, G | Import & QA offen |

| 201–300 n. Chr. | 201 n. Chr. | A, B, C, G | Import & QA offen |

| 301–400 n. Chr. | 301 n. Chr. | A, B, C, G | Import & QA offen |

| 401–500 n. Chr. | 401 n. Chr. | A, B, C, G | Import & QA offen |

| 501–600 n. Chr. | 501 n. Chr. | A, B, C, G | Import & QA offen |

| 601–700 n. Chr. | 601 n. Chr. | A, B, C, G | Import & QA offen |

| 701–800 n. Chr. | 701 n. Chr. | A, B, C, G | Import & QA offen |

| 801–900 n. Chr. | 801 n. Chr. | A, B, C, G | Import & QA offen |

| 901–1000 n. Chr. | 901 n. Chr. | A, B, C, G | Import & QA offen |

| 1001–1100 n. Chr. | 1001 n. Chr. | A, B, C, G | Import & QA offen |

| 1101–1200 n. Chr. | 1101 n. Chr. | A, B, C, G | Import & QA offen |

| 1201–1300 n. Chr. | 1201 n. Chr. | A, B, C, G | Import & QA offen |

| 1301–1400 n. Chr. | 1301 n. Chr. | A, B, C, G | Import & QA offen |

| 1401–1500 n. Chr. | 1401 n. Chr. | A, B, C, G | Import & QA offen |

| 1501–1600 n. Chr. | 1501 n. Chr. | A, B, C, G | Import & QA offen |

| 1601–1700 n. Chr. | 1601 n. Chr. | A, B, C, G | Import & QA offen |

| 1701–1800 n. Chr. | 1701 n. Chr. | A, B, C, G | Import & QA offen |

| 1801–1900 n. Chr. | 1801 n. Chr. | A, B, C, D, G | Import & QA offen |

| 1901–2000 n. Chr. | 1901 n. Chr. | A, B, C, D, G | Import & QA offen |

| 2001–heute | 2001 + 2024 + aktueller Stand | A, C, D, E, F, H | Import & QA offen |

Quellenstand aus der ursprünglichen Spezifikation (25.11.2026); Aktualität, Lizenz und Datenabdeckung vor Import erneut prüfen.
