# Ventana del demodulador FSK-2 / MFSK universal (SA_UI_SPEC.md sección 8).

# Text panel.
demod-heading =
    SA - MFSK dem
    Levels: { $levels }
    Fd: { $fs }
    Br: { $br }
    Data as: { $form }
    Data:
demod-as-default = Tabla por defecto
demod-as-raw = Bruto
demod-as-bits = Bits, natural
demod-as-gray = Bits, Gray
demod-as-c4fm = Dibits APCO-25 C4FM
demod-polarity-inverted = [polaridad: invertida, bits según la tabla C4FM invertida]
demod-polarity-normal = [polaridad: normal]
demod-polarity-unknown = [polaridad: desconocida, bits según la tabla C4FM normal]
demod-truncated = (primeros 20000 caracteres; Copiar y Guardar como toman todo)
demod-nothing-yet = Aún no hay nada demodulado: revise Br, los niveles y los marcadores.

# Jobs.
demod-job-track = Demodulador: pista de frecuencia
demod-job = Demodulando FSK/MFSK

# Top bar.
demod-rrc = RRC
    .hover = Filtro de raíz de coseno alzado antes de las decisiones; el campo es su corte en Hz (Br por defecto)
demod-cutoff = Corte
    .hover = Corte del RRC en Hz: escriba un valor o elija uno de la lista
demod-alpha = Alfa
    .hover = Caída del RRC, 0,05 a 1 (Intro o salir del campo para aplicar)
demod-br = Br
    .hover = Velocidad de modulación; pulse Intro para aplicar
demod-br-step = Paso
    .hover = Br más o menos { $step }
demod-auto = A
    .hover = Sincronización automática de fase y de bits
demod-auto-button = A
    .hover = Buscar la fase automáticamente ahora y conservarla (desactiva el modo automático)
demod-exactly = <= Exactly
    .hover = Fase de muestreo fina (+-{ $span } % de un símbolo)
demod-roughly = <= Roughly
    .hover = Fase de muestreo gruesa a lo largo de un símbolo
demod-levels = Levels
    .hover = Número de niveles, 2 a 256
demod-raw = Raw
    .hover = Escribir los niveles como bytes hexadecimales separados por x

# Bottom bar.
demod-back-fast = Atrás 10
    .hover = Retroceder 10 páginas
demod-back = Atrás
    .hover = Página anterior
demod-forward = Adelante
    .hover = Página siguiente
demod-forward-fast = Adelante 10
    .hover = Avanzar 10 páginas
demod-page = Página
    .hover = Página mostrada (de { $count }); Intro para ir
demod-scale-up = Ampliar
    .hover = Ampliar la escala vertical
demod-scale-down = Reducir
    .hover = Reducir la escala vertical
demod-progress = Progreso
    .hover = Progreso del cálculo (clic derecho para detener)
demod-stop = Detener
demod-filter = Filtro
    .hover = Aplicar el filtro RRC
demod-revert = Revertir
    .hover = Volver a la señal sin filtrar
demod-d = D
    .hover = Escribir el flujo de bits en el panel de texto

# Plot.
demod-plot-name = Niveles del demodulador
demod-info = Br = { $baud } | reloj { $clock } % | calidad { $quality } | ojo { $eye } | símbolos: { $count }
demod-menu-tones = Niveles en los tonos encontrados ({ $count })
    .hover = Usar como niveles los tonos hallados con Sum en WF en lugar de las zonas regulares
    .disabled = Busque antes los tonos: WF, Sum, Buscar tonos
demod-menu-average = Filtro de promedio
    .hover = Integrar sobre medio símbolo cuando RRC está desactivado (FSK rectangular)
demod-menu-adaptive = Niveles adaptativos
    .hover = Dejar que los centros de los niveles sigan una deriva lenta de los datos
demod-menu-info = Info
demod-menu-reset-view = Restablecer la escala vertical
demod-menu-cancel = Cancelar

# Text panel popup.
demod-menu-copy = Copiar
demod-menu-save = Guardar como...
demod-menu-data-as = Datos como
demod-save-title = Guardar los datos demodulados
demod-save-filter = Texto
demod-save-failed = No se pudo escribir { $path }: { $error }
demod-cutoff-invalid = El corte RRC "{ $text }" no es un número positivo; escríbalo en Hz, por ejemplo el Br.
demod-waiting-cutoff = Esperando un corte RRC válido: el resultado mostrado es de los ajustes anteriores.
demod-br-invalid = El Br "{ $text }" no es un número positivo; se mantiene el Br en uso.
