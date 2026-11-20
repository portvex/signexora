# Audio playback of a time span: errors and status messages.

playback-rate-too-low = Частота дискретизации слишком низкая для воспроизведения.
# $rate: the sample rate in Hz.
playback-rate-too-high = Воспроизводятся только сигналы звуковой частоты; у этого частота дискретизации { $rate } Hz. Сначала перенесите нужную полосу вниз и выполните передискретизацию.
playback-nothing = Нечего воспроизводить: выделите интервал времени маркерами V.
# $got and $total: seconds; $name: the signal, e.g. "n3".
playback-short-read = Прочитано только { $got } s из интервала { $total } s сигнала { $name }; возможно, файл на диске изменился. Откройте запись заново.
# $error: the audio system's error text.
playback-no-output = Не удаётся открыть аудиовыход: { $error }
playback-output-unavailable = Аудиовыход недоступен.
# $seconds: the length played.
playback-playing = Воспроизведение { $seconds } s
# $seconds: the length played; $limit: the playback limit at this rate, in seconds.
playback-playing-first-markers = Воспроизводятся только первые { $seconds } s (при этой частоте предел { $limit } s); чтобы прослушать остальное, переместите V1 в конец этой части.
playback-playing-first-view = Воспроизводятся только первые { $seconds } s (при этой частоте предел { $limit } s); чтобы прослушать остальное, поставьте на него маркеры V.
playback-bad-samples = { $count } отсчётов, не являющихся числами, воспроизводятся как тишина.
