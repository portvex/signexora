# The sonogram work window: SA's top toolbar captions, the bottom toolbar hints,
# the right-click popup menu and the Correction of BR panel. SA's control names
# (Fl, Fu, ALC, HRes, Info, WaveFile, DataFile) are names: keep them.

## Top toolbar (SizeFFT, Shear, Image and WinF are in app.ftl).

sw-fl = Fl
    .hover = Нижня показана частота, Hz (синя, поки смугу розтягнуто)
sw-fu = Fu
    .hover = Верхня показана частота, Hz (синя, поки смугу розтягнуто)
sw-alc = ALC
sw-hres = HRes
sw-info = Info
    .hover = Показати панель результатів (Fu, Fc, Fl, dF, t1, t2, dT, S, Br, Sc)

## Bottom toolbar hints, left to right.

sw-hint-zoom-in = Розтягнути смугу між H-маркерами на все вікно
sw-hint-zoom-back = Крок назад
sw-hint-zoom-reset = Повернути початковий масштаб
sw-hint-abort = Зупинити процес
sw-hint-play = Відтворити
sw-hint-stop-play = Стоп
sw-hint-band-select = Виділення смуги
sw-hint-area-delete = Видалення області
sw-hint-band-reject = Режекція смуги
sw-hint-am = Квадратурний AM-детектор
sw-hint-modified-am = Модифікований AM-детектор
sw-hint-half-wave = Двонапівперіодний детектор
sw-hint-derivative = Перша похідна
sw-hint-power = Модуль піднесення до степеня
sw-hint-phase-plane = PhasePlane
sw-hint-acf = Автокореляція
sw-hint-ccf = Взаємна кореляція
sw-hint-ofdm = Модуль OFDM
sw-hint-phase-detector = Фазовий детектор
sw-hint-limiter = Жорсткий обмежувач
sw-hint-iq = I/Q у дійсний
sw-hint-shift-base = Прямий перенос до нижньої межі
sw-hint-shift-base-inverted = Перенос з інверсією
sw-hint-inverse = Інверсія спектра в смузі (весь спектр, якщо H-маркери суміщені)
sw-hint-copy = Копіювати фрагмент
sw-hint-resample = Передискретизувати на частоту з поля
sw-hint-shift = Зсунути спектр на значення з поля (Hz; від'ємне вниз)
sw-value = R / S
    .hover = Нова частота для R або зсув для S, у Hz (дробова частина через кому або крапку)
sw-value-presets = Готові значення

## Why a button is disabled.

sw-need-history = Немає кроку масштабу, до якого можна повернутися.
sw-need-running = Для цього вікна нічого не виконується.
sw-need-playing = Нічого не відтворюється.

## Popup menu.

sw-popup-save-as = Зберегти як
sw-popup-wave-file = WaveFile
sw-popup-data-file = DataFile
sw-popup-wave-file-float = WaveFile (32-бітний float)
sw-popup-sigmf = SigMF
sw-popup-waveform = Модуль Wave Form
sw-popup-measure = Вимірювання частоти
sw-popup-adp = Автовизначення параметрів
sw-popup-brc = Корекція BR
sw-popup-cancel = Скасувати

## Correction of BR panel.

sw-brc-existing = Наявна швидкість
sw-brc-required = Потрібна швидкість
sw-brc-max-delta = Макс. зміна: +/- 30%
sw-brc-yes = Так

## Messages.

sw-check-settings = Перевірте налаштування!
# Result boxes in the plot (SA's own text). $value: a number with two decimals.
sw-label-freq = Freq: { $value } Hz
sw-label-brate = Brate: { $value } Hz
sw-rs-resample-invalid = Значення R/S "{ $text }" не є додатною частотою дискретизації; введіть частоту в Hz, наприклад 8000.
sw-rs-shift-invalid = Значення R/S "{ $text }" не є числом; введіть зсув у Hz, наприклад -150.
sw-brc-existing-invalid = Existent Baud Rate "{ $text }" не є числом; введіть виміряну Br у Bd або спершу запустіть ADP.
sw-brc-required-invalid = Required Baud Rate "{ $text }" не є числом; введіть потрібну Br у Bd.
sw-brc-not-positive = Обидві швидкості мають бути більшими за 0 Bd.
sw-brc-too-far = Потрібна Br має бути від { $lo } до { $hi } Bd (не більше 30% від наявної).
sw-brc-measured = Заповнено Br, виміряною ADP
sw-brc-not-measured = Br ще не виміряно: введіть її або спершу запустіть ADP
