# Herramientas > Generador de señales (OCG): la ventana del generador, su vista
# previa y las calculadoras OFDM. No traducir unidades (Hz, Bd, s, ms, dB, ppm),
# nombres de modulación (FSK, MSK, GMSK, PSK, QAM, C4FM, OFDM, BPSK, ...),
# BT, Es/N0, SNR, RC, RRC, LU, LG, LS, K, Br, Fd ni los modelos A, B, C.

gen-menu = Generador de señales (OCG)...
    .hover = Crear una señal de prueba con parámetros exactamente conocidos: tonos, ruido, FSK, PSK/QAM, C4FM u OFDM, con degradaciones del canal
gen-keys-open = Mostrar u ocultar el generador de señales
gen-title = Generador de señales (OCG)
gen-job = Generando { $name }

gen-kind = Tipo
gen-kind-tones = Tonos
gen-kind-noise = Ruido
gen-kind-fsk = FSK/MFSK
gen-kind-linear = PSK/QAM
gen-kind-c4fm = C4FM
gen-kind-ofdm = OFDM
gen-preset = Preajuste
    .hover = Cargar una señal ya preparada
gen-preset-defaults = Predeterminados
    .hover = Restablecer el tipo actual a sus valores predeterminados

gen-section-signal = Señal
gen-section-common = Común
gen-section-channel = Degradaciones del canal
gen-section-derived = Valores derivados
gen-section-preview = Vista previa
gen-section-calc = Calculadoras

gen-complex = Salida compleja (I/Q)
    .hover = Generar muestras I/Q en lugar de una señal real; las frecuencias son entonces desplazamientos desde 0 Hz
gen-amplitude-rms = Amplitud RMS
gen-amplitude-peak = Amplitud de pico
gen-phase-rad = Fase (rad)
gen-remove = Quitar
gen-baud = Velocidad de símbolos
gen-carrier = Portadora
gen-center = Centro
gen-alpha = Caída (roll-off)

gen-tones-frequency = Frecuencia
gen-tones-add = Añadir tono
gen-tones-last = Se necesita al menos un tono

gen-fsk-order = Tonos
gen-fsk-spacing = Separación
gen-fsk-shaping = Conformación
gen-fsk-shaping-none = Ninguna
gen-fsk-shaping-gaussian = Gaussiana
gen-fsk-bt = BT
gen-fsk-msk = MSK
    .hover = Dos tonos, separación igual a la mitad de la velocidad de símbolos, sin conformación
gen-fsk-gmsk = GMSK
    .hover = MSK con un filtro gaussiano de BT 0,3, como en GSM

gen-lin-constellation = Constelación
gen-lin-shaping = Forma del pulso
gen-lin-rect = Rectangular
gen-lin-rc = Coseno alzado (RC)
gen-lin-rrc = Raíz de coseno alzado (RRC)
gen-lin-differential = Diferencial
    .hover = Enviar cada símbolo como un salto de fase respecto al anterior
gen-lin-differential-qam = QAM transporta amplitud además de fase, así que no puede ser diferencial
gen-lin-differential-pi4 = pi/4-DQPSK es siempre diferencial

gen-c4fm-pulse = Pulso
gen-c4fm-shaped = Conformado
    .hover = Desactivado: la frecuencia salta entre los cuatro niveles
gen-c4fm-apco = APCO-25
    .hover = El pulso TIA-102, pensado para un receptor de integración y descarga
gen-c4fm-note = Las desviaciones están fijas en +-600 y +-1800 Hz; la salida C4FM es siempre real.

gen-ofdm-lu = LU
    .hover = Longitud útil (tamaño de FFT) en muestras a la frecuencia de muestreo
gen-ofdm-lg = LG
    .hover = Longitud de guarda (prefijo cíclico) en muestras
gen-ofdm-channels = Canales
gen-ofdm-first-bin = Primer bin
    .hover = Bin del primer canal; el bin 0 es el centro
gen-ofdm-modulation = Modulación
gen-ofdm-unused = Posiciones sin uso
    .hover = Posiciones de canal que no transportan nada, contadas desde 0, por ejemplo 3, 7, 10-12
gen-ofdm-overrides = Excepciones
    .hover = Modulación de posiciones concretas, por ejemplo 0: pilot, 5-6: QAM-16
gen-ofdm-model = Modelo
gen-ofdm-model-a = A: cada canal envía valores absolutos
gen-ofdm-model-b = B: cada canal es diferencial de un símbolo al siguiente
gen-ofdm-model-c = C: los pilotos llevan la referencia, los datos son absolutos respecto a ellos
gen-ofdm-pilots = Pilotos
gen-ofdm-pilots-none = Ninguno
gen-ofdm-pilots-list = Posiciones
    .hover = Posiciones de los pilotos, por ejemplo 0, 8, 16
gen-ofdm-pilots-every = Cada N
gen-ofdm-step = Paso
gen-ofdm-offset = Primero
gen-ofdm-reference = Referencia piloto
    .hover = Lo que llevan los pilotos en el modelo C
    .disabled = Solo el modelo C usa referencia piloto
gen-ofdm-reference-constant = Constante
gen-ofdm-reference-differential = Fase común aleatoria
gen-ofdm-center = Centro
    .hover = Dónde cae el bin 0; con salida compleja, el desplazamiento desde 0 Hz

gen-label = Etiqueta
    .hover = Nombre de la nueva señal; si está vacía se usa la descripción
gen-sample-rate = Frec. de muestreo
gen-length = Duración
gen-length-seconds = Segundos
gen-length-symbols = Símbolos
    .disabled = Los tonos y el ruido no tienen símbolos
gen-seed = Semilla
gen-new-seed = Nueva semilla
    .hover = Elegir una semilla aleatoria: datos nuevos y ruido nuevo
gen-center-frequency = Frec. central
    .hover = Frecuencia de RF que representa 0 Hz de la salida compleja (se guarda con la señal)
    .disabled = Solo la salida compleja tiene frecuencia central

gen-ch-offset = Desplazamiento de frecuencia
gen-ch-drift = Deriva
gen-ch-clock = Desviación de reloj
    .hover = Error del reloj de muestreo; positivo si el reloj del receptor adelanta
gen-ch-noise = Ruido
gen-ch-noise-none = Ninguno
gen-ch-noise-full = SNR en toda la banda
gen-ch-noise-inband = SNR en banda
gen-ch-noise-esn0 = Es/N0
gen-ch-bandwidth = Ancho de banda
    .hover = Desactivado: el ancho de banda que ocupa la señal
gen-ch-clip = Recortar en
    .hover = Limitar la amplitud de pico; las muestras complejas conservan su fase
gen-ch-multipath = Trayectos múltiples
    .hover = Los trayectos son todo el canal: incluya el trayecto directo (retardo 0, ganancia 1)
gen-ch-delay = Retardo
gen-ch-gain = Ganancia
gen-ch-add-tap = Añadir trayecto

gen-d-none = Corrija los parámetros para ver los valores derivados.
gen-d-symbol-rate = Velocidad de símbolos
gen-d-sps = Muestras por símbolo
gen-d-symbols = Símbolos
gen-d-bandwidth = Ancho de banda ocupado
gen-d-samples = Muestras
gen-d-duration = Duración
gen-d-index = Índice de modulación h
gen-d-ls = LS
gen-d-k = K = LG/LU
gen-d-spacing = Separación
gen-d-br = Br

gen-preview = Vista previa del generador
gen-preview-spectrum = Espectro
gen-preview-wave = Forma de onda
gen-preview-name = Vista previa de la señal generada
gen-preview-building = Creando la vista previa...
gen-preview-invalid = Sin vista previa hasta que los parámetros sean válidos
gen-preview-failed = Falló la vista previa: { $error }
gen-preview-short = Demasiado corta para un espectro
gen-preview-axis = Espectro medio, de { $lo } a { $hi } Hz, rango de 80 dB
gen-preview-wave-axis = Primeros { $ms } ms, pico { $peak }

gen-generate = Generar
    .hover = Añadir la señal a la sesión como una nueva grabación
gen-generate-save = Generar y guardar como
    .hover = Generar y luego guardar la nueva señal en el formato elegido
gen-busy = Se está generando una señal
gen-fix-first = Corrija primero los parámetros
gen-invalid = No se puede generar: { $error }
gen-field-error = { $field }: { $error }
gen-ready = Lista: { $samples } muestras, { $duration } s
gen-list-bad = "{ $item }" no es una posición ni un rango como 10-12
gen-list-too-long = Demasiadas posiciones (como máximo { $max })
gen-overrides-bad = "{ $item }" no es una posición con una modulación como 5: QAM-16

gen-calc-cp = OFDM con CP
gen-calc-cp-lengths = A partir de Fd, LU y LG
gen-calc-spacing = Separación
gen-calc-symbol = Símbolo
gen-calc-useful = Parte útil
gen-calc-guard = Guarda
gen-calc-use = Usar en OFDM
    .hover = Copiar estas longitudes (y Fd) al editor OFDM
    .disabled = LU y LG deben ser números enteros de muestras
# Why Use in OFDM is disabled. $max: the generator's limit in samples.
gen-calc-lu-not-whole = LU debe ser un número entero de muestras, al menos 2.
gen-calc-lg-not-whole = LG debe ser un número entero de muestras.
gen-calc-lu-too-large = LU debe ser como máximo { $max } muestras.
gen-calc-lg-too-large = LG debe ser como máximo { $max } muestras.
gen-calc-use-k = Usar en OFDM
    .hover = Copiar este K al editor OFDM: el menor LU que le corresponde, desde el LU actual hacia arriba
gen-calc-inverse = A partir de Br y la separación a Fd: LU y LG enteros más cercanos
gen-calc-exact = exacto
gen-calc-error = Error
gen-calc-candidates = LU y LG enteros para Br y K
gen-calc-lu-range = LU desde
gen-calc-tolerance = Tolerancia
gen-calc-found = { $count ->
    [one] { $count } candidato
   *[other] { $count } candidatos, se muestran los { $shown } primeros
}
gen-calc-klist = Lista de K
gen-calc-k-from-rates = A partir de Br y la separación
gen-calc-k-direct = A partir de K
gen-calc-uncertainty = Incertidumbre
    .hover = Cuánto puede desviarse cada tasa medida, en porcentaje de la tasa. La lista muestra las fracciones más simples dentro de la incertidumbre de K resultante; para un K introducido directamente, la tolerancia es la de un K medido a partir de dos tasas con esa desviación.
gen-calc-uncertainty-bad = La incertidumbre debe estar entre 0 y { $max } %.
gen-calc-k-none = Ninguna fracción está dentro de la incertidumbre. Aumente la incertidumbre o el mayor denominador.
gen-calc-max-den = Mayor denominador
gen-calc-count = Cantidad
gen-calc-value = Valor
gen-calc-fd = Fd
    .hover = Frecuencia de muestreo en Hz que hace exactas estas longitudes para el Br: Br x (LU + LG)
gen-calc-k = K
    .hover = K = LG/LU, la relación entre guarda y longitud útil que tienen realmente estas longitudes enteras
gen-calc-lg-error = Error (muestras)
    .hover = LG menos K x LU: cuánto se aleja la longitud de guarda entera de la exacta, en muestras
gen-calc-k-fraction = K
    .hover = K como fracción p/q irreducible
gen-calc-k-error = Error
    .hover = El valor de la fracción menos el K medido (con signo; K no tiene unidad, así que esto tampoco)
gen-preview-symbol-too-long = Sin vista previa: un símbolo tiene { $samples } muestras, más que una vista previa. Suba la velocidad o baje la frecuencia de muestreo.
gen-calc-fraction-too-big = K = { $k } no cabe: LU debe ser como mucho { $lu } y LG como mucho { $lg } muestras. Elija un LU o K menor.
