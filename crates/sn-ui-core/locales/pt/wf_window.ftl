# The WF (waveform) window: SA's top toolbar, bottom toolbar hints, popup menu and
# result box. SA's control names (LPF, RRC, Sum, Info, VMW, Br, Z+, Z-) and the
# result box text are names: keep them.

## Top toolbar, left to right.

wf-lpf = LPF
    .hover = Corte do passa-baixa em Hz, ou taxa do filtro RRC (digite ou escolha um valor)
wf-copy-br = <==
    .hover = Copiar o Br medido para o campo LPF
wf-rrc = RRC
    .hover = Ligar ou desligar o filtro RRC (o botão LPF aplica-o então)
wf-rrc-alpha = RRC
    .hover = Roll-off do RRC
wf-freq = 0-0
    .hover = Clique: frequência do fragmento curto entre os marcadores V e os seus períodos inteiros
wf-vmw = VMW
    .hover = Mudar para VMW (o sinal dobrado num período de linha)
wf-sum = Sum
    .hover = Mostrar o histograma dos valores (depois Contar frequências no menu)
wf-info = Info
    .hover = Mostrar os resultados (t1, t2, dT, S, Br)

## Bottom toolbar hints, left to right.

wf-hint-zoom-in = Ampliar (tempo)
wf-hint-zoom-out = Reduzir (tempo)
wf-hint-scale-up = Escala vertical maior
wf-hint-scale-down = Escala vertical menor
wf-hint-zoom-plus = Z+: Zoom entre os marcadores V
wf-hint-zoom-minus = Z-: Voltar um passo de zoom
wf-hint-lpf = Aplicar o filtro LPF (o filtro RRC com RRC ligado)
wf-hint-revert = Voltar ao original
wf-hint-synth = Síntese FSK/MFSK inversa
wf-hint-save = Guardar como WAV de 16 bits (clique direito: WAV float, DataFile ou SigMF)
wf-hint-copy = Copiar o fragmento entre os marcadores V para uma janela nova
wf-hint-demod = Enviar o fragmento ao desmodulador FSK/MFSK
wf-hint-fit = Ajustar a escala vertical ao sinal
wf-hint-acf = Autocorrelação do fragmento (janela nova)
wf-hint-ccf = Correlação cruzada do fragmento com todo o sinal (desenhada aqui)

## Why a button is disabled.

wf-need-revert = Nada a reverter: nem filtro nem CCF.
wf-need-derived = Esta janela mostra o ficheiro aberto, que já está guardado. Guardar serve para resultados feitos aqui.

## Popup menu.

wf-popup-count-freqs = Contar frequências
wf-popup-more = Ferramentas SigNexora
    .hover = Ferramentas do SigNexora que a janela WF do SA não tinha
wf-popup-operations = Operações

## The Count freqs result box (SA's text, not translated). $size: a whole number;
## $value: a number with two decimals.

wf-label-grid = Freq grid: { $size }
wf-label-delta = Delta: { $value } Hz
wf-count-needs-h = Coloque antes os marcadores H em volta dos tons.
wf-count-tones-busy = Find tones ainda está em execução; tente Count freqs de novo quando terminar.
wf-count-freqs-failed = Nenhuma grade de frequências encontrada ({ $error }). Alargue os marcadores H em volta dos tons ou afaste o zoom para ver mais tons.
wf-copy-br-value = Copiar Br = { $br } Bd para o campo LPF
wf-copy-br-missing = Meça Br antes (menu do sonograma: Auto define params)
