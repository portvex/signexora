# Audio playback of a time span: errors and status messages.

playback-rate-too-low = Частота дискретизації занизька для відтворення.
# $rate: the sample rate in Hz.
playback-rate-too-high = Відтворення призначене для сигналів звукової частоти; цей дискретизовано на { $rate } Hz. Спершу зсуньте потрібну смугу вниз і передискретизуйте її.
playback-nothing = Нічого відтворювати: виділіть інтервал часу маркерами V.
# $got and $total: seconds; $name: the signal, e.g. "n3".
playback-short-read = Вдалося прочитати лише { $got } s з інтервалу { $total } s сигналу { $name }; можливо, файл на диску змінився. Відкрийте запис знову.
# $error: the audio system's error text.
playback-no-output = Не вдається відкрити аудіовихід: { $error }
playback-output-unavailable = Аудіовихід недоступний.
# $seconds: the length played.
playback-playing = Відтворення { $seconds } s
# $seconds: the length played; $limit: the playback limit at this rate, in seconds.
playback-playing-first-markers = Відтворюються лише перші { $seconds } s (на цій частоті відтворення обмежено { $limit } s); щоб почути решту, перемістіть V1 у кінець цієї частини.
playback-playing-first-view = Відтворюються лише перші { $seconds } s (на цій частоті відтворення обмежено { $limit } s); щоб почути решту, встановіть на неї маркери V.
playback-bad-samples = { $count } відліків, що не є числами, відтворюються як тиша.
