# Fenster des FSK-2- / universellen MFSK-Demodulators (SA_UI_SPEC.md Abschnitt 8).

# Text panel.
demod-heading =
    SA - MFSK dem
    Levels: { $levels }
    Fd: { $fs }
    Br: { $br }
    Data as: { $form }
    Data:
demod-as-default = Standardtabelle
demod-as-raw = Roh
demod-as-bits = Bits, natürlich
demod-as-gray = Bits, Gray
demod-as-c4fm = APCO-25-C4FM-Dibits
demod-polarity-inverted = [Polarität: invertiert, Bits nach der invertierten C4FM-Tabelle]
demod-polarity-normal = [Polarität: normal]
demod-polarity-unknown = [Polarität: unbekannt, Bits nach der normalen C4FM-Tabelle]
demod-truncated = (erste 20000 Zeichen; Kopieren und Speichern unter nehmen alles)
demod-nothing-yet = Noch nichts demoduliert: Br, Pegel und Marker prüfen.

# Jobs.
demod-job-track = Demodulator: Frequenzverlauf
demod-job = FSK/MFSK wird demoduliert

# Top bar.
demod-rrc = RRC
    .hover = Wurzel-Kosinus-Filter vor den Entscheidungen; das Feld ist seine Grenzfrequenz in Hz (Standard: Br)
demod-cutoff = Grenze
    .hover = RRC-Grenzfrequenz in Hz: Wert eingeben oder aus der Liste wählen
demod-alpha = Alpha
    .hover = RRC-Roll-off, 0,05 bis 1 (Eingabe oder Verlassen des Felds übernimmt)
demod-br = Br
    .hover = Schrittgeschwindigkeit; Eingabe übernimmt
demod-br-step = Schritt
    .hover = Br um { $step } erhöhen oder senken
demod-auto = A
    .hover = Automatische Phasen- und Bitsynchronisation
demod-auto-button = A
    .hover = Phase jetzt automatisch suchen und behalten (schaltet den Automatikmodus aus)
demod-exactly = <= Exactly
    .hover = Feine Abtastphase (+-{ $span } % eines Symbols)
demod-roughly = <= Roughly
    .hover = Grobe Abtastphase über ein Symbol
demod-levels = Levels
    .hover = Anzahl der Pegel, 2 bis 256
demod-raw = Raw
    .hover = Pegel als Hex-Bytes ausgeben, getrennt durch x

# Bottom bar.
demod-back-fast = Zurück 10
    .hover = 10 Seiten zurück
demod-back = Zurück
    .hover = Vorige Seite
demod-forward = Vor
    .hover = Nächste Seite
demod-forward-fast = Vor 10
    .hover = 10 Seiten vor
demod-page = Seite
    .hover = Angezeigte Seite (von { $count }); Eingabe springt dorthin
demod-scale-up = Vergrößern
    .hover = Vertikalen Maßstab vergrößern
demod-scale-down = Verkleinern
    .hover = Vertikalen Maßstab verkleinern
demod-progress = Fortschritt
    .hover = Fortschritt der Berechnung (Rechtsklick hält an)
demod-stop = Anhalten
demod-filter = Filter
    .hover = RRC-Filter anwenden
demod-revert = Zurücksetzen
    .hover = Zurück zum ungefilterten Signal
demod-d = D
    .hover = Bitstrom im Textfeld ausgeben

# Plot.
demod-plot-name = Demodulatorpegel
demod-info = Br = { $baud } | Takt { $clock } % | Qualität { $quality } | Auge { $eye } | Symbole: { $count }
demod-menu-tones = Pegel auf den gefundenen Tönen ({ $count })
    .hover = Die mit Sum in WF gefundenen Töne statt gleichmäßiger Zonen als Pegel verwenden
    .disabled = Erst die Töne suchen: WF, Sum, Töne finden
demod-menu-average = Mittelungsfilter
    .hover = Über ein halbes Symbol integrieren, wenn RRC aus ist (rechteckige FSK)
demod-menu-adaptive = Adaptive Pegel
    .hover = Pegelmitten einer langsamen Drift der Daten folgen lassen
demod-menu-info = Info
demod-menu-reset-view = Vertikalen Maßstab zurücksetzen
demod-menu-cancel = Abbrechen

# Text panel popup.
demod-menu-copy = Kopieren
demod-menu-save = Speichern unter...
demod-menu-data-as = Daten als
demod-save-title = Demodulierte Daten speichern
demod-save-filter = Text
demod-save-failed = { $path } konnte nicht geschrieben werden: { $error }
demod-cutoff-invalid = Die RRC-Grenze "{ $text }" ist keine positive Zahl; geben Sie sie in Hz ein, etwa die Br.
demod-waiting-cutoff = Warte auf eine gültige RRC-Grenzfrequenz: das gezeigte Ergebnis stammt von den vorigen Einstellungen.
demod-br-invalid = Die Br "{ $text }" ist keine positive Zahl; die verwendete Br bleibt.
