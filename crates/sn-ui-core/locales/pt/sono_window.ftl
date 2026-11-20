# The sonogram work window: SA's top toolbar captions, the bottom toolbar hints,
# the right-click popup menu and the Correction of BR panel. SA's control names
# (Fl, Fu, ALC, HRes, Info, WaveFile, DataFile) are names: keep them.

## Top toolbar (SizeFFT, Shear, Image and WinF are in app.ftl).

sw-fl = Fl
    .hover = Frequência mais baixa exibida, Hz (azul enquanto a banda está ampliada)
sw-fu = Fu
    .hover = Frequência mais alta exibida, Hz (azul enquanto a banda está ampliada)
sw-alc = ALC
sw-hres = HRes
sw-info = Info
    .hover = Mostrar o painel de resultados (Fu, Fc, Fl, dF, t1, t2, dT, S, Br, Sc)

## Bottom toolbar hints, left to right.

sw-hint-zoom-in = Estender a banda entre os marcadores H à janela
sw-hint-zoom-back = Voltar um passo de zoom
sw-hint-zoom-reset = Redefinir o zoom
sw-hint-abort = Interromper
sw-hint-play = Reproduzir
sw-hint-stop-play = Parar a reprodução
sw-hint-band-select = Selecionar banda
sw-hint-area-delete = Apagar área
sw-hint-band-reject = Rejeitar banda
sw-hint-am = Detector AM em quadratura
sw-hint-modified-am = Detector AM modificado
sw-hint-half-wave = Detector de meia onda duplo
sw-hint-derivative = Primeira derivada
sw-hint-power = Módulo de potência N
sw-hint-phase-plane = PhasePlane
sw-hint-acf = Autocorrelação
sw-hint-ccf = Correlação cruzada
sw-hint-ofdm = Módulo OFDM
sw-hint-phase-detector = Detector de fase
sw-hint-limiter = Limitador rígido
sw-hint-iq = I/Q para real
sw-hint-shift-base = Deslocar banda para a base
sw-hint-shift-base-inverted = Deslocar banda para a base, invertida
sw-hint-inverse = Inversão dentro da banda (todo o espectro se os marcadores H coincidirem)
sw-hint-copy = Copiar fragmento
sw-hint-resample = Reamostrar para a taxa do campo
sw-hint-shift = Deslocar o espectro pelo valor do campo (Hz; negativo desce)
sw-value = R / S
    .hover = Nova taxa para R, ou deslocamento para S, em Hz (vírgula ou ponto decimal)
sw-value-presets = Valores predefinidos

## Why a button is disabled.

sw-need-history = Não há passo de zoom para voltar.
sw-need-running = Nada está em execução nesta janela.
sw-need-playing = Nada está sendo reproduzido.

## Popup menu.

sw-popup-save-as = Salvar como
sw-popup-wave-file = WaveFile
sw-popup-data-file = DataFile
sw-popup-wave-file-float = WaveFile (ponto flutuante de 32 bits)
sw-popup-sigmf = SigMF
sw-popup-waveform = Módulo Wave Form
sw-popup-measure = Medição de frequência
sw-popup-adp = Definir parâmetros (ADP)
sw-popup-brc = Correção de BR
sw-popup-cancel = Cancelar

## Correction of BR panel.

sw-brc-existing = Taxa existente
sw-brc-required = Taxa desejada
sw-brc-max-delta = Mudança máx.: +/- 30%
sw-brc-yes = Sim

## Messages.

sw-check-settings = Verifique as configurações!
# Result boxes in the plot (SA's own text). $value: a number with two decimals.
sw-label-freq = Freq: { $value } Hz
sw-label-brate = Brate: { $value } Hz
sw-rs-resample-invalid = O valor R/S "{ $text }" não é uma taxa de amostragem positiva; digite uma taxa em Hz, por exemplo 8000.
sw-rs-shift-invalid = O valor R/S "{ $text }" não é um número; digite um deslocamento em Hz, por exemplo -150.
sw-brc-existing-invalid = Existent Baud Rate "{ $text }" não é um número; digite o Br medido em Bd ou execute ADP antes.
sw-brc-required-invalid = Required Baud Rate "{ $text }" não é um número; digite o Br desejado em Bd.
sw-brc-not-positive = Ambas as taxas devem ser maiores que 0 Bd.
sw-brc-too-far = O Br pedido deve estar entre { $lo } e { $hi } Bd (no máximo 30% do existente).
sw-brc-measured = Preenchido com o Br medido pelo ADP
sw-brc-not-measured = Nenhum Br medido ainda: digite-o ou execute ADP antes
