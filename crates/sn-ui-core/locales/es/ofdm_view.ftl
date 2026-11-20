# OFDM module window, SA-OFDM (SA_UI_SPEC.md section 10).

# Top bar (SA's control names).
ofdm-min-br = Min Br-LS
    .hover = Velocidad mínima buscada, Bd (entero); el campo siguiente es el LS más largo
ofdm-max-br = Max Br-LS
    .hover = Velocidad máxima buscada, Bd (entero); el campo siguiente es el LS más corto
ofdm-total-ls = Total LS
    .hover = Símbolos de la mayor longitud (Min Br-LS) en la selección
ofdm-current-ls = Current LS
    .hover = Símbolo mostrado (0 durante Find CT)
ofdm-survey = Survey
    .hover = Vista general: una ventana espectral de 1024 puntos movida con la barra inferior fija el inicio de Find CT
ofdm-list-k = List of K
    .hover = Calculadora Magic K: los valores de K y las frecuencias nativas más probables
    .disabled = Ejecute antes Find CT y ADP

# Controls (SA's control names).
ofdm-symbols = Symbols
    .hover = Símbolos plegados en el triángulo de correlación (y bloque de ADP); 20 a 40 va bien, nunca menos de 6
ofdm-scale-fft = Scale FFT
    .hover = Ancho de las barras de canales del espectro LU
ofdm-shift = Shift Frequency
    .hover = Frecuencia restada antes de la FFT, Hz; Intro para aplicar
ofdm-vshift = VShift
    .hover = Desplazamiento virtual de un canal (pasos exactos de la rejilla, sin límite)
ofdm-shift-step = Paso
    .hover = Shift Frequency más o menos { $step } Hz
ofdm-br = Br
    .hover = Velocidad de símbolos: el valor de ADP, o escriba uno y pulse Intro
ofdm-adp = ADP
    .hover = Velocidad precisa y posición en frecuencia (Get Br de SA)
    .disabled = Ejecute antes Find CT
ofdm-show = Show
    .hover = Mostrar los canales marcados símbolo a símbolo con sincronismo (Break detiene)
ofdm-break = Break
ofdm-find-ct = Find CT
    .hover = Buscar el triángulo de correlación: LS, LG y el inicio de los símbolos
ofdm-mode = Mode
    .hover = Modelo de señal de las constelaciones
ofdm-model-a = A
    .hover = Modelo A: cada canal tal como se generó (valores absolutos)
ofdm-model-b = B
    .hover = Modelo B: cada canal es su propio piloto (valores relativos)
ofdm-model-c = C
    .hover = Modelo C: datos referidos al piloto del marcador rojo
ofdm-each = Each
    .hover = Mostrar uno de cada N símbolos
ofdm-start-plus = Start+
    .hover = Primer símbolo mostrado por Show

# Plot texts (SA's formats).
ofdm-info-br = Br = { $br }
ofdm-info-sh = Sh = { $sh }
ofdm-info-magic = Magic K = { $num }/{ $den }
ofdm-info-k = * { $k } *
ofdm-info-lg = LG(CP) = { $lg }
ofdm-info-lu = LU = { $lu }
ofdm-info-up = CH_Up = { $ch }, Freq = { $freq }
ofdm-info-down = CH_Down = { $ch }, Freq = { $freq }
ofdm-info-sel = CH_Sel = { $count }, dFreqs = { $df }
ofdm-info-vshift = VShift = { $bins }
ofdm-line = LS={ $ls }; LG={ $lg }; Br={ $br }; Sh={ $sh }
ofdm-k-line = SR={ $sr }; LS={ $ls }; LU={ $lu }; LG={ $lg }; K={ $num }/{ $den }
ofdm-finish = Fin normal, tiempo { $secs } s.
ofdm-stopped = Detenido por el usuario, tiempo { $secs } s.
ofdm-wait = ¡Que no cunda el pánico! Espere...
ofdm-wait-ct = Buscando el triángulo de correlación
ofdm-wait-adp = Calculando el Br preciso
ofdm-wait-k = Calculando "Magic K"
ofdm-wait-left = Tiempo restante: { $secs } s.

# Plots.
ofdm-survey-plot = Espectro de vista general
ofdm-triangle-plot = Triángulo de correlación
ofdm-triangle = Triángulo
    .hover = Haga clic en la pendiente buena (izquierda) para colocar la ventana LU; en el vértice es el LU padre
ofdm-spectrum-plot = Espectro LU
ofdm-spectrum = Espectro
    .hover = Clic en un canal mueve el marcador más cercano, doble clic coloca los dos
ofdm-pane = Canal
    .hover = Canal { $ch }, símbolos: { $count }
ofdm-k = K
    .hover = K = { $k }, error LU { $err }, ICI { $ici } dB, { $lu }
ofdm-k-lu-even = LU par
ofdm-k-lu-odd = LU impar

# Popup menus.
ofdm-sort-sr = Ordenar por SR
ofdm-sort-k = Ordenar por K
ofdm-sort-am = Ordenar por AM
ofdm-k-close = Cerrar
ofdm-menu-cancel = Cancelar
ofdm-menu-reset = Borrar los resultados
ofdm-menu-around = Límites alrededor de Br = { $rate }
    .hover = Poner Min Br-LS y Max Br-LS en 0,8 y 1,25 veces el Br medido
ofdm-resample = Remuestrear a { $rate } Hz
    .hover = Remuestrear la señal a esta frecuencia nativa (ventana nueva)

# Jobs and messages.
ofdm-job-ct = OFDM: Find CT
ofdm-job-br = OFDM: ADP
ofdm-job-k = OFDM: List of K
ofdm-job-show = OFDM: Show
ofdm-needs-ct = Ejecute antes Find CT.
ofdm-needs-br = Ejecute antes ADP.
ofdm-baseband-freed = La ventana se cerró entretanto: ejecute Find CT de nuevo.
ofdm-check-settings = ¡Revise los ajustes! Show necesita Symbols de al menos 6 y 10 símbolos tras Start+.
ofdm-failed = La búsqueda falló, tiempo { $secs } s.
ofdm-pick-busy = Espere a que termine el trabajo o pulse Break, y luego elija una línea.
ofdm-br-invalid = Br "{ $text }" no es utilizable: escriba un número mayor que 0 y hasta { $max } Bd (fs/LU).
ofdm-br-invalid-nogrid = Br "{ $text }" no es utilizable: escriba un número mayor que 0.
ofdm-plot-empty = Sin datos: ejecute Find CT.
ofdm-shift-invalid = Shift Frequency "{ $text }" no es un número; escríbala en Hz, por ejemplo 12.5.
