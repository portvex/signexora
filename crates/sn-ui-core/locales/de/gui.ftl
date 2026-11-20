# Texts of the Windows 95 front end (sn-gui) that the egui app does not have:
# message box buttons, the appearance schemes, the system menu of a child
# window, credits and notices. An & marks the mnemonic letter (Alt+letter).

## Message box buttons.

gui-button-yes = &Ja
gui-button-no = &Nein

## Options > Theme: SigNexora's themes as Windows appearance schemes.

gui-scheme-standard = Windows-Standard
gui-scheme-high-contrast-black = Kontrast Schwarz
gui-scheme-high-contrast-white = Kontrast Weiß

## The system menu of a child window.

gui-sys-restore = &Wiederherstellen
gui-sys-move = &Verschieben
gui-sys-size = &Größe ändern
gui-sys-minimize = Mi&nimieren
gui-sys-maximize = Ma&ximieren
gui-sys-close = S&chließen
gui-sys-next = Nächs&tes

## Help > About: credits of the Windows 95 front end.

gui-about-credit-gpui = GPUI von Zed Industries (Apache-2.0).
gui-about-credit-w95fa = Schrift W95FA von Alina Sava (FontsArena), SIL Open Font License 1.1.
gui-about-credit-selawik = Schrift Selawik von Microsoft, SIL Open Font License 1.1 (wenn Segoe UI, Tahoma oder Trebuchet MS nicht installiert ist).
gui-about-credit-linux-fonts = Schriften Cantarell und Noto Sans, SIL Open Font License 1.1; Schrift DejaVu Sans, Bitstream-Vera-Lizenz (Linux-Stile).

## Notices.

# $path: the settings file; $error: why it could not be written.
gui-prefs-not-saved = Die Einstellungen konnten nicht in { $path } gespeichert werden: { $error }. Prüfen Sie, ob der Ordner beschreibbar ist.
# $path: the settings file.
gui-prefs-not-encoded = Die Einstellungen konnten nicht in { $path } gespeichert werden: Sie konnten nicht in das Dateiformat umgewandelt werden. Dies ist ein Programmfehler; bitte melden Sie ihn. Die bisherige Einstellungsdatei bleibt erhalten.

## The signal generator (OCG) window and its calculators.

gui-gen-preset = &Vorlage...
gui-gen-generate = &Erzeugen
gui-gen-generate-save = Erzeugen und &speichern unter...
gui-gen-calculators = &Rechner...
gui-gen-calc-title = OCG-Rechner
# $text: what was typed in a number field.
gui-gen-not-number = "{ $text }" ist keine Zahl
gui-gen-not-whole = "{ $text }" ist keine ganze Zahl
# $min, $max: the limits of the field.
gui-gen-range = muss zwischen { $min } und { $max } liegen

## Help > Use help: the Windows 95 keys.

gui-keys-menu = Menüleiste auswählen
gui-keys-menu-letter = Das Menü öffnen oder die Schaltfläche drücken, die diesen unterstrichenen Buchstaben hat
gui-keys-next-window = Das nächste Fenster (mit Umschalt: das vorige) nach vorn holen
