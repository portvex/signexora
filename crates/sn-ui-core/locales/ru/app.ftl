# Main window (MDI frame): messages, File menu extras, the WF window's toolbar
# and popup menu, and the confirmation dialogs.

## View kinds (child windows)

app-tab-sonogram = Сонограмма
app-tab-waveform = Осциллограмма
app-tab-raster = Растр (VMW)
app-tab-phase = Фазовая плоскость
app-tab-demod = Демодулятор FSK/MFSK
app-tab-ofdm = OFDM

## Lists of names: "A", "A and B", "A, B and C"

app-list-separator = { ", " }
app-list-and = { $rest } и { $last }

## Background jobs (shown in the status bar while they run)

app-job-loading = Загрузка { $name }
app-job-overview = Построение обзора осциллограммы
app-job-saving = Сохранение { $path }
app-job-measure-line = Измерение частоты линии
app-job-adp = Оценка скорости манипуляции (ADP)
app-job-writing = Запись { $path }
app-job-replay = Восстановление сессии
app-job-script = Выполнение скрипта
app-job-find-tones = Поиск тонов
app-job-playback = Подготовка воспроизведения
app-job-demodulator = Демодулятор

## Status messages

app-ready = Готово
app-signal-gone = Этого сигнала больше нет.
app-set-h-markers-first = Сначала поставьте маркеры H вокруг линии.
app-sc-unmappable = Sc нельзя пересчитать через { $edit }: частоты на выходе не связаны с частотами на входе. Измерьте несущую до шага { $edit }.
app-recorded = { $kind } = { $value } по { $node } ({ $tool })
app-recorded-uncertain = { $kind } = { $value } +/- { $uncertainty } по { $node } ({ $tool })
app-not-recorded = { $kind } не записано: { $error }
app-mapped-onto = { $from }, пересчитано на { $node }
app-session-save-running = Сессия уже сохраняется
app-export-report-failed = Экспорт отчёта: { $error }
app-save-session-failed = Сохранение сессии: { $error }
app-open-session-failed = Открытие сессии: { $error }
app-script-error = Ошибка: { $error }
app-replay-skipped = { $what ->
    [markers] { $node } ({ $label }): сохранённые маркеры не прочитаны и пропущены: { $error }
    [tones] { $node } ({ $label }): сохранённые тоны не прочитаны и пропущены: { $error }
    [grid] { $node } ({ $label }): сохранённая сетка тонов не прочитана и пропущена: { $error }
   *[adp] { $node } ({ $label }): сохранённые кандидаты ADP не прочитаны и пропущены: { $error }
}
app-session-restored = Сессия восстановлена
app-session-restored-problems = { $count ->
    [one] Сессия восстановлена, { $count } проблема (см. окно)
    [few] Сессия восстановлена, { $count } проблемы (см. окно)
    [many] Сессия восстановлена, { $count } проблем (см. окно)
   *[other] Сессия восстановлена, { $count } проблемы (см. окно)
}
app-wrote-report = Отчёт записан: { $path }
app-saved = Сохранено: { $path }
app-saved-session = Сессия сохранена: { $path }
app-saved-session-replaced = Сессия сохранена: { $path } (прежняя сессия в файле заменена)
app-cancel-too-late = { $name } завершилось до отмены; результат сохранён.
app-job-cancelled = { $name }: отменено
app-job-failed = { $name }: { $error }
app-playback-cancelled = Воспроизведение отменено
app-derived-opened = { $label }: новый сигнал { $node } открыт в { $view }
app-derived-opened-mapped = { $label }: новый сигнал { $node } открыт в { $view }; маркеры перенесены на его оси
app-derived-opened-cleared = { $label }: новый сигнал { $node } открыт в { $view }; часть маркеров сброшена: у нового сигнала другая ось времени или частоты
app-derive-failed = { $label }: { $error }
app-measured-line = { $kind } = { $value } +/- { $uncertainty } по { $node } (измерение линии; { $level } dBFS, на { $snr } dB выше полосы)
app-measured-line-mapped = { $kind } = { $value } +/- { $uncertainty } по { $node } (измерение линии; { $level } dBFS, на { $snr } dB выше полосы), линия на { $line } Hz пересчитана обратно через степень { $power }
app-loaded = Загружено { $label }: { $samples } отсчётов, { $rate } Hz
app-loaded-notes = Загружено { $label }: { $samples } отсчётов, { $rate } Hz. { $notes }
app-script-no-signals = Скрипт { $path } не создал сигналов
app-script-finished = Скрипт { $path } выполнен
app-adp-none = ADP не нашёл скорости манипуляции в выделении
app-tones-found = { $count ->
    [one] Найден { $count } тон на { $node }
    [few] Найдено { $count } тона на { $node }
    [many] Найдено { $count } тонов на { $node }
   *[other] Найдено { $count } тона на { $node }
}
app-markers-restored = Маркеры восстановлены на { $node }
app-closed-tab-stopped = { $count ->
    [one] Окно { $name } закрыто; выполнявшееся задание { $jobs } остановлено. Откройте окно снова и запустите его заново.
    [few] Окно { $name } закрыто; выполнявшиеся задания { $jobs } остановлены. Откройте окно снова и запустите их заново.
    [many] Окно { $name } закрыто; выполнявшиеся задания { $jobs } остановлены. Откройте окно снова и запустите их заново.
   *[other] Окно { $name } закрыто; выполнявшиеся задания { $jobs } остановлены. Откройте окно снова и запустите их заново.
}
app-nothing-to-play = Нечего воспроизводить: откройте сонограмму или поставьте маркеры V.
app-find-tones-failed = Поиск тонов: { $error }

## Confirmation before work is lost

app-confirm-open-title = Открыть сессию
app-confirm-open-question = Заменить текущую сессию?
app-confirm-replace = Заменить
app-confirm-script-title = Запуск скрипта
app-confirm-script-question = Сигналы скрипта заменят текущую сессию.
app-confirm-quit-title = Выход
app-confirm-quit-unsaved = Сохранить сессию перед выходом?
app-confirm-quit-without-saving = Выйти без сохранения
app-confirm-quit-writing = Файл ещё записывается.
app-confirm-quit-anyway = Всё равно выйти
app-confirm-lost = Будут закрыты несохранённые данные: { $signals ->
    [one] { $signals } сигнал
    [few] { $signals } сигнала
    [many] { $signals } сигналов
   *[other] { $signals } сигнала
} и { $measurements ->
    [one] { $measurements } измерение
    [few] { $measurements } измерения
    [many] { $measurements } измерений
   *[other] { $measurements } измерения
}. Это нельзя отменить.
app-confirm-write-running = { $name } ещё выполняется; при выходе сейчас оно будет прервано.
app-confirm-save-first = Сначала сохранить сессию
app-confirm-save-first-ask = Сначала сохранить сессию...
app-confirm-wait = Подождать
app-confirm-keep-open = Не закрывать программу
app-cancel = Отмена

## File dialogs

app-dialog-save-as = Сохранить как { $format }
app-dialog-save-session-as = Сохранить сессию как
app-filter-session = Сессия SigNexora
app-dialog-export-report = Экспорт отчёта об анализе
app-filter-report = Отчёт
app-dialog-open-session = Открыть сессию
app-dialog-run-script = Запуск скрипта
app-filter-rhai = Скрипт Rhai

## File menu

app-menu-file = Файл
app-menu-open-session = Открыть сессию...
    .hover = Её сигналы заменят текущую сессию
app-nothing-to-save = Нечего сохранять
app-menu-save-session = Сохранить сессию
app-menu-save-session-ask = Сохранить сессию...
app-menu-save-session-choose = Выбрать файл для сессии
app-menu-save-session-to = Сохранить в { $path }
app-menu-save-session-as = Сохранить сессию как...
app-menu-export-report = Экспорт отчёта об анализе
app-menu-report-html = HTML...
app-menu-report-markdown = Markdown...

## File menu extras and the WF popup menu

app-menu-run-script = Запустить скрипт (Rhai)...
    .hover = Выполняет скрипт; его сигналы заменят текущую сессию
app-resynth = Ресинтез FSK/MFSK
    .hover = Сформировать чистый сигнал на тонах из «Найти тоны» со скоростью Br
    .disabled = Сначала найдите тоны (Сумма на осциллограмме) и измерьте Br.
app-menu-measure = Измерение
app-measure-hint = Поставьте маркеры H вокруг линии (и маркеры V для интервала).
app-measure-br = Измерить линию как скорость манипуляции (Br)
app-measure-sc = Измерить линию как несущую (Sc)
    .hover = После возведения в степень N линия пересчитывается к несущей (делится на N, сдвиги отменяются)
app-measure-adp = Оценить скорость манипуляции автоматически (ADP)
    .hover = Используется интервал маркеров V или весь сигнал
app-measure-dt-disabled = Сначала поставьте маркеры V.

## Hints

app-no-signals-hint = Откройте запись (Файл > Открыть файл... ({ $shortcut }) или перетащите файл в окно).
app-open-in = Открыть...
    .hover = Открыть этот сигнал в другом виде или инструменте

## Sonogram toolbar

app-sono-shear = Shear
    .hover = Шаг между кадрами FFT в долях размера FFT. Меньший шаг сильнее перекрывает кадры: точнее по времени, но дольше расчёт
app-sono-image = Image
    .hover = Отображаемый динамический диапазон ниже самой сильной составляющей
app-sono-hres-hover = Повысить резкость спектральных линий переназначением частоты (помогает найти слабые тактовые линии). Тоны сохраняют уровень; шум выглядит примерно на 2-6 dB ниже, чем без HRes (зависит от окна), так как показывается на ширину бина.
app-sono-alc-hover = Нормировать каждый столбец по его самой сильной составляющей (только отображение)
app-sono-combine = При уменьшении
    .hover = Как объединяются при уменьшении кадры FFT, попадающие в один столбец изображения
app-sono-peak = Пик
    .hover = При уменьшении: сохранять видимость коротких пачек
app-sono-mean = Среднее
    .hover = При уменьшении: усреднение, более гладкий шум

## Waveform toolbar

app-wave-show-q = Показать Q
app-find-tones = Найти тоны
    .hover = Пики гистограммы: частоты тонов на выходе фазового детектора

## Zoom and playback

app-play = Пуск
    .hover = Воспроизвести интервал маркеров V или видимый интервал ({ $shortcut })
app-play-stop = Стоп
    .hover = Остановить воспроизведение ({ $shortcut })

## Child windows

app-tab-signal-gone = Этого сигнала больше нет.

## Windows

app-replay-problems-title = Сессия восстановлена с проблемами
app-script-output = Вывод скрипта: { $path }
app-script-no-output = (нет вывода)
# $lines: the lines kept; $dropped: the lines left out.
app-script-output-truncated = Вывод обрезан после { $lines } строк; ещё { $dropped } строк не сохранено.

## Closing a window, the sonogram toolbar

app-confirm-close-tab-title = Закрыть окно?
# $name: the window title; $jobs: the job names, joined.
app-confirm-close-tab-question = { $count ->
    [one] Закрытие { $name } остановит выполняемую задачу { $jobs }. Всё равно закрыть?
   *[other] Закрытие { $name } остановит выполняемые задачи { $jobs }. Всё равно закрыть?
}
app-confirm-close-and-stop = Закрыть и остановить
app-confirm-keep-tab = Оставить открытой
# $prev, $next: the keys that show the previous and next signal.
app-select-signal-first = Сначала выведите окно на передний план ({ $prev } или { $next } показывают сигналы по очереди).
app-sono-fft = SizeFFT
    .hover = Размер FFT: больший размер даёт более точное разрешение по частоте и более грубое по времени
app-sono-window = WinF
    .hover = Оконная функция каждого FFT (WinF в SA)
app-sono-colormap = Цвета
    .hover = Цветовая схема изображения сонограммы
app-confirm-close-all-title = Закрыть все окна
app-confirm-close-all-question = Это удалит все сигналы и измерения из сессии.
app-confirm-close-all = Закрыть все
app-confirm-close-all-stops = { $name } еще выполняется; закрытие всех окон остановит его.
app-replay-session-skipped = { $what ->
    [windows] Не удалось прочитать сохраненное расположение окон, поэтому для каждого результата открывается окно: { $error }
   *[counters] Не удалось прочитать счетчики имен, новые имена могут повторять номера: { $error }
}
app-replay-windows-missing = { $count ->
    [one] { $count } сохраненное окно не открыто, так как его сигнал не воспроизведен
    [few] { $count } сохраненных окна не открыты, так как их сигналы не воспроизведены
    [many] { $count } сохраненных окон не открыто, так как их сигналы не воспроизведены
   *[other] { $count } сохраненного окна не открыто, так как сигналы не воспроизведены
}
app-drop-loader-open = Диалог файла открыт: завершите или отмените его, затем перетащите файл снова
app-drop-first-only = Перетащено файлов: { $count }; открывается только первый, { $name }
app-sono-target = { $active ->
    [yes] Меняет активное окно сонограммы.
   *[no] Меняет значение по умолчанию для новых сонограмм.
}
app-edit-unrecordable = не удалось записать в сессию для воспроизведения ({ $error })
app-empty-open = Открыть запись... ({ $key })
app-empty-drop = или перетащите файл сюда (также File > Open file...).
app-empty-guide = Руководство ({ $key }) показывает анализ шаг за шагом.
# $action: what the chooser was for, e.g. "Open session"; $error: why it failed.
app-chooser-failed = { $action }: не удалось показать окно выбора файла ({ $error }). Повторите попытку или проверьте, что системная служба файловых диалогов (desktop portal) запущена.

## The tools that record a measurement, by the id stored in the session.

measure-tool-adp = Автоопределение параметров (ADP)
measure-tool-adp-coarse = Автоопределение параметров, грубо (с точностью до половины элемента FFT)
measure-tool-measure-line = Измерение частоты (линия измерения)
measure-tool-v-markers = V-маркеры (dT)
measure-tool-ofdm-adp = Инструмент OFDM, Get Br (ADP)
measure-tool-script-record = Скрипт, record()
