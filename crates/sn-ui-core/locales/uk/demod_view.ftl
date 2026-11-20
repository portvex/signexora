# Вікно демодулятора FSK-2 / універсального MFSK (SA_UI_SPEC.md розділ 8).

# Text panel.
demod-heading =
    SA - MFSK dem
    Levels: { $levels }
    Fd: { $fs }
    Br: { $br }
    Data as: { $form }
    Data:
demod-as-default = Стандартна таблиця
demod-as-raw = Сирі дані
demod-as-bits = Біти, природний код
demod-as-gray = Біти, код Грея
demod-as-c4fm = Дибіти APCO-25 C4FM
demod-polarity-inverted = [полярність: інвертована, біти за інвертованою таблицею C4FM]
demod-polarity-normal = [полярність: нормальна]
demod-polarity-unknown = [полярність: невідома, біти за нормальною таблицею C4FM]
demod-truncated = (перші 20000 символів; Копіювати та Зберегти як беруть усе)
demod-nothing-yet = Ще нічого не демодульовано: перевірте Br, рівні та маркери.

# Jobs.
demod-job-track = Демодулятор: частотна траєкторія
demod-job = Демодуляція FSK/MFSK

# Top bar.
demod-rrc = RRC
    .hover = Фільтр кореня з піднятого косинуса перед рішеннями; поле - його частота зрізу в Hz (типово Br)
demod-cutoff = Зріз
    .hover = Частота зрізу RRC у Hz: введіть значення або виберіть зі списку
demod-alpha = Альфа
    .hover = Коефіцієнт скруглення RRC, від 0,05 до 1 (Enter або вихід з поля застосовує)
demod-br = Br
    .hover = Швидкість маніпуляції; Enter застосовує
demod-br-step = Крок
    .hover = Br більше або менше на { $step }
demod-auto = A
    .hover = Автоматична фазова та бітова синхронізація
demod-auto-button = A
    .hover = Знайти фазу автоматично зараз і зберегти її (вимикає автоматичний режим)
demod-exactly = <= Exactly
    .hover = Точна фаза відліку (+-{ $span } % символу)
demod-roughly = <= Roughly
    .hover = Груба фаза відліку в межах символу
demod-levels = Levels
    .hover = Кількість рівнів, від 2 до 256
demod-raw = Raw
    .hover = Виводити рівні як шістнадцяткові байти, розділені x

# Bottom bar.
demod-back-fast = Назад 10
    .hover = На 10 сторінок назад
demod-back = Назад
    .hover = Попередня сторінка
demod-forward = Вперед
    .hover = Наступна сторінка
demod-forward-fast = Вперед 10
    .hover = На 10 сторінок вперед
demod-page = Сторінка
    .hover = Показана сторінка (усього: { $count }); Enter переходить
demod-scale-up = Збільшити
    .hover = Збільшити вертикальний масштаб
demod-scale-down = Зменшити
    .hover = Зменшити вертикальний масштаб
demod-progress = Хід
    .hover = Хід обчислення (правий клік зупиняє)
demod-stop = Зупинити
demod-filter = Фільтр
    .hover = Застосувати фільтр RRC
demod-revert = Повернути
    .hover = Повернутися до нефільтрованого сигналу
demod-d = D
    .hover = Вивести бітовий потік у текстову панель

# Plot.
demod-plot-name = Рівні демодулятора
demod-info = Br = { $baud } | такт { $clock } % | якість { $quality } | око { $eye } | символів: { $count }
demod-menu-tones = Рівні на знайдених тонах ({ $count })
    .hover = Використати як рівні тони, знайдені через Sum у WF, замість рівномірних зон
    .disabled = Спершу знайдіть тони: WF, Sum, Знайти тони
demod-menu-average = Усереднювальний фільтр
    .hover = Інтегрувати по пів символу, коли RRC вимкнено (прямокутна FSK)
demod-menu-adaptive = Адаптивні рівні
    .hover = Дозволити центрам рівнів стежити за повільним дрейфом даних
demod-menu-info = Info
demod-menu-reset-view = Скинути вертикальний масштаб
demod-menu-cancel = Скасувати

# Text panel popup.
demod-menu-copy = Копіювати
demod-menu-save = Зберегти як...
demod-menu-data-as = Дані як
demod-save-title = Зберегти демодульовані дані
demod-save-filter = Текст
demod-save-failed = Не вдалося записати { $path }: { $error }
demod-cutoff-invalid = Зріз RRC "{ $text }" не є додатним числом; введіть його в Hz, наприклад Br.
demod-waiting-cutoff = Очікується допустима частота зрізу RRC: показано результат попередніх налаштувань.
demod-br-invalid = Br "{ $text }" не є додатним числом; залишено поточний Br.
