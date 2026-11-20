# The PhasePlane form (SA_UI_SPEC.md section 6): spectrum mode toolbar, ISD
# corrector, phase viewer, status bar, demodulator output and the LoadFile dialog.
# The control captions are SA's own English names (SA showed them in English under
# every Windows language): keep them as they are; translate the hints and dialogs.

pp-fmin = Fmin
    .hover = Frecuencia más baja mostrada, Hz
pp-fmax = Fmax
    .hover = Frecuencia más alta mostrada, Hz
pp-scale = Scale
    .hover = Parte de la banda mostrada; la barra bajo el espectro la desplaza
pp-fft-size = FFTSize
    .hover = Tamaño FFT del espectro promediado: mayor es más fino (hasta 1 Hz aprox.)
pp-low-level = LowLevel
    .hover = Nivel más bajo mostrado
pp-stretch = Stretch
    .hover = Estirar el espectro a toda la altura
pp-mode = Mode
    .hover = AM: corrige sobre la envolvente; X^N: sobre la potencia N de la señal
pp-blocks = Blocks
    .hover = Número de bloques de señal que promedia el corrector
pp-corr-off = Corr-OFF
    .hover = El corrector ISD está apagado: pulse Auto para corregir la señal
pp-corr-on = Corr-ON
    .hover = El corrector ISD está activo (SaveAs guarda la señal corregida)
pp-save-as = SaveAs
    .hover = Guardar la señal corregida en un archivo
pp-auto = Auto
    .hover = Un paso del corrector sobre los bloques elegidos
pp-break = Break
pp-reset = Reset
    .hover = Restablecer el corrector (apagarlo)
pp-carrier = Carrier
    .hover = Frecuencia portadora, Hz (última medida Sc)
pp-br = BR
    .hover = Velocidad de manipulación, Bd (última medida Br)
pp-nary = n-Ary
    .hover = Orden de la modulación, 2-32 (1: sin seguimiento de portadora)
pp-synch-br = Synch BR
    .hover = Sincronización de reloj
pp-nonstop = NonStop
    .hover = Funcionamiento continuo: volver a empezar al final del archivo
pp-rrc = RRC filter
    .hover = Filtro de raíz de coseno alzado; el deslizador fija alpha
pp-pll = PLL
    .hover = Velocidad del lazo de portadora
pp-pll-slow = Slow
pp-pll-medium = Medium
pp-pll-quick = Quick
pp-pll-hard = Hard
pp-auto-reset = AutoReset
    .hover = Con NonStop: reiniciar generadores y lazos PLL al final del archivo (señales con deriva)
pp-transitions = Transitions
    .hover = Dibujar las transiciones entre los puntos (la trayectoria)
pp-life = Life
    .hover = Visualización dinámica: solo quedan los últimos símbolos (Points...)
pp-diff = Diff
    .hover = 0: ángulos absolutos; 1-3: diferencias de fase (modos relativos como pi/4 DQPSK)
pp-offset-detected = Offset mode detected
pp-angles = Angles
    .hover = Construir el histograma de ángulos en el gráfico del corrector (requiere Synch BR)
pp-vectors = Vectors of dem
    .hover = Mostrar las posiciones del demodulador; el vector cero es azul claro
pp-output-dem = Output Dem as
    .hover = Escribir el flujo demodulado como códigos de ángulos absolutos en el panel de texto
pp-trimming = Trimming { $value }
    .hover = Ajuste fino de fase, grados
pp-delay = Delay { $value }
    .hover = Retardo real por símbolo, segundos (señales muy cortas)
pp-points = Points... { $value }
    .hover = Puntos mostrados a la vez con Life
pp-contrast = Contrast { $value }
    .hover = Brillo de los puntos
pp-start = Start
    .hover = Iniciar o detener la constelación; tras detenerla empieza de nuevo desde el principio del archivo
pp-clear = Clear
    .hover = Borrar la visualización
pp-load-file = LoadFile
    .hover = Cargar uno de los archivos abiertos en el plano de fase
pp-status-f = F = { $value }
pp-status-fl = Fl = { $value }
pp-status-fu = Fu = { $value }
pp-status-df = dF = { $value }
pp-status-angle-d = Angle_D = { $value }
pp-status-angle-i = Angle_I = { $value }
pp-dem-view-mode = View mode: diff { $diff }
pp-dem-mode = Dem mode: { $mode }
pp-dem-absolute = Data presented as absolute angles!
pp-dem-coding = The coding positions of angles:
pp-dem-data = Data:

pp-isd-n = Armónico de la señal con el que trabaja el corrector
    .hover = Armónico de la señal con el que trabaja el corrector
pp-isd-b = Activado: el mejor algoritmo; desactivado: uno algo peor y más rápido (sin efecto en modo AM)
    .hover = Activado: el mejor algoritmo; desactivado: uno algo peor y más rápido (sin efecto en modo AM)
pp-isd-break = Detener el paso actual
    .hover = Detener el paso actual
pp-alpha = Factor alpha del filtro RRC
    .hover = Factor alpha del filtro RRC
pp-quality = Relación señal/ruido convencional de la constelación, dB
    .hover = Relación señal/ruido convencional de la constelación, dB
pp-load-title = SA - elegir archivo para PhasePlane
pp-ok = Aceptar
pp-cancel = Cancelar
pp-check-settings = ¡Compruebe los ajustes!
pp-run-limited = Solo se demodulan los primeros { $symbols } símbolos. Para analizar una parte posterior, coloque los marcadores V sobre ella en el sonograma, use { sw-hint-copy } y abra { sw-hint-phase-plane } sobre la copia.
pp-copy-panel = Copiar el texto
pp-copy-codes = Copiar todos los códigos desde Start
pp-eye = Diagrama de ojo
    .hover = Mostrar el diagrama de ojo (I, dos símbolos) en lugar de la constelación
pp-job-corrector = Espectro del corrector ISD
pp-job-isd = Corrector ISD
pp-job-spectrum = Espectro PhasePlane
# $count symbols, $offset Hz, $rings radii, $iq_offset symbols.
pp-quality-details = { $count ->
        [one] { $count } símbolo
       *[other] { $count } símbolos
    }, desvío del lazo { $offset } Hz, anillos [{ $rings }], desfase I/Q { $iq_offset } símbolos
pp-source-gone = El archivo cargado ya no está en la sesión; cargue de nuevo un archivo de trabajo con LoadFile.
pp-load-gone = El archivo elegido se cerró; elija otro archivo de trabajo.
