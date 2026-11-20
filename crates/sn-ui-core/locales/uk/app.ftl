# Main window (MDI frame): messages, File menu extras, the WF window's toolbar
# and popup menu, and the confirmation dialogs.

## View kinds (child windows)

app-tab-sonogram = Сонограма
app-tab-waveform = Осцилограма
app-tab-raster = Растр (VMW)
app-tab-phase = PhasePlane
app-tab-demod = Демодулятор FSK/MFSK
app-tab-ofdm = OFDM

## Lists of names: "A", "A and B", "A, B and C"

app-list-separator = { ", " }
app-list-and = { $rest } і { $last }

## Background jobs (shown in the status bar while they run)

app-job-loading = Завантаження { $name }
app-job-overview = Побудова огляду осцилограми
app-job-saving = Збереження { $path }
app-job-measure-line = Вимірювання частоти лінії
app-job-adp = Оцінка символьної швидкості (ADP)
app-job-writing = Запис { $path }
app-job-replay = Відтворення сеансу
app-job-script = Виконання скрипту
app-job-find-tones = Пошук тонів
app-job-playback = Підготовка відтворення
app-job-demodulator = Демодулятор

## Status messages

app-ready = Готово
app-signal-gone = Цього сигналу більше не існує.
app-set-h-markers-first = Спершу встановіть маркери H навколо лінії.
app-sc-unmappable = Sc не можна перенести через { $edit }: вихід цього кроку не має частотного зв'язку з входом. Виміряйте несучу до кроку { $edit }.
app-recorded = { $kind } = { $value } з { $node } ({ $tool })
app-recorded-uncertain = { $kind } = { $value } +/- { $uncertainty } з { $node } ({ $tool })
app-not-recorded = { $kind } не записано: { $error }
app-mapped-onto = { $from }, перенесено на { $node }
app-session-save-running = Збереження сеансу вже триває
app-export-report-failed = Експорт звіту: { $error }
app-save-session-failed = Збереження сеансу: { $error }
app-open-session-failed = Відкриття сеансу: { $error }
app-script-error = Помилка: { $error }
app-replay-skipped = { $what ->
    [markers] { $node } ({ $label }): збережені маркери не вдалося прочитати, їх пропущено: { $error }
    [tones] { $node } ({ $label }): збережені тони не вдалося прочитати, їх пропущено: { $error }
    [grid] { $node } ({ $label }): збережену сітку тонів не вдалося прочитати, її пропущено: { $error }
   *[adp] { $node } ({ $label }): збережених кандидатів ADP не вдалося прочитати, їх пропущено: { $error }
}
app-session-restored = Сеанс відновлено
app-session-restored-problems = { $count ->
    [one] Сеанс відновлено, { $count } проблема (див. вікно)
    [few] Сеанс відновлено, { $count } проблеми (див. вікно)
    [many] Сеанс відновлено, { $count } проблем (див. вікно)
   *[other] Сеанс відновлено, { $count } проблеми (див. вікно)
}
app-wrote-report = Звіт записано: { $path }
app-saved = Збережено: { $path }
app-saved-session = Сеанс збережено: { $path }
app-saved-session-replaced = Сеанс збережено: { $path } (сеанс, що був у файлі, замінено)
app-cancel-too-late = { $name }: завершено раніше, ніж спрацювало скасування; результат збережено.
app-job-cancelled = { $name }: скасовано
app-job-failed = { $name }: { $error }
app-playback-cancelled = Відтворення скасовано
app-derived-opened = { $label }: новий сигнал { $node } відкрито у { $view }
app-derived-opened-mapped = { $label }: новий сигнал { $node } відкрито у { $view }; маркери перенесено на його осі
app-derived-opened-cleared = { $label }: новий сигнал { $node } відкрито у { $view }; частину маркерів очищено: новий сигнал має іншу вісь часу або частоти
app-derive-failed = { $label }: { $error }
app-measured-line = { $kind } = { $value } +/- { $uncertainty } з { $node } (вимірювання лінії; { $level } dBFS, на { $snr } dB вище смуги)
app-measured-line-mapped = { $kind } = { $value } +/- { $uncertainty } з { $node } (вимірювання лінії; { $level } dBFS, на { $snr } dB вище смуги), лінію на { $line } Hz перераховано назад через степінь { $power }
app-loaded = Завантажено { $label }: { $samples } відліків при { $rate } Hz
app-loaded-notes = Завантажено { $label }: { $samples } відліків при { $rate } Hz. { $notes }
app-script-no-signals = Скрипт { $path } не створив жодного сигналу
app-script-finished = Скрипт { $path } завершено
app-adp-none = ADP не знайшов швидкості маніпуляції у виділенні
app-tones-found = { $count ->
    [one] Знайдено { $count } тон на { $node }
    [few] Знайдено { $count } тони на { $node }
    [many] Знайдено { $count } тонів на { $node }
   *[other] Знайдено { $count } тону на { $node }
}
app-markers-restored = Маркери відновлено на { $node }
app-closed-tab-stopped = { $count ->
    [one] Вікно { $name } закрито; завдання { $jobs }, що виконувалося, зупинено. Відкрийте вікно знову й запустіть його ще раз.
    [few] Вікно { $name } закрито; завдання { $jobs }, що виконувалися, зупинено. Відкрийте вікно знову й запустіть їх ще раз.
    [many] Вікно { $name } закрито; завдання { $jobs }, що виконувалися, зупинено. Відкрийте вікно знову й запустіть їх ще раз.
   *[other] Вікно { $name } закрито; завдання { $jobs }, що виконувалися, зупинено. Відкрийте вікно знову й запустіть їх ще раз.
}
app-nothing-to-play = Нічого відтворювати: відкрийте сонограму або встановіть маркери V.
app-find-tones-failed = Пошук тонів: { $error }

## Confirmation before work is lost

app-confirm-open-title = Відкрити сеанс
app-confirm-open-question = Замінити поточний сеанс?
app-confirm-replace = Замінити
app-confirm-script-title = Виконати скрипт
app-confirm-script-question = Сигнали скрипту замінять поточний сеанс.
app-confirm-quit-title = Вихід
app-confirm-quit-unsaved = Зберегти сеанс перед виходом?
app-confirm-quit-without-saving = Вийти без збереження
app-confirm-quit-writing = Файл ще записується.
app-confirm-quit-anyway = Все одно вийти
app-confirm-lost = Буде закрито незбережені { $signals ->
    [one] { $signals } сигнал
    [few] { $signals } сигнали
    [many] { $signals } сигналів
   *[other] { $signals } сигналу
} і { $measurements ->
    [one] { $measurements } вимірювання
    [few] { $measurements } вимірювання
    [many] { $measurements } вимірювань
   *[other] { $measurements } вимірювання
}. Цю дію не можна скасувати.
app-confirm-write-running = { $name }: ще виконується; вихід зараз перерве цю операцію.
app-confirm-save-first = Спершу зберегти сеанс
app-confirm-save-first-ask = Спершу зберегти сеанс...
app-confirm-wait = Зачекати
app-confirm-keep-open = Не закривати програму
app-cancel = Скасувати

## File dialogs

app-dialog-save-as = Зберегти як { $format }
app-dialog-save-session-as = Зберегти сеанс як
app-filter-session = Сеанс SigNexora
app-dialog-export-report = Експорт звіту аналізу
app-filter-report = Звіт
app-dialog-open-session = Відкрити сеанс
app-dialog-run-script = Виконати скрипт
app-filter-rhai = Скрипт Rhai

## File menu

app-menu-file = Файл
app-menu-open-session = Відкрити сеанс...
    .hover = Його сигнали замінять поточний сеанс
app-nothing-to-save = Нічого зберігати
app-menu-save-session = Зберегти сеанс
app-menu-save-session-ask = Зберегти сеанс...
app-menu-save-session-choose = Виберіть файл для сеансу
app-menu-save-session-to = Зберегти в { $path }
app-menu-save-session-as = Зберегти сеанс як...
app-menu-export-report = Експорт звіту аналізу
app-menu-report-html = HTML...
app-menu-report-markdown = Markdown...

## File menu extras and the WF popup menu

app-menu-run-script = Виконати скрипт (Rhai)...
    .hover = Виконує скрипт; його сигнали замінять поточний сеанс
app-resynth = Ресинтез FSK/MFSK
    .hover = Згенерувати чистий сигнал на тонах із «Пошуку тонів» зі швидкістю Br
    .disabled = Спершу знайдіть тони (Sum на осцилограмі) і виміряйте Br.
app-menu-measure = Вимірювання
app-measure-hint = Встановіть маркери H навколо лінії (і маркери V для інтервалу).
app-measure-br = Виміряти лінію як символьну швидкість (Br)
app-measure-sc = Виміряти лінію як несучу (Sc)
    .hover = Після піднесення до степеня N лінію перераховано назад на несучу (поділено на N, зсуви скасовано)
app-measure-adp = Автоматична оцінка символьної швидкості (ADP)
    .hover = Використовує інтервал між маркерами V або весь сигнал
app-measure-dt-disabled = Спершу встановіть маркери V.

## Hints

app-no-signals-hint = Відкрийте запис (Файл > Відкрити файл... ({ $shortcut }) або перетягніть файл у вікно).
app-open-in = Відкрити...
    .hover = Відкрити цей сигнал в іншому поданні чи інструменті

## Sonogram toolbar

app-sono-shear = Shear
    .hover = Крок між кадрами FFT у частках розміру FFT. Менший крок сильніше перекриває кадри: точніше за часом, але довший розрахунок
app-sono-image = Image
    .hover = Динамічний діапазон, показаний нижче найсильнішої складової
app-sono-hres-hover = Загострити спектральні лінії перепризначенням частоти (допомагає знайти слабкі тактові лінії). Тони зберігають рівень; шум виглядає приблизно на 2-6 dB нижчим, ніж без HRes (залежно від вікна), бо показується на ширину біна.
app-sono-alc-hover = Нормувати кожен стовпець за його найсильнішою складовою (лише відображення)
app-sono-combine = При зменшенні масштабу
    .hover = Як об'єднуються при зменшенні масштабу кадри FFT, що потрапляють в один стовпець зображення
app-sono-peak = Пік
    .hover = При зменшенні масштабу: короткі пакети лишаються видимими
app-sono-mean = Середнє
    .hover = При зменшенні масштабу: усереднення, гладкіший шум

## Waveform toolbar

app-wave-show-q = Показати Q
app-find-tones = Знайти тони
    .hover = Піки гістограми: частоти тонів на виході фазового детектора

## Zoom and playback

app-play = Відтворити
    .hover = Відтворити інтервал між маркерами V або видимий інтервал ({ $shortcut })
app-play-stop = Стоп
    .hover = Зупинити відтворення ({ $shortcut })

## Child windows

app-tab-signal-gone = Цього сигналу більше не існує.

## Windows

app-replay-problems-title = Сеанс відновлено з проблемами
app-script-output = Вивід скрипту: { $path }
app-script-no-output = (виводу немає)
# $lines: the lines kept; $dropped: the lines left out.
app-script-output-truncated = Вивід обрізано після { $lines } рядків; ще { $dropped } рядків не збережено.

## Closing a window, the sonogram toolbar

app-confirm-close-tab-title = Закрити вікно?
# $name: the window title; $jobs: the job names, joined.
app-confirm-close-tab-question = { $count ->
    [one] Закриття { $name } зупинить запущене завдання { $jobs }. Усе одно закрити?
   *[other] Закриття { $name } зупинить запущені завдання { $jobs }. Усе одно закрити?
}
app-confirm-close-and-stop = Закрити й зупинити
app-confirm-keep-tab = Залишити відкритою
# $prev, $next: the keys that show the previous and next signal.
app-select-signal-first = Спершу виведіть вікно на передній план ({ $prev } або { $next } показують сигнали по черзі).
app-sono-fft = SizeFFT
    .hover = Розмір FFT: більший дає точнішу роздільність за частотою і грубішу за часом
app-sono-window = WinF
    .hover = Віконна функція кожного FFT (WinF у SA)
app-sono-colormap = Кольори
    .hover = Колірна схема зображення сонограми
app-confirm-close-all-title = Закрити всі вікна
app-confirm-close-all-question = Це видалить усі сигнали та вимірювання із сеансу.
app-confirm-close-all = Закрити все
app-confirm-close-all-stops = { $name } ще виконується; закриття всіх вікон зупинить його.
app-replay-session-skipped = { $what ->
    [windows] Не вдалося прочитати збережене розташування вікон, тому для кожного результату відкривається вікно: { $error }
   *[counters] Не вдалося прочитати лічильники назв, нові назви можуть повторювати номери: { $error }
}
app-replay-windows-missing = { $count ->
    [one] { $count } збережене вікно не відкрито, бо його сигнал не відтворено
    [few] { $count } збережені вікна не відкрито, бо їхні сигнали не відтворено
    [many] { $count } збережених вікон не відкрито, бо їхні сигнали не відтворено
   *[other] { $count } збереженого вікна не відкрито, бо сигнали не відтворено
}
app-drop-loader-open = Діалог файлу відкрито: завершіть або скасуйте його, потім перетягніть файл знову
app-drop-first-only = Перетягнуто файлів: { $count }; відкривається лише перший, { $name }
app-sono-target = { $active ->
    [yes] Змінює активне вікно сонограми.
   *[no] Змінює типове значення для нових сонограм.
}
app-edit-unrecordable = не вдалося записати в сеанс для відтворення ({ $error })
app-empty-open = Відкрити запис... ({ $key })
app-empty-drop = або перетягніть файл сюди (також File > Open file...).
app-empty-guide = Посібник ({ $key }) показує аналіз крок за кроком.
# $action: what the chooser was for, e.g. "Open session"; $error: why it failed.
app-chooser-failed = { $action }: не вдалося показати вікно вибору файлу ({ $error }). Спробуйте ще раз або перевірте, що системна служба файлових діалогів (desktop portal) запущена.

## The tools that record a measurement, by the id stored in the session.

measure-tool-adp = Автовизначення параметрів (ADP)
measure-tool-adp-coarse = Автовизначення параметрів, грубо (з точністю до половини елемента FFT)
measure-tool-measure-line = Вимірювання частоти (лінія вимірювання)
measure-tool-v-markers = V-маркери (dT)
measure-tool-ofdm-adp = Інструмент OFDM, Get Br (ADP)
measure-tool-script-record = Скрипт, record()
