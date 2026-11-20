# The workflow guide: SA's analysis method as a checklist with buttons.

guide-title = Guía de trabajo
guide-no-signal = Abra primero una grabación (Archivo > Abrir archivo...).
# $signal: the active window's signal, which the buttons act on.
guide-acts-on = Los botones actúan sobre: { $signal }
# $key: a key name. Shown after a menu path only when that key acts on the guide's signal.
guide-key = , tecla { $key }

## Steps: the message is the step's title, .text its explanation.

guide-step-record = 1. Grabar correctamente
    .text = WAV PCM de 16 bits (o I/Q) con el AGC del receptor desactivado, sin saturación ni edición. Los archivos comprimidos o de 8 bits se señalan al abrirlos.
guide-step-quality = 2. Comprobar la calidad de la grabación
    .text = Abra la ventana WF (clic derecho en el sonograma > { sw-popup-waveform }). Busque recortes, escalones de 8 bits, componente continua, bombeo del AGC y cortes. Active Suma para ver la distribución de amplitudes.
guide-step-isolate = 3. Aislar la señal
    .text = En el sonograma, coloque los marcadores H alrededor de la señal. Luego use { sw-hint-band-select } y { sw-hint-shift-base } de la barra, y R para remuestrear más bajo si es ancha.
guide-step-rate = 4. Hallar la velocidad de manipulación
    .text = Ejecute un detector, encuadre su línea de reloj con los marcadores y use { sw-popup-measure } del menú contextual{ $br }. { sw-hint-am } para PSK/QAM/FSK, { sw-hint-modified-am } para MFSK, o pruebe { sw-popup-adp }.
guide-step-family = 5. Estudiar la familia de modulación
    .text = FSK/MFSK: { sw-hint-phase-detector }, luego en la ventana WF { edits-median } sobre 1/3 de símbolo, Suma, Buscar tonos, Contar frecuencias. PSK/QAM: { sw-hint-power } para el orden y la portadora ({ sw-popup-measure } en su ventana{ $sc }), luego PhasePlane; con señales distorsionadas use antes el corrector ISD. OFDM: Buscar CT, Obtener Br, Canales y la lista de K para la velocidad nativa.
guide-step-structure = 6. Hallar la estructura
    .text = { sw-hint-acf } para periodos de trama, { sw-hint-ccf } para buscar un fragmento en toda la señal, luego coloque los marcadores V sobre una trama y elija { sw-popup-measure } > { raster-dt-menu-item } en el menú emergente para registrar dT (el periodo de trama) y abra { guide-btn-raster } con ese periodo para ver tramas y palabras de sincronismo.

## Step buttons.

guide-btn-waveform = Forma de onda
guide-btn-sonogram = Sonograma
guide-btn-am-detector = Detector AM
guide-btn-modified-am = AM modificado
guide-btn-phase-detector = Detector de fase
guide-btn-demod = Demodulador FSK/MFSK
# $n: the power, a number.
guide-btn-power = Elevar a la potencia { $n }
    .hover = Crea una señal nueva elevada a la potencia { $n }; una señal PSK-{ $n } muestra entonces una línea en { $n } veces la portadora
guide-btn-phase-plane = PhasePlane
guide-btn-raster = Raster (VMW)
guide-btn-adp = ADP
    .hover = Estimar automáticamente la velocidad de manipulación (ADP) en el intervalo de los marcadores V o en toda la señal
guide-btn-ofdm = OFDM
    .hover = Abrir la herramienta OFDM (Buscar CT, Obtener Br, Lista de K)
guide-open-view = Abrir esta vista de la señal de la ventana activa
guide-new-signal = Crea una señal nueva a partir de la de la ventana activa (se abre en una ventana nueva)
# $kind: Br, Sc or dT (untranslated).
guide-not-measured = { $kind }: aún sin medir
