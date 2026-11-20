# The WF (waveform) window: SA's top toolbar, bottom toolbar hints, popup menu and
# result box. SA's control names (LPF, RRC, Sum, Info, VMW, Br, Z+, Z-) and the
# result box text are names: keep them.

## Top toolbar, left to right.

wf-lpf = LPF
    .hover = Corte del paso bajo en Hz, o velocidad del filtro RRC (escriba o elija un valor)
wf-copy-br = <==
    .hover = Copiar el Br medido al campo LPF
wf-rrc = RRC
    .hover = Activar o desactivar el filtro RRC (el botón LPF lo aplica entonces)
wf-rrc-alpha = RRC
    .hover = Caída del RRC
wf-freq = 0-0
    .hover = Clic: frecuencia del fragmento corto entre los marcadores V y sus periodos enteros
wf-vmw = VMW
    .hover = Cambiar a VMW (la señal plegada en un periodo de línea)
wf-sum = Sum
    .hover = Mostrar el histograma de valores (luego Contar frecuencias en el menú)
wf-info = Info
    .hover = Mostrar los resultados (t1, t2, dT, S, Br)

## Bottom toolbar hints, left to right.

wf-hint-zoom-in = Acercar (tiempo)
wf-hint-zoom-out = Alejar (tiempo)
wf-hint-scale-up = Escala vertical mayor
wf-hint-scale-down = Escala vertical menor
wf-hint-zoom-plus = Z+: Zoom entre los marcadores V
wf-hint-zoom-minus = Z-: Volver un paso de zoom
wf-hint-lpf = Aplicar el filtro LPF (el filtro RRC con RRC activo)
wf-hint-revert = Volver al original
wf-hint-synth = Síntesis FSK/MFSK inversa
wf-hint-save = Guardar como WAV de 16 bits (clic derecho: WAV flotante, DataFile o SigMF)
wf-hint-copy = Copiar el fragmento entre los marcadores V a una ventana nueva
wf-hint-demod = Enviar el fragmento al demodulador FSK/MFSK
wf-hint-fit = Ajustar la escala vertical a la señal
wf-hint-acf = Autocorrelación del fragmento (ventana nueva)
wf-hint-ccf = Correlación cruzada del fragmento con toda la señal (dibujada aquí)

## Why a button is disabled.

wf-need-revert = Nada que deshacer: ni filtro ni CCF.
wf-need-derived = Esta ventana muestra el archivo abierto, que ya está guardado. Guardar es para resultados creados aquí.

## Popup menu.

wf-popup-count-freqs = Contar frecuencias
wf-popup-more = Herramientas SigNexora
    .hover = Herramientas de SigNexora que la ventana WF de SA no tenía
wf-popup-operations = Operaciones

## The Count freqs result box (SA's text, not translated). $size: a whole number;
## $value: a number with two decimals.

wf-label-grid = Freq grid: { $size }
wf-label-delta = Delta: { $value } Hz
wf-count-needs-h = Coloque antes los marcadores H alrededor de los tonos.
wf-count-tones-busy = Find tones aún está en marcha; vuelva a probar Count freqs cuando termine.
wf-count-freqs-failed = No se encontró ninguna rejilla de frecuencias ({ $error }). Amplíe los marcadores H alrededor de los tonos o aleje el zoom para ver más tonos.
wf-copy-br-value = Copiar Br = { $br } Bd en el campo LPF
wf-copy-br-missing = Mida antes Br (menú del sonograma: Auto define params)
