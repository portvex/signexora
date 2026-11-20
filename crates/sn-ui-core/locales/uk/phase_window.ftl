# The PhasePlane form (SA_UI_SPEC.md section 6): spectrum mode toolbar, ISD
# corrector, phase viewer, status bar, demodulator output and the LoadFile dialog.
# The control captions are SA's own English names (SA showed them in English under
# every Windows language): keep them as they are; translate the hints and dialogs.

pp-fmin = Fmin
    .hover = Найнижча показана частота, Hz
pp-fmax = Fmax
    .hover = Найвища показана частота, Hz
pp-scale = Scale
    .hover = Показана частина смуги; смуга прокрутки під спектром зсуває її
pp-fft-size = FFTSize
    .hover = Розмір FFT усередненого спектра: більший дає тонший (до 1 Hz)
pp-low-level = LowLevel
    .hover = Найнижчий показаний рівень
pp-stretch = Stretch
    .hover = Розтягнути спектр на всю висоту
pp-mode = Mode
    .hover = AM: корекція за обвідною; X^N: за N-м степенем сигналу
pp-blocks = Blocks
    .hover = Кількість блоків сигналу, які усереднює коректор
pp-corr-off = Corr-OFF
    .hover = Коректор ISD вимкнено: натисніть Auto, щоб виправити сигнал
pp-corr-on = Corr-ON
    .hover = Коректор ISD увімкнено (SaveAs зберігає виправлений сигнал)
pp-save-as = SaveAs
    .hover = Зберегти виправлений сигнал у файл
pp-auto = Auto
    .hover = Один крок коректора за вибраними блоками
pp-break = Break
pp-reset = Reset
    .hover = Скинути коректор (вимкнути)
pp-carrier = Carrier
    .hover = Частота несучої, Hz (останнє вимірювання Sc)
pp-br = BR
    .hover = Швидкість маніпуляції, Bd (останнє вимірювання Br)
pp-nary = n-Ary
    .hover = Порядок модуляції, 2-32 (1: без стеження за несучою)
pp-synch-br = Synch BR
    .hover = Тактова синхронізація
pp-nonstop = NonStop
    .hover = Безперервна робота: з кінця файлу знову спочатку
pp-rrc = RRC filter
    .hover = Фільтр кореня з піднятого косинуса; повзунок задає alpha
pp-pll = PLL
    .hover = Швидкість петлі несучої
pp-pll-slow = Slow
pp-pll-medium = Medium
pp-pll-quick = Quick
pp-pll-hard = Hard
pp-auto-reset = AutoReset
    .hover = З NonStop: скидати генератори та петлі PLL у кінці файлу (для сигналів з дрейфом)
pp-transitions = Transitions
    .hover = Малювати переходи між точками (траєкторію)
pp-life = Life
    .hover = Динамічне відображення: лишаються тільки останні символи (Points...)
pp-diff = Diff
    .hover = 0: абсолютні кути; 1-3: різниці фаз (відносні режими, як pi/4 DQPSK)
pp-offset-detected = Offset mode detected
pp-angles = Angles
    .hover = Будувати гістограму кутів у вікні коректора (потрібен Synch BR)
pp-vectors = Vectors of dem
    .hover = Показати позиції демодулятора; нульовий вектор блакитний
pp-output-dem = Output Dem as
    .hover = Виводити демодульований потік кодами абсолютних кутів у текстове вікно
pp-trimming = Trimming { $value }
    .hover = Точне підстроювання фази, градуси
pp-delay = Delay { $value }
    .hover = Реальна затримка на символ, секунди (для дуже коротких сигналів)
pp-points = Points... { $value }
    .hover = Точок одночасно при Life
pp-contrast = Contrast { $value }
    .hover = Яскравість точок
pp-start = Start
    .hover = Запустити або зупинити сузір'я; після зупинки знову з початку файлу
pp-clear = Clear
    .hover = Очистити вікно відображення
pp-load-file = LoadFile
    .hover = Завантажити один з відкритих файлів у фазову площину
pp-status-f = F = { $value }
pp-status-fl = Fl = { $value }
pp-status-fu = Fu = { $value }
pp-status-df = dF = { $value }
pp-status-angle-d = Angle_D = { $value }
pp-status-angle-i = Angle_I = { $value }
pp-dem-view-mode = View mode: diff { $diff }
pp-dem-mode = Dem mode: { $mode }
pp-dem-absolute = Data presented as absolute angles!
pp-dem-coding = The coding positions of angles:
pp-dem-data = Data:

pp-isd-n = Гармоніка сигналу, з якою працює коректор
    .hover = Гармоніка сигналу, з якою працює коректор
pp-isd-b = Увімкнено: кращий алгоритм; вимкнено: трохи гірший, швидший (у режимі AM не діє)
    .hover = Увімкнено: кращий алгоритм; вимкнено: трохи гірший, швидший (у режимі AM не діє)
pp-isd-break = Зупинити поточний крок
    .hover = Зупинити поточний крок
pp-alpha = Коефіцієнт alpha фільтра RRC
    .hover = Коефіцієнт alpha фільтра RRC
pp-quality = Умовне відношення сигнал/шум сузір'я, dB
    .hover = Умовне відношення сигнал/шум сузір'я, dB
pp-load-title = SA - вибір файлу для PhasePlane
pp-ok = Ok
pp-cancel = Скасувати
pp-check-settings = Перевірте налаштування!
pp-run-limited = Демодулюються лише перші { $symbols } символів. Щоб проаналізувати пізнішу частину, поставте на неї маркери V на сонограмі, виберіть { sw-hint-copy } і відкрийте { sw-hint-phase-plane } на копії.
pp-copy-panel = Копіювати текст
pp-copy-codes = Копіювати всі коди від Start
pp-eye = Глазкова діаграма
    .hover = Показати глазкову діаграму (I, два символи) замість сузір'я
pp-job-corrector = Спектр коректора ISD
pp-job-isd = Коректор ISD
pp-job-spectrum = Спектр PhasePlane
# $count symbols, $offset Hz, $rings radii, $iq_offset symbols.
pp-quality-details = { $count ->
        [one] { $count } символ
        [few] { $count } символи
        [many] { $count } символів
       *[other] { $count } символу
    }, зсув петлі { $offset } Hz, кільця [{ $rings }], зсув I/Q { $iq_offset } символу
pp-source-gone = Завантаженого файлу більше немає в сеансі; завантажте робочий файл знову через LoadFile.
pp-load-gone = Вибраний файл закрито; виберіть інший робочий файл.
