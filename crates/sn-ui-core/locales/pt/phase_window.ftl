# The PhasePlane form (SA_UI_SPEC.md section 6): spectrum mode toolbar, ISD
# corrector, phase viewer, status bar, demodulator output and the LoadFile dialog.
# The control captions are SA's own English names (SA showed them in English under
# every Windows language): keep them as they are; translate the hints and dialogs.

pp-fmin = Fmin
    .hover = Frequência mais baixa mostrada, Hz
pp-fmax = Fmax
    .hover = Frequência mais alta mostrada, Hz
pp-scale = Scale
    .hover = Parte da banda mostrada; a barra sob o espectro desloca-a
pp-fft-size = FFTSize
    .hover = Tamanho FFT do espetro médio: maior é mais fino (até cerca de 1 Hz)
pp-low-level = LowLevel
    .hover = Nível mais baixo mostrado
pp-stretch = Stretch
    .hover = Esticar o espetro a toda a altura
pp-mode = Mode
    .hover = AM: corrige pela envoltória; X^N: pela potência N do sinal
pp-blocks = Blocks
    .hover = Número de blocos do sinal que o corretor faz a média
pp-corr-off = Corr-OFF
    .hover = O corretor ISD está desligado: prima Auto para corrigir o sinal
pp-corr-on = Corr-ON
    .hover = O corretor ISD está ligado (SaveAs guarda o sinal corrigido)
pp-save-as = SaveAs
    .hover = Guardar o sinal corrigido num ficheiro
pp-auto = Auto
    .hover = Um passo do corretor sobre os blocos escolhidos
pp-break = Break
pp-reset = Reset
    .hover = Repor o corretor (desligá-lo)
pp-carrier = Carrier
    .hover = Frequência portadora, Hz (última medição Sc)
pp-br = BR
    .hover = Velocidade de manipulação, Bd (última medição Br)
pp-nary = n-Ary
    .hover = Ordem da modulação, 2-32 (1: sem seguimento da portadora)
pp-synch-br = Synch BR
    .hover = Sincronização de relógio
pp-nonstop = NonStop
    .hover = Funcionamento contínuo: recomeçar no fim do ficheiro
pp-rrc = RRC filter
    .hover = Filtro de raiz de cosseno elevado; o cursor define alpha
pp-pll = PLL
    .hover = Velocidade do laço de portadora
pp-pll-slow = Slow
pp-pll-medium = Medium
pp-pll-quick = Quick
pp-pll-hard = Hard
pp-auto-reset = AutoReset
    .hover = Com NonStop: repor geradores e malhas PLL no fim do ficheiro (sinais com deriva)
pp-transitions = Transitions
    .hover = Desenhar as transições entre os pontos (a trajetória)
pp-life = Life
    .hover = Visualização dinâmica: só ficam os últimos símbolos (Points...)
pp-diff = Diff
    .hover = 0: ângulos absolutos; 1-3: diferenças de fase (modos relativos como pi/4 DQPSK)
pp-offset-detected = Offset mode detected
pp-angles = Angles
    .hover = Construir o histograma de ângulos no gráfico do corretor (requer Synch BR)
pp-vectors = Vectors of dem
    .hover = Mostrar as posições do desmodulador; o vetor zero é azul-claro
pp-output-dem = Output Dem as
    .hover = Escrever o fluxo desmodulado como códigos de ângulos absolutos no painel de texto
pp-trimming = Trimming { $value }
    .hover = Ajuste fino de fase, graus
pp-delay = Delay { $value }
    .hover = Atraso real por símbolo, segundos (sinais muito curtos)
pp-points = Points... { $value }
    .hover = Pontos mostrados de uma vez com Life
pp-contrast = Contrast { $value }
    .hover = Brilho dos pontos
pp-start = Start
    .hover = Iniciar ou parar a constelação; após parar recomeça do início do ficheiro
pp-clear = Clear
    .hover = Limpar a visualização
pp-load-file = LoadFile
    .hover = Carregar um dos ficheiros abertos no plano de fase
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

pp-isd-n = Harmónico do sinal com que o corretor trabalha
    .hover = Harmónico do sinal com que o corretor trabalha
pp-isd-b = Ligado: o melhor algoritmo; desligado: um pouco pior e mais rápido (sem efeito no modo AM)
    .hover = Ligado: o melhor algoritmo; desligado: um pouco pior e mais rápido (sem efeito no modo AM)
pp-isd-break = Parar o passo atual
    .hover = Parar o passo atual
pp-alpha = Fator alpha do filtro RRC
    .hover = Fator alpha do filtro RRC
pp-quality = Relação sinal/ruído convencional da constelação, dB
    .hover = Relação sinal/ruído convencional da constelação, dB
pp-load-title = SA - escolher ficheiro para PhasePlane
pp-ok = Ok
pp-cancel = Cancelar
pp-check-settings = Verifique as definições!
pp-run-limited = Apenas os primeiros { $symbols } símbolos são desmodulados. Para analisar uma parte posterior, posicione os marcadores V sobre ela no sonograma, use { sw-hint-copy } e abra { sw-hint-phase-plane } na cópia.
pp-copy-panel = Copiar o texto
pp-copy-codes = Copiar todos os códigos desde Start
pp-eye = Diagrama de olho
    .hover = Mostrar o diagrama de olho (I, dois símbolos) em vez da constelação
pp-job-corrector = Espetro do corretor ISD
pp-job-isd = Corretor ISD
pp-job-spectrum = Espetro PhasePlane
# $count symbols, $offset Hz, $rings radii, $iq_offset symbols.
pp-quality-details = { $count ->
        [one] { $count } símbolo
       *[other] { $count } símbolos
    }, desvio da malha { $offset } Hz, anéis [{ $rings }], desvio I/Q { $iq_offset } símbolos
pp-source-gone = O arquivo carregado não está mais na sessão; carregue de novo um arquivo de trabalho com LoadFile.
pp-load-gone = O arquivo escolhido foi fechado; escolha outro arquivo de trabalho.
