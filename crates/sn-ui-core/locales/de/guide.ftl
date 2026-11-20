# The workflow guide: SA's analysis method as a checklist with buttons.

guide-title = Analyse-Leitfaden
guide-no-signal = Öffnen Sie zuerst eine Aufnahme (Datei > Datei öffnen...).
# $signal: the active window's signal, which the buttons act on.
guide-acts-on = Schaltflächen wirken auf: { $signal }
# $key: a key name. Shown after a menu path only when that key acts on the guide's signal.
guide-key = , Taste { $key }

## Steps: the message is the step's title, .text its explanation.

guide-step-record = 1. Sauber aufnehmen
    .text = 16-Bit PCM WAV (oder I/Q) mit abgeschalteter AGC des Empfängers, ohne Übersteuerung, ohne Bearbeitung. Komprimierte oder 8-Bit-Dateien werden beim Öffnen gemeldet.
guide-step-quality = 2. Aufnahmequalität prüfen
    .text = Öffnen Sie das WF-Fenster (Rechtsklick ins Sonagramm > { sw-popup-waveform }). Achten Sie auf Übersteuerung, 8-Bit-Stufen, Gleichanteil, AGC-Pumpen und Aussetzer. Schalten Sie Summe ein, um die Amplitudenverteilung zu sehen.
guide-step-isolate = 3. Signal isolieren
    .text = Setzen Sie im Sonagramm die H-Marker um das Signal. Nutzen Sie dann { sw-hint-band-select } und { sw-hint-shift-base } der Leiste und R, um ein breites Signal auf eine niedrigere Rate abzutasten.
guide-step-rate = 4. Schrittgeschwindigkeit finden
    .text = Wenden Sie einen Detektor an, rahmen Sie seine Taktlinie mit den Markern ein und wählen Sie { sw-popup-measure } im Kontextmenü{ $br }. { sw-hint-am } für PSK/QAM/FSK, { sw-hint-modified-am } für MFSK, oder versuchen Sie { sw-popup-adp }.
guide-step-family = 5. Modulationsfamilie untersuchen
    .text = FSK/MFSK: { sw-hint-phase-detector }, dann im WF-Fenster { edits-median } über 1/3 Symbol, Summe, Töne finden, Frequenzen zählen. PSK/QAM: { sw-hint-power } für Ordnung und Träger ({ sw-popup-measure } in seinem Fenster{ $sc }), dann PhasePlane; bei verzerrten Signalen zuerst den ISD-Korrektor. OFDM: CT finden, Br ermitteln, Kanäle, dann die K-Liste für die native Rate.
guide-step-structure = 6. Struktur finden
    .text = { sw-hint-acf } für Rahmenperioden, { sw-hint-ccf }, um ein Fragment überall zu finden, dann die V-Marker auf einen Rahmen setzen und im Kontextmenü { sw-popup-measure } > { raster-dt-menu-item } wählen, um dT (die Rahmenperiode) zu erfassen, und { guide-btn-raster } mit dieser Periode öffnen, um Rahmen und Synchronwörter zu sehen.

## Step buttons.

guide-btn-waveform = Wellenform
guide-btn-sonogram = Sonagramm
guide-btn-am-detector = AM-Detektor
guide-btn-modified-am = Modifizierter AM
guide-btn-phase-detector = Phasendetektor
guide-btn-demod = FSK/MFSK-Demodulator
# $n: the power, a number.
guide-btn-power = Potenz { $n }
    .hover = Erzeugt ein neues Signal hoch { $n }; ein PSK-{ $n }-Signal zeigt dann eine Linie beim { $n }-fachen Träger
guide-btn-phase-plane = PhasePlane
guide-btn-raster = Raster (VMW)
guide-btn-adp = ADP
    .hover = Taktrate automatisch schätzen (ADP), im V-Marker-Bereich oder im ganzen Signal
guide-btn-ofdm = OFDM
    .hover = OFDM-Werkzeug öffnen (Find CT, Get Br, Liste der K)
guide-open-view = Diese Ansicht des Signals im aktiven Fenster öffnen
guide-new-signal = Erzeugt ein neues Signal aus dem des aktiven Fensters (öffnet sich in einem neuen Fenster)
# $kind: Br, Sc or dT (untranslated).
guide-not-measured = { $kind }: noch nicht gemessen
