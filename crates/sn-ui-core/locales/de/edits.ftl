# The Signal menu, the parameter dialog of its operations and the save formats.

edits-save-wav-pcm16 = WAV, 16-Bit PCM
edits-save-wav-float = WAV, 32-Bit Float
edits-save-sigmf = SigMF (behält gebrochene Raten)
edits-save-raw-f32 = Roh, 32-Bit Float

# $from: where a measured value came from, e.g. "n4, measure line".
edits-measured-other-axis = { $from } (andere Achse, prüfen)

## Notes under the dialog fields: where a pre-filled value came from.

edits-note-cutoff-raised = Grenzfrequenz: Br aus { $from } (auf das Minimum von 30 Hz angehoben; Br ist { $br } Bd)
edits-note-cutoff-lowered = Grenzfrequenz: Br aus { $from } (unter die halbe Abtastrate gesenkt; Br ist { $br } Bd)
edits-note-cutoff-br = Grenzfrequenz: Br aus { $from }
edits-note-cutoff-guessed = Grenzfrequenz: geschätzt (fs/8), zuerst Br messen
# $count: the longest window offered, in samples.
edits-note-window-capped = { $count ->
    [one] Fenster: 1/3 Symbol bei Br aus { $from } (begrenzt auf { $count } Sample; für echte 1/3 Symbol herunter abtasten)
   *[other] Fenster: 1/3 Symbol bei Br aus { $from } (begrenzt auf { $count } Samples; für echte 1/3 Symbol herunter abtasten)
}
edits-note-window-br = Fenster: 1/3 Symbol bei Br aus { $from }
edits-note-window-default = Fenster: Standard 5 Samples; Br messen für 1/3 Symbol
edits-note-measured-br = Gemessene Br aus { $from }
edits-note-measured-s = Gemessen: S = 1/dT der V-Marker
edits-note-carrier-sc = Träger: Sc aus { $from }
edits-note-carrier-h = Träger: Mitte der H-Marker
edits-note-carrier-guessed = Träger: geschätzt (fs/4), zuerst Sc messen
edits-note-br-from = Br: aus { $from }
edits-note-br-guessed-8 = Br: geschätzt (fs/8), zuerst Br messen
edits-note-br-s = Br: S = 1/dT der V-Marker
edits-note-br-guessed-10 = Br: geschätzt (fs/10), zuerst Br messen

## Why a menu item is disabled.

edits-need-h = Zuerst die H-Marker setzen (im Sonagramm ziehen).
edits-need-v = Zuerst die V-Marker setzen (im Sonagramm oder in der Wellenform ziehen).
edits-need-vh = Zuerst die V- und H-Marker setzen (im Sonagramm ziehen).
edits-need-complex = Das Signal ist bereits reell.
edits-need-baud = Zuerst Br messen (Menü Messen) oder V-Marker über ein Symbol setzen.

## Signal menu sections and items.

edits-section-spectrum = Spektrum (H-Marker)
edits-band-select = Band auswählen
edits-band-reject = Band unterdrücken
edits-area-delete = Bereich löschen
edits-shift-to-base = Band ins Basisband schieben
edits-shift-to-base-inverted = Band ins Basisband schieben, invertiert
edits-invert-band = Band invertieren
edits-section-whole = Ganzes Signal
edits-shift = Spektrum verschieben...
edits-invert = Spektrum invertieren
edits-resample = Neu abtasten...
edits-iq-to-real = I/Q zu reell
edits-baud-correction = Baudraten-Korrektur...
edits-derivative = Erste Ableitung
edits-limiter = Hartbegrenzer
edits-section-isd = ISD-Korrektor
edits-isd-am = ISD-Korrektor, AM-Modus (blind)...
edits-isd-xn = ISD-Korrektor, X^N-Modus...
edits-section-detectors = Detektoren
edits-am-detector = AM-Detektor (Takt von PSK/QAM/FSK)
edits-modified-am = Modifizierter AM-Detektor (Takt von MFSK)
edits-half-wave = Doppel-Halbwellendetektor
edits-phase-detector = Phasendetektor (Frequenz in Hz)
edits-power = Potenz N...
edits-acf = AKF der Auswahl
edits-ccf = KKF: Auswahl überall suchen
edits-section-time = Zeit (V-Marker)
edits-copy = Auswahl in neues Signal kopieren
edits-section-filters = Filter
edits-lowpass = Tiefpass...
edits-median = Median...
edits-rrc = RRC (Matched-Filter)...

## Parameterdialog (kleiner modaler Dialog von SA).

# $operation: the dialog's operation; $signal: the window name, e.g. "test(0)(u1)".
edits-dialog-title = SA - { $operation }: { $signal }
edits-title-shift = Spektrum verschieben
edits-title-resample = Neu abtasten
edits-title-baud = BR-Korrektur
edits-title-lowpass = Tiefpass
edits-title-median = Medianfilter
edits-title-rrc = RRC-Filter
edits-title-power = Potenz N
edits-title-isd-am = ISD-Korrektor (AM-Modus)
edits-title-isd-xn = ISD-Korrektor (X^N-Modus)

edits-field-carrier = Träger (Hz)
edits-field-br = Br, Symbolrate (Bd)
edits-field-blocks = Blöcke
edits-field-power-psk = Potenz N (PSK-Ordnung)
edits-field-window = Fenster (Samples, ungerade)
edits-field-shift = Verschiebung (Hz, + ist aufwärts)
edits-field-rate = Neue Abtastrate (Hz)
edits-field-measured-br = Vorhandene Baudrate
edits-field-required-br = Gewünschte Baudrate
edits-field-cutoff = Grenzfrequenz (Hz)
edits-presets = Vorgaben
edits-field-power = Potenz N
edits-field-alpha = Roll-off alpha
# $lo, $hi: the limits of a field.
edits-range = Von { $lo } bis { $hi }
# $max: the sample rate in Hz.
edits-hint-shift = Bis +/- { $max } Hz, nicht 0; Nachkommastellen erlaubt
# $max: the longest window, in samples.
edits-hint-window = Von 1 bis { $max }; ein gerader Wert wird zum nächsten ungeraden
edits-ok = Ok
edits-cancel = Abbrechen
edits-field-invalid = { $field } "{ $text }" ist ungültig. { $range }.
edits-range-open = Über { $lo } und unter { $hi }
