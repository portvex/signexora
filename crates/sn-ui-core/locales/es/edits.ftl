# The Signal menu, the parameter dialog of its operations and the save formats.

edits-save-wav-pcm16 = WAV, PCM de 16 bits
edits-save-wav-float = WAV, flotante de 32 bits
edits-save-sigmf = SigMF (conserva frecuencias fraccionarias)
edits-save-raw-f32 = Sin formato, flotante de 32 bits

# $from: where a measured value came from, e.g. "n4, measure line".
edits-measured-other-axis = { $from } (otro eje, verifique)

## Notes under the dialog fields: where a pre-filled value came from.

edits-note-cutoff-raised = Corte: Br de { $from } (elevado al mínimo de 30 Hz; Br es { $br } Bd)
edits-note-cutoff-lowered = Corte: Br de { $from } (reducido por debajo de la mitad de la frecuencia de muestreo; Br es { $br } Bd)
edits-note-cutoff-br = Corte: Br de { $from }
edits-note-cutoff-guessed = Corte: estimado (fs/8), mida Br primero
# $count: the longest window offered, in samples.
edits-note-window-capped = { $count ->
    [one] Ventana: 1/3 de símbolo a Br de { $from } (limitada a { $count } muestra; remuestree hacia abajo para un 1/3 de símbolo real)
   *[other] Ventana: 1/3 de símbolo a Br de { $from } (limitada a { $count } muestras; remuestree hacia abajo para un 1/3 de símbolo real)
}
edits-note-window-br = Ventana: 1/3 de símbolo a Br de { $from }
edits-note-window-default = Ventana: 5 muestras por defecto; mida Br para obtener 1/3 de símbolo
edits-note-measured-br = Br medido de { $from }
edits-note-measured-s = Medido: S = 1/dT de los marcadores V
edits-note-carrier-sc = Portadora: Sc de { $from }
edits-note-carrier-h = Portadora: centro de los marcadores H
edits-note-carrier-guessed = Portadora: estimada (fs/4), mida Sc primero
edits-note-br-from = Br: de { $from }
edits-note-br-guessed-8 = Br: estimado (fs/8), mida Br primero
edits-note-br-s = Br: S = 1/dT de los marcadores V
edits-note-br-guessed-10 = Br: estimado (fs/10), mida Br primero

## Why a menu item is disabled.

edits-need-h = Coloque primero los marcadores H (arrastre en el sonograma).
edits-need-v = Coloque primero los marcadores V (arrastre en el sonograma o la forma de onda).
edits-need-vh = Coloque primero los marcadores V y H (arrastre en el sonograma).
edits-need-complex = La señal ya es real.
edits-need-baud = Mida Br (menú Medir) o coloque antes los marcadores V sobre un símbolo.

## Signal menu sections and items.

edits-section-spectrum = Espectro (marcadores H)
edits-band-select = Seleccionar banda
edits-band-reject = Rechazar banda
edits-area-delete = Borrar área
edits-shift-to-base = Desplazar banda a base
edits-shift-to-base-inverted = Desplazar banda a base, invertida
edits-invert-band = Inversión en banda
edits-section-whole = Señal completa
edits-shift = Desplazar espectro...
edits-invert = Inversión de espectro
edits-resample = Remuestrear...
edits-iq-to-real = I/Q a real
edits-baud-correction = Corrección de velocidad en baudios...
edits-derivative = Primera derivada
edits-limiter = Limitador duro
edits-section-isd = Corrector ISD
edits-isd-am = Corrector ISD, modo AM (ciego)...
edits-isd-xn = Corrector ISD, modo X^N...
edits-section-detectors = Detectores
edits-am-detector = Detector AM (reloj de PSK/QAM/FSK)
edits-modified-am = Detector AM modificado (reloj de MFSK)
edits-half-wave = Detector de doble media onda
edits-phase-detector = Detector de fase (frecuencia en Hz)
edits-power = Elevar a la potencia N...
edits-acf = ACF de la selección
edits-ccf = CCF: buscar la selección en toda la señal
edits-section-time = Tiempo (marcadores V)
edits-copy = Copiar selección a nueva señal
edits-section-filters = Filtros
edits-lowpass = Paso bajo...
edits-median = Mediana...
edits-rrc = RRC (filtro adaptado)...

## Diálogo de parámetros (pequeño diálogo modal de SA).

# $operation: the dialog's operation; $signal: the window name, e.g. "test(0)(u1)".
edits-dialog-title = SA - { $operation }: { $signal }
edits-title-shift = Desplazar espectro
edits-title-resample = Remuestrear
edits-title-baud = Corrección de BR
edits-title-lowpass = Paso bajo
edits-title-median = Filtro de mediana
edits-title-rrc = Filtro RRC
edits-title-power = Elevar a la potencia N
edits-title-isd-am = Corrector ISD (modo AM)
edits-title-isd-xn = Corrector ISD (modo X^N)

edits-field-carrier = Portadora (Hz)
edits-field-br = Br, velocidad de símbolos (Bd)
edits-field-blocks = Bloques
edits-field-power-psk = Potencia N (orden PSK)
edits-field-window = Ventana (muestras, impar)
edits-field-shift = Desplazamiento (Hz, + hacia arriba)
edits-field-rate = Nueva frec. de muestreo (Hz)
edits-field-measured-br = Velocidad existente
edits-field-required-br = Velocidad requerida
edits-field-cutoff = Corte (Hz)
edits-presets = predefinidos
edits-field-power = Potencia N
edits-field-alpha = Roll-off alfa
# $lo, $hi: the limits of a field.
edits-range = De { $lo } a { $hi }
# $max: the sample rate in Hz.
edits-hint-shift = Hasta +/- { $max } Hz, no 0; se admiten decimales
# $max: the longest window, in samples.
edits-hint-window = De 1 a { $max }; un valor par se usa como el impar siguiente
edits-ok = Aceptar
edits-cancel = Cancelar
edits-field-invalid = { $field } "{ $text }" no es válido. { $range }.
edits-range-open = Mayor que { $lo } y menor que { $hi }
