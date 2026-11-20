# The Signal menu, the parameter dialog of its operations and the save formats.

edits-save-wav-pcm16 = WAV, 16-bit PCM
edits-save-wav-float = WAV, 32-bit float
edits-save-sigmf = SigMF (сохраняет дробные частоты)
edits-save-raw-f32 = Raw, 32-bit float

# $from: where a measured value came from, e.g. "n4, measure line".
edits-measured-other-axis = { $from } (другая ось, проверьте)

## Notes under the dialog fields: where a pre-filled value came from.

edits-note-cutoff-raised = Срез: Br из { $from } (повышено до минимума 30 Hz; Br = { $br } Bd)
edits-note-cutoff-lowered = Срез: Br из { $from } (понижено ниже половины частоты дискретизации; Br = { $br } Bd)
edits-note-cutoff-br = Срез: Br из { $from }
edits-note-cutoff-guessed = Срез: предположение (fs/8), сначала измерьте Br
# $count: the longest window offered, in samples.
edits-note-window-capped = { $count ->
    [one] Окно: 1/3 символа при Br из { $from } (ограничено { $count } отсчётом; для точной 1/3 символа понизьте частоту дискретизации)
   *[other] Окно: 1/3 символа при Br из { $from } (ограничено { $count } отсчётами; для точной 1/3 символа понизьте частоту дискретизации)
}
edits-note-window-br = Окно: 1/3 символа при Br из { $from }
edits-note-window-default = Окно: по умолчанию 5 отсчётов; измерьте Br, чтобы получить 1/3 символа
edits-note-measured-br = Измеренная Br из { $from }
edits-note-measured-s = Измерено: S = 1/dT маркеров V
edits-note-carrier-sc = Несущая: Sc из { $from }
edits-note-carrier-h = Несущая: центр маркеров H
edits-note-carrier-guessed = Несущая: предположение (fs/4), сначала измерьте Sc
edits-note-br-from = Br: из { $from }
edits-note-br-guessed-8 = Br: предположение (fs/8), сначала измерьте Br
edits-note-br-s = Br: S = 1/dT маркеров V
edits-note-br-guessed-10 = Br: предположение (fs/10), сначала измерьте Br

## Why a menu item is disabled.

edits-need-h = Сначала поставьте маркеры H (перетаскиванием на сонограмме).
edits-need-v = Сначала поставьте маркеры V (перетаскиванием на сонограмме или осциллограмме).
edits-need-vh = Сначала поставьте маркеры V и H (перетаскиванием на сонограмме).
edits-need-complex = Сигнал уже действительный.
edits-need-baud = Сначала измерьте Br (меню «Измерение») или поставьте маркеры V на один символ.

## Signal menu sections and items.

edits-section-spectrum = Спектр (маркеры H)
edits-band-select = Выделить полосу
edits-band-reject = Подавить полосу
edits-area-delete = Удалить область
edits-shift-to-base = Перенести полосу в ноль
edits-shift-to-base-inverted = Перенести полосу в ноль с инверсией
edits-invert-band = Инверсия в полосе
edits-section-whole = Весь сигнал
edits-shift = Сдвиг спектра...
edits-invert = Инверсия спектра
edits-resample = Передискретизация...
edits-iq-to-real = I/Q в действительный
edits-baud-correction = Коррекция скорости манипуляции...
edits-derivative = Первая производная
edits-limiter = Жёсткий ограничитель
edits-section-isd = Корректор ISD
edits-isd-am = Корректор ISD, режим АМ (слепой)...
edits-isd-xn = Корректор ISD, режим X^N...
edits-section-detectors = Детекторы
edits-am-detector = АМ-детектор (такт PSK/QAM/FSK)
edits-modified-am = Модифицированный АМ-детектор (такт MFSK)
edits-half-wave = Двухполупериодный детектор
edits-phase-detector = Фазовый детектор (частота в Hz)
edits-power = Возведение в степень N...
edits-acf = АКФ выделения
edits-ccf = ВКФ: найти выделение по всему сигналу
edits-section-time = Время (маркеры V)
edits-copy = Копировать выделение в новый сигнал
edits-section-filters = Фильтры
edits-lowpass = ФНЧ...
edits-median = Медианный...
edits-rrc = RRC (согласованный фильтр)...

## Диалог параметров (малый модальный диалог SA).

# $operation: the dialog's operation; $signal: the window name, e.g. "test(0)(u1)".
edits-dialog-title = SA - { $operation }: { $signal }
edits-title-shift = Сдвиг спектра
edits-title-resample = Передискретизация
edits-title-baud = Коррекция BR
edits-title-lowpass = ФНЧ
edits-title-median = Медианный фильтр
edits-title-rrc = Фильтр RRC
edits-title-power = Возведение в степень N
edits-title-isd-am = Корректор ISD (режим АМ)
edits-title-isd-xn = Корректор ISD (режим X^N)

edits-field-carrier = Несущая (Hz)
edits-field-br = Br, символьная скорость (Bd)
edits-field-blocks = Блоки
edits-field-power-psk = Степень N (порядок PSK)
edits-field-window = Окно (отсчёты, нечётное)
edits-field-shift = Сдвиг (Hz, + вверх)
edits-field-rate = Новая частота дискретизации (Hz)
edits-field-measured-br = Имеющаяся скорость
edits-field-required-br = Требуемая скорость
edits-field-cutoff = Срез (Hz)
edits-presets = пресеты
edits-field-power = Степень N
edits-field-alpha = Скругление alpha
# $lo, $hi: the limits of a field.
edits-range = От { $lo } до { $hi }
# $max: the sample rate in Hz.
edits-hint-shift = До +/- { $max } Hz, не 0; дробные значения допустимы
# $max: the longest window, in samples.
edits-hint-window = От 1 до { $max }; чётное значение становится следующим нечётным
edits-ok = Ok
edits-cancel = Отмена
edits-field-invalid = { $field } "{ $text }" недопустимо. { $range }.
edits-range-open = Больше { $lo } и меньше { $hi }
