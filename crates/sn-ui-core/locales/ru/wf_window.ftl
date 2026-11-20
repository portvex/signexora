# The WF (waveform) window: SA's top toolbar, bottom toolbar hints, popup menu and
# result box. SA's control names (LPF, RRC, Sum, Info, VMW, Br, Z+, Z-) and the
# result box text are names: keep them.

## Top toolbar, left to right.

wf-lpf = LPF
    .hover = Частота среза ФНЧ в Гц или скорость RRC-фильтра (введите или выберите)
wf-copy-br = <==
    .hover = Скопировать измеренную Br в поле LPF
wf-rrc = RRC
    .hover = Включить или выключить RRC-фильтр (тогда его применяет кнопка LPF)
wf-rrc-alpha = RRC
    .hover = Коэффициент скругления RRC
wf-freq = 0-0
    .hover = Щелчок: частота короткого фрагмента между маркерами V и число целых периодов
wf-vmw = VMW
    .hover = Перейти в VMW (сигнал, сложенный по периоду строки)
wf-sum = Sum
    .hover = Показать гистограмму значений (затем Подсчёт частот в меню)
wf-info = Info
    .hover = Показать результаты (t1, t2, dT, S, Br)

## Bottom toolbar hints, left to right.

wf-hint-zoom-in = Увеличить (время)
wf-hint-zoom-out = Уменьшить (время)
wf-hint-scale-up = Больший вертикальный масштаб
wf-hint-scale-down = Меньший вертикальный масштаб
wf-hint-zoom-plus = Z+: Масштаб между маркерами V
wf-hint-zoom-minus = Z-: Вернуться на шаг масштаба
wf-hint-lpf = Применить фильтр LPF (RRC-фильтр, если RRC включён)
wf-hint-revert = Вернуть оригинал
wf-hint-synth = Обратный синтез FSK/MFSK
wf-hint-save = Сохранить как 16-битный WAV (правый щелчок: WAV float, DataFile или SigMF)
wf-hint-copy = Копировать фрагмент между маркерами V в новое окно
wf-hint-demod = Отправить фрагмент в демодулятор FSK/MFSK
wf-hint-fit = Подогнать вертикальный масштаб к сигналу
wf-hint-acf = Автокорреляция фрагмента (новое окно)
wf-hint-ccf = Взаимная корреляция фрагмента со всем сигналом (здесь)

## Why a button is disabled.

wf-need-revert = Нечего возвращать: нет фильтра и CCF.
wf-need-derived = Это окно показывает открытый файл, который уже сохранен. Сохранение нужно для результатов, полученных здесь.

## Popup menu.

wf-popup-count-freqs = Подсчёт частот
wf-popup-more = Инструменты SigNexora
    .hover = Инструменты SigNexora, которых не было в окне WF программы SA
wf-popup-operations = Операции

## The Count freqs result box (SA's text, not translated). $size: a whole number;
## $value: a number with two decimals.

wf-label-grid = Freq grid: { $size }
wf-label-delta = Delta: { $value } Hz
wf-count-needs-h = Сначала установите маркеры H вокруг тонов.
wf-count-tones-busy = Find tones еще работает; повторите Count freqs, когда он завершится.
wf-count-freqs-failed = Сетка частот не найдена ({ $error }). Расширьте маркеры H вокруг тонов или уменьшите масштаб, чтобы видеть больше тонов.
wf-copy-br-value = Скопировать Br = { $br } Bd в поле LPF
wf-copy-br-missing = Сначала измерьте Br (меню сонограммы: Auto define params)
