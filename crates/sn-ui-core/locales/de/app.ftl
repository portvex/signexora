# Main window (MDI frame): messages, File menu extras, the WF window's toolbar
# and popup menu, and the confirmation dialogs.

## View kinds (child windows)

app-tab-sonogram = Sonagramm
app-tab-waveform = Wellenform
app-tab-raster = Raster (VMW)
app-tab-phase = PhasePlane
app-tab-demod = FSK/MFSK-Demodulator
app-tab-ofdm = OFDM

## Lists of names: "A", "A and B", "A, B and C"

app-list-separator = { ", " }
app-list-and = { $rest } und { $last }

## Background jobs (shown in the status bar while they run)

app-job-loading = Lade { $name }
app-job-overview = Erstelle Wellenform-Übersicht
app-job-saving = Speichere { $path }
app-job-measure-line = Messe Linienfrequenz
app-job-adp = Schätze Baudrate (ADP)
app-job-writing = Schreibe { $path }
app-job-replay = Stelle Sitzung wieder her
app-job-script = Führe Skript aus
app-job-find-tones = Suche Töne
app-job-playback = Bereite Wiedergabe vor
app-job-demodulator = Demodulator

## Status messages

app-ready = Bereit
app-signal-gone = Dieses Signal existiert nicht mehr.
app-set-h-markers-first = Zuerst die H-Marker um die Linie setzen.
app-sc-unmappable = Sc kann nicht durch { $edit } abgebildet werden: Die Ausgabe hat keinen Frequenzbezug zur Eingabe. Den Träger vor dem Schritt { $edit } messen.
app-recorded = { $kind } = { $value } aus { $node } ({ $tool })
app-recorded-uncertain = { $kind } = { $value } +/- { $uncertainty } aus { $node } ({ $tool })
app-not-recorded = { $kind } nicht erfasst: { $error }
app-mapped-onto = { $from }, abgebildet auf { $node }
app-session-save-running = Die Sitzung wird bereits gespeichert
app-export-report-failed = Bericht exportieren: { $error }
app-save-session-failed = Sitzung speichern: { $error }
app-open-session-failed = Sitzung öffnen: { $error }
app-script-error = Fehler: { $error }
app-replay-skipped = { $what ->
    [markers] { $node } ({ $label }): Gespeicherte Marker waren nicht lesbar und wurden übersprungen: { $error }
    [tones] { $node } ({ $label }): Gespeicherte Töne waren nicht lesbar und wurden übersprungen: { $error }
    [grid] { $node } ({ $label }): Gespeichertes Tonraster war nicht lesbar und wurde übersprungen: { $error }
   *[adp] { $node } ({ $label }): Gespeicherte ADP-Kandidaten waren nicht lesbar und wurden übersprungen: { $error }
}
app-session-restored = Sitzung wiederhergestellt
app-session-restored-problems = { $count ->
    [one] Sitzung wiederhergestellt, 1 Problem (siehe Fenster)
   *[other] Sitzung wiederhergestellt, { $count } Probleme (siehe Fenster)
}
app-wrote-report = Bericht { $path } geschrieben
app-saved = { $path } gespeichert
app-saved-session = Sitzung { $path } gespeichert
app-saved-session-replaced = Sitzung { $path } gespeichert (die enthaltene Sitzung wurde ersetzt)
app-cancel-too-late = { $name } war vor dem Abbruch fertig; das Ergebnis wurde behalten.
app-job-cancelled = { $name }: abgebrochen
app-job-failed = { $name }: { $error }
app-playback-cancelled = Wiedergabe abgebrochen
app-derived-opened = { $label }: neues Signal { $node } in { $view } geöffnet
app-derived-opened-mapped = { $label }: neues Signal { $node } in { $view } geöffnet; Marker auf seine Achsen übertragen
app-derived-opened-cleared = { $label }: neues Signal { $node } in { $view } geöffnet; einige Marker wurden entfernt: Das neue Signal hat eine andere Zeit- oder Frequenzachse
app-derive-failed = { $label }: { $error }
app-measured-line = { $kind } = { $value } +/- { $uncertainty } aus { $node } (Linienmessung; { $level } dBFS, { $snr } dB über dem Band)
app-measured-line-mapped = { $kind } = { $value } +/- { $uncertainty } aus { $node } (Linienmessung; { $level } dBFS, { $snr } dB über dem Band), Linie bei { $line } Hz über Potenz { $power } zurückgerechnet
app-loaded = { $label } geladen: { $samples } Samples bei { $rate } Hz
app-loaded-notes = { $label } geladen: { $samples } Samples bei { $rate } Hz. { $notes }
app-script-no-signals = Skript { $path } hat keine Signale erzeugt
app-script-finished = Skript { $path } beendet
app-adp-none = ADP hat in der Auswahl keine Taktrate gefunden
app-tones-found = { $count ->
    [one] 1 Ton auf { $node } gefunden
   *[other] { $count } Töne auf { $node } gefunden
}
app-markers-restored = Marker auf { $node } wiederhergestellt
app-closed-tab-stopped = { $count ->
    [one] { $name } geschlossen; der laufende Auftrag { $jobs } wurde gestoppt. Öffnen Sie das Fenster erneut und starten Sie ihn neu.
   *[other] { $name } geschlossen; die laufenden Aufträge { $jobs } wurden gestoppt. Öffnen Sie das Fenster erneut und starten Sie sie neu.
}
app-nothing-to-play = Nichts abzuspielen: Sonagramm öffnen oder V-Marker setzen.
app-find-tones-failed = Töne suchen: { $error }

## Confirmation before work is lost

app-confirm-open-title = Sitzung öffnen
app-confirm-open-question = Aktuelle Sitzung ersetzen?
app-confirm-replace = Ersetzen
app-confirm-script-title = Skript ausführen
app-confirm-script-question = Die Signale des Skripts ersetzen die aktuelle Sitzung.
app-confirm-quit-title = Beenden
app-confirm-quit-unsaved = Sitzung vor dem Beenden speichern?
app-confirm-quit-without-saving = Ohne Speichern beenden
app-confirm-quit-writing = Eine Datei wird noch geschrieben.
app-confirm-quit-anyway = Trotzdem beenden
app-confirm-lost = { $signals ->
    [one] 1 Signal
   *[other] { $signals } Signale
} und { $measurements ->
    [one] 1 Messung
   *[other] { $measurements } Messungen
}, die nicht gespeichert sind, werden geschlossen. Dies kann nicht rückgängig gemacht werden.
app-confirm-write-running = { $name } läuft noch; beim Beenden wird es abgebrochen.
app-confirm-save-first = Zuerst Sitzung speichern
app-confirm-save-first-ask = Zuerst Sitzung speichern...
app-confirm-wait = Warten
app-confirm-keep-open = App geöffnet lassen
app-cancel = Abbrechen

## File dialogs

app-dialog-save-as = Als { $format } speichern
app-dialog-save-session-as = Sitzung speichern unter
app-filter-session = SigNexora-Sitzung
app-dialog-export-report = Analysebericht exportieren
app-filter-report = Bericht
app-dialog-open-session = Sitzung öffnen
app-dialog-run-script = Skript ausführen
app-filter-rhai = Rhai-Skript

## File menu

app-menu-file = Datei
app-menu-open-session = Sitzung öffnen...
    .hover = Ihre Signale ersetzen die aktuelle Sitzung
app-nothing-to-save = Nichts zu speichern
app-menu-save-session = Sitzung speichern
app-menu-save-session-ask = Sitzung speichern...
app-menu-save-session-choose = Datei für die Sitzung wählen
app-menu-save-session-to = In { $path } speichern
app-menu-save-session-as = Sitzung speichern unter...
app-menu-export-report = Analysebericht exportieren
app-menu-report-html = HTML...
app-menu-report-markdown = Markdown...

## File menu extras and the WF popup menu

app-menu-run-script = Skript ausführen (Rhai)...
    .hover = Führt ein Skript aus; seine Signale ersetzen die aktuelle Sitzung
app-resynth = FSK/MFSK neu synthetisieren
    .hover = Sauberes Signal auf den Tönen aus "Töne suchen" mit Br neu erzeugen
    .disabled = Zuerst Töne suchen (Wellenform Sum) und Br messen.
app-menu-measure = Messen
app-measure-hint = H-Marker um die Linie setzen (und V-Marker für den Zeitbereich).
app-measure-br = Linie als Baudrate messen (Br)
app-measure-sc = Linie als Träger messen (Sc)
    .hover = Nach Potenz N wird die Linie auf den Träger zurückgerechnet (durch N geteilt, Verschiebungen aufgehoben)
app-measure-adp = Baudrate automatisch schätzen (ADP)
    .hover = Nutzt den V-Marker-Bereich oder das ganze Signal
app-measure-dt-disabled = Zuerst die V-Marker setzen.

## Hints

app-no-signals-hint = Öffnen Sie eine Aufnahme (Datei > Datei öffnen... ({ $shortcut }), oder ziehen Sie eine Datei auf das Fenster).
app-open-in = Öffnen...
    .hover = Dieses Signal in einer anderen Ansicht oder einem Werkzeug öffnen

## Sonogram toolbar

app-sono-shear = Shear
    .hover = Schritt zwischen FFT-Blöcken als Bruchteil der FFT-Größe. Ein kleinerer Schritt überlappt die Blöcke stärker: feinere Zeitauflösung, langsamere Berechnung
app-sono-image = Image
    .hover = Dynamikbereich unterhalb der stärksten Komponente
app-sono-hres-hover = Spektrallinien durch Frequenz-Reassignment schärfen (hilft beim Finden schwacher Taktlinien). Töne behalten ihren Pegel; Rauschen erscheint etwa 2 bis 6 dB niedriger als ohne HRes (je nach Fenster), da es pro Bin-Breite dargestellt wird.
app-sono-alc-hover = Jede Spalte auf ihre stärkste Komponente normieren (nur Anzeige)
app-sono-combine = Herausgezoomt
    .hover = Wie FFT-Blöcke, die sich herausgezoomt eine Bildspalte teilen, zusammengefasst werden
app-sono-peak = Spitze
    .hover = Herausgezoomt: kurze Bursts sichtbar halten
app-sono-mean = Mittel
    .hover = Herausgezoomt: mitteln, glatteres Rauschen

## Waveform toolbar

app-wave-show-q = Q zeigen
app-find-tones = Töne suchen
    .hover = Spitzen des Histogramms: Tonfrequenzen am Ausgang des Phasendetektors

## Zoom and playback

app-play = Abspielen
    .hover = V-Marker-Bereich oder sichtbaren Bereich abspielen ({ $shortcut })
app-play-stop = Stopp
    .hover = Wiedergabe stoppen ({ $shortcut })

## Child windows

app-tab-signal-gone = Dieses Signal existiert nicht mehr.

## Windows

app-replay-problems-title = Sitzung mit Problemen wiederhergestellt
app-script-output = Skriptausgabe: { $path }
app-script-no-output = (keine Ausgabe)
# $lines: the lines kept; $dropped: the lines left out.
app-script-output-truncated = Ausgabe nach { $lines } Zeilen gekürzt; { $dropped } weitere Zeilen wurden nicht behalten.

## Closing a window, the sonogram toolbar

app-confirm-close-tab-title = Fenster schließen?
# $name: the window title; $jobs: the job names, joined.
app-confirm-close-tab-question = { $count ->
    [one] Das Schließen von { $name } stoppt den laufenden Auftrag { $jobs }. Trotzdem schließen?
   *[other] Das Schließen von { $name } stoppt die laufenden Aufträge { $jobs }. Trotzdem schließen?
}
app-confirm-close-and-stop = Schließen und stoppen
app-confirm-keep-tab = Geöffnet lassen
# $prev, $next: the keys that show the previous and next signal.
app-select-signal-first = Holen Sie zuerst ein Fenster nach vorn ({ $prev } oder { $next } zeigt die Signale der Reihe nach).
app-sono-fft = SizeFFT
    .hover = FFT-Größe: größer ergibt feinere Frequenz- und gröbere Zeitauflösung
app-sono-window = WinF
    .hover = Fensterfunktion jeder FFT (WinF in SA)
app-sono-colormap = Farben
    .hover = Farbskala des Sonagrammbilds
app-confirm-close-all-title = Alle Fenster schließen
app-confirm-close-all-question = Damit werden alle Signale und Messungen aus der Sitzung entfernt.
app-confirm-close-all = Alle schließen
app-confirm-close-all-stops = { $name } läuft noch; Alle Fenster schließen hält es an.
app-replay-session-skipped = { $what ->
    [windows] Die gespeicherte Fensteranordnung ist unlesbar, daher öffnet sich ein Fenster je Ergebnis: { $error }
   *[counters] Die gespeicherten Namenszähler sind unlesbar, neue Namen können Nummern wiederholen: { $error }
}
app-replay-windows-missing = { $count ->
    [one] 1 gespeichertes Fenster wurde nicht geöffnet, da sein Signal nicht wiederhergestellt wurde
   *[other] { $count } gespeicherte Fenster wurden nicht geöffnet, da ihre Signale nicht wiederhergestellt wurden
}
app-drop-loader-open = Der Dateidialog ist offen: erst abschließen oder abbrechen, dann die Datei erneut ablegen
app-drop-first-only = { $count } Dateien abgelegt; nur die erste, { $name }, wird geöffnet
app-sono-target = { $active ->
    [yes] Ändert das aktive Sonogrammfenster.
   *[no] Ändert die Vorgabe für neue Sonogrammfenster.
}
app-edit-unrecordable = es konnte nicht zur Wiederholung in der Sitzung gespeichert werden ({ $error })
app-empty-open = Aufnahme öffnen... ({ $key })
app-empty-drop = oder eine Datei hier ablegen (auch über File > Open file...).
app-empty-guide = Der Leitfaden ({ $key }) zeigt die Analyse Schritt für Schritt.
# $action: what the chooser was for, e.g. "Open session"; $error: why it failed.
app-chooser-failed = { $action }: Der Dateiauswahldialog konnte nicht angezeigt werden ({ $error }). Versuchen Sie es erneut oder prüfen Sie, ob der Dateidialogdienst des Systems (Desktop-Portal) läuft.

## The tools that record a measurement, by the id stored in the session.

measure-tool-adp = Parameter automatisch bestimmen (ADP)
measure-tool-adp-coarse = Parameter automatisch bestimmen, grob (auf eine halbe FFT-Zelle genau)
measure-tool-measure-line = Frequenzmessung (Messlinie)
measure-tool-v-markers = V-Marker (dT)
measure-tool-ofdm-adp = OFDM-Werkzeug, Get Br (ADP)
measure-tool-script-record = Skript, record()
