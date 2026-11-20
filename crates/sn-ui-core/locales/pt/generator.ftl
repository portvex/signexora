# File > Signal generator (OCG): the generator window, its preview and the
# OFDM calculators. Keep units (Hz, Bd, s, ms, dB, ppm), modulation names
# (FSK, MSK, GMSK, PSK, QAM, C4FM, OFDM, BPSK, ...), BT, Es/N0, SNR, RC, RRC,
# LU, LG, LS, K, Br, Fd and the model letters A, B, C untranslated.

gen-menu = Gerador de sinais (OCG)...
    .hover = Cria um sinal de teste com parâmetros exatamente conhecidos: tons, ruído, FSK, PSK/QAM, C4FM ou OFDM, com degradações do canal
gen-keys-open = Mostrar ou ocultar o gerador de sinais
gen-title = Gerador de sinais (OCG)
gen-job = Gerando { $name }

gen-kind = Tipo
gen-kind-tones = Tons
gen-kind-noise = Ruído
gen-kind-fsk = FSK/MFSK
gen-kind-linear = PSK/QAM
gen-kind-c4fm = C4FM
gen-kind-ofdm = OFDM
gen-preset = Predefinição
    .hover = Carrega um sinal pronto
gen-preset-defaults = Padrões
    .hover = Restaura os valores padrão do tipo atual

gen-section-signal = Sinal
gen-section-common = Geral
gen-section-channel = Degradações do canal
gen-section-derived = Valores derivados
gen-section-preview = Prévia
gen-section-calc = Calculadoras

gen-complex = Saída complexa (I/Q)
    .hover = Gera amostras I/Q em vez de um sinal real; as frequências passam a ser desvios a partir de 0 Hz
gen-amplitude-rms = Amplitude RMS
gen-amplitude-peak = Amplitude de pico
gen-phase-rad = Fase (rad)
gen-remove = Remover
gen-baud = Taxa de baud
gen-carrier = Portadora
gen-center = Centro
gen-alpha = Roll-off

gen-tones-frequency = Frequência
gen-tones-add = Adicionar tom
gen-tones-last = É preciso pelo menos um tom

gen-fsk-order = Tons
gen-fsk-spacing = Espaçamento
gen-fsk-shaping = Formatação
gen-fsk-shaping-none = Nenhuma
gen-fsk-shaping-gaussian = Gaussiana
gen-fsk-bt = BT
gen-fsk-msk = MSK
    .hover = Dois tons, espaçamento de metade da taxa de baud, sem formatação
gen-fsk-gmsk = GMSK
    .hover = MSK com filtro gaussiano de BT 0,3, como no GSM

gen-lin-constellation = Constelação
gen-lin-shaping = Formato do pulso
gen-lin-rect = Retangular
gen-lin-rc = Cosseno levantado (RC)
gen-lin-rrc = Raiz de cosseno levantado (RRC)
gen-lin-differential = Diferencial
    .hover = Envia cada símbolo como um passo de fase em relação ao anterior
gen-lin-differential-qam = QAM transporta amplitude além da fase, por isso não pode ser diferencial
gen-lin-differential-pi4 = pi/4-DQPSK é sempre diferencial

gen-c4fm-pulse = Pulso
gen-c4fm-shaped = Formatado
    .hover = Desligado: a frequência salta entre os quatro níveis
gen-c4fm-apco = APCO-25
    .hover = O pulso TIA-102, feito para um receptor integrate-and-dump
gen-c4fm-note = Os desvios são fixos em +-600 e +-1800 Hz; a saída C4FM é sempre real.

gen-ofdm-lu = LU
    .hover = Comprimento útil (tamanho da FFT) em amostras na taxa de amostragem
gen-ofdm-lg = LG
    .hover = Comprimento da guarda (prefixo cíclico) em amostras
gen-ofdm-channels = Canais
gen-ofdm-first-bin = Primeiro bin
    .hover = Bin do primeiro canal; o bin 0 é o centro
gen-ofdm-modulation = Modulação
gen-ofdm-unused = Posições sem uso
    .hover = Posições de canal que não transportam nada, contadas a partir de 0, por exemplo 3, 7, 10-12
gen-ofdm-overrides = Exceções
    .hover = Modulação de posições individuais, por exemplo 0: pilot, 5-6: QAM-16
gen-ofdm-model = Modelo
gen-ofdm-model-a = A: todo canal envia valores absolutos
gen-ofdm-model-b = B: todo canal é diferencial de símbolo para símbolo
gen-ofdm-model-c = C: os pilotos levam a referência, os dados são absolutos em relação a eles
gen-ofdm-pilots = Pilotos
gen-ofdm-pilots-none = Nenhum
gen-ofdm-pilots-list = Posições
    .hover = Posições dos pilotos, por exemplo 0, 8, 16
gen-ofdm-pilots-every = A cada N
gen-ofdm-step = Passo
gen-ofdm-offset = Primeiro
gen-ofdm-reference = Referência piloto
    .hover = O que os pilotos transportam no modelo C
    .disabled = Só o modelo C usa referência piloto
gen-ofdm-reference-constant = Constante
gen-ofdm-reference-differential = Fase comum aleatória
gen-ofdm-center = Centro
    .hover = Onde fica o bin 0; na saída complexa, o desvio a partir de 0 Hz

gen-label = Rótulo
    .hover = Nome do novo sinal; vazio usa a descrição
gen-sample-rate = Taxa de amostragem
gen-length = Duração
gen-length-seconds = Segundos
gen-length-symbols = Símbolos
    .disabled = Tons e ruído não têm símbolos
gen-seed = Semente
gen-new-seed = Nova semente
    .hover = Escolhe uma semente aleatória: novos dados e novo ruído
gen-center-frequency = Frequência central
    .hover = Frequência de RF que o 0 Hz da saída complexa representa (salva com o sinal)
    .disabled = Só a saída complexa tem frequência central

gen-ch-offset = Desvio de frequência
gen-ch-drift = Deriva
gen-ch-clock = Desvio de clock
    .hover = Erro do clock de amostragem; positivo significa que o clock do receptor está adiantado
gen-ch-noise = Ruído
gen-ch-noise-none = Nenhum
gen-ch-noise-full = SNR na banda inteira
gen-ch-noise-inband = SNR na banda
gen-ch-noise-esn0 = Es/N0
gen-ch-bandwidth = Largura de banda
    .hover = Desligado: a largura de banda ocupada pelo sinal
gen-ch-clip = Limitar em
    .hover = Limita a amplitude de pico; amostras complexas mantêm a fase
gen-ch-multipath = Taps de multipercurso
    .hover = Os taps são o canal inteiro: inclua o caminho direto (atraso 0, ganho 1)
gen-ch-delay = Atraso
gen-ch-gain = Ganho
gen-ch-add-tap = Adicionar tap

gen-d-none = Corrija os parâmetros para ver os valores derivados.
gen-d-symbol-rate = Taxa de símbolos
gen-d-sps = Amostras por símbolo
gen-d-symbols = Símbolos
gen-d-bandwidth = Largura de banda ocupada
gen-d-samples = Amostras
gen-d-duration = Duração
gen-d-index = Índice de modulação h
gen-d-ls = LS
gen-d-k = K = LG/LU
gen-d-spacing = Espaçamento
gen-d-br = Br

gen-preview = Prévia do gerador
gen-preview-spectrum = Espectro
gen-preview-wave = Forma de onda
gen-preview-name = Prévia do sinal gerado
gen-preview-building = Gerando prévia...
gen-preview-invalid = Sem prévia até os parâmetros serem válidos
gen-preview-failed = Falha na prévia: { $error }
gen-preview-short = Curto demais para um espectro
gen-preview-axis = Espectro médio, { $lo } a { $hi } Hz, faixa de 80 dB
gen-preview-wave-axis = Primeiros { $ms } ms, pico { $peak }

gen-generate = Gerar
    .hover = Adiciona o sinal à sessão como uma nova gravação
gen-generate-save = Gerar e salvar como
    .hover = Gera e depois salva o novo sinal no formato escolhido
gen-busy = Um sinal está sendo gerado
gen-fix-first = Corrija os parâmetros antes
gen-invalid = Não é possível gerar: { $error }
gen-field-error = { $field }: { $error }
gen-ready = Pronto: { $samples } amostras, { $duration } s
gen-list-bad = "{ $item }" não é uma posição nem um intervalo como 10-12
gen-list-too-long = Posições demais (no máximo { $max })
gen-overrides-bad = "{ $item }" não é uma posição com modulação como 5: QAM-16

gen-calc-cp = OFDM com CP
gen-calc-cp-lengths = A partir de Fd, LU e LG
gen-calc-spacing = Espaçamento
gen-calc-symbol = Símbolo
gen-calc-useful = Parte útil
gen-calc-guard = Guarda
gen-calc-use = Usar no OFDM
    .hover = Copia estes comprimentos (e Fd) para o editor OFDM
    .disabled = LU e LG precisam ser números inteiros de amostras
# Why Use in OFDM is disabled. $max: the generator's limit in samples.
gen-calc-lu-not-whole = LU deve ser um número inteiro de amostras, pelo menos 2.
gen-calc-lg-not-whole = LG deve ser um número inteiro de amostras.
gen-calc-lu-too-large = LU deve ser no máximo { $max } amostras.
gen-calc-lg-too-large = LG deve ser no máximo { $max } amostras.
gen-calc-use-k = Usar no OFDM
    .hover = Copia este K para o editor OFDM: o menor LU compatível, a partir do LU atual
gen-calc-inverse = A partir de Br e espaçamento em Fd: LU e LG inteiros mais próximos
gen-calc-exact = exato
gen-calc-error = Erro
gen-calc-candidates = LU e LG inteiros para Br e K
gen-calc-lu-range = LU a partir de
gen-calc-tolerance = Tolerância
gen-calc-found = { $count ->
    [one] { $count } candidato
   *[other] { $count } candidatos, os primeiros { $shown } exibidos
}
gen-calc-klist = Lista de K
gen-calc-k-from-rates = A partir de Br e espaçamento
gen-calc-k-direct = A partir de K
gen-calc-uncertainty = Incerteza
    .hover = Quanto cada taxa medida pode desviar, em percentagem da taxa. A lista mostra as frações mais simples dentro da incerteza de K resultante; para K introduzido diretamente, a tolerância é a de um K medido a partir de duas taxas com esse desvio.
gen-calc-uncertainty-bad = A incerteza deve estar entre 0 e { $max } %.
gen-calc-k-none = Nenhuma fração está dentro da incerteza. Aumente a incerteza ou o maior denominador.
gen-calc-max-den = Maior denominador
gen-calc-count = Quantidade
gen-calc-value = Valor
gen-calc-fd = Fd
    .hover = Taxa de amostragem em Hz que torna estes comprimentos exatos para o Br: Br x (LU + LG)
gen-calc-k = K
    .hover = K = LG/LU, a razão entre guarda e comprimento útil que estes comprimentos inteiros realmente têm
gen-calc-lg-error = Erro (amostras)
    .hover = LG menos K x LU: quanto o comprimento de guarda inteiro se afasta do exato, em amostras
gen-calc-k-fraction = K
    .hover = K como fração p/q irredutível
gen-calc-k-error = Erro
    .hover = O valor da fração menos o K medido (com sinal; K não tem unidade, portanto isto também não)
gen-preview-symbol-too-long = Sem pré-visualização: um símbolo tem { $samples } amostras, mais do que uma pré-visualização. Aumente a taxa de símbolos ou baixe a taxa de amostragem.
gen-calc-fraction-too-big = K = { $k } não cabe: LU deve ser no máximo { $lu } e LG no máximo { $lg } amostras. Escolha um LU ou K menor.
