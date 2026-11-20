# Texts of the Windows 95 front end (sn-gui) that the egui app does not have:
# message box buttons, the appearance schemes, the system menu of a child
# window, credits and notices. An & marks the mnemonic letter (Alt+letter).

## Message box buttons.

gui-button-yes = &Да
gui-button-no = &Нет

## Options > Theme: SigNexora's themes as Windows appearance schemes.

gui-scheme-standard = Стандартная Windows
gui-scheme-high-contrast-black = Контрастная чёрная
gui-scheme-high-contrast-white = Контрастная белая

## The system menu of a child window.

gui-sys-restore = &Восстановить
gui-sys-move = &Переместить
gui-sys-size = &Размер
gui-sys-minimize = &Свернуть
gui-sys-maximize = Р&азвернуть
gui-sys-close = &Закрыть
gui-sys-next = С&ледующее

## Help > About: credits of the Windows 95 front end.

gui-about-credit-gpui = GPUI от Zed Industries (Apache-2.0).
gui-about-credit-w95fa = Шрифт W95FA от Alina Sava (FontsArena), SIL Open Font License 1.1.
gui-about-credit-selawik = Шрифт Selawik от Microsoft, SIL Open Font License 1.1 (когда Segoe UI, Tahoma или Trebuchet MS не установлен).
gui-about-credit-linux-fonts = Шрифты Cantarell и Noto Sans, SIL Open Font License 1.1; шрифт DejaVu Sans, лицензия Bitstream Vera (стили Linux).

## Notices.

# $path: the settings file; $error: why it could not be written.
gui-prefs-not-saved = Не удалось сохранить настройки в { $path }: { $error }. Проверьте, что папка доступна для записи.
# $path: the settings file.
gui-prefs-not-encoded = Не удалось сохранить настройки в { $path }: их не удалось преобразовать в формат файла. Это ошибка программы; сообщите о ней. Прежний файл настроек сохранён.

## The signal generator (OCG) window and its calculators.

gui-gen-preset = &Пресет...
gui-gen-generate = &Сгенерировать
gui-gen-generate-save = Сгенерировать и со&хранить как...
gui-gen-calculators = &Калькуляторы...
gui-gen-calc-title = Калькуляторы OCG
# $text: what was typed in a number field.
gui-gen-not-number = "{ $text }" не является числом
gui-gen-not-whole = "{ $text }" не является целым числом
# $min, $max: the limits of the field.
gui-gen-range = должно быть от { $min } до { $max }

## Help > Use help: the Windows 95 keys.

gui-keys-menu = Выбрать строку меню
gui-keys-menu-letter = Открыть меню или нажать кнопку с этой подчёркнутой буквой
gui-keys-next-window = Вывести на передний план следующее окно (с Shift: предыдущее)
