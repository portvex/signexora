# SA's file loader, the "SA Open file" window, and its "SA - select channel"
# question. Keep the window titles, the SA control captions (Use options, Data
# start, Channels, Bits, Freq rate, Wave tag, Bits per sample, Data size, Ext.
# Info) and the list entries (All - I/Q, Ch - 1, 16, 24-A, ...) untranslated.

loader-title = SA Open file
loader-ok = Ok
loader-cancel = Отмена

## Toolbar buttons.

loader-back = Назад
    .hover = Назад к предыдущей папке
loader-up = Вверх
    .hover = На одну папку вверх (Backspace)
loader-view = Вид
    .hover = Сменить вид: список или таблица
loader-refresh = Обновить
    .hover = Обновить список

## The list.

loader-col-name = Имя
loader-col-type = Тип/Расш
loader-col-size = Размер
loader-col-date = Дата
loader-col-time = Время
loader-type-folder = Папка
loader-type-link = F-Link
loader-popup-update = Обновить
loader-popup-auto-preview = Автопросмотр
loader-popup-copy = Копировать
loader-popup-paste = Вставить
loader-popup-cancel = Отмена
loader-search = Быстрый поиск
    .hover = Введите часть имени, чтобы выбрать первый файл, который её содержит
loader-filter-wav-mp3 = wav & mp3
loader-filter-wav = wav
loader-filter-mp3 = mp3
loader-filter-supported = все поддерживаемые
loader-filter-all = *.*
# $path: what was typed or pasted.
loader-path-not-found = Путь не найден: { $path }

## The info panel and the options.

loader-folder-link = Ссылка на папку
# $value: a number from the file header.
loader-info-tag = Wave tag = { $value }
loader-info-channels = Channels = { $value }
loader-info-rate = Freq rate = { $value }
loader-info-bits = Bits per sample = { $value }
loader-info-size = Data size = { $value }
loader-info-no-header = Нет заголовка
loader-info-unreadable = Неизвестный формат
# $codec: the codec name of the file, e.g. "Intel DVI ADPCM (IMA ADPCM)".
loader-ext-info = Ext. Info: { $codec }
loader-use-options = Use options
    .hover = Читать файл с этими настройками вместо заголовка (нужно для файлов без заголовка)
loader-use-options-compressed = Сжатые файлы декодируются по заголовку; с ручными параметрами читался бы шум.
loader-use-options-headerless = Данные без заголовка можно прочитать только с этими параметрами.
loader-data-start = Data start
    .hover = Байты, пропускаемые перед отсчетами
loader-channels = Channels
    .hover = Канал для загрузки, или All для первых двух как I/Q
loader-bits = Bits
    .hover = Формат отсчетов и порядок байтов
loader-freq-rate = Freq rate
    .hover = Частота дискретизации, Hz
loader-in-file = Каналов в файле
    .hover = Чередующиеся каналы этого файла без заголовка
# $error: why the file cannot be read.
loader-cannot-read = Невозможно прочитать файл: { $error }
loader-check-settings = Проверьте настройки!

## The preview.

loader-preview-building = Строится просмотр...
loader-prescaler = Предусиление
    .hover = Предусиление: { $gain } dB. Отсчёты выше полной шкалы (красные линии) ограничиваются.

## SA - select channel.

loader-select-channel-title = SA - select channel
loader-select-channel-group = Этот файл стерео, выберите
loader-channel-1 = Канал - 1
loader-channel-2 = Канал - 2
loader-all-samples = Все отсчёты
loader-break = Прервать
loader-needs-options = У файла нет заголовка, который SigNexora может прочитать: отметьте Use options и задайте формат.
loader-no-file = Сначала выберите файл.
loader-rate-invalid = Freq rate "{ $text }" недопустимо: введите частоту больше 0 и до 1000000000 Hz, например 48000.
loader-data-start-invalid = Data start "{ $text }" недопустимо: введите целое число байтов, например 0 или 44.
loader-list-error = Не удается прочитать папку { $path }: { $error }. Проверьте права или выберите другую папку.
