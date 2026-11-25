# Simulationsverträge

## Simulationsvertrag

Commands werden autorisiert, validiert, in stabiler Reihenfolge atomar angewandt und als Events protokolliert. Tickfolge: fällige Regeln/Events → Demografie/Arbeitsangebot → Produktion → Handel/Bestände → Preise/FX → Buchungen/Budget → Versorgung/Konflikte → politische Stimmung/KI → Statistik/Snapshot. KI plant für den nächsten Tick. Simulationszeit: tägliche Ereignisse, wöchentliche/monatliche Wirtschaft, jährliche Demografie mit unterjähriger Verteilung. Keine Abhängigkeit von Render-FPS oder Wandzeit. Seed, sortierte IDs, Fixed-Point-Werte, definierte Rundung und State-Hash sichern Reproduzierbarkeit.

## Karten- und Zeitmodell

Interne Datumsdarstellung astronomisch: 100 v. Chr. = -99, 1 v. Chr. = 0, 1 n. Chr. = 1; Jahr 0 niemals in der Anzeige. Historische Quelljahre explizit konvertieren, keine pauschale Interpretation negativer Zahlen. Intervalle [validFrom, validTo), Granularität und Kalendersystem speichern; Default 1. Januar pro Jahr im proleptischen Kalender, nicht als gesicherter historischer Stichtag ausgeben. Basiszellen mit stabilen IDs und Adjazenz; Provinzen sind veränderbare Gruppen. Kontrolle, Verwaltung, Anspruch und Souveränität als getrennte Beziehungen. Unbekannt ist nicht unbewohnt. Historische Küstenveränderungen separat; topologische Fehler beheben und protokollieren.

## Wirtschaftsmodell

Warenbestand nächster Tick = Bestand + Produktion + Import − Konsum − Export − Verlust. Produktion begrenzt durch Kapazität, Arbeit, Energie und Inputs. Preise reagieren gedämpft auf Nachfrage/Angebot und Lagerquote, mit positiven Grenzen und Transaktionskosten. Geldbeträge als Fixed-Point, Buchungen in Kontowährung, FX-Differenzen separat; Geldschöpfung nur über explizite Bilanztransaktionen. Wechselkurse aus Zahlungsströmen, Inflationserwartung, Vertrauen und Reserven, nicht aus beliebigen BIP-Prozentboni; Referenzkorb pro Epoche, moderne EUR-Anzeige nur optional. Nominal-/Realwerte und Kaufkraft trennen. Keine 1:1-Vorhersage realer Wechselkurse.

## Gesellschaft & Politik

Kohorten statt einzelner Milliarden Einwohner; einzelne Personen nur für relevante Rollen. Kohortenbilanz: Bevölkerung + Geburten − Todesfälle + Zuzug − Wegzug. Gesetzes-DSL enthält Parameter, Typ, Geltungsbereich, Datum und Durchsetzung; Effekte über Domainregeln. Institutionen bestimmen rechtliche Kompetenzen, Verwaltungskapazität bestimmt Umsetzung. Politische Zustimmung aus Lebensbedingungen, Interessen und Ereignissen; keine pauschale Bewertung von Kultur/Religion.

## Epochenmodell

Epochenpakete definieren verfügbare Institutionen, Währungen, Einheiten, Transport, Produktion, Rechtsformen und Technologien. Antike: Tausch-/Metallgeld, Landwirtschaft, historische Stadt-/Herrschaftsformen; Mittelalter: Feudalstrukturen, regionale Märkte; frühe Neuzeit: Handelsnetze und Staatenbildung; Industriezeit: Fabriken, Schienen, Massenpolitik; Moderne: Zentralbanken, komplexe Lieferketten, moderne Institutionen. Technologie ist Voraussetzung, kein hartes Kalenderjahr; historischer Start hat epochenplausible Ausstattung.
