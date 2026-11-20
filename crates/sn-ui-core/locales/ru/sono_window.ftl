# The sonogram work window: SA's top toolbar captions, the bottom toolbar hints,
# the right-click popup menu and the Correction of BR panel. SA's control names
# (Fl, Fu, ALC, HRes, Info, WaveFile, DataFile) are names: keep them.

## Top toolbar (SizeFFT, Shear, Image and WinF are in app.ftl).

sw-fl = Fl
    .hover = Нижняя показанная частота, Hz (синяя, пока полоса растянута)
sw-fu = Fu
    .hover = Верхняя показанная частота, Hz (синяя, пока полоса растянута)
sw-alc = ALC
sw-hres = HRes
sw-info = Info
    .hover = Показать панель результатов (Fu, Fc, Fl, dF, t1, t2, dT, S, Br, Sc)

## Bottom toolbar hints, left to right.

sw-hint-zoom-in = Растянуть полосу между H-маркерами на всё окно
sw-hint-zoom-back = Шаг назад
sw-hint-zoom-reset = Полный откат в исходное состояние
sw-hint-abort = Остановить процесс
sw-hint-play = Воспроизвести
sw-hint-stop-play = Стоп
sw-hint-band-select = Выделение полосы
sw-hint-area-delete = Удаление области
sw-hint-band-reject = Режекция полосы
sw-hint-am = Квадратурный АМ детектор
sw-hint-modified-am = Модифицированный АМ детектор
sw-hint-half-wave = Двухполупериодный детектор
sw-hint-derivative = Первая производная
sw-hint-power = Модуль возведения в степень
sw-hint-phase-plane = PhasePlane
sw-hint-acf = АКФ
sw-hint-ccf = ВКФ
sw-hint-ofdm = Модуль OFDM
sw-hint-phase-detector = Фазовый детектор
sw-hint-limiter = Жёсткий ограничитель
sw-hint-iq = I/Q в действительный
sw-hint-shift-base = Прямой перенос к нижней границе
sw-hint-shift-base-inverted = Перенос с инверсией
sw-hint-inverse = Инверсия спектра в полосе (весь спектр, если H-маркеры совмещены)
sw-hint-copy = Копировать фрагмент
sw-hint-resample = Передискретизация на частоту из поля
sw-hint-shift = Сдвиг спектра на значение из поля (Hz; отрицательное вниз)
sw-value = R / S
    .hover = Новая частота для R или сдвиг для S, в Hz (дробная часть через запятую или точку)
sw-value-presets = Готовые значения

## Why a button is disabled.

sw-need-history = Нет шага масштаба, к которому можно вернуться.
sw-need-running = Для этого окна ничего не выполняется.
sw-need-playing = Ничего не воспроизводится.

## Popup menu.

sw-popup-save-as = Сохранить как
sw-popup-wave-file = WaveFile
sw-popup-data-file = DataFile
sw-popup-wave-file-float = WaveFile (32-битный float)
sw-popup-sigmf = SigMF
sw-popup-waveform = Модуль Wave Form
sw-popup-measure = Измерение частоты
sw-popup-adp = Автоопределение параметров
sw-popup-brc = Коррекция BR
sw-popup-cancel = Отмена

## Correction of BR panel.

sw-brc-existing = Имеющаяся скорость
sw-brc-required = Требуемая скорость
sw-brc-max-delta = Макс. изменение: +/- 30%
sw-brc-yes = Да

## Messages.

sw-check-settings = Проверьте настройки!
# Result boxes in the plot (SA's own text). $value: a number with two decimals.
sw-label-freq = Freq: { $value } Hz
sw-label-brate = Brate: { $value } Hz
sw-rs-resample-invalid = Значение R/S "{ $text }" не является положительной частотой дискретизации; введите частоту в Hz, например 8000.
sw-rs-shift-invalid = Значение R/S "{ $text }" не является числом; введите сдвиг в Hz, например -150.
sw-brc-existing-invalid = Existent Baud Rate "{ $text }" не является числом; введите измеренную Br в Bd или сначала запустите ADP.
sw-brc-required-invalid = Required Baud Rate "{ $text }" не является числом; введите нужную Br в Bd.
sw-brc-not-positive = Обе скорости должны быть больше 0 Bd.
sw-brc-too-far = Требуемая Br должна быть от { $lo } до { $hi } Bd (не более 30% от имеющейся).
sw-brc-measured = Заполнено Br, измеренной ADP
sw-brc-not-measured = Br еще не измерена: введите ее или сначала запустите ADP
