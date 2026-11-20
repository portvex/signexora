# Texts of the Windows 95 front end (sn-gui) that the egui app does not have:
# message box buttons, the appearance schemes, the system menu of a child
# window, credits and notices. An & marks the mnemonic letter (Alt+letter).

## Message box buttons.

gui-button-yes = &Yes
gui-button-no = &No

## Options > Theme: SigNexora's themes as Windows appearance schemes.

gui-scheme-standard = Windows Standard
gui-scheme-high-contrast-black = High Contrast Black
gui-scheme-high-contrast-white = High Contrast White

## The system menu of a child window.

gui-sys-restore = &Restore
gui-sys-move = &Move
gui-sys-size = &Size
gui-sys-minimize = Mi&nimize
gui-sys-maximize = Ma&ximize
gui-sys-close = &Close
gui-sys-next = Nex&t

## Help > About: credits of the Windows 95 front end.

gui-about-credit-gpui = GPUI by Zed Industries (Apache-2.0).
gui-about-credit-w95fa = W95FA font by Alina Sava (FontsArena), SIL Open Font License 1.1.
gui-about-credit-selawik = Selawik font by Microsoft, SIL Open Font License 1.1 (when Segoe UI, Tahoma or Trebuchet MS is not installed).
gui-about-credit-linux-fonts = Cantarell and Noto Sans fonts, SIL Open Font License 1.1; DejaVu Sans font, Bitstream Vera licence (the Linux looks).

## Notices.

# $path: the settings file; $error: why it could not be written.
gui-prefs-not-saved = The settings could not be saved to { $path }: { $error }. Check that the folder is writable.
# $path: the settings file.
gui-prefs-not-encoded = The settings could not be saved to { $path }: they could not be converted to the file format. This is a program error; please report it. The previous settings file is kept.

## The signal generator (OCG) window and its calculators.

gui-gen-preset = &Preset...
gui-gen-generate = &Generate
gui-gen-generate-save = Generate and &save as...
gui-gen-calculators = &Calculators...
gui-gen-calc-title = OCG calculators
# $text: what was typed in a number field.
gui-gen-not-number = "{ $text }" is not a number
gui-gen-not-whole = "{ $text }" is not a whole number
# $min, $max: the limits of the field.
gui-gen-range = must be between { $min } and { $max }

## Help > Use help: the Windows 95 keys.

gui-keys-menu = Select the menu bar
gui-keys-menu-letter = Open the menu, or press the button, with that underlined letter
gui-keys-next-window = Bring the next window (with Shift: the previous one) to the front
