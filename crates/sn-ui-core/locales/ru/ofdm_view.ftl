# OFDM module window, SA-OFDM (SA_UI_SPEC.md section 10).

# Top bar (SA's control names).
ofdm-min-br = Min Br-LS
    .hover = Наименьшая искомая символьная скорость, Bd (целое); поле после неё - самый длинный LS
ofdm-max-br = Max Br-LS
    .hover = Наибольшая искомая символьная скорость, Bd (целое); поле после неё - самый короткий LS
ofdm-total-ls = Total LS
    .hover = Символы наибольшей длины (Min Br-LS) в выделении
ofdm-current-ls = Current LS
    .hover = Показанный символ (0 во время Find CT)
ofdm-survey = Survey
    .hover = Обзор: окно спектра на 1024 точки, которое двигает полоса внизу, задаёт начало Find CT
ofdm-list-k = List of K
    .hover = Калькулятор Magic K: наиболее вероятные значения K и собственные частоты дискретизации
    .disabled = Сначала выполните Find CT и ADP

# Controls (SA's control names).
ofdm-symbols = Symbols
    .hover = Символы, свёрнутые в корреляционный треугольник (и блок ADP); 20-40 хорошо, никогда меньше 6
ofdm-scale-fft = Scale FFT
    .hover = Ширина полос каналов спектра LU
ofdm-shift = Shift Frequency
    .hover = Частота, вычитаемая перед FFT, Hz; Enter применяет
ofdm-vshift = VShift
    .hover = Виртуальный сдвиг на один канал (точные шаги сетки, без ограничения)
ofdm-shift-step = Шаг
    .hover = Shift Frequency больше или меньше на { $step } Hz
ofdm-br = Br
    .hover = Символьная скорость: значение ADP, или введите своё и нажмите Enter
ofdm-adp = ADP
    .hover = Точная символьная скорость и положение по частоте (Get Br в SA)
    .disabled = Сначала выполните Find CT
ofdm-show = Show
    .hover = Показывать отмеченные каналы символ за символом с символьной синхронизацией (Break останавливает)
ofdm-break = Break
ofdm-find-ct = Find CT
    .hover = Найти корреляционный треугольник: LS, LG и начало символов
ofdm-mode = Mode
    .hover = Модель сигнала для созвездий
ofdm-model-a = A
    .hover = Модель A: каждый канал как сгенерирован (абсолютные значения)
ofdm-model-b = B
    .hover = Модель B: каждый канал сам себе пилот (относительные значения)
ofdm-model-c = C
    .hover = Модель C: данные относительно пилота на красном маркере
ofdm-each = Each
    .hover = Показывать каждый N-й символ
ofdm-start-plus = Start+
    .hover = Первый символ, который показывает Show

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
ofdm-finish = Нормальное завершение, время { $secs } с.
ofdm-stopped = Остановлено пользователем, время { $secs } с.
ofdm-wait = Без паники! Подождите...
ofdm-wait-ct = Поиск корреляционного треугольника
ofdm-wait-adp = Вычисление точного Br
ofdm-wait-k = Вычисление "Magic K"
ofdm-wait-left = Осталось: { $secs } с.

# Plots.
ofdm-survey-plot = Обзорный спектр
ofdm-triangle-plot = Корреляционный треугольник
ofdm-triangle = Треугольник
    .hover = Щёлкните по хорошему (левому) склону, чтобы поставить окно LU; на вершине это родительский LU
ofdm-spectrum-plot = Спектр LU
ofdm-spectrum = Спектр
    .hover = Щелчок по каналу двигает ближний маркер, двойной щелчок ставит оба
ofdm-pane = Канал
    .hover = Канал { $ch }, символов: { $count }
ofdm-k = K
    .hover = K = { $k }, ошибка LU { $err }, ICI { $ici } dB, { $lu }
ofdm-k-lu-even = чётный LU
ofdm-k-lu-odd = нечётный LU

# Popup menus.
ofdm-sort-sr = Сортировать по SR
ofdm-sort-k = Сортировать по K
ofdm-sort-am = Сортировать по AM
ofdm-k-close = Закрыть
ofdm-menu-cancel = Отмена
ofdm-menu-reset = Очистить результаты
ofdm-menu-around = Границы вокруг Br = { $rate }
    .hover = Установить Min Br-LS и Max Br-LS в 0,8 и 1,25 от измеренного Br
ofdm-resample = Передискретизировать до { $rate } Hz
    .hover = Передискретизировать сигнал до этой собственной частоты (новое окно)

# Jobs and messages.
ofdm-job-ct = OFDM: Find CT
ofdm-job-br = OFDM: ADP
ofdm-job-k = OFDM: List of K
ofdm-job-show = OFDM: Show
ofdm-needs-ct = Сначала выполните Find CT.
ofdm-needs-br = Сначала выполните ADP.
ofdm-baseband-freed = Окно тем временем закрыли: выполните Find CT снова.
ofdm-check-settings = Проверьте настройки! Show требует Symbols не меньше 6 и 10 символов после Start+.
ofdm-failed = Поиск не удался, время { $secs } с.
ofdm-pick-busy = Дождитесь завершения работы или нажмите Break, затем выберите строку.
ofdm-br-invalid = Br "{ $text }" не подходит: введите число больше 0 и до { $max } Bd (fs/LU).
ofdm-br-invalid-nogrid = Br "{ $text }" не подходит: введите число больше 0.
ofdm-plot-empty = Нет данных: запустите Find CT.
ofdm-shift-invalid = Shift Frequency "{ $text }" не является числом; введите в Hz, например 12.5.
