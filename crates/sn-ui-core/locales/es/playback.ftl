# Audio playback of a time span: errors and status messages.

playback-rate-too-low = La frecuencia de muestreo es demasiado baja para reproducir.
# $rate: the sample rate in Hz.
playback-rate-too-high = La reproducción es para señales a frecuencia de audio; esta está muestreada a { $rate } Hz. Desplace antes la banda de interés hacia abajo y remuestréela.
playback-nothing = Nada que reproducir: seleccione un intervalo de tiempo con los marcadores V.
# $got and $total: seconds; $name: the signal, e.g. "n3".
playback-short-read = Solo se pudieron leer { $got } s del intervalo de { $total } s de { $name }; puede que el archivo haya cambiado en disco. Vuelva a abrir la grabación.
# $error: the audio system's error text.
playback-no-output = No se puede abrir la salida de audio: { $error }
playback-output-unavailable = La salida de audio no está disponible.
# $seconds: the length played.
playback-playing = Reproduciendo { $seconds } s
# $seconds: the length played; $limit: the playback limit at this rate, in seconds.
playback-playing-first-markers = Reproduciendo solo los primeros { $seconds } s (la reproducción se limita a { $limit } s a esta frecuencia); mueva V1 al final de esta parte para oír el resto.
playback-playing-first-view = Reproduciendo solo los primeros { $seconds } s (la reproducción se limita a { $limit } s a esta frecuencia); coloque los marcadores V sobre el resto para oírlo.
playback-bad-samples = { $count } muestras que no eran números se reproducen como silencio.
