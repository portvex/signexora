# SA's file loader, the "SA Open file" window, and its "SA - select channel"
# question. Keep the window titles, the SA control captions (Use options, Data
# start, Channels, Bits, Freq rate, Wave tag, Bits per sample, Data size, Ext.
# Info) and the list entries (All - I/Q, Ch - 1, 16, 24-A, ...) untranslated.

loader-title = SA Open file
loader-ok = Ok
loader-cancel = Скасувати

## Toolbar buttons.

loader-back = Назад
    .hover = Назад до попередньої теки
loader-up = Вгору
    .hover = На одну теку вгору (Backspace)
loader-view = Вигляд
    .hover = Змінити вигляд: список або таблиця
loader-refresh = Оновити
    .hover = Оновити список

## The list.

loader-col-name = Ім'я
loader-col-type = Тип/Розш
loader-col-size = Розмір
loader-col-date = Дата
loader-col-time = Час
loader-type-folder = Тека
loader-type-link = F-Link
loader-popup-update = Оновити
loader-popup-auto-preview = Автоперегляд
loader-popup-copy = Копіювати
loader-popup-paste = Вставити
loader-popup-cancel = Скасувати
loader-search = Швидкий пошук
    .hover = Введіть частину імені, щоб вибрати перший файл, що її містить
loader-filter-wav-mp3 = wav & mp3
loader-filter-wav = wav
loader-filter-mp3 = mp3
loader-filter-supported = усі підтримувані
loader-filter-all = *.*
# $path: what was typed or pasted.
loader-path-not-found = Шлях не знайдено: { $path }

## The info panel and the options.

loader-folder-link = Посилання на теку
# $value: a number from the file header.
loader-info-tag = Wave tag = { $value }
loader-info-channels = Channels = { $value }
loader-info-rate = Freq rate = { $value }
loader-info-bits = Bits per sample = { $value }
loader-info-size = Data size = { $value }
loader-info-no-header = Немає заголовка
loader-info-unreadable = Невідомий формат
# $codec: the codec name of the file, e.g. "Intel DVI ADPCM (IMA ADPCM)".
loader-ext-info = Ext. Info: { $codec }
loader-use-options = Use options
    .hover = Читати файл із цими налаштуваннями замість заголовка (потрібно для файлів без заголовка)
loader-use-options-compressed = Стиснені файли декодуються за заголовком; з ручними параметрами читався б шум.
loader-use-options-headerless = Дані без заголовка можна прочитати лише з цими параметрами.
loader-data-start = Data start
    .hover = Байти, які пропускаються перед відліками
loader-channels = Channels
    .hover = Канал для завантаження, або All для перших двох як I/Q
loader-bits = Bits
    .hover = Формат відліків і порядок байтів
loader-freq-rate = Freq rate
    .hover = Частота дискретизації, Hz
loader-in-file = Каналів у файлі
    .hover = Перемежовані канали цього файлу без заголовка
# $error: why the file cannot be read.
loader-cannot-read = Неможливо прочитати файл: { $error }
loader-check-settings = Перевірте налаштування!

## The preview.

loader-preview-building = Будується перегляд...
loader-prescaler = Передпідсилення
    .hover = Передпідсилення: { $gain } dB. Відліки понад повну шкалу (червоні лінії) обмежуються.

## SA - select channel.

loader-select-channel-title = SA - select channel
loader-select-channel-group = Цей файл стерео, виберіть
loader-channel-1 = Канал - 1
loader-channel-2 = Канал - 2
loader-all-samples = Усі відліки
loader-break = Перервати
loader-needs-options = Файл не має заголовка, який SigNexora може прочитати: позначте Use options і задайте формат.
loader-no-file = Спершу виберіть файл.
loader-rate-invalid = Freq rate "{ $text }" недійсне: введіть частоту більше 0 і до 1000000000 Hz, наприклад 48000.
loader-data-start-invalid = Data start "{ $text }" недійсне: введіть ціле число байтів, наприклад 0 або 44.
loader-list-error = Не вдається прочитати теку { $path }: { $error }. Перевірте права або виберіть іншу теку.
