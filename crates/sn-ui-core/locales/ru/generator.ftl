# File > Signal generator (OCG): the generator window, its preview and the
# OFDM calculators. Keep units (Hz, Bd, s, ms, dB, ppm), modulation names
# (FSK, MSK, GMSK, PSK, QAM, C4FM, OFDM, BPSK, ...), BT, Es/N0, SNR, RC, RRC,
# LU, LG, LS, K, Br, Fd and the model letters A, B, C untranslated.

gen-menu = Генератор сигналов (OCG)...
    .hover = Создать тестовый сигнал с точно известными параметрами: тоны, шум, FSK, PSK/QAM, C4FM или OFDM, с искажениями канала
gen-keys-open = Показать или скрыть генератор сигналов
gen-title = Генератор сигналов (OCG)
gen-job = Генерация { $name }

gen-kind = Тип
gen-kind-tones = Тоны
gen-kind-noise = Шум
gen-kind-fsk = FSK/MFSK
gen-kind-linear = PSK/QAM
gen-kind-c4fm = C4FM
gen-kind-ofdm = OFDM
gen-preset = Пресет
    .hover = Загрузить готовый сигнал
gen-preset-defaults = По умолчанию
    .hover = Вернуть текущему типу значения по умолчанию

gen-section-signal = Сигнал
gen-section-common = Общее
gen-section-channel = Искажения канала
gen-section-derived = Производные величины
gen-section-preview = Предпросмотр
gen-section-calc = Калькуляторы

gen-complex = Комплексный выход (I/Q)
    .hover = Создавать отсчёты I/Q вместо вещественного сигнала; частоты тогда задаются как сдвиги от 0 Hz
gen-amplitude-rms = Амплитуда RMS
gen-amplitude-peak = Пиковая амплитуда
gen-phase-rad = Фаза (рад)
gen-remove = Удалить
gen-baud = Скорость модуляции
gen-carrier = Несущая
gen-center = Центр
gen-alpha = Коэф. скругления

gen-tones-frequency = Частота
gen-tones-add = Добавить тон
gen-tones-last = Нужен хотя бы один тон

gen-fsk-order = Тоны
gen-fsk-spacing = Разнос
gen-fsk-shaping = Формирование
gen-fsk-shaping-none = Нет
gen-fsk-shaping-gaussian = Гауссово
gen-fsk-bt = BT
gen-fsk-msk = MSK
    .hover = Два тона, разнос равен половине скорости модуляции, без формирования
gen-fsk-gmsk = GMSK
    .hover = MSK с гауссовым фильтром BT 0.3, как в GSM

gen-lin-constellation = Созвездие
gen-lin-shaping = Форма импульса
gen-lin-rect = Прямоугольная
gen-lin-rc = Приподнятый косинус (RC)
gen-lin-rrc = Корень из приподнятого косинуса (RRC)
gen-lin-differential = Дифференциальная
    .hover = Передавать каждый символ как шаг фазы от предыдущего
gen-lin-differential-qam = QAM несёт и амплитуду, и фазу, поэтому не может быть дифференциальной
gen-lin-differential-pi4 = pi/4-DQPSK всегда дифференциальная

gen-c4fm-pulse = Импульс
gen-c4fm-shaped = Сформированный
    .hover = Выключено: частота скачет между четырьмя уровнями
gen-c4fm-apco = APCO-25
    .hover = Импульс TIA-102 для приёмника с интегрированием и сбросом
gen-c4fm-note = Девиации фиксированы: +-600 и +-1800 Hz; выход C4FM всегда вещественный.

gen-ofdm-lu = LU
    .hover = Полезная длина (размер FFT) в отсчётах на частоте дискретизации
gen-ofdm-lg = LG
    .hover = Длина защитного интервала (циклического префикса) в отсчётах
gen-ofdm-channels = Каналы
gen-ofdm-first-bin = Первый бин
    .hover = Бин первого канала; бин 0 - центр
gen-ofdm-modulation = Модуляция
gen-ofdm-unused = Неиспользуемые позиции
    .hover = Позиции каналов без сигнала, считая от 0, например 3, 7, 10-12
gen-ofdm-overrides = Исключения
    .hover = Модуляция отдельных позиций, например 0: pilot, 5-6: QAM-16
gen-ofdm-model = Модель
gen-ofdm-model-a = A: каждый канал передаёт абсолютные значения
gen-ofdm-model-b = B: каждый канал дифференциальный от символа к символу
gen-ofdm-model-c = C: пилоты несут опору, данные абсолютны относительно них
gen-ofdm-pilots = Пилоты
gen-ofdm-pilots-none = Нет
gen-ofdm-pilots-list = Позиции
    .hover = Позиции пилотов, например 0, 8, 16
gen-ofdm-pilots-every = Каждый N-й
gen-ofdm-step = Шаг
gen-ofdm-offset = Первый
gen-ofdm-reference = Опора пилотов
    .hover = Что несут пилоты в модели C
    .disabled = Опору пилотов использует только модель C
gen-ofdm-reference-constant = Постоянная
gen-ofdm-reference-differential = Случайная общая фаза
gen-ofdm-center = Центр
    .hover = Куда попадает бин 0; для комплексного выхода - сдвиг от 0 Hz

gen-label = Название
    .hover = Название нового сигнала; если пусто, берётся описание
gen-sample-rate = Частота дискретизации
gen-length = Длина
gen-length-seconds = Секунды
gen-length-symbols = Символы
    .disabled = У тонов и шума нет символов
gen-seed = Зерно
gen-new-seed = Новое зерно
    .hover = Выбрать случайное зерно: новые данные и новый шум
gen-center-frequency = Центральная частота
    .hover = RF-частота, которой соответствует 0 Hz комплексного выхода (хранится с сигналом)
    .disabled = Центральная частота есть только у комплексного выхода

gen-ch-offset = Сдвиг частоты
gen-ch-drift = Дрейф
gen-ch-clock = Сдвиг тактовой частоты
    .hover = Ошибка тактовой частоты дискретизации; положительная - часы приёмника спешат
gen-ch-noise = Шум
gen-ch-noise-none = Нет
gen-ch-noise-full = SNR во всей полосе
gen-ch-noise-inband = SNR в полосе сигнала
gen-ch-noise-esn0 = Es/N0
gen-ch-bandwidth = Полоса
    .hover = Выключено: полоса, которую занимает сигнал
gen-ch-clip = Ограничение на
    .hover = Ограничить пиковую амплитуду; комплексные отсчёты сохраняют фазу
gen-ch-multipath = Многолучевые отводы
    .hover = Отводы - это весь канал: включите прямой путь (задержка 0, усиление 1)
gen-ch-delay = Задержка
gen-ch-gain = Усиление
gen-ch-add-tap = Добавить отвод

gen-d-none = Исправьте параметры, чтобы увидеть производные величины.
gen-d-symbol-rate = Символьная скорость
gen-d-sps = Отсчётов на символ
gen-d-symbols = Символы
gen-d-bandwidth = Занимаемая полоса
gen-d-samples = Отсчёты
gen-d-duration = Длительность
gen-d-index = Индекс модуляции h
gen-d-ls = LS
gen-d-k = K = LG/LU
gen-d-spacing = Разнос
gen-d-br = Br

gen-preview = Предпросмотр генератора
gen-preview-spectrum = Спектр
gen-preview-wave = Осциллограмма
gen-preview-name = Предпросмотр сгенерированного сигнала
gen-preview-building = Построение предпросмотра...
gen-preview-invalid = Предпросмотра нет, пока параметры некорректны
gen-preview-failed = Ошибка предпросмотра: { $error }
gen-preview-short = Слишком коротко для спектра
gen-preview-axis = Средний спектр, от { $lo } до { $hi } Hz, диапазон 80 dB
gen-preview-wave-axis = Первые { $ms } ms, пик { $peak }

gen-generate = Сгенерировать
    .hover = Добавить сигнал в сессию как новую запись
gen-generate-save = Сгенерировать и сохранить как
    .hover = Сгенерировать, затем сохранить новый сигнал в выбранном формате
gen-busy = Идёт генерация сигнала
gen-fix-first = Сначала исправьте параметры
gen-invalid = Невозможно сгенерировать: { $error }
gen-field-error = { $field }: { $error }
gen-ready = Готово: { $samples } отсчётов, { $duration } s
gen-list-bad = "{ $item }" не является позицией или диапазоном вида 10-12
gen-list-too-long = Слишком много позиций (не более { $max })
gen-overrides-bad = "{ $item }" не является позицией с модуляцией вида 5: QAM-16

gen-calc-cp = OFDM с CP
gen-calc-cp-lengths = По Fd, LU и LG
gen-calc-spacing = Разнос
gen-calc-symbol = Символ
gen-calc-useful = Полезная часть
gen-calc-guard = Защитный интервал
gen-calc-use = Применить в OFDM
    .hover = Скопировать эти длины (и Fd) в редактор OFDM
    .disabled = LU и LG должны быть целыми числами отсчётов
# Why Use in OFDM is disabled. $max: the generator's limit in samples.
gen-calc-lu-not-whole = LU должно быть целым числом отсчётов, не меньше 2.
gen-calc-lg-not-whole = LG должно быть целым числом отсчётов.
gen-calc-lu-too-large = LU должно быть не больше { $max } отсчётов.
gen-calc-lg-too-large = LG должно быть не больше { $max } отсчётов.
gen-calc-use-k = Применить в OFDM
    .hover = Скопировать этот K в редактор OFDM: наименьшее подходящее LU, от текущего LU и выше
gen-calc-inverse = По Br и разносу при Fd: ближайшие целые LU и LG
gen-calc-exact = точно
gen-calc-error = Погрешность
gen-calc-candidates = Целые LU и LG для Br и K
gen-calc-lu-range = LU от
gen-calc-tolerance = Допуск
gen-calc-found = { $count ->
    [one] { $count } кандидат, показано { $shown }
    [few] { $count } кандидата, показаны первые { $shown }
    [many] { $count } кандидатов, показаны первые { $shown }
   *[other] { $count } кандидата, показаны первые { $shown }
}
gen-calc-klist = Список K
gen-calc-k-from-rates = По Br и разносу
gen-calc-k-direct = По K
gen-calc-uncertainty = Погрешность
    .hover = Насколько может отличаться каждая измеренная скорость, в процентах от скорости. В списке показаны простейшие дроби в пределах получающейся погрешности K; для K, введённого напрямую, допуск равен допуску K, измеренного по двум скоростям с такой погрешностью.
gen-calc-uncertainty-bad = Погрешность должна быть от 0 до { $max } %.
gen-calc-k-none = Ни одна дробь не попадает в пределы погрешности. Увеличьте погрешность или наибольший знаменатель.
gen-calc-max-den = Наибольший знаменатель
gen-calc-count = Количество
gen-calc-value = Значение
gen-calc-fd = Fd
    .hover = Частота дискретизации в Гц, при которой эти длины точны для Br: Br x (LU + LG)
gen-calc-k = K
    .hover = K = LG/LU, отношение защитного интервала к полезной длине, которое на самом деле имеют эти целые длины
gen-calc-lg-error = Погрешность (отсчёты)
    .hover = LG минус K x LU: насколько целая длина защитного интервала отличается от точной, в отсчётах
gen-calc-k-fraction = K
    .hover = K в виде несократимой дроби p/q
gen-calc-k-error = Погрешность
    .hover = Значение дроби минус измеренное K (со знаком; у K нет единиц, поэтому и у этой величины тоже)
gen-preview-symbol-too-long = Нет предпросмотра: один символ занимает { $samples } отсчетов, больше предпросмотра. Повысьте скорость или понизьте частоту дискретизации.
gen-calc-fraction-too-big = K = { $k } не помещается: LU должно быть не больше { $lu }, а LG не больше { $lg } отсчетов. Выберите меньшее LU или K.
