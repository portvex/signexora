# File > Signal generator (OCG): the generator window, its preview and the
# OFDM calculators. Keep units (Hz, Bd, s, ms, dB, ppm), modulation names
# (FSK, MSK, GMSK, PSK, QAM, C4FM, OFDM, BPSK, ...), BT, Es/N0, SNR, RC, RRC,
# LU, LG, LS, K, Br, Fd and the model letters A, B, C untranslated.

gen-menu = Генератор сигналів (OCG)...
    .hover = Створити тестовий сигнал із точно відомими параметрами: тони, шум, FSK, PSK/QAM, C4FM або OFDM, зі спотвореннями каналу
gen-keys-open = Показати або сховати генератор сигналів
gen-title = Генератор сигналів (OCG)
gen-job = Генерування { $name }

gen-kind = Тип
gen-kind-tones = Тони
gen-kind-noise = Шум
gen-kind-fsk = FSK/MFSK
gen-kind-linear = PSK/QAM
gen-kind-c4fm = C4FM
gen-kind-ofdm = OFDM
gen-preset = Пресет
    .hover = Завантажити готовий сигнал
gen-preset-defaults = Типові
    .hover = Повернути поточному типу типові значення

gen-section-signal = Сигнал
gen-section-common = Загальне
gen-section-channel = Спотворення каналу
gen-section-derived = Похідні величини
gen-section-preview = Попередній перегляд
gen-section-calc = Калькулятори

gen-complex = Комплексний вихід (I/Q)
    .hover = Створювати відліки I/Q замість дійсного сигналу; частоти тоді є зсувами від 0 Hz
gen-amplitude-rms = Амплітуда RMS
gen-amplitude-peak = Пікова амплітуда
gen-phase-rad = Фаза (рад)
gen-remove = Видалити
gen-baud = Швидкість модуляції
gen-carrier = Несуча
gen-center = Центр
gen-alpha = Коеф. заокруглення

gen-tones-frequency = Частота
gen-tones-add = Додати тон
gen-tones-last = Потрібен щонайменше один тон

gen-fsk-order = Тони
gen-fsk-spacing = Рознесення
gen-fsk-shaping = Формування
gen-fsk-shaping-none = Немає
gen-fsk-shaping-gaussian = Гаусове
gen-fsk-bt = BT
gen-fsk-msk = MSK
    .hover = Два тони, рознесення - половина швидкості, без формування
gen-fsk-gmsk = GMSK
    .hover = MSK з гаусовим фільтром BT 0.3, як у GSM

gen-lin-constellation = Сузір'я
gen-lin-shaping = Форма імпульсу
gen-lin-rect = Прямокутна
gen-lin-rc = Піднятий косинус (RC)
gen-lin-rrc = Корінь із піднятого косинуса (RRC)
gen-lin-differential = Диференційна
    .hover = Передавати кожен символ як крок фази від попереднього
gen-lin-differential-qam = QAM несе і амплітуду, і фазу, тому не може бути диференційною
gen-lin-differential-pi4 = pi/4-DQPSK завжди диференційна

gen-c4fm-pulse = Імпульс
gen-c4fm-shaped = Сформований
    .hover = Вимкнено: частота стрибає між чотирма рівнями
gen-c4fm-apco = APCO-25
    .hover = Імпульс TIA-102 для приймача з інтегруванням і скиданням
gen-c4fm-note = Девіації фіксовані: +-600 і +-1800 Hz; вихід C4FM завжди дійсний.

gen-ofdm-lu = LU
    .hover = Корисна довжина (розмір FFT) у відліках на частоті дискретизації
gen-ofdm-lg = LG
    .hover = Довжина захисного інтервалу (циклічного префікса) у відліках
gen-ofdm-channels = Канали
gen-ofdm-first-bin = Перший бін
    .hover = Бін першого каналу; бін 0 - центр
gen-ofdm-modulation = Модуляція
gen-ofdm-unused = Невикористані позиції
    .hover = Позиції каналів без сигналу, від 0, наприклад 3, 7, 10-12
gen-ofdm-overrides = Винятки
    .hover = Модуляція окремих позицій, наприклад 0: pilot, 5-6: QAM-16
gen-ofdm-model = Модель
gen-ofdm-model-a = A: кожен канал передає абсолютні значення
gen-ofdm-model-b = B: кожен канал диференційний від символу до символу
gen-ofdm-model-c = C: пілоти несуть опору, дані абсолютні відносно них
gen-ofdm-pilots = Пілоти
gen-ofdm-pilots-none = Немає
gen-ofdm-pilots-list = Позиції
    .hover = Позиції пілотів, наприклад 0, 8, 16
gen-ofdm-pilots-every = Кожен N-й
gen-ofdm-step = Крок
gen-ofdm-offset = Перший
gen-ofdm-reference = Опора пілотів
    .hover = Що несуть пілоти в моделі C
    .disabled = Опору пілотів використовує лише модель C
gen-ofdm-reference-constant = Стала
gen-ofdm-reference-differential = Випадкова спільна фаза
gen-ofdm-center = Центр
    .hover = Куди потрапляє бін 0; для комплексного виходу - зсув від 0 Hz

gen-label = Назва
    .hover = Назва нового сигналу; якщо порожня, береться опис
gen-sample-rate = Частота дискретизації
gen-length = Довжина
gen-length-seconds = Секунди
gen-length-symbols = Символи
    .disabled = Тони й шум не мають символів
gen-seed = Зерно
gen-new-seed = Нове зерно
    .hover = Вибрати випадкове зерно: нові дані й новий шум
gen-center-frequency = Центральна частота
    .hover = RF-частота, якій відповідає 0 Hz комплексного виходу (зберігається із сигналом)
    .disabled = Центральна частота є лише в комплексного виходу

gen-ch-offset = Зсув частоти
gen-ch-drift = Дрейф
gen-ch-clock = Зсув тактової частоти
    .hover = Похибка тактової частоти дискретизації; додатна - годинник приймача поспішає
gen-ch-noise = Шум
gen-ch-noise-none = Немає
gen-ch-noise-full = SNR у всій смузі
gen-ch-noise-inband = SNR у смузі сигналу
gen-ch-noise-esn0 = Es/N0
gen-ch-bandwidth = Смуга
    .hover = Вимкнено: смуга, яку займає сигнал
gen-ch-clip = Обмеження на
    .hover = Обмежити пікову амплітуду; комплексні відліки зберігають фазу
gen-ch-multipath = Багатопроменеві відводи
    .hover = Відводи - це весь канал: додайте прямий шлях (затримка 0, підсилення 1)
gen-ch-delay = Затримка
gen-ch-gain = Підсилення
gen-ch-add-tap = Додати відвід

gen-d-none = Виправте параметри, щоб побачити похідні величини.
gen-d-symbol-rate = Символьна швидкість
gen-d-sps = Відліків на символ
gen-d-symbols = Символи
gen-d-bandwidth = Зайнята смуга
gen-d-samples = Відліки
gen-d-duration = Тривалість
gen-d-index = Індекс модуляції h
gen-d-ls = LS
gen-d-k = K = LG/LU
gen-d-spacing = Рознесення
gen-d-br = Br

gen-preview = Попередній перегляд генератора
gen-preview-spectrum = Спектр
gen-preview-wave = Осцилограма
gen-preview-name = Попередній перегляд згенерованого сигналу
gen-preview-building = Створення попереднього перегляду...
gen-preview-invalid = Попереднього перегляду немає, доки параметри некоректні
gen-preview-failed = Помилка попереднього перегляду: { $error }
gen-preview-short = Закоротко для спектра
gen-preview-axis = Середній спектр, від { $lo } до { $hi } Hz, діапазон 80 dB
gen-preview-wave-axis = Перші { $ms } ms, пік { $peak }

gen-generate = Згенерувати
    .hover = Додати сигнал до сеансу як новий запис
gen-generate-save = Згенерувати і зберегти як
    .hover = Згенерувати, потім зберегти новий сигнал у вибраному форматі
gen-busy = Сигнал генерується
gen-fix-first = Спершу виправте параметри
gen-invalid = Неможливо згенерувати: { $error }
gen-field-error = { $field }: { $error }
gen-ready = Готово: { $samples } відліків, { $duration } s
gen-list-bad = "{ $item }" не є позицією чи діапазоном на кшталт 10-12
gen-list-too-long = Забагато позицій (не більше { $max })
gen-overrides-bad = "{ $item }" не є позицією з модуляцією на кшталт 5: QAM-16

gen-calc-cp = OFDM з CP
gen-calc-cp-lengths = З Fd, LU і LG
gen-calc-spacing = Рознесення
gen-calc-symbol = Символ
gen-calc-useful = Корисна частина
gen-calc-guard = Захисний інтервал
gen-calc-use = Застосувати в OFDM
    .hover = Скопіювати ці довжини (і Fd) до редактора OFDM
    .disabled = LU і LG мають бути цілими числами відліків
# Why Use in OFDM is disabled. $max: the generator's limit in samples.
gen-calc-lu-not-whole = LU має бути цілим числом відліків, щонайменше 2.
gen-calc-lg-not-whole = LG має бути цілим числом відліків.
gen-calc-lu-too-large = LU має бути не більше { $max } відліків.
gen-calc-lg-too-large = LG має бути не більше { $max } відліків.
gen-calc-use-k = Застосувати в OFDM
    .hover = Скопіювати цей K до редактора OFDM: найменше LU, що йому відповідає, від поточного LU і вище
gen-calc-inverse = З Br і рознесення при Fd: найближчі цілі LU і LG
gen-calc-exact = точно
gen-calc-error = Похибка
gen-calc-candidates = Цілі LU і LG для Br і K
gen-calc-lu-range = LU від
gen-calc-tolerance = Допуск
gen-calc-found = { $count ->
    [one] { $count } кандидат, показано { $shown }
    [few] { $count } кандидати, показано перші { $shown }
    [many] { $count } кандидатів, показано перші { $shown }
   *[other] { $count } кандидата, показано перші { $shown }
}
gen-calc-klist = Список K
gen-calc-k-from-rates = З Br і рознесення
gen-calc-k-direct = З K
gen-calc-uncertainty = Похибка
    .hover = Наскільки може відрізнятися кожна виміряна швидкість, у відсотках від швидкості. У списку показано найпростіші дроби в межах похибки K, що з цього випливає; для K, введеного напряму, допуск дорівнює допуску K, виміряного за двома швидкостями з такою похибкою.
gen-calc-uncertainty-bad = Похибка має бути від 0 до { $max } %.
gen-calc-k-none = Жоден дріб не потрапляє в межі похибки. Збільште похибку або найбільший знаменник.
gen-calc-max-den = Найбільший знаменник
gen-calc-count = Кількість
gen-calc-value = Значення
gen-calc-fd = Fd
    .hover = Частота дискретизації в Гц, за якої ці довжини точні для Br: Br x (LU + LG)
gen-calc-k = K
    .hover = K = LG/LU, відношення захисного інтервалу до корисної довжини, яке насправді мають ці цілі довжини
gen-calc-lg-error = Похибка (відліки)
    .hover = LG мінус K x LU: наскільки ціла довжина захисного інтервалу відрізняється від точної, у відліках
gen-calc-k-fraction = K
    .hover = K як нескоротний дріб p/q
gen-calc-k-error = Похибка
    .hover = Значення дробу мінус виміряне K (зі знаком; K не має одиниць, тому й ця величина теж)
gen-preview-symbol-too-long = Без попереднього перегляду: один символ має { $samples } відліків, більше за перегляд. Підвищте швидкість або знизьте частоту дискретизації.
gen-calc-fraction-too-big = K = { $k } не вміщується: LU має бути не більше { $lu }, а LG не більше { $lg } відліків. Виберіть менше LU або K.
