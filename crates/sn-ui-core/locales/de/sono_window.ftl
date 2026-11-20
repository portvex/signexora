# The sonogram work window: SA's top toolbar captions, the bottom toolbar hints,
# the right-click popup menu and the Correction of BR panel. SA's control names
# (Fl, Fu, ALC, HRes, Info, WaveFile, DataFile) are names: keep them.

## Top toolbar (SizeFFT, Shear, Image and WinF are in app.ftl).

sw-fl = Fl
    .hover = Tiefste angezeigte Frequenz, Hz (blau, solange das Band gezoomt ist)
sw-fu = Fu
    .hover = Höchste angezeigte Frequenz, Hz (blau, solange das Band gezoomt ist)
sw-alc = ALC
sw-hres = HRes
sw-info = Info
    .hover = Ergebnisanzeige einblenden (Fu, Fc, Fl, dF, t1, t2, dT, S, Br, Sc)

## Bottom toolbar hints, left to right.

sw-hint-zoom-in = Band zwischen den H-Markern auf das Fenster dehnen
sw-hint-zoom-back = Einen Zoomschritt zurück
sw-hint-zoom-reset = Zoom zurücksetzen
sw-hint-abort = Abbrechen
sw-hint-play = Abspielen
sw-hint-stop-play = Wiedergabe stoppen
sw-hint-band-select = Band auswählen
sw-hint-area-delete = Bereich löschen
sw-hint-band-reject = Band unterdrücken
sw-hint-am = Quadratur-AM-Detektor
sw-hint-modified-am = Modifizierter AM-Detektor
sw-hint-half-wave = Doppel-Halbwellendetektor
sw-hint-derivative = Erste Ableitung
sw-hint-power = Potenz-N-Modul
sw-hint-phase-plane = PhasePlane
sw-hint-acf = Autokorrelation
sw-hint-ccf = Kreuzkorrelation
sw-hint-ofdm = OFDM-Modul
sw-hint-phase-detector = Phasendetektor
sw-hint-limiter = Hartbegrenzer
sw-hint-iq = I/Q zu reell
sw-hint-shift-base = Band ins Basisband schieben
sw-hint-shift-base-inverted = Band ins Basisband schieben, invertiert
sw-hint-inverse = Band invertieren (das ganze Spektrum, wenn die H-Marker zusammenfallen)
sw-hint-copy = Fragment kopieren
sw-hint-resample = Auf die Rate im Feld neu abtasten
sw-hint-shift = Spektrum um den Wert im Feld verschieben (Hz; negativ nach unten)
sw-value = R / S
    .hover = Neue Rate für R oder Verschiebung für S, in Hz (Komma oder Punkt als Dezimalzeichen)
sw-value-presets = Vorgabewerte

## Why a button is disabled.

sw-need-history = Es gibt keinen Zoomschritt, zu dem man zurückkann.
sw-need-running = Für dieses Fenster läuft nichts.
sw-need-playing = Es wird nichts abgespielt.

## Popup menu.

sw-popup-save-as = Speichern unter
sw-popup-wave-file = WaveFile
sw-popup-data-file = DataFile
sw-popup-wave-file-float = WaveFile (32-Bit-Gleitkomma)
sw-popup-sigmf = SigMF
sw-popup-waveform = Wave-Form-Modul
sw-popup-measure = Frequenzmessung
sw-popup-adp = Parameter automatisch bestimmen
sw-popup-brc = BR-Korrektur
sw-popup-cancel = Abbrechen

## Correction of BR panel.

sw-brc-existing = Vorhandene Baudrate
sw-brc-required = Gewünschte Baudrate
sw-brc-max-delta = Max. Änderung: +/- 30%
sw-brc-yes = Ja

## Messages.

sw-check-settings = Bitte Einstellungen prüfen!
# Result boxes in the plot (SA's own text). $value: a number with two decimals.
sw-label-freq = Freq: { $value } Hz
sw-label-brate = Brate: { $value } Hz
sw-rs-resample-invalid = Der R/S-Wert "{ $text }" ist keine positive Abtastrate; geben Sie eine Rate in Hz ein, etwa 8000.
sw-rs-shift-invalid = Der R/S-Wert "{ $text }" ist keine Zahl; geben Sie eine Verschiebung in Hz ein, etwa -150.
sw-brc-existing-invalid = Existent Baud Rate "{ $text }" ist keine Zahl; geben Sie die gemessene Br in Bd ein oder führen Sie zuerst ADP aus.
sw-brc-required-invalid = Required Baud Rate "{ $text }" ist keine Zahl; geben Sie die gewünschte Br in Bd ein.
sw-brc-not-positive = Beide Baudraten müssen über 0 Bd liegen.
sw-brc-too-far = Die gewünschte Br muss zwischen { $lo } und { $hi } Bd liegen (höchstens 30% von der vorhandenen).
sw-brc-measured = Mit der von ADP gemessenen Br vorbelegt
sw-brc-not-measured = Noch keine Br gemessen: eingeben oder zuerst ADP ausführen
