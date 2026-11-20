# Main window (MDI frame): messages, File menu extras, the WF window's toolbar
# and popup menu, and the confirmation dialogs.

## View kinds (child windows)

app-tab-sonogram = Sonograma
app-tab-waveform = Forma de onda
app-tab-raster = Raster (VMW)
app-tab-phase = PhasePlane
app-tab-demod = Demodulador FSK/MFSK
app-tab-ofdm = OFDM

## Lists of names: "A", "A and B", "A, B and C"

app-list-separator = { ", " }
app-list-and = { $rest } y { $last }

## Background jobs (shown in the status bar while they run)

app-job-loading = Cargando { $name }
app-job-overview = Creando vista general de la forma de onda
app-job-saving = Guardando { $path }
app-job-measure-line = Midiendo frecuencia de la línea
app-job-adp = Estimando velocidad en baudios (ADP)
app-job-writing = Escribiendo { $path }
app-job-replay = Reproduciendo sesión
app-job-script = Ejecutando script
app-job-find-tones = Buscando tonos
app-job-playback = Preparando reproducción
app-job-demodulator = Demodulador

## Status messages

app-ready = Listo
app-signal-gone = Esa señal ya no existe.
app-set-h-markers-first = Coloque primero los marcadores H alrededor de la línea.
app-sc-unmappable = Sc no se puede transferir a través de { $edit }: su salida no tiene relación de frecuencia con su entrada. Mida la portadora antes del paso { $edit }.
app-recorded = { $kind } = { $value } de { $node } ({ $tool })
app-recorded-uncertain = { $kind } = { $value } +/- { $uncertainty } de { $node } ({ $tool })
app-not-recorded = { $kind } no registrado: { $error }
app-mapped-onto = { $from }, transferido a { $node }
app-session-save-running = Ya se está guardando una sesión
app-export-report-failed = Exportar informe: { $error }
app-save-session-failed = Guardar sesión: { $error }
app-open-session-failed = Abrir sesión: { $error }
app-script-error = Error: { $error }
app-replay-skipped = { $what ->
    [markers] { $node } ({ $label }): no se pudieron leer los marcadores guardados y se omitieron: { $error }
    [tones] { $node } ({ $label }): no se pudieron leer los tonos guardados y se omitieron: { $error }
    [grid] { $node } ({ $label }): no se pudo leer la rejilla de tonos guardada y se omitió: { $error }
   *[adp] { $node } ({ $label }): no se pudieron leer los candidatos ADP guardados y se omitieron: { $error }
}
app-session-restored = Sesión restaurada
app-session-restored-problems = { $count ->
    [one] Sesión restaurada, 1 problema (vea la ventana)
   *[other] Sesión restaurada, { $count } problemas (vea la ventana)
}
app-wrote-report = Informe escrito: { $path }
app-saved = Guardado: { $path }
app-saved-session = Sesión guardada: { $path }
app-saved-session-replaced = Sesión guardada: { $path } (se reemplazó la sesión que contenía)
app-cancel-too-late = { $name } terminó antes de que la cancelación surtiera efecto; se conservó el resultado.
app-job-cancelled = { $name }: cancelado
app-job-failed = { $name }: { $error }
app-playback-cancelled = Reproducción cancelada
app-derived-opened = { $label }: nueva señal { $node } abierta en { $view }
app-derived-opened-mapped = { $label }: nueva señal { $node } abierta en { $view }; marcadores trasladados a sus ejes
app-derived-opened-cleared = { $label }: nueva señal { $node } abierta en { $view }; se borraron algunos marcadores: la nueva señal tiene otro eje de tiempo o de frecuencia
app-derive-failed = { $label }: { $error }
app-measured-line = { $kind } = { $value } +/- { $uncertainty } de { $node } (medición de línea; { $level } dBFS, { $snr } dB sobre la banda)
app-measured-line-mapped = { $kind } = { $value } +/- { $uncertainty } de { $node } (medición de línea; { $level } dBFS, { $snr } dB sobre la banda), línea en { $line } Hz transferida a través de la potencia { $power }
app-loaded = Cargado { $label }: { $samples } muestras a { $rate } Hz
app-loaded-notes = Cargado { $label }: { $samples } muestras a { $rate } Hz. { $notes }
app-script-no-signals = El script { $path } no produjo señales
app-script-finished = Script { $path } terminado
app-adp-none = ADP no encontró velocidad de manipulación en la selección
app-tones-found = { $count ->
    [one] Se encontró 1 tono en { $node }
   *[other] Se encontraron { $count } tonos en { $node }
}
app-markers-restored = Marcadores restaurados en { $node }
app-closed-tab-stopped = { $count ->
    [one] Se cerró { $name }; se detuvo su tarea { $jobs } en curso. Vuelva a abrir la ventana y ejecútela de nuevo.
   *[other] Se cerró { $name }; se detuvieron sus tareas { $jobs } en curso. Vuelva a abrir la ventana y ejecútelas de nuevo.
}
app-nothing-to-play = Nada que reproducir: abra el sonograma o coloque los marcadores V.
app-find-tones-failed = Buscar tonos: { $error }

## Confirmation before work is lost

app-confirm-open-title = Abrir sesión
app-confirm-open-question = ¿Reemplazar la sesión actual?
app-confirm-replace = Reemplazar
app-confirm-script-title = Ejecutar script
app-confirm-script-question = Las señales del script reemplazan la sesión actual.
app-confirm-quit-title = Salir
app-confirm-quit-unsaved = ¿Guardar la sesión antes de salir?
app-confirm-quit-without-saving = Salir sin guardar
app-confirm-quit-writing = Todavía se está escribiendo un archivo.
app-confirm-quit-anyway = Salir de todos modos
app-confirm-lost = Se cerrarán { $signals ->
    [one] 1 señal
   *[other] { $signals } señales
} y { $measurements ->
    [one] 1 medición
   *[other] { $measurements } mediciones
} sin guardar. Esto no se puede deshacer.
app-confirm-write-running = { $name } sigue en curso; si sale ahora, se abandona.
app-confirm-save-first = Guardar la sesión antes
app-confirm-save-first-ask = Guardar la sesión antes...
app-confirm-wait = Esperar
app-confirm-keep-open = Mantener la aplicación abierta
app-cancel = Cancelar

## File dialogs

app-dialog-save-as = Guardar como { $format }
app-dialog-save-session-as = Guardar sesión como
app-filter-session = Sesión de SigNexora
app-dialog-export-report = Exportar informe de análisis
app-filter-report = Informe
app-dialog-open-session = Abrir sesión
app-dialog-run-script = Ejecutar script
app-filter-rhai = Script Rhai

## File menu

app-menu-file = Archivo
app-menu-open-session = Abrir sesión...
    .hover = Sus señales reemplazan la sesión actual
app-nothing-to-save = Nada que guardar
app-menu-save-session = Guardar sesión
app-menu-save-session-ask = Guardar sesión...
app-menu-save-session-choose = Elija un archivo para la sesión
app-menu-save-session-to = Guardar en { $path }
app-menu-save-session-as = Guardar sesión como...
app-menu-export-report = Exportar informe de análisis
app-menu-report-html = HTML...
app-menu-report-markdown = Markdown...

## File menu extras and the WF popup menu

app-menu-run-script = Ejecutar script (Rhai)...
    .hover = Ejecuta un script; sus señales reemplazan la sesión actual
app-resynth = Resintetizar FSK/MFSK
    .hover = Regenera una señal limpia con los tonos de Buscar tonos, a Br
    .disabled = Busque primero los tonos (Suma de la forma de onda) y mida Br.
app-menu-measure = Medir
app-measure-hint = Coloque los marcadores H alrededor de la línea (y los V para el intervalo).
app-measure-br = Medir línea como velocidad en baudios (Br)
app-measure-sc = Medir línea como portadora (Sc)
    .hover = Tras la potencia N, la línea se transfiere a la portadora (dividida por N, desplazamientos deshechos)
app-measure-adp = Estimar velocidad en baudios automáticamente (ADP)
    .hover = Usa el intervalo de los marcadores V o toda la señal
app-measure-dt-disabled = Coloque primero los marcadores V.

## Hints

app-no-signals-hint = Abra una grabación (Archivo > Abrir archivo... ({ $shortcut }), o suelte un archivo en la ventana).
app-open-in = Abrir...
    .hover = Abrir esta señal en otra vista o herramienta

## Sonogram toolbar

app-sono-shear = Shear
    .hover = Paso entre tramas FFT, como fracción del tamaño de la FFT. Un paso menor solapa más las tramas: más detalle en tiempo, cálculo más lento
app-sono-image = Image
    .hover = Rango dinámico mostrado por debajo de la componente más fuerte
app-sono-hres-hover = Afina las líneas espectrales por reasignación de frecuencia (ayuda a encontrar líneas de reloj débiles). Los tonos conservan su nivel; el ruido aparece unos 2 a 6 dB más bajo que con HRes desactivado (según la ventana), porque se muestra por ancho de bin.
app-sono-alc-hover = Normalizar cada columna a su componente más fuerte (solo visualización)
app-sono-combine = Zoom alejado
    .hover = Cómo se combinan, con zoom alejado, las tramas FFT que comparten una columna de la imagen
app-sono-peak = Pico
    .hover = Con zoom alejado: mantener visibles las ráfagas cortas
app-sono-mean = Media
    .hover = Con zoom alejado: promedio, ruido más suave

## Waveform toolbar

app-wave-show-q = Mostrar Q
app-find-tones = Buscar tonos
    .hover = Picos del histograma: frecuencias de los tonos en la salida del detector de fase

## Zoom and playback

app-play = Reproducir
    .hover = Reproducir el intervalo de los marcadores V o el intervalo visible ({ $shortcut })
app-play-stop = Detener
    .hover = Detener la reproducción ({ $shortcut })

## Child windows

app-tab-signal-gone = Esta señal ya no existe.

## Windows

app-replay-problems-title = Sesión restaurada con problemas
app-script-output = Salida del script: { $path }
app-script-no-output = (sin salida)
# $lines: the lines kept; $dropped: the lines left out.
app-script-output-truncated = Salida truncada tras { $lines } líneas; no se conservaron { $dropped } líneas más.

## Closing a window, the sonogram toolbar

app-confirm-close-tab-title = ¿Cerrar la ventana?
# $name: the window title; $jobs: the job names, joined.
app-confirm-close-tab-question = { $count ->
    [one] Cerrar { $name } detiene su tarea { $jobs } en curso. ¿Cerrar de todos modos?
   *[other] Cerrar { $name } detiene sus tareas { $jobs } en curso. ¿Cerrar de todos modos?
}
app-confirm-close-and-stop = Cerrar y detener
app-confirm-keep-tab = Mantener abierta
# $prev, $next: the keys that show the previous and next signal.
app-select-signal-first = Traiga primero una ventana al frente ({ $prev } o { $next } muestran las señales por turno).
app-sono-fft = SizeFFT
    .hover = Tamaño de la FFT: mayor da más resolución en frecuencia y menos resolución en tiempo
app-sono-window = WinF
    .hover = Función de ventana de cada FFT (WinF en SA)
app-sono-colormap = Colores
    .hover = Mapa de colores de la imagen del sonograma
app-confirm-close-all-title = Cerrar todas las ventanas
app-confirm-close-all-question = Esto quita todas las señales y medidas de la sesión.
app-confirm-close-all = Cerrar todo
app-confirm-close-all-stops = { $name } sigue en marcha; cerrar todas las ventanas lo detiene.
app-replay-session-skipped = { $what ->
    [windows] No se pudo leer la disposición de ventanas guardada; se abre una ventana por resultado: { $error }
   *[counters] No se pudieron leer los contadores de nombres; los nombres nuevos pueden repetir números: { $error }
}
app-replay-windows-missing = { $count ->
    [one] 1 ventana guardada no se abrió porque su señal no se reprodujo
   *[other] { $count } ventanas guardadas no se abrieron porque sus señales no se reprodujeron
}
app-drop-loader-open = El diálogo de archivo está abierto: termínelo o cancélelo y vuelva a soltar el archivo
app-drop-first-only = { $count } archivos soltados; solo se abre el primero, { $name }
app-sono-target = { $active ->
    [yes] Cambia la ventana de sonograma activa.
   *[no] Cambia el valor por defecto de los sonogramas nuevos.
}
app-edit-unrecordable = no se pudo registrar en la sesión para reproducirla ({ $error })
app-empty-open = Abrir una grabación... ({ $key })
app-empty-drop = o suelte un archivo aquí (File > Open file... también abre uno).
app-empty-guide = La guía ({ $key }) muestra el análisis paso a paso.
# $action: what the chooser was for, e.g. "Open session"; $error: why it failed.
app-chooser-failed = { $action }: no se pudo mostrar el selector de archivos ({ $error }). Inténtelo de nuevo o compruebe que el servicio de diálogos de archivos del sistema (portal de escritorio) está en marcha.

## The tools that record a measurement, by the id stored in the session.

measure-tool-adp = Definir parámetros (ADP)
measure-tool-adp-coarse = Definir parámetros, aproximado (dentro de medio bin de FFT)
measure-tool-measure-line = Medida de frecuencia (línea de medida)
measure-tool-v-markers = Marcadores V (dT)
measure-tool-ofdm-adp = Herramienta OFDM, Get Br (ADP)
measure-tool-script-record = Script, record()
