# The PhasePlane form (SA_UI_SPEC.md section 6): spectrum mode toolbar, ISD
# corrector, phase viewer, status bar, demodulator output and the LoadFile dialog.
# The control captions are SA's own English names (SA showed them in English under
# every Windows language): keep them as they are; translate the hints and dialogs.

pp-fmin = Fmin
    .hover = Niedrigste angezeigte Frequenz, Hz
pp-fmax = Fmax
    .hover = Höchste angezeigte Frequenz, Hz
pp-scale = Scale
    .hover = Gezeigter Teil des Bandes; die Bildlaufleiste darunter verschiebt ihn
pp-fft-size = FFTSize
    .hover = FFT-Größe des gemittelten Spektrums: größer ist feiner (bis etwa 1 Hz)
pp-low-level = LowLevel
    .hover = Niedrigster gezeigter Pegel
pp-stretch = Stretch
    .hover = Spektrum auf die volle Höhe strecken
pp-mode = Mode
    .hover = AM: Korrektur an der Hüllkurve; X^N: an der N-ten Potenz des Signals
pp-blocks = Blocks
    .hover = Anzahl der Signalblöcke, die der Korrektor mittelt
pp-corr-off = Corr-OFF
    .hover = Der ISD-Korrektor ist aus: Auto drücken, um das Signal zu korrigieren
pp-corr-on = Corr-ON
    .hover = Der ISD-Korrektor ist an (SaveAs speichert das korrigierte Signal)
pp-save-as = SaveAs
    .hover = Korrigiertes Signal in einer Datei speichern
pp-auto = Auto
    .hover = Ein Schritt des Korrektors über die gewählten Blöcke
pp-break = Break
pp-reset = Reset
    .hover = Korrektor zurücksetzen (ausschalten)
pp-carrier = Carrier
    .hover = Trägerfrequenz, Hz (letzte Sc-Messung)
pp-br = BR
    .hover = Schrittgeschwindigkeit, Bd (letzte Br-Messung)
pp-nary = n-Ary
    .hover = Ordnung der Modulation, 2-32 (1: ohne Trägernachführung)
pp-synch-br = Synch BR
    .hover = Taktsynchronisation
pp-nonstop = NonStop
    .hover = Dauerbetrieb: am Dateiende neu beginnen
pp-rrc = RRC filter
    .hover = Wurzel-Kosinus-Filter; der Regler stellt alpha ein
pp-pll = PLL
    .hover = Geschwindigkeit der Trägerschleife
pp-pll-slow = Slow
pp-pll-medium = Medium
pp-pll-quick = Quick
pp-pll-hard = Hard
pp-auto-reset = AutoReset
    .hover = Mit NonStop: Generatoren und PLL-Schleifen am Dateiende zurücksetzen (driftende Signale)
pp-transitions = Transitions
    .hover = Übergänge zwischen den Punkten zeichnen (die Trajektorie)
pp-life = Life
    .hover = Dynamische Anzeige: nur die letzten Symbole (Points...) bleiben
pp-diff = Diff
    .hover = 0: absolute Winkel; 1-3: Phasendifferenzen (relative Verfahren wie pi/4 DQPSK)
pp-offset-detected = Offset mode detected
pp-angles = Angles
    .hover = Winkelhistogramm im Korrektorbild aufbauen (braucht Synch BR)
pp-vectors = Vectors of dem
    .hover = Positionen des Demodulators zeigen; der Nullvektor ist hellblau
pp-output-dem = Output Dem as
    .hover = Demodulierten Strom als Codes absoluter Winkel ins Textfeld schreiben
pp-trimming = Trimming { $value }
    .hover = Feinabgleich der Phase, Grad
pp-delay = Delay { $value }
    .hover = Echte Verzögerung je Symbol, Sekunden (sehr kurze Signale)
pp-points = Points... { $value }
    .hover = Gleichzeitig gezeigte Punkte mit Life
pp-contrast = Contrast { $value }
    .hover = Helligkeit der Punkte
pp-start = Start
    .hover = Konstellation starten oder anhalten; nach dem Anhalten beginnt sie wieder am Dateianfang
pp-clear = Clear
    .hover = Anzeige löschen
pp-load-file = LoadFile
    .hover = Eine der offenen Dateien in die Phasenebene laden
pp-status-f = F = { $value }
pp-status-fl = Fl = { $value }
pp-status-fu = Fu = { $value }
pp-status-df = dF = { $value }
pp-status-angle-d = Angle_D = { $value }
pp-status-angle-i = Angle_I = { $value }
pp-dem-view-mode = View mode: diff { $diff }
pp-dem-mode = Dem mode: { $mode }
pp-dem-absolute = Data presented as absolute angles!
pp-dem-coding = The coding positions of angles:
pp-dem-data = Data:

pp-isd-n = Harmonische des Signals, mit der der Korrektor arbeitet
    .hover = Harmonische des Signals, mit der der Korrektor arbeitet
pp-isd-b = Ein: der bessere Algorithmus; aus: etwas schlechter und schneller (ohne Wirkung im AM-Modus)
    .hover = Ein: der bessere Algorithmus; aus: etwas schlechter und schneller (ohne Wirkung im AM-Modus)
pp-isd-break = Aktuellen Schritt abbrechen
    .hover = Aktuellen Schritt abbrechen
pp-alpha = Roll-off alpha des RRC-Filters
    .hover = Roll-off alpha des RRC-Filters
pp-quality = Konventioneller Signal-Rausch-Abstand der Konstellation, dB
    .hover = Konventioneller Signal-Rausch-Abstand der Konstellation, dB
pp-load-title = SA - Datei für PhasePlane wählen
pp-ok = Ok
pp-cancel = Abbrechen
pp-check-settings = Bitte Einstellungen prüfen!
pp-run-limited = Nur die ersten { $symbols } Symbole werden demoduliert. Um einen späteren Teil zu analysieren, setzen Sie im Sonogramm die V-Marker darauf, verwenden Sie { sw-hint-copy } und öffnen Sie { sw-hint-phase-plane } auf der Kopie.
pp-copy-panel = Text kopieren
pp-copy-codes = Alle Codes seit Start kopieren
pp-eye = Augendiagramm
    .hover = Augendiagramm (I, zwei Symbole) statt der Konstellation zeigen
pp-job-corrector = Spektrum des ISD-Korrektors
pp-job-isd = ISD-Korrektor
pp-job-spectrum = PhasePlane-Spektrum
# $count symbols, $offset Hz, $rings radii, $iq_offset symbols.
pp-quality-details = { $count ->
        [one] { $count } Symbol
       *[other] { $count } Symbole
    }, Schleifenablage { $offset } Hz, Ringe [{ $rings }], I/Q-Versatz { $iq_offset } Symbole
pp-source-gone = Die geladene Datei ist nicht mehr in der Sitzung; laden Sie mit LoadFile erneut eine Arbeitsdatei.
pp-load-gone = Die gewählte Datei wurde geschlossen; eine andere Arbeitsdatei wählen.
