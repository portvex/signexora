# The WF (waveform) window: SA's top toolbar, bottom toolbar hints, popup menu and
# result box. SA's control names (LPF, RRC, Sum, Info, VMW, Br, Z+, Z-) and the
# result box text are names: keep them.

## Top toolbar, left to right.

wf-lpf = LPF
    .hover = Частота зрізу ФНЧ у Гц або швидкість RRC-фільтра (введіть або виберіть)
wf-copy-br = <==
    .hover = Скопіювати виміряну Br у поле LPF
wf-rrc = RRC
    .hover = Увімкнути або вимкнути RRC-фільтр (тоді його застосовує кнопка LPF)
wf-rrc-alpha = RRC
    .hover = Коефіцієнт згладжування RRC
wf-freq = 0-0
    .hover = Клацання: частота короткого фрагмента між маркерами V і кількість цілих періодів
wf-vmw = VMW
    .hover = Перейти до VMW (сигнал, складений за періодом рядка)
wf-sum = Sum
    .hover = Показати гістограму значень (далі Підрахунок частот у меню)
wf-info = Info
    .hover = Показати результати (t1, t2, dT, S, Br)

## Bottom toolbar hints, left to right.

wf-hint-zoom-in = Збільшити (час)
wf-hint-zoom-out = Зменшити (час)
wf-hint-scale-up = Більший вертикальний масштаб
wf-hint-scale-down = Менший вертикальний масштаб
wf-hint-zoom-plus = Z+: Масштаб між маркерами V
wf-hint-zoom-minus = Z-: Повернутися на крок масштабу
wf-hint-lpf = Застосувати фільтр LPF (RRC-фільтр, якщо RRC увімкнено)
wf-hint-revert = Повернути оригінал
wf-hint-synth = Зворотний синтез FSK/MFSK
wf-hint-save = Зберегти як 16-бітний WAV (правий клік: WAV float, DataFile або SigMF)
wf-hint-copy = Копіювати фрагмент між маркерами V у нове вікно
wf-hint-demod = Надіслати фрагмент до демодулятора FSK/MFSK
wf-hint-fit = Підігнати вертикальний масштаб до сигналу
wf-hint-acf = Автокореляція фрагмента (нове вікно)
wf-hint-ccf = Взаємна кореляція фрагмента з усім сигналом (тут)

## Why a button is disabled.

wf-need-revert = Нічого повертати: немає фільтра й CCF.
wf-need-derived = Це вікно показує відкритий файл, який уже збережено. Зберегти призначено для результатів, створених тут.

## Popup menu.

wf-popup-count-freqs = Підрахунок частот
wf-popup-more = Інструменти SigNexora
    .hover = Інструменти SigNexora, яких не було у вікні WF програми SA
wf-popup-operations = Операції

## The Count freqs result box (SA's text, not translated). $size: a whole number;
## $value: a number with two decimals.

wf-label-grid = Freq grid: { $size }
wf-label-delta = Delta: { $value } Hz
wf-count-needs-h = Спершу встановіть маркери H навколо тонів.
wf-count-tones-busy = Find tones ще працює; спробуйте Count freqs знову, коли завершиться.
wf-count-freqs-failed = Сітку частот не знайдено ({ $error }). Розширте маркери H навколо тонів або зменште масштаб, щоб бачити більше тонів.
wf-copy-br-value = Скопіювати Br = { $br } Bd у поле LPF
wf-copy-br-missing = Спершу виміряйте Br (меню сонограми: Auto define params)
