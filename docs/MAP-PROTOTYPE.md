# Kartenprototyp für #9

Nach dem Build von KingsOfDawnEditor kann der Prototyp direkt aus dem Checkout gestartet werden. UE bezeichnet die ausführbare Datei auf macOS als UnrealEditor, unter Windows als UnrealEditor.exe.

```sh
"$UE_ROOT/Engine/Binaries/Mac/UnrealEditor" "$PWD/unreal/KingsOfDawn.uproject" '/Engine/Maps/Entry?game=/Script/KingsOfDawn.MapPrototypeMode' -game -windowed -ResX=1280 -ResY=720
```

Das Mausrad zoomt. Ein Linksklick zeigt die Zell-ID. Die 100 × 100 Zellen verwenden die Zeilenreihenfolge (ID = y × 100 + x). Vier GPU-instanzierte Gruppen stellen vier synthetische Besitzer dar. Für die Übersicht ab einer orthografischen Breite von über 20.000 Unreal Units werden 400 größere Kacheln verwendet. Die Zuordnung beim Klicken bleibt zellgenau. Besitzerwechsel im Raster richten sich am gröberen Gitter aus. Es handelt sich um Testdaten für das Rendering.

Für den automatisierten Picking-Test den Editor mit `-NullRHI -unattended "-ExecCmds=Automation RunTests KingsOfDawn.Map.Picking" "-TestExit=Automation Test Queue Empty"` starten. Der Test prüft die Mittelpunkte aller 10.000 Zellen und die äußeren Grenzen.

Für die Messung den Spielstart oben um `-RenderOffscreen -NoVSync -KodMapBenchmark` ergänzen. Nach 120 Aufwärmframes misst der Prototyp 300 Frames per Wanduhr und beendet das Spiel. Die Ausgabe „Map prototype“ im Projektlog enthält die mittlere Framezeit. Der Wert gilt für die ruhende Detailansicht auf dem jeweiligen Gerät.
