# ADR-0005 – Datenmodell des historischen Atlas

- Status: angenommen (Issue #12, F-02, Schritt 1)
- Bereich: `rust/crates/atlas` (Crate `kod-atlas`)

## Kontext

F-02 verlangt zeitabhängige Grenzen mit Herkunftsnachweis, getrennte Ebenen für de facto und de jure, unsichere Gebiete, keine stille Interpolation und explizit gemeldete Lücken. Die Daten werden offline aus den Quellen in `data/sources/manifest.json` aufbereitet (ADR-0004) und mit dem Spiel ausgeliefert.

## Entscheidung

- **Gültigkeit** als halboffenes Intervall `[from, to)` in astronomischen Jahren (`ValidityInterval`, nie leer), wie in F-01.
- **Objekte:** `PolityAssertion` (Staat mit Name und Datengüte je Intervall) und `BorderVersion` (Territorium je Staat, Ebene `DeFacto`/`DeJure`, `Certainty`, `Precision` Jahr/Jahrhundert). Jedes Objekt trägt eine `SourceRef` (Quell-ID aus dem Manifest, Datensatz-ID, Lizenz des Datensatzes).
- **Geometrie** in ganzzahligen Mikrograd (WGS84 × 10⁶, `i32`). Alle Prüfungen sind exakt ganzzahlig und damit plattformunabhängig deterministisch, ohne Gleitkomma. Ringe müssen geschlossen, frei von Selbstüberschneidung und flächig sein. Löcher müssen strikt innerhalb des Außenrings liegen.
- **Abfrage** `Atlas::state_at(year)` liefert nur Objekte, deren Intervall das Jahr enthält. Zwischen zwei Versionen wird nichts interpoliert. Die Abdeckung ist `Covered`, `Gap` (deklarierte Lücke) oder `Undeclared` (fehlende Daten, müssen in Abnahmetests auffallen).
- **Lizenzregel des Inhabers (#57)** wird beim Erzeugen *und* beim Laden erzwungen. Ein Objekt ist nur zulässig, wenn
  - seine Quelle im Manifest steht,
  - die Manifest-Entscheidung Ausliefern erlaubt (`bundle`, `bundle-filtered`, `conditional`),
  - `commercial_use` gesetzt ist
  - und die Datensatzlizenz in der Liste der auslieferbaren Lizenzen dieser Quelle steht.
- **Invarianten:**
  - keine überlappenden Zusicherungen je Staat;
  - höchstens eine Grenzversion je Staat, Ebene und Jahr;
  - jede Grenzversion liegt vollständig in einer Zusicherung ihres Staats.
- **Datenformat:** kanonische Binärkodierung über `kod_core::codec` mit Kennung `KODA` und Formatversion 1. Beim Dekodieren werden alle Invarianten erneut geprüft. Die Prüfsumme ist `StateHash` über die kanonischen Bytes.

## Folgen

- Der Import (Schritt 2) muss Koordinaten runden und ungültige Geometrien reparieren oder ausschließen, statt sie zu übernehmen. Jede Korrektur wird protokolliert.
- Die Selbstüberschneidungsprüfung ist O(n²) je Ring, mit Bounding-Box-Vorfilter. Mit den echten Cliopatria-Daten wird gemessen; bei Bedarf folgt ein Sweep-Verfahren.
- `AtlasSource` (F-01) verlangt einen Datensatz je Staat. Die Anbindung folgt in Schritt 5 und fasst mehrere Zusicherungen eines Staats zusammen.
