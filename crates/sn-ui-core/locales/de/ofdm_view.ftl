# OFDM module window, SA-OFDM (SA_UI_SPEC.md section 10).

# Top bar (SA's control names).
ofdm-min-br = Min Br-LS
    .hover = Kleinste gesuchte Symbolrate, Bd (ganzzahlig); das Feld danach ist das längste LS
ofdm-max-br = Max Br-LS
    .hover = Größte gesuchte Symbolrate, Bd (ganzzahlig); das Feld danach ist das kürzeste LS
ofdm-total-ls = Total LS
    .hover = Symbole der größten Länge (Min Br-LS) in der Auswahl
ofdm-current-ls = Current LS
    .hover = Angezeigtes Symbol (0 während Find CT)
ofdm-survey = Survey
    .hover = Übersicht: ein 1024-Punkte-Spektrum, mit dem Balken darunter verschoben, legt den Start von Find CT fest
ofdm-list-k = List of K
    .hover = Magic-K-Rechner: die wahrscheinlichsten K-Werte und nativen Abtastraten
    .disabled = Erst Find CT und ADP ausführen

# Controls (SA's control names).
ofdm-symbols = Symbols
    .hover = Im Korrelationsdreieck gefaltete Symbole (und ADP-Block); 20 bis 40 ist gut, nie unter 6
ofdm-scale-fft = Scale FFT
    .hover = Breite der Kanalbalken des LU-Spektrums
ofdm-shift = Shift Frequency
    .hover = Vor der FFT abgezogene Frequenz, Hz; Eingabe übernimmt
ofdm-vshift = VShift
    .hover = Virtuelle Verschiebung um einen Kanal (exakte Rasterschritte, ohne Grenze)
ofdm-shift-step = Schritt
    .hover = Shift Frequency um { $step } Hz erhöhen oder senken
ofdm-br = Br
    .hover = Symbolrate: Wert von ADP, oder einen eingeben und Eingabe drücken
ofdm-adp = ADP
    .hover = Genaue Symbolrate und Frequenzlage (Get Br in SA)
    .disabled = Erst Find CT ausführen
ofdm-show = Show
    .hover = Die markierten Kanäle Symbol für Symbol mit Symbolsynchronisation zeigen (Break hält an)
ofdm-break = Break
ofdm-find-ct = Find CT
    .hover = Korrelationsdreieck suchen: LS, LG und Symbolbeginn
ofdm-mode = Mode
    .hover = Signalmodell der Konstellationen
ofdm-model-a = A
    .hover = Modell A: jeder Kanal wie erzeugt (absolute Werte)
ofdm-model-b = B
    .hover = Modell B: jeder Kanal ist sein eigener Pilot (relative Werte)
ofdm-model-c = C
    .hover = Modell C: Daten bezogen auf den Piloten am roten Marker
ofdm-each = Each
    .hover = Jedes N-te Symbol zeigen
ofdm-start-plus = Start+
    .hover = Erstes von Show gezeigtes Symbol

# Plot texts (SA's formats).
ofdm-info-br = Br = { $br }
ofdm-info-sh = Sh = { $sh }
ofdm-info-magic = Magic K = { $num }/{ $den }
ofdm-info-k = * { $k } *
ofdm-info-lg = LG(CP) = { $lg }
ofdm-info-lu = LU = { $lu }
ofdm-info-up = CH_Up = { $ch }, Freq = { $freq }
ofdm-info-down = CH_Down = { $ch }, Freq = { $freq }
ofdm-info-sel = CH_Sel = { $count }, dFreqs = { $df }
ofdm-info-vshift = VShift = { $bins }
ofdm-line = LS={ $ls }; LG={ $lg }; Br={ $br }; Sh={ $sh }
ofdm-k-line = SR={ $sr }; LS={ $ls }; LU={ $lu }; LG={ $lg }; K={ $num }/{ $den }
ofdm-finish = Normales Ende, Zeit { $secs } s.
ofdm-stopped = Vom Benutzer angehalten, Zeit { $secs } s.
ofdm-wait = Keine Panik! Bitte warten...
ofdm-wait-ct = Korrelationsdreieck wird gesucht
ofdm-wait-adp = Genaues Br wird berechnet
ofdm-wait-k = "Magic K" wird berechnet
ofdm-wait-left = Restzeit: { $secs } s.

# Plots.
ofdm-survey-plot = Übersichtsspektrum
ofdm-triangle-plot = Korrelationsdreieck
ofdm-triangle = Dreieck
    .hover = Auf die gute (linke) Flanke klicken, um das LU-Fenster zu setzen; an der Spitze ist es das Eltern-LU
ofdm-spectrum-plot = LU-Spektrum
ofdm-spectrum = Spektrum
    .hover = Klick auf einen Kanal verschiebt den näheren Marker, Doppelklick setzt beide
ofdm-pane = Kanal
    .hover = Kanal { $ch }, Symbole: { $count }
ofdm-k = K
    .hover = K = { $k }, LU-Fehler { $err }, ICI { $ici } dB, { $lu }
ofdm-k-lu-even = gerades LU
ofdm-k-lu-odd = ungerades LU

# Popup menus.
ofdm-sort-sr = Nach SR sortieren
ofdm-sort-k = Nach K sortieren
ofdm-sort-am = Nach AM sortieren
ofdm-k-close = Schließen
ofdm-menu-cancel = Abbrechen
ofdm-menu-reset = Ergebnisse löschen
ofdm-menu-around = Grenzen um Br = { $rate }
    .hover = Min Br-LS und Max Br-LS auf das 0,8- und 1,25-fache des gemessenen Br setzen
ofdm-resample = Auf { $rate } Hz neu abtasten
    .hover = Das Signal mit dieser nativen Rate neu abtasten (neues Fenster)

# Jobs and messages.
ofdm-job-ct = OFDM: Find CT
ofdm-job-br = OFDM: ADP
ofdm-job-k = OFDM: List of K
ofdm-job-show = OFDM: Show
ofdm-needs-ct = Erst Find CT ausführen.
ofdm-needs-br = Erst ADP ausführen.
ofdm-baseband-freed = Das Fenster wurde zwischendurch geschlossen: Find CT erneut ausführen.
ofdm-check-settings = Bitte Einstellungen prüfen! Show braucht Symbols von mindestens 6 und 10 Symbole nach Start+.
ofdm-failed = Suche fehlgeschlagen, Zeit { $secs } s.
ofdm-pick-busy = Auf das Ende der laufenden Arbeit warten oder Break drücken, dann eine Zeile wählen.
ofdm-br-invalid = Br "{ $text }" ist nicht verwendbar: eine Zahl über 0 und bis { $max } Bd (fs/LU) eingeben.
ofdm-br-invalid-nogrid = Br "{ $text }" ist nicht verwendbar: eine Zahl über 0 eingeben.
ofdm-plot-empty = Keine Daten: Find CT ausführen.
ofdm-shift-invalid = Shift Frequency "{ $text }" ist keine Zahl; in Hz eingeben, etwa 12.5.
