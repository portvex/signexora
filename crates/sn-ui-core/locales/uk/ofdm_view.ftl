# OFDM module window, SA-OFDM (SA_UI_SPEC.md section 10).

# Top bar (SA's control names).
ofdm-min-br = Min Br-LS
    .hover = Найменша шукана символьна швидкість, Bd (ціле); поле після неї - найдовший LS
ofdm-max-br = Max Br-LS
    .hover = Найбільша шукана символьна швидкість, Bd (ціле); поле після неї - найкоротший LS
ofdm-total-ls = Total LS
    .hover = Символи найбільшої довжини (Min Br-LS) у виділенні
ofdm-current-ls = Current LS
    .hover = Показаний символ (0 під час Find CT)
ofdm-survey = Survey
    .hover = Огляд: вікно спектра на 1024 точки, яке рухає смуга внизу, задає початок Find CT
ofdm-list-k = List of K
    .hover = Калькулятор Magic K: найімовірніші значення K та власні частоти дискретизації
    .disabled = Спершу виконайте Find CT і ADP

# Controls (SA's control names).
ofdm-symbols = Symbols
    .hover = Символи, згорнуті в кореляційний трикутник (і блок ADP); 20-40 добре, ніколи менше 6
ofdm-scale-fft = Scale FFT
    .hover = Ширина смуг каналів спектра LU
ofdm-shift = Shift Frequency
    .hover = Частота, що віднімається перед FFT, Hz; Enter застосовує
ofdm-vshift = VShift
    .hover = Віртуальний зсув на один канал (точні кроки сітки, без обмеження)
ofdm-shift-step = Крок
    .hover = Shift Frequency більше або менше на { $step } Hz
ofdm-br = Br
    .hover = Символьна швидкість: значення ADP, або введіть своє й натисніть Enter
ofdm-adp = ADP
    .hover = Точна символьна швидкість і положення за частотою (Get Br у SA)
    .disabled = Спершу виконайте Find CT
ofdm-show = Show
    .hover = Показувати позначені канали символ за символом із символьною синхронізацією (Break зупиняє)
ofdm-break = Break
ofdm-find-ct = Find CT
    .hover = Знайти кореляційний трикутник: LS, LG і початок символів
ofdm-mode = Mode
    .hover = Модель сигналу для сузір'їв
ofdm-model-a = A
    .hover = Модель A: кожен канал як згенеровано (абсолютні значення)
ofdm-model-b = B
    .hover = Модель B: кожен канал сам собі пілот (відносні значення)
ofdm-model-c = C
    .hover = Модель C: дані відносно пілота на червоному маркері
ofdm-each = Each
    .hover = Показувати кожен N-й символ
ofdm-start-plus = Start+
    .hover = Перший символ, який показує Show

# Plot texts (SA's formats).
ofdm-info-br = Br = { $br }
ofdm-info-sh = Sh = { $sh }
ofdm-info-magic = Magic K = { $num }/{ $den }
ofdm-info-k = * { $k } *
ofdm-info-lg = LG(CP) = { $lg }
ofdm-info-lu = LU = { $lu }
ofdm-info-up = CH_Up = { $ch }, Freq = { $freq }
ofdm-info-down = CH_Down = { $ch }, Freq = { $freq }
ofdm-info-sel = CH_Sel = { $count }, dFreqs = { $df }
ofdm-info-vshift = VShift = { $bins }
ofdm-line = LS={ $ls }; LG={ $lg }; Br={ $br }; Sh={ $sh }
ofdm-k-line = SR={ $sr }; LS={ $ls }; LU={ $lu }; LG={ $lg }; K={ $num }/{ $den }
ofdm-finish = Нормальне завершення, час { $secs } с.
ofdm-stopped = Зупинено користувачем, час { $secs } с.
ofdm-wait = Без паніки! Зачекайте...
ofdm-wait-ct = Пошук кореляційного трикутника
ofdm-wait-adp = Обчислення точного Br
ofdm-wait-k = Обчислення "Magic K"
ofdm-wait-left = Залишилося: { $secs } с.

# Plots.
ofdm-survey-plot = Оглядовий спектр
ofdm-triangle-plot = Кореляційний трикутник
ofdm-triangle = Трикутник
    .hover = Клацніть на добрий (лівий) схил, щоб поставити вікно LU; на вершині це батьківський LU
ofdm-spectrum-plot = Спектр LU
ofdm-spectrum = Спектр
    .hover = Клік на каналі переміщує ближчий маркер, подвійний клік ставить обидва
ofdm-pane = Канал
    .hover = Канал { $ch }, символів: { $count }
ofdm-k = K
    .hover = K = { $k }, похибка LU { $err }, ICI { $ici } dB, { $lu }
ofdm-k-lu-even = парний LU
ofdm-k-lu-odd = непарний LU

# Popup menus.
ofdm-sort-sr = Сортувати за SR
ofdm-sort-k = Сортувати за K
ofdm-sort-am = Сортувати за AM
ofdm-k-close = Закрити
ofdm-menu-cancel = Скасувати
ofdm-menu-reset = Очистити результати
ofdm-menu-around = Межі навколо Br = { $rate }
    .hover = Встановити Min Br-LS і Max Br-LS у 0,8 та 1,25 від виміряного Br
ofdm-resample = Передискретизувати до { $rate } Hz
    .hover = Передискретизувати сигнал до цієї власної частоти (нове вікно)

# Jobs and messages.
ofdm-job-ct = OFDM: Find CT
ofdm-job-br = OFDM: ADP
ofdm-job-k = OFDM: List of K
ofdm-job-show = OFDM: Show
ofdm-needs-ct = Спершу виконайте Find CT.
ofdm-needs-br = Спершу виконайте ADP.
ofdm-baseband-freed = Тим часом вікно закрили: виконайте Find CT знову.
ofdm-check-settings = Перевірте налаштування! Show потребує Symbols не менше 6 і 10 символів після Start+.
ofdm-failed = Пошук не вдався, час { $secs } с.
ofdm-pick-busy = Дочекайтеся завершення роботи або натисніть Break, потім виберіть рядок.
ofdm-br-invalid = Br "{ $text }" не підходить: введіть число більше 0 і до { $max } Bd (fs/LU).
ofdm-br-invalid-nogrid = Br "{ $text }" не підходить: введіть число більше 0.
ofdm-plot-empty = Немає даних: запустіть Find CT.
ofdm-shift-invalid = Shift Frequency "{ $text }" не є числом; введіть у Hz, наприклад 12.5.
