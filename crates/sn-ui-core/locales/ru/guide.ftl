# The workflow guide: SA's analysis method as a checklist with buttons.

guide-title = Порядок анализа
guide-no-signal = Сначала откройте запись (Файл > Открыть файл...).
# $signal: the active window's signal, which the buttons act on.
guide-acts-on = Кнопки действуют на: { $signal }
# $key: a key name. Shown after a menu path only when that key acts on the guide's signal.
guide-key = , клавиша { $key }

## Steps: the message is the step's title, .text its explanation.

guide-step-record = 1. Правильная запись
    .text = WAV 16-bit PCM (или I/Q) с выключенной АРУ приёмника, без ограничения, без редактирования. Сжатые и 8-битные файлы помечаются при открытии.
guide-step-quality = 2. Проверка качества записи
    .text = Откройте окно WF (правый щелчок по сонограмме > { sw-popup-waveform }). Ищите ограничение, 8-битные ступени, постоянную составляющую, «дыхание» АРУ и пропадания. Включите «Сумма», чтобы увидеть распределение амплитуд.
guide-step-isolate = 3. Выделение сигнала
    .text = На сонограмме поставьте маркеры H вокруг сигнала. Затем примените кнопки «{ sw-hint-band-select }» и «{ sw-hint-shift-base }», а широкий сигнал передискретизируйте кнопкой R на меньшую частоту.
guide-step-rate = 4. Скорость манипуляции
    .text = Примените детектор, охватите его тактовую линию маркерами и выберите «{ sw-popup-measure }» в контекстном меню{ $br }. { sw-hint-am } для PSK/QAM/FSK, { sw-hint-modified-am } для MFSK, или попробуйте «{ sw-popup-adp }».
guide-step-family = 5. Анализ вида модуляции
    .text = FSK/MFSK: { sw-hint-phase-detector }, затем в окне WF { edits-median } на 1/3 символа, Сумма, Найти тоны, Счёт частот. PSK/QAM: { sw-hint-power } для порядка и несущей («{ sw-popup-measure }» в его окне{ $sc }), затем PhasePlane; для искажённых сигналов сначала корректор ISD. OFDM: Найти CT, Получить Br, Каналы, затем Список K для исходной частоты.
guide-step-structure = 6. Поиск структуры
    .text = { sw-hint-acf } для периодов кадров, { sw-hint-ccf } для поиска фрагмента по всему сигналу, затем поставьте маркеры V на один кадр и выберите во всплывающем меню { sw-popup-measure } > { raster-dt-menu-item }, чтобы записать dT (период кадра), и откройте { guide-btn-raster } с этим периодом, чтобы увидеть кадры и синхрослова.

## Step buttons.

guide-btn-waveform = Осциллограмма
guide-btn-sonogram = Сонограмма
guide-btn-am-detector = АМ-детектор
guide-btn-modified-am = Модифицированный АМ
guide-btn-phase-detector = Фазовый детектор
guide-btn-demod = Демодулятор FSK/MFSK
# $n: the power, a number.
guide-btn-power = Возведение в степень { $n }
    .hover = Создаёт новый сигнал в степени { $n }; сигнал PSK-{ $n } тогда даёт линию на { $n }-кратной несущей
guide-btn-phase-plane = Фазовая плоскость
guide-btn-raster = Растр (VMW)
guide-btn-adp = ADP
    .hover = Автоматически оценить скорость манипуляции (ADP) на интервале маркеров V или по всему сигналу
guide-btn-ofdm = OFDM
    .hover = Открыть инструмент OFDM (Найти CT, Получить Br, Список K)
guide-open-view = Открыть этот вид сигнала активного окна
guide-new-signal = Создаёт новый сигнал из сигнала активного окна (открывается в новом окне)
# $kind: Br, Sc or dT (untranslated).
guide-not-measured = { $kind }: ещё не измерено
