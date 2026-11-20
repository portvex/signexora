# Окно демодулятора FSK-2 / универсального MFSK (SA_UI_SPEC.md раздел 8).

# Text panel.
demod-heading =
    SA - MFSK dem
    Levels: { $levels }
    Fd: { $fs }
    Br: { $br }
    Data as: { $form }
    Data:
demod-as-default = Стандартная таблица
demod-as-raw = Сырые данные
demod-as-bits = Биты, натуральный код
demod-as-gray = Биты, код Грея
demod-as-c4fm = Дибиты APCO-25 C4FM
demod-polarity-inverted = [полярность: инвертирована, биты по инвертированной таблице C4FM]
demod-polarity-normal = [полярность: нормальная]
demod-polarity-unknown = [полярность: неизвестна, биты по нормальной таблице C4FM]
demod-truncated = (первые 20000 символов; Копировать и Сохранить как берут всё)
demod-nothing-yet = Ещё ничего не демодулировано: проверьте Br, уровни и маркеры.

# Jobs.
demod-job-track = Демодулятор: частотная траектория
demod-job = Демодуляция FSK/MFSK

# Top bar.
demod-rrc = RRC
    .hover = Фильтр корня из приподнятого косинуса перед решениями; поле - его частота среза в Hz (по умолчанию Br)
demod-cutoff = Срез
    .hover = Частота среза RRC в Hz: введите значение или выберите из списка
demod-alpha = Альфа
    .hover = Коэффициент скругления RRC, от 0,05 до 1 (Enter или выход из поля применяет)
demod-br = Br
    .hover = Скорость манипуляции; Enter применяет
demod-br-step = Шаг
    .hover = Br больше или меньше на { $step }
demod-auto = A
    .hover = Автоматическая фазовая и битовая синхронизация
demod-auto-button = A
    .hover = Найти фазу автоматически сейчас и сохранить её (выключает автоматический режим)
demod-exactly = <= Exactly
    .hover = Точная фаза отсчёта (+-{ $span } % символа)
demod-roughly = <= Roughly
    .hover = Грубая фаза отсчёта в пределах символа
demod-levels = Levels
    .hover = Число уровней, от 2 до 256
demod-raw = Raw
    .hover = Выводить уровни как шестнадцатеричные байты, разделённые x

# Bottom bar.
demod-back-fast = Назад 10
    .hover = На 10 страниц назад
demod-back = Назад
    .hover = Предыдущая страница
demod-forward = Вперёд
    .hover = Следующая страница
demod-forward-fast = Вперёд 10
    .hover = На 10 страниц вперёд
demod-page = Страница
    .hover = Показанная страница (всего: { $count }); Enter переходит
demod-scale-up = Увеличить
    .hover = Увеличить вертикальный масштаб
demod-scale-down = Уменьшить
    .hover = Уменьшить вертикальный масштаб
demod-progress = Ход
    .hover = Ход вычисления (правый щелчок останавливает)
demod-stop = Остановить
demod-filter = Фильтр
    .hover = Применить фильтр RRC
demod-revert = Вернуть
    .hover = Вернуться к нефильтрованному сигналу
demod-d = D
    .hover = Вывести битовый поток в текстовую панель

# Plot.
demod-plot-name = Уровни демодулятора
demod-info = Br = { $baud } | такт { $clock } % | качество { $quality } | глаз { $eye } | символов: { $count }
demod-menu-tones = Уровни на найденных тонах ({ $count })
    .hover = Использовать как уровни тоны, найденные через Sum в WF, вместо равномерных зон
    .disabled = Сначала найдите тоны: WF, Sum, Найти тоны
demod-menu-average = Усредняющий фильтр
    .hover = Интегрировать по половине символа, когда RRC выключен (прямоугольная FSK)
demod-menu-adaptive = Адаптивные уровни
    .hover = Позволить центрам уровней следовать за медленным дрейфом данных
demod-menu-info = Info
demod-menu-reset-view = Сбросить вертикальный масштаб
demod-menu-cancel = Отмена

# Text panel popup.
demod-menu-copy = Копировать
demod-menu-save = Сохранить как...
demod-menu-data-as = Данные как
demod-save-title = Сохранить демодулированные данные
demod-save-filter = Текст
demod-save-failed = Не удалось записать { $path }: { $error }
demod-cutoff-invalid = Срез RRC "{ $text }" не является положительным числом; введите его в Hz, например Br.
demod-waiting-cutoff = Ожидается допустимая частота среза RRC: показан результат прежних настроек.
demod-br-invalid = Br "{ $text }" не является положительным числом; оставлен используемый Br.
