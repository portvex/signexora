# The workflow guide: SA's analysis method as a checklist with buttons.

guide-title = Порядок аналізу
guide-no-signal = Спершу відкрийте запис (Файл > Відкрити файл...).
# $signal: the active window's signal, which the buttons act on.
guide-acts-on = Кнопки діють на: { $signal }
# $key: a key name. Shown after a menu path only when that key acts on the guide's signal.
guide-key = , клавіша { $key }

## Steps: the message is the step's title, .text its explanation.

guide-step-record = 1. Правильний запис
    .text = 16-бітний PCM WAV (або I/Q) з вимкненим АРП приймача, без обмеження, без редагування. Стиснуті або 8-бітні файли позначаються під час відкриття.
guide-step-quality = 2. Перевірка якості запису
    .text = Відкрийте вікно WF (правий клік по сонограмі > { sw-popup-waveform }). Шукайте обмеження, 8-бітні сходинки, постійну складову, «дихання» АРП і пропадання. Увімкніть «Сума», щоб побачити розподіл амплітуд.
guide-step-isolate = 3. Виділення сигналу
    .text = На сонограмі поставте маркери H навколо сигналу. Потім застосуйте кнопки «{ sw-hint-band-select }» і «{ sw-hint-shift-base }», а широкий сигнал передискретизуйте кнопкою R на меншу частоту.
guide-step-rate = 4. Швидкість маніпуляції
    .text = Застосуйте детектор, охопіть його тактову лінію маркерами й виберіть «{ sw-popup-measure }» у контекстному меню{ $br }. { sw-hint-am } для PSK/QAM/FSK, { sw-hint-modified-am } для MFSK, або спробуйте «{ sw-popup-adp }».
guide-step-family = 5. Аналіз виду модуляції
    .text = FSK/MFSK: { sw-hint-phase-detector }, потім у вікні WF { edits-median } на 1/3 символу, Сума, Знайти тони, Лічба частот. PSK/QAM: { sw-hint-power } для порядку й несучої («{ sw-popup-measure }» у його вікні{ $sc }), потім PhasePlane; для спотворених сигналів спершу коректор ISD. OFDM: Знайти CT, Отримати Br, Канали, потім Список K для вихідної частоти.
guide-step-structure = 6. Пошук структури
    .text = { sw-hint-acf } для періодів кадрів, { sw-hint-ccf } для пошуку фрагмента по всьому сигналу, потім поставте маркери V на один кадр і виберіть у спливному меню { sw-popup-measure } > { raster-dt-menu-item }, щоб записати dT (період кадру), і відкрийте { guide-btn-raster } з цим періодом, щоб побачити кадри й синхрослова.

## Step buttons.

guide-btn-waveform = Осцилограма
guide-btn-sonogram = Сонограма
guide-btn-am-detector = AM-детектор
guide-btn-modified-am = Модифікований AM
guide-btn-phase-detector = Фазовий детектор
guide-btn-demod = Демодулятор FSK/MFSK
# $n: the power, a number.
guide-btn-power = Піднести до степеня { $n }
    .hover = Створює новий сигнал у степені { $n }; сигнал PSK-{ $n } тоді показує лінію на { $n }-кратній несучій
guide-btn-phase-plane = PhasePlane
guide-btn-raster = Растр (VMW)
guide-btn-adp = ADP
    .hover = Автоматично оцінити швидкість маніпуляції (ADP) на інтервалі маркерів V або на всьому сигналі
guide-btn-ofdm = OFDM
    .hover = Відкрити інструмент OFDM (Знайти CT, Уточнити Br, Список K)
guide-open-view = Відкрити цей вид сигналу активного вікна
guide-new-signal = Створює новий сигнал із сигналу активного вікна (відкривається в новому вікні)
# $kind: Br, Sc or dT (untranslated).
guide-not-measured = { $kind }: ще не виміряно
