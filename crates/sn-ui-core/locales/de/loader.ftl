# SA's file loader, the "SA Open file" window, and its "SA - select channel"
# question. Keep the window titles, the SA control captions (Use options, Data
# start, Channels, Bits, Freq rate, Wave tag, Bits per sample, Data size, Ext.
# Info) and the list entries (All - I/Q, Ch - 1, 16, 24-A, ...) untranslated.

loader-title = SA Open file
loader-ok = Ok
loader-cancel = Abbrechen

## Toolbar buttons.

loader-back = Zurück
    .hover = Zurück zum vorigen Ordner
loader-up = Nach oben
    .hover = Einen Ordner nach oben (Rücktaste)
loader-view = Ansicht
    .hover = Ansicht wechseln: Liste oder Details
loader-refresh = Aktualisieren
    .hover = Liste aktualisieren

## The list.

loader-col-name = Name
loader-col-type = Typ/Ext
loader-col-size = Größe
loader-col-date = Datum
loader-col-time = Zeit
loader-type-folder = Ordner
loader-type-link = F-Link
loader-popup-update = Aktualisieren
loader-popup-auto-preview = Automatische Vorschau
loader-popup-copy = Kopieren
loader-popup-paste = Einfügen
loader-popup-cancel = Abbrechen
loader-search = Schnellsuche
    .hover = Einen Teil eines Namens eingeben, um die erste passende Datei auszuwählen
loader-filter-wav-mp3 = wav & mp3
loader-filter-wav = wav
loader-filter-mp3 = mp3
loader-filter-supported = alle unterstützten
loader-filter-all = *.*
# $path: what was typed or pasted.
loader-path-not-found = Pfad nicht gefunden: { $path }

## The info panel and the options.

loader-folder-link = Ordnerverknüpfung
# $value: a number from the file header.
loader-info-tag = Wave tag = { $value }
loader-info-channels = Channels = { $value }
loader-info-rate = Freq rate = { $value }
loader-info-bits = Bits per sample = { $value }
loader-info-size = Data size = { $value }
loader-info-no-header = Kein Header
loader-info-unreadable = Unbekanntes Format
# $codec: the codec name of the file, e.g. "Intel DVI ADPCM (IMA ADPCM)".
loader-ext-info = Ext. Info: { $codec }
loader-use-options = Use options
    .hover = Datei mit diesen Einstellungen statt ihres Headers lesen (nötig bei Dateien ohne Header)
loader-use-options-compressed = Komprimierte Dateien werden anhand ihres Headers dekodiert; Rohoptionen würden Rauschen lesen.
loader-use-options-headerless = Daten ohne Header können nur mit diesen Optionen gelesen werden.
loader-data-start = Data start
    .hover = Bytes, die vor den Samples übersprungen werden
loader-channels = Channels
    .hover = Der zu ladende Kanal, oder All für die ersten zwei als I/Q
loader-bits = Bits
    .hover = Sampleformat und Bytereihenfolge
loader-freq-rate = Freq rate
    .hover = Abtastrate, Hz
loader-in-file = Kanäle in Datei
    .hover = Verschachtelte Kanäle dieser Datei ohne Header
# $error: why the file cannot be read.
loader-cannot-read = Diese Datei kann nicht gelesen werden: { $error }
loader-check-settings = Bitte Einstellungen prüfen!

## The preview.

loader-preview-building = Vorschau wird erstellt...
loader-prescaler = Vorverstärkung
    .hover = Vorverstärkung: { $gain } dB. Samples über Vollaussteuerung (rote Linien) werden begrenzt.

## SA - select channel.

loader-select-channel-title = SA - select channel
loader-select-channel-group = Diese Datei ist Stereo, bitte wählen
loader-channel-1 = Kanal - 1
loader-channel-2 = Kanal - 2
loader-all-samples = Alle Samples
loader-break = Abbruch
loader-needs-options = Diese Datei hat keinen lesbaren Header: Use options ankreuzen und das Format einstellen.
loader-no-file = Zuerst eine Datei auswählen.
loader-rate-invalid = Freq rate "{ $text }" ist ungültig: eine Rate über 0 und bis 1000000000 Hz eingeben, etwa 48000.
loader-data-start-invalid = Data start "{ $text }" ist ungültig: eine ganze Zahl von Bytes eingeben, etwa 0 oder 44.
loader-list-error = Ordner { $path } kann nicht gelesen werden: { $error }. Berechtigungen prüfen oder einen anderen Ordner wählen.
