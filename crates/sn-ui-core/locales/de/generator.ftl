# File > Signal generator (OCG): the generator window, its preview and the
# OFDM calculators. Keep units (Hz, Bd, s, ms, dB, ppm), modulation names
# (FSK, MSK, GMSK, PSK, QAM, C4FM, OFDM, BPSK, ...), BT, Es/N0, SNR, RC, RRC,
# LU, LG, LS, K, Br, Fd and the model letters A, B, C untranslated.

gen-menu = Signalgenerator (OCG)...
    .hover = Testsignal mit genau bekannten Parametern erzeugen: Töne, Rauschen, FSK, PSK/QAM, C4FM oder OFDM, mit Kanalstörungen
gen-keys-open = Signalgenerator ein- oder ausblenden
gen-title = Signalgenerator (OCG)
gen-job = Erzeuge { $name }

gen-kind = Art
gen-kind-tones = Töne
gen-kind-noise = Rauschen
gen-kind-fsk = FSK/MFSK
gen-kind-linear = PSK/QAM
gen-kind-c4fm = C4FM
gen-kind-ofdm = OFDM
gen-preset = Vorlage
    .hover = Fertiges Signal laden
gen-preset-defaults = Standardwerte
    .hover = Aktuelle Art auf ihre Standardwerte zurücksetzen

gen-section-signal = Signal
gen-section-common = Allgemein
gen-section-channel = Kanalstörungen
gen-section-derived = Abgeleitete Werte
gen-section-preview = Vorschau
gen-section-calc = Rechner

gen-complex = Komplexe Ausgabe (I/Q)
    .hover = I/Q-Samples statt eines reellen Signals erzeugen; Frequenzen sind dann Ablagen von 0 Hz
gen-amplitude-rms = RMS-Amplitude
gen-amplitude-peak = Spitzenamplitude
gen-phase-rad = Phase (rad)
gen-remove = Entfernen
gen-baud = Baudrate
gen-carrier = Träger
gen-center = Mitte
gen-alpha = Roll-off

gen-tones-frequency = Frequenz
gen-tones-add = Ton hinzufügen
gen-tones-last = Mindestens ein Ton ist nötig

gen-fsk-order = Töne
gen-fsk-spacing = Abstand
gen-fsk-shaping = Formung
gen-fsk-shaping-none = Keine
gen-fsk-shaping-gaussian = Gauß
gen-fsk-bt = BT
gen-fsk-msk = MSK
    .hover = Zwei Töne, Abstand gleich halber Baudrate, keine Formung
gen-fsk-gmsk = GMSK
    .hover = MSK mit Gauß-Filter, BT 0,3, wie bei GSM

gen-lin-constellation = Konstellation
gen-lin-shaping = Impulsform
gen-lin-rect = Rechteck
gen-lin-rc = Raised Cosine (RC)
gen-lin-rrc = Root Raised Cosine (RRC)
gen-lin-differential = Differenziell
    .hover = Jedes Symbol als Phasenschritt gegenüber dem vorherigen senden
gen-lin-differential-qam = QAM trägt Amplitude und Phase und kann daher nicht differenziell sein
gen-lin-differential-pi4 = pi/4-DQPSK ist immer differenziell

gen-c4fm-pulse = Impuls
gen-c4fm-shaped = Geformt
    .hover = Aus: Die Frequenz springt zwischen den vier Stufen
gen-c4fm-apco = APCO-25
    .hover = Der TIA-102-Impuls, gedacht für einen Integrate-and-Dump-Empfänger
gen-c4fm-note = Die Hübe sind fest auf +-600 und +-1800 Hz; die C4FM-Ausgabe ist immer reell.

gen-ofdm-lu = LU
    .hover = Nutzlänge (FFT-Größe) in Samples bei der Abtastrate
gen-ofdm-lg = LG
    .hover = Länge des Schutzintervalls (zyklisches Präfix) in Samples
gen-ofdm-channels = Kanäle
gen-ofdm-first-bin = Erster Bin
    .hover = Bin des ersten Kanals; Bin 0 ist die Mitte
gen-ofdm-modulation = Modulation
gen-ofdm-unused = Ungenutzte Positionen
    .hover = Kanalpositionen ohne Inhalt, ab 0 gezählt, zum Beispiel 3, 7, 10-12
gen-ofdm-overrides = Abweichungen
    .hover = Modulation einzelner Positionen, zum Beispiel 0: pilot, 5-6: QAM-16
gen-ofdm-model = Modell
gen-ofdm-model-a = A: Jeder Kanal sendet Absolutwerte
gen-ofdm-model-b = B: Jeder Kanal ist von Symbol zu Symbol differenziell
gen-ofdm-model-c = C: Piloten tragen die Referenz, Daten sind absolut relativ zu ihnen
gen-ofdm-pilots = Piloten
gen-ofdm-pilots-none = Keine
gen-ofdm-pilots-list = Positionen
    .hover = Pilotpositionen, zum Beispiel 0, 8, 16
gen-ofdm-pilots-every = Jede N-te
gen-ofdm-step = Schritt
gen-ofdm-offset = Erste
gen-ofdm-reference = Pilotreferenz
    .hover = Was die Piloten in Modell C tragen
    .disabled = Nur Modell C nutzt eine Pilotreferenz
gen-ofdm-reference-constant = Konstant
gen-ofdm-reference-differential = Zufällige gemeinsame Phase
gen-ofdm-center = Mitte
    .hover = Wo Bin 0 liegt; bei komplexer Ausgabe die Ablage von 0 Hz

gen-label = Bezeichnung
    .hover = Name des neuen Signals; leer verwendet die Beschreibung
gen-sample-rate = Abtastrate
gen-length = Länge
gen-length-seconds = Sekunden
gen-length-symbols = Symbole
    .disabled = Töne und Rauschen haben keine Symbole
gen-seed = Startwert
gen-new-seed = Neuer Startwert
    .hover = Zufälligen Startwert wählen: neue Daten und neues Rauschen
gen-center-frequency = Mittenfrequenz
    .hover = HF-Frequenz, für die 0 Hz der komplexen Ausgabe steht (wird mit dem Signal gespeichert)
    .disabled = Nur komplexe Ausgabe hat eine Mittenfrequenz

gen-ch-offset = Frequenzversatz
gen-ch-drift = Drift
gen-ch-clock = Taktversatz
    .hover = Fehler des Abtasttakts; positiv heißt, der Empfängertakt läuft zu schnell
gen-ch-noise = Rauschen
gen-ch-noise-none = Keines
gen-ch-noise-full = SNR über das ganze Band
gen-ch-noise-inband = SNR im Band
gen-ch-noise-esn0 = Es/N0
gen-ch-bandwidth = Bandbreite
    .hover = Aus: die vom Signal belegte Bandbreite
gen-ch-clip = Begrenzen bei
    .hover = Spitzenamplitude begrenzen; komplexe Samples behalten ihre Phase
gen-ch-multipath = Mehrwege-Taps
    .hover = Die Taps sind der ganze Kanal: den direkten Pfad (Verzögerung 0, Gewinn 1) einschließen
gen-ch-delay = Verzögerung
gen-ch-gain = Gewinn
gen-ch-add-tap = Tap hinzufügen

gen-d-none = Parameter korrigieren, um die abgeleiteten Werte zu sehen.
gen-d-symbol-rate = Symbolrate
gen-d-sps = Samples pro Symbol
gen-d-symbols = Symbole
gen-d-bandwidth = Belegte Bandbreite
gen-d-samples = Samples
gen-d-duration = Dauer
gen-d-index = Modulationsindex h
gen-d-ls = LS
gen-d-k = K = LG/LU
gen-d-spacing = Abstand
gen-d-br = Br

gen-preview = Generatorvorschau
gen-preview-spectrum = Spektrum
gen-preview-wave = Wellenform
gen-preview-name = Vorschau des erzeugten Signals
gen-preview-building = Erstelle Vorschau...
gen-preview-invalid = Keine Vorschau, solange die Parameter ungültig sind
gen-preview-failed = Vorschau fehlgeschlagen: { $error }
gen-preview-short = Zu kurz für ein Spektrum
gen-preview-axis = Mittleres Spektrum, { $lo } bis { $hi } Hz, 80 dB Bereich
gen-preview-wave-axis = Erste { $ms } ms, Spitze { $peak }

gen-generate = Erzeugen
    .hover = Signal als neue Aufnahme zur Sitzung hinzufügen
gen-generate-save = Erzeugen und speichern unter
    .hover = Erzeugen, dann das neue Signal im gewählten Format speichern
gen-busy = Ein Signal wird erzeugt
gen-fix-first = Zuerst die Parameter korrigieren
gen-invalid = Erzeugen nicht möglich: { $error }
gen-field-error = { $field }: { $error }
gen-ready = Bereit: { $samples } Samples, { $duration } s
gen-list-bad = "{ $item }" ist keine Position und kein Bereich wie 10-12
gen-list-too-long = Zu viele Positionen (höchstens { $max })
gen-overrides-bad = "{ $item }" ist keine Position mit Modulation wie 5: QAM-16

gen-calc-cp = OFDM mit CP
gen-calc-cp-lengths = Aus Fd, LU und LG
gen-calc-spacing = Abstand
gen-calc-symbol = Symbol
gen-calc-useful = Nutzteil
gen-calc-guard = Schutzintervall
gen-calc-use = In OFDM übernehmen
    .hover = Diese Längen (und Fd) in den OFDM-Editor kopieren
    .disabled = LU und LG müssen ganze Sample-Zahlen sein
# Why Use in OFDM is disabled. $max: the generator's limit in samples.
gen-calc-lu-not-whole = LU muss eine ganze Zahl von Abtastwerten sein, mindestens 2.
gen-calc-lg-not-whole = LG muss eine ganze Zahl von Abtastwerten sein.
gen-calc-lu-too-large = LU darf höchstens { $max } Abtastwerte betragen.
gen-calc-lg-too-large = LG darf höchstens { $max } Abtastwerte betragen.
gen-calc-use-k = In OFDM übernehmen
    .hover = Dieses K in den OFDM-Editor kopieren: das kleinste passende LU ab dem aktuellen LU
gen-calc-inverse = Aus Br und Abstand bei Fd: nächstes ganzzahliges LU und LG
gen-calc-exact = exakt
gen-calc-error = Fehler
gen-calc-candidates = Ganzzahlige LU und LG für Br und K
gen-calc-lu-range = LU ab
gen-calc-tolerance = Toleranz
gen-calc-found = { $count ->
    [one] { $count } Kandidat
   *[other] { $count } Kandidaten, die ersten { $shown } angezeigt
}
gen-calc-klist = Liste der K
gen-calc-k-from-rates = Aus Br und Abstand
gen-calc-k-direct = Aus K
gen-calc-uncertainty = Unsicherheit
    .hover = Wie weit jede gemessene Rate abweichen kann, in Prozent der Rate. Die Liste zeigt die einfachsten Brüche innerhalb der sich daraus ergebenden K-Unsicherheit; bei direkt eingegebenem K gilt die Toleranz eines K, das aus zwei so ungenauen Raten gemessen wurde.
gen-calc-uncertainty-bad = Die Unsicherheit muss zwischen 0 und { $max } % liegen.
gen-calc-k-none = Kein Bruch liegt innerhalb der Unsicherheit. Erhöhen Sie die Unsicherheit oder den größten Nenner.
gen-calc-max-den = Größter Nenner
gen-calc-count = Anzahl
gen-calc-value = Wert
gen-calc-fd = Fd
    .hover = Abtastrate in Hz, bei der diese Längen für die Br exakt sind: Br x (LU + LG)
gen-calc-k = K
    .hover = K = LG/LU, das Verhältnis von Schutz- zu Nutzlänge, das diese ganzen Längen tatsächlich haben
gen-calc-lg-error = Fehler (Abtastwerte)
    .hover = LG minus K x LU: wie weit die ganze Schutzlänge von der exakten abweicht, in Abtastwerten
gen-calc-k-fraction = K
    .hover = K als gekürzter Bruch p/q
gen-calc-k-error = Fehler
    .hover = Wert des Bruchs minus gemessenes K (mit Vorzeichen; K hat keine Einheit, dieser Wert daher auch nicht)
gen-preview-symbol-too-long = Keine Vorschau: ein Symbol hat { $samples } Samples, mehr als eine Vorschau. Baudrate erhöhen oder Abtastrate senken.
gen-calc-fraction-too-big = K = { $k } passt nicht: LU darf höchstens { $lu } und LG höchstens { $lg } Samples sein. Kleineres LU oder K wählen.
