# OFDM module window, SA-OFDM (SA_UI_SPEC.md section 10).

# Top bar (SA's control names).
ofdm-min-br = Min Br-LS
    .hover = Velocidade mínima procurada, Bd (inteiro); o campo seguinte é o LS mais longo
ofdm-max-br = Max Br-LS
    .hover = Velocidade máxima procurada, Bd (inteiro); o campo seguinte é o LS mais curto
ofdm-total-ls = Total LS
    .hover = Símbolos do maior comprimento (Min Br-LS) na seleção
ofdm-current-ls = Current LS
    .hover = Símbolo mostrado (0 durante Find CT)
ofdm-survey = Survey
    .hover = Visão geral: uma janela espectral de 1024 pontos movida com a barra de baixo fixa o início de Find CT
ofdm-list-k = List of K
    .hover = Calculadora Magic K: os valores de K e as taxas nativas mais prováveis
    .disabled = Execute primeiro Find CT e ADP

# Controls (SA's control names).
ofdm-symbols = Symbols
    .hover = Símbolos dobrados no triângulo de correlação (e bloco do ADP); 20 a 40 é bom, nunca menos de 6
ofdm-scale-fft = Scale FFT
    .hover = Largura das barras de canais do espectro LU
ofdm-shift = Shift Frequency
    .hover = Frequência retirada antes da FFT, Hz; Enter aplica
ofdm-vshift = VShift
    .hover = Deslocamento virtual de um canal (passos exatos da grelha, sem limite)
ofdm-shift-step = Passo
    .hover = Shift Frequency mais ou menos { $step } Hz
ofdm-br = Br
    .hover = Velocidade de símbolos: o valor do ADP, ou escreva um e prima Enter
ofdm-adp = ADP
    .hover = Velocidade precisa e posição em frequência (Get Br do SA)
    .disabled = Execute primeiro Find CT
ofdm-show = Show
    .hover = Mostrar os canais marcados símbolo a símbolo com sincronismo (Break para)
ofdm-break = Break
ofdm-find-ct = Find CT
    .hover = Encontrar o triângulo de correlação: LS, LG e o início dos símbolos
ofdm-mode = Mode
    .hover = Modelo de sinal das constelações
ofdm-model-a = A
    .hover = Modelo A: cada canal tal como gerado (valores absolutos)
ofdm-model-b = B
    .hover = Modelo B: cada canal é o seu próprio piloto (valores relativos)
ofdm-model-c = C
    .hover = Modelo C: dados referidos ao piloto do marcador vermelho
ofdm-each = Each
    .hover = Mostrar um em cada N símbolos
ofdm-start-plus = Start+
    .hover = Primeiro símbolo mostrado por Show

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
ofdm-finish = Fim normal, tempo { $secs } s.
ofdm-stopped = Parado pelo utilizador, tempo { $secs } s.
ofdm-wait = Sem pânico! Aguarde...
ofdm-wait-ct = A procurar o triângulo de correlação
ofdm-wait-adp = A calcular o Br preciso
ofdm-wait-k = A calcular "Magic K"
ofdm-wait-left = Tempo restante: { $secs } s.

# Plots.
ofdm-survey-plot = Espectro de visão geral
ofdm-triangle-plot = Triângulo de correlação
ofdm-triangle = Triângulo
    .hover = Clique na vertente boa (esquerda) para colocar a janela LU; no vértice é o LU pai
ofdm-spectrum-plot = Espectro LU
ofdm-spectrum = Espectro
    .hover = Clique num canal para mover o marcador mais próximo, duplo clique coloca os dois
ofdm-pane = Canal
    .hover = Canal { $ch }, símbolos: { $count }
ofdm-k = K
    .hover = K = { $k }, erro LU { $err }, ICI { $ici } dB, { $lu }
ofdm-k-lu-even = LU par
ofdm-k-lu-odd = LU ímpar

# Popup menus.
ofdm-sort-sr = Ordenar por SR
ofdm-sort-k = Ordenar por K
ofdm-sort-am = Ordenar por AM
ofdm-k-close = Fechar
ofdm-menu-cancel = Cancelar
ofdm-menu-reset = Limpar os resultados
ofdm-menu-around = Limites em torno de Br = { $rate }
    .hover = Pôr Min Br-LS e Max Br-LS em 0,8 e 1,25 vezes o Br medido
ofdm-resample = Reamostrar a { $rate } Hz
    .hover = Reamostrar o sinal a esta taxa nativa (nova janela)

# Jobs and messages.
ofdm-job-ct = OFDM: Find CT
ofdm-job-br = OFDM: ADP
ofdm-job-k = OFDM: List of K
ofdm-job-show = OFDM: Show
ofdm-needs-ct = Execute primeiro Find CT.
ofdm-needs-br = Execute primeiro ADP.
ofdm-baseband-freed = A janela foi fechada entretanto: execute Find CT de novo.
ofdm-check-settings = Verifique as definições! Show precisa de Symbols de pelo menos 6 e 10 símbolos após Start+.
ofdm-failed = A busca falhou, tempo { $secs } s.
ofdm-pick-busy = Espere o trabalho em curso ou prima Break, depois escolha uma linha.
ofdm-br-invalid = Br "{ $text }" não é utilizável: digite um número acima de 0 e até { $max } Bd (fs/LU).
ofdm-br-invalid-nogrid = Br "{ $text }" não é utilizável: digite um número acima de 0.
ofdm-plot-empty = Sem dados: execute Find CT.
ofdm-shift-invalid = Shift Frequency "{ $text }" não é um número; digite em Hz, por exemplo 12.5.
