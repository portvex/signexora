# The PhasePlane form (SA_UI_SPEC.md section 6): spectrum mode toolbar, ISD
# corrector, phase viewer, status bar, demodulator output and the LoadFile dialog.
# The control captions are SA's own English names (SA showed them in English under
# every Windows language): keep them as they are; translate the hints and dialogs.

pp-fmin = Fmin
    .hover = Самая низкая показанная частота, Hz
pp-fmax = Fmax
    .hover = Самая высокая показанная частота, Hz
pp-scale = Scale
    .hover = Показываемая часть полосы; полоса прокрутки под спектром сдвигает её
pp-fft-size = FFTSize
    .hover = Размер FFT усреднённого спектра: больше - детальнее (до 1 Hz)
pp-low-level = LowLevel
    .hover = Нижний показываемый уровень
pp-stretch = Stretch
    .hover = Растянуть спектр на всю высоту
pp-mode = Mode
    .hover = AM: коррекция по огибающей; X^N: по N-й степени сигнала
pp-blocks = Blocks
    .hover = Число блоков сигнала, усредняемых корректором
pp-corr-off = Corr-OFF
    .hover = Корректор ISD выключен: нажмите Auto, чтобы исправить сигнал
pp-corr-on = Corr-ON
    .hover = Корректор ISD включен (SaveAs сохраняет исправленный сигнал)
pp-save-as = SaveAs
    .hover = Сохранить обработанный сигнал в файл
pp-auto = Auto
    .hover = Один шаг корректора по выбранным блокам
pp-break = Break
pp-reset = Reset
    .hover = Сбросить корректор (выключить)
pp-carrier = Carrier
    .hover = Несущая частота, Hz (последнее измерение Sc)
pp-br = BR
    .hover = Скорость манипуляции, Bd (последнее измерение Br)
pp-nary = n-Ary
    .hover = Позиционность модуляции, 2-32 (1: без слежения за несущей)
pp-synch-br = Synch BR
    .hover = Тактовая синхронизация
pp-nonstop = NonStop
    .hover = Непрерывная работа: с конца файла снова сначала
pp-rrc = RRC filter
    .hover = Фильтр корня из приподнятого косинуса; ползунок задаёт alpha
pp-pll = PLL
    .hover = Скорость петли несущей
pp-pll-slow = Slow
pp-pll-medium = Medium
pp-pll-quick = Quick
pp-pll-hard = Hard
pp-auto-reset = AutoReset
    .hover = При NonStop: сбрасывать генераторы и петли PLL в конце файла (для сигналов с дрейфом)
pp-transitions = Transitions
    .hover = Рисовать переходы между точками (траекторию)
pp-life = Life
    .hover = Динамическое отображение: остаются только последние символы (Points...)
pp-diff = Diff
    .hover = 0: абсолютные углы; 1-3: разности фаз (относительные режимы, как pi/4 DQPSK)
pp-offset-detected = Offset mode detected
pp-angles = Angles
    .hover = Строить гистограмму углов в окне корректора (нужен Synch BR)
pp-vectors = Vectors of dem
    .hover = Показать позиции демодулятора; нулевой вектор голубой
pp-output-dem = Output Dem as
    .hover = Выводить демодулированный поток кодами абсолютных углов в текстовое окно
pp-trimming = Trimming { $value }
    .hover = Точная подстройка фазы, градусы
pp-delay = Delay { $value }
    .hover = Реальная задержка на символ, секунды (для очень коротких сигналов)
pp-points = Points... { $value }
    .hover = Точек одновременно при Life
pp-contrast = Contrast { $value }
    .hover = Яркость точек
pp-start = Start
    .hover = Запустить или остановить созвездие; после остановки снова с начала файла
pp-clear = Clear
    .hover = Очистить окно отображения
pp-load-file = LoadFile
    .hover = Загрузить один из открытых файлов в фазовую плоскость
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

pp-isd-n = Гармоника сигнала, с которой работает корректор
    .hover = Гармоника сигнала, с которой работает корректор
pp-isd-b = Вкл.: улучшенный алгоритм; выкл.: чуть хуже и быстрее (в режиме AM не влияет)
    .hover = Вкл.: улучшенный алгоритм; выкл.: чуть хуже и быстрее (в режиме AM не влияет)
pp-isd-break = Остановить текущий шаг
    .hover = Остановить текущий шаг
pp-alpha = Коэффициент скругления alpha фильтра RRC
    .hover = Коэффициент скругления alpha фильтра RRC
pp-quality = Условное отношение сигнал/шум созвездия, dB
    .hover = Условное отношение сигнал/шум созвездия, dB
pp-load-title = SA - выбор файла для PhasePlane
pp-ok = Ok
pp-cancel = Отмена
pp-check-settings = Проверьте настройки!
pp-run-limited = Демодулируются только первые { $symbols } символов. Чтобы проанализировать более позднюю часть, поставьте на неё маркеры V на сонограмме, выберите { sw-hint-copy } и откройте { sw-hint-phase-plane } на копии.
pp-copy-panel = Копировать текст
pp-copy-codes = Копировать все коды от Start
pp-eye = Глазковая диаграмма
    .hover = Показать глазковую диаграмму (I, два символа) вместо созвездия
pp-job-corrector = Спектр корректора ISD
pp-job-isd = Корректор ISD
pp-job-spectrum = Спектр PhasePlane
# $count symbols, $offset Hz, $rings radii, $iq_offset symbols.
pp-quality-details = { $count ->
        [one] { $count } символ
        [few] { $count } символа
        [many] { $count } символов
       *[other] { $count } символа
    }, сдвиг петли { $offset } Hz, кольца [{ $rings }], сдвиг I/Q { $iq_offset } символа
pp-source-gone = Загруженного файла больше нет в сессии; загрузите рабочий файл снова через LoadFile.
pp-load-gone = Выбранный файл закрыт; выберите другой рабочий файл.
