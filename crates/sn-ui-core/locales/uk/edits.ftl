# The Signal menu, the parameter dialog of its operations and the save formats.

edits-save-wav-pcm16 = WAV, 16-біт PCM
edits-save-wav-float = WAV, 32-біт float
edits-save-sigmf = SigMF (зберігає дробові частоти)
edits-save-raw-f32 = Raw, 32-біт float

# $from: where a measured value came from, e.g. "n4, measure line".
edits-measured-other-axis = { $from } (інша вісь, перевірте)

## Notes under the dialog fields: where a pre-filled value came from.

edits-note-cutoff-raised = Зріз: Br з { $from } (піднято до мінімуму 30 Hz; Br = { $br } Bd)
edits-note-cutoff-lowered = Зріз: Br з { $from } (знижено нижче половини частоти дискретизації; Br = { $br } Bd)
edits-note-cutoff-br = Зріз: Br з { $from }
edits-note-cutoff-guessed = Зріз: приблизно (fs/8), спершу виміряйте Br
# $count: the longest window offered, in samples.
edits-note-window-capped = { $count ->
    [one] Вікно: 1/3 символу при Br з { $from } (обмежено { $count } відліком; для точної 1/3 символу знизьте частоту дискретизації)
    [few] Вікно: 1/3 символу при Br з { $from } (обмежено { $count } відліками; для точної 1/3 символу знизьте частоту дискретизації)
    [many] Вікно: 1/3 символу при Br з { $from } (обмежено { $count } відліками; для точної 1/3 символу знизьте частоту дискретизації)
   *[other] Вікно: 1/3 символу при Br з { $from } (обмежено { $count } відліку; для точної 1/3 символу знизьте частоту дискретизації)
}
edits-note-window-br = Вікно: 1/3 символу при Br з { $from }
edits-note-window-default = Вікно: типово 5 відліків; виміряйте Br, щоб отримати 1/3 символу
edits-note-measured-br = Виміряна Br з { $from }
edits-note-measured-s = Виміряно: S = 1/dT маркерів V
edits-note-carrier-sc = Несуча: Sc з { $from }
edits-note-carrier-h = Несуча: центр між маркерами H
edits-note-carrier-guessed = Несуча: приблизно (fs/4), спершу виміряйте Sc
edits-note-br-from = Br: з { $from }
edits-note-br-guessed-8 = Br: приблизно (fs/8), спершу виміряйте Br
edits-note-br-s = Br: S = 1/dT маркерів V
edits-note-br-guessed-10 = Br: приблизно (fs/10), спершу виміряйте Br

## Why a menu item is disabled.

edits-need-h = Спершу встановіть маркери H (перетягуванням на сонограмі).
edits-need-v = Спершу встановіть маркери V (перетягуванням на сонограмі або осцилограмі).
edits-need-vh = Спершу встановіть маркери V і H (перетягуванням на сонограмі).
edits-need-complex = Сигнал уже дійсний.
edits-need-baud = Спершу виміряйте Br (меню «Вимірювання») або встановіть маркери V на один символ.

## Signal menu sections and items.

edits-section-spectrum = Спектр (маркери H)
edits-band-select = Виділити смугу
edits-band-reject = Режекція смуги
edits-area-delete = Видалити область
edits-shift-to-base = Зсунути смугу до нуля
edits-shift-to-base-inverted = Зсунути смугу до нуля з інверсією
edits-invert-band = Інверсія в смузі
edits-section-whole = Увесь сигнал
edits-shift = Зсув спектра...
edits-invert = Інверсія спектра
edits-resample = Передискретизувати...
edits-iq-to-real = I/Q у дійсний
edits-baud-correction = Корекція символьної швидкості...
edits-derivative = Перша похідна
edits-limiter = Жорсткий обмежувач
edits-section-isd = Коректор ISD
edits-isd-am = Коректор ISD, режим AM (сліпий)...
edits-isd-xn = Коректор ISD, режим X^N...
edits-section-detectors = Детектори
edits-am-detector = AM-детектор (такт PSK/QAM/FSK)
edits-modified-am = Модифікований AM-детектор (такт MFSK)
edits-half-wave = Двонапівперіодний детектор
edits-phase-detector = Фазовий детектор (частота в Hz)
edits-power = Піднести до степеня N...
edits-acf = ACF виділення
edits-ccf = CCF: знайти виділення по всьому сигналу
edits-section-time = Час (маркери V)
edits-copy = Копіювати виділення в новий сигнал
edits-section-filters = Фільтри
edits-lowpass = ФНЧ...
edits-median = Медіанний...
edits-rrc = RRC (узгоджений фільтр)...

## Діалог параметрів (малий модальний діалог SA).

# $operation: the dialog's operation; $signal: the window name, e.g. "test(0)(u1)".
edits-dialog-title = SA - { $operation }: { $signal }
edits-title-shift = Зсув спектра
edits-title-resample = Передискретизація
edits-title-baud = Корекція BR
edits-title-lowpass = ФНЧ
edits-title-median = Медіанний фільтр
edits-title-rrc = Фільтр RRC
edits-title-power = Піднесення до степеня N
edits-title-isd-am = Коректор ISD (режим AM)
edits-title-isd-xn = Коректор ISD (режим X^N)

edits-field-carrier = Несуча (Hz)
edits-field-br = Br, символьна швидкість (Bd)
edits-field-blocks = Блоки
edits-field-power-psk = Степінь N (порядок PSK)
edits-field-window = Вікно (відліки, непарне)
edits-field-shift = Зсув (Hz, + вгору)
edits-field-rate = Нова частота дискретизації (Hz)
edits-field-measured-br = Наявна швидкість
edits-field-required-br = Потрібна швидкість
edits-field-cutoff = Частота зрізу (Hz)
edits-presets = набори
edits-field-power = Степінь N
edits-field-alpha = Коефіцієнт згладжування alpha
# $lo, $hi: the limits of a field.
edits-range = Від { $lo } до { $hi }
# $max: the sample rate in Hz.
edits-hint-shift = До +/- { $max } Hz, не 0; дробові значення дозволені
# $max: the longest window, in samples.
edits-hint-window = Від 1 до { $max }; парне значення стає наступним непарним
edits-ok = Ok
edits-cancel = Скасувати
edits-field-invalid = { $field } "{ $text }" недійсне. { $range }.
edits-range-open = Більше { $lo } і менше { $hi }
