# The sonogram work window: SA's top toolbar captions, the bottom toolbar hints,
# the right-click popup menu and the Correction of BR panel. SA's control names
# (Fl, Fu, ALC, HRes, Info, WaveFile, DataFile) are names: keep them.

## Top toolbar (SizeFFT, Shear, Image and WinF are in app.ftl).

sw-fl = Fl
    .hover = Frecuencia más baja mostrada, Hz (azul mientras la banda está ampliada)
sw-fu = Fu
    .hover = Frecuencia más alta mostrada, Hz (azul mientras la banda está ampliada)
sw-alc = ALC
sw-hres = HRes
sw-info = Info
    .hover = Mostrar el panel de resultados (Fu, Fc, Fl, dF, t1, t2, dT, S, Br, Sc)

## Bottom toolbar hints, left to right.

sw-hint-zoom-in = Extender la banda entre los marcadores H a la ventana
sw-hint-zoom-back = Volver un paso de zoom
sw-hint-zoom-reset = Restablecer el zoom
sw-hint-abort = Detener
sw-hint-play = Reproducir
sw-hint-stop-play = Detener la reproducción
sw-hint-band-select = Seleccionar banda
sw-hint-area-delete = Borrar área
sw-hint-band-reject = Rechazar banda
sw-hint-am = Detector AM en cuadratura
sw-hint-modified-am = Detector AM modificado
sw-hint-half-wave = Detector de doble media onda
sw-hint-derivative = Primera derivada
sw-hint-power = Módulo de potencia N
sw-hint-phase-plane = PhasePlane
sw-hint-acf = Autocorrelación
sw-hint-ccf = Correlación cruzada
sw-hint-ofdm = Módulo OFDM
sw-hint-phase-detector = Detector de fase
sw-hint-limiter = Limitador duro
sw-hint-iq = I/Q a real
sw-hint-shift-base = Desplazar banda a base
sw-hint-shift-base-inverted = Desplazar banda a base, invertida
sw-hint-inverse = Inversión en banda (todo el espectro si los marcadores H coinciden)
sw-hint-copy = Copiar fragmento
sw-hint-resample = Remuestrear a la frecuencia del campo
sw-hint-shift = Desplazar el espectro el valor del campo (Hz; negativo hacia abajo)
sw-value = R / S
    .hover = Nueva frecuencia para R, o desplazamiento para S, en Hz (coma o punto decimal)
sw-value-presets = Valores predefinidos

## Why a button is disabled.

sw-need-history = No hay paso de zoom al que volver.
sw-need-running = No hay nada en marcha para esta ventana.
sw-need-playing = No se está reproduciendo nada.

## Popup menu.

sw-popup-save-as = Guardar como
sw-popup-wave-file = WaveFile
sw-popup-data-file = DataFile
sw-popup-wave-file-float = WaveFile (flotante de 32 bits)
sw-popup-sigmf = SigMF
sw-popup-waveform = Módulo Wave Form
sw-popup-measure = Medida de frecuencia
sw-popup-adp = Definir parámetros (ADP)
sw-popup-brc = Corrección de BR
sw-popup-cancel = Cancelar

## Correction of BR panel.

sw-brc-existing = Velocidad existente
sw-brc-required = Velocidad requerida
sw-brc-max-delta = Cambio máx.: +/- 30%
sw-brc-yes = Sí

## Messages.

sw-check-settings = Compruebe los ajustes.
# Result boxes in the plot (SA's own text). $value: a number with two decimals.
sw-label-freq = Freq: { $value } Hz
sw-label-brate = Brate: { $value } Hz
sw-rs-resample-invalid = El valor R/S "{ $text }" no es una frecuencia de muestreo positiva; escriba un valor en Hz, por ejemplo 8000.
sw-rs-shift-invalid = El valor R/S "{ $text }" no es un número; escriba un desplazamiento en Hz, por ejemplo -150.
sw-brc-existing-invalid = Existent Baud Rate "{ $text }" no es un número; escriba el Br medido en Bd o ejecute antes ADP.
sw-brc-required-invalid = Required Baud Rate "{ $text }" no es un número; escriba el Br deseado en Bd.
sw-brc-not-positive = Ambas velocidades deben ser mayores que 0 Bd.
sw-brc-too-far = El Br requerido debe estar entre { $lo } y { $hi } Bd (como mucho 30% del existente).
sw-brc-measured = Rellenado con el Br medido por ADP
sw-brc-not-measured = Aún no hay Br medido: escríbalo o ejecute antes ADP
