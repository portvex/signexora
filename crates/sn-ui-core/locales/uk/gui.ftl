# Texts of the Windows 95 front end (sn-gui) that the egui app does not have:
# message box buttons, the appearance schemes, the system menu of a child
# window, credits and notices. An & marks the mnemonic letter (Alt+letter).

## Message box buttons.

gui-button-yes = &Так
gui-button-no = &Ні

## Options > Theme: SigNexora's themes as Windows appearance schemes.

gui-scheme-standard = Стандартна Windows
gui-scheme-high-contrast-black = Висококонтрастна чорна
gui-scheme-high-contrast-white = Висококонтрастна біла

## The system menu of a child window.

gui-sys-restore = &Відновити
gui-sys-move = &Перемістити
gui-sys-size = &Розмір
gui-sys-minimize = &Згорнути
gui-sys-maximize = Р&озгорнути
gui-sys-close = За&крити
gui-sys-next = &Наступне

## Help > About: credits of the Windows 95 front end.

gui-about-credit-gpui = GPUI від Zed Industries (Apache-2.0).
gui-about-credit-w95fa = Шрифт W95FA від Alina Sava (FontsArena), SIL Open Font License 1.1.
gui-about-credit-selawik = Шрифт Selawik від Microsoft, SIL Open Font License 1.1 (коли Segoe UI, Tahoma або Trebuchet MS не встановлено).
gui-about-credit-linux-fonts = Шрифти Cantarell і Noto Sans, SIL Open Font License 1.1; шрифт DejaVu Sans, ліцензія Bitstream Vera (стилі Linux).

## Notices.

# $path: the settings file; $error: why it could not be written.
gui-prefs-not-saved = Не вдалося зберегти налаштування в { $path }: { $error }. Перевірте, що папка доступна для запису.
# $path: the settings file.
gui-prefs-not-encoded = Не вдалося зберегти налаштування в { $path }: їх не вдалося перетворити у формат файлу. Це помилка програми; повідомте про неї. Попередній файл налаштувань збережено.

## The signal generator (OCG) window and its calculators.

gui-gen-preset = &Пресет...
gui-gen-generate = &Згенерувати
gui-gen-generate-save = Згенерувати і з&берегти як...
gui-gen-calculators = &Калькулятори...
gui-gen-calc-title = Калькулятори OCG
# $text: what was typed in a number field.
gui-gen-not-number = "{ $text }" не є числом
gui-gen-not-whole = "{ $text }" не є цілим числом
# $min, $max: the limits of the field.
gui-gen-range = має бути від { $min } до { $max }

## Help > Use help: the Windows 95 keys.

gui-keys-menu = Вибрати рядок меню
gui-keys-menu-letter = Відкрити меню або натиснути кнопку з цією підкресленою літерою
gui-keys-next-window = Вивести на передній план наступне вікно (з Shift: попереднє)
