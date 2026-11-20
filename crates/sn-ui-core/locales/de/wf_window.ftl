# The WF (waveform) window: SA's top toolbar, bottom toolbar hints, popup menu and
# result box. SA's control names (LPF, RRC, Sum, Info, VMW, Br, Z+, Z-) and the
# result box text are names: keep them.

## Top toolbar, left to right.

wf-lpf = LPF
    .hover = Tiefpass-Grenzfrequenz in Hz oder Baudrate des RRC-Filters (Wert eingeben oder wählen)
wf-copy-br = <==
    .hover = Gemessene Br in das LPF-Feld übernehmen
wf-rrc = RRC
    .hover = RRC-Filter ein- oder ausschalten (die LPF-Taste wendet es dann an)
wf-rrc-alpha = RRC
    .hover = RRC-Roll-off
wf-freq = 0-0
    .hover = Klick: Frequenz des kurzen Abschnitts zwischen den V-Markern und seine ganzen Perioden
wf-vmw = VMW
    .hover = Zu VMW wechseln (das Signal mit einer Zeilenperiode gefaltet)
wf-sum = Sum
    .hover = Histogramm der Werte zeigen (dann Frequenzen zählen im Kontextmenü)
wf-info = Info
    .hover = Ergebnisse einblenden (t1, t2, dT, S, Br)

## Bottom toolbar hints, left to right.

wf-hint-zoom-in = Hineinzoomen (Zeit)
wf-hint-zoom-out = Herauszoomen (Zeit)
wf-hint-scale-up = Größere vertikale Skala
wf-hint-scale-down = Kleinere vertikale Skala
wf-hint-zoom-plus = Z+: Zwischen die V-Marker zoomen
wf-hint-zoom-minus = Z-: Einen Zoomschritt zurück
wf-hint-lpf = LPF-Filter anwenden (bei aktivem RRC das RRC-Filter)
wf-hint-revert = Zum Original zurück
wf-hint-synth = Umgekehrte FSK/MFSK-Synthese
wf-hint-save = Als 16-Bit-WAV speichern (Rechtsklick: Float-WAV, DataFile oder SigMF)
wf-hint-copy = Abschnitt zwischen den V-Markern in ein neues Fenster kopieren
wf-hint-demod = Abschnitt an den FSK/MFSK-Demodulator senden
wf-hint-fit = Vertikale Skala an das Signal anpassen
wf-hint-acf = Autokorrelation des Abschnitts (neues Fenster)
wf-hint-ccf = Kreuzkorrelation des Abschnitts mit dem ganzen Signal (hier gezeichnet)

## Why a button is disabled.

wf-need-revert = Nichts rückgängig zu machen: kein Filter, keine CCF.
wf-need-derived = Dieses Fenster zeigt die geöffnete Datei, die schon gespeichert ist. Speichern gilt für hier erzeugte Ergebnisse.

## Popup menu.

wf-popup-count-freqs = Frequenzen zählen
wf-popup-more = SigNexora-Werkzeuge
    .hover = Werkzeuge von SigNexora, die das WF-Fenster von SA nicht hatte
wf-popup-operations = Operationen

## The Count freqs result box (SA's text, not translated). $size: a whole number;
## $value: a number with two decimals.

wf-label-grid = Freq grid: { $size }
wf-label-delta = Delta: { $value } Hz
wf-count-needs-h = Setzen Sie zuerst die H-Marker um die Töne.
wf-count-tones-busy = Find tones läuft noch; Count freqs nach dem Ende erneut versuchen.
wf-count-freqs-failed = Kein Frequenzraster gefunden ({ $error }). H-Marker um die Töne erweitern oder herauszoomen, damit mehr Töne sichtbar sind.
wf-copy-br-value = Br = { $br } Bd in das LPF-Feld kopieren
wf-copy-br-missing = Zuerst Br messen (Sonogramm-Menü: Auto define params)
