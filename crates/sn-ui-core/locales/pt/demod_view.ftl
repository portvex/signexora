# Janela do demodulador FSK-2 / MFSK universal (SA_UI_SPEC.md secção 8).

# Text panel.
demod-heading =
    SA - MFSK dem
    Levels: { $levels }
    Fd: { $fs }
    Br: { $br }
    Data as: { $form }
    Data:
demod-as-default = Tabela padrão
demod-as-raw = Bruto
demod-as-bits = Bits, natural
demod-as-gray = Bits, Gray
demod-as-c4fm = Dibits APCO-25 C4FM
demod-polarity-inverted = [polaridade: invertida, bits pela tabela C4FM invertida]
demod-polarity-normal = [polaridade: normal]
demod-polarity-unknown = [polaridade: desconhecida, bits pela tabela C4FM normal]
demod-truncated = (primeiros 20000 caracteres; Copiar e Guardar como levam tudo)
demod-nothing-yet = Ainda nada demodulado: verifique Br, os níveis e os marcadores.

# Jobs.
demod-job-track = Demodulador: trilha de frequência
demod-job = A demodular FSK/MFSK

# Top bar.
demod-rrc = RRC
    .hover = Filtro de raiz de cosseno levantado antes das decisões; o campo é o seu corte em Hz (Br por omissão)
demod-cutoff = Corte
    .hover = Corte do RRC em Hz: escreva um valor ou escolha um da lista
demod-alpha = Alfa
    .hover = Decaimento do RRC, 0,05 a 1 (Enter ou sair do campo aplica)
demod-br = Br
    .hover = Velocidade de modulação; Enter aplica
demod-br-step = Passo
    .hover = Br mais ou menos { $step }
demod-auto = A
    .hover = Sincronização automática de fase e de bits
demod-auto-button = A
    .hover = Encontrar a fase automaticamente agora e mantê-la (desliga o modo automático)
demod-exactly = <= Exactly
    .hover = Fase de amostragem fina (+-{ $span } % de um símbolo)
demod-roughly = <= Roughly
    .hover = Fase de amostragem grosseira ao longo de um símbolo
demod-levels = Levels
    .hover = Número de níveis, 2 a 256
demod-raw = Raw
    .hover = Escrever os níveis como bytes hexadecimais separados por x

# Bottom bar.
demod-back-fast = Recuar 10
    .hover = Recuar 10 páginas
demod-back = Recuar
    .hover = Página anterior
demod-forward = Avançar
    .hover = Página seguinte
demod-forward-fast = Avançar 10
    .hover = Avançar 10 páginas
demod-page = Página
    .hover = Página mostrada (de { $count }); Enter vai para ela
demod-scale-up = Ampliar
    .hover = Ampliar a escala vertical
demod-scale-down = Reduzir
    .hover = Reduzir a escala vertical
demod-progress = Progresso
    .hover = Progresso do cálculo (clique direito para parar)
demod-stop = Parar
demod-filter = Filtro
    .hover = Aplicar o filtro RRC
demod-revert = Reverter
    .hover = Voltar ao sinal sem filtro
demod-d = D
    .hover = Escrever o fluxo de bits no painel de texto

# Plot.
demod-plot-name = Níveis do demodulador
demod-info = Br = { $baud } | relógio { $clock } % | qualidade { $quality } | olho { $eye } | símbolos: { $count }
demod-menu-tones = Níveis nos tons encontrados ({ $count })
    .hover = Usar como níveis os tons encontrados com Sum no WF em vez das zonas regulares
    .disabled = Encontre primeiro os tons: WF, Sum, Encontrar tons
demod-menu-average = Filtro de média
    .hover = Integrar sobre meio símbolo quando o RRC está desligado (FSK retangular)
demod-menu-adaptive = Níveis adaptativos
    .hover = Deixar os centros dos níveis seguir uma deriva lenta dos dados
demod-menu-info = Info
demod-menu-reset-view = Repor a escala vertical
demod-menu-cancel = Cancelar

# Text panel popup.
demod-menu-copy = Copiar
demod-menu-save = Guardar como...
demod-menu-data-as = Dados como
demod-save-title = Guardar os dados demodulados
demod-save-filter = Texto
demod-save-failed = Não foi possível escrever { $path }: { $error }
demod-cutoff-invalid = O corte RRC "{ $text }" não é um número positivo; digite-o em Hz, por exemplo o Br.
demod-waiting-cutoff = Aguardando um corte RRC válido: o resultado mostrado é das configurações anteriores.
demod-br-invalid = O Br "{ $text }" não é um número positivo; o Br em uso foi mantido.
