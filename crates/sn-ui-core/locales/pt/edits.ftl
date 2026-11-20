# The Signal menu, the parameter dialog of its operations and the save formats.

edits-save-wav-pcm16 = WAV, PCM 16 bits
edits-save-wav-float = WAV, float 32 bits
edits-save-sigmf = SigMF (mantém taxas fracionárias)
edits-save-raw-f32 = Bruto, float 32 bits

# $from: where a measured value came from, e.g. "n4, measure line".
edits-measured-other-axis = { $from } (outro eixo, verifique)

## Notes under the dialog fields: where a pre-filled value came from.

edits-note-cutoff-raised = Corte: Br de { $from } (elevado ao mínimo de 30 Hz; Br é { $br } Bd)
edits-note-cutoff-lowered = Corte: Br de { $from } (reduzido abaixo de metade da taxa de amostragem; Br é { $br } Bd)
edits-note-cutoff-br = Corte: Br de { $from }
edits-note-cutoff-guessed = Corte: estimado (fs/8), meça Br antes
# $count: the longest window offered, in samples.
edits-note-window-capped = { $count ->
    [one] Janela: 1/3 de símbolo a Br de { $from } (limitada a { $count } amostra; reamostre para baixo para ter 1/3 de símbolo real)
   *[other] Janela: 1/3 de símbolo a Br de { $from } (limitada a { $count } amostras; reamostre para baixo para ter 1/3 de símbolo real)
}
edits-note-window-br = Janela: 1/3 de símbolo a Br de { $from }
edits-note-window-default = Janela: padrão de 5 amostras; meça Br para obter 1/3 de símbolo
edits-note-measured-br = Br medido de { $from }
edits-note-measured-s = Medido: S = 1/dT dos marcadores V
edits-note-carrier-sc = Portadora: Sc de { $from }
edits-note-carrier-h = Portadora: centro dos marcadores H
edits-note-carrier-guessed = Portadora: estimada (fs/4), meça Sc antes
edits-note-br-from = Br: de { $from }
edits-note-br-guessed-8 = Br: estimado (fs/8), meça Br antes
edits-note-br-s = Br: S = 1/dT dos marcadores V
edits-note-br-guessed-10 = Br: estimado (fs/10), meça Br antes

## Why a menu item is disabled.

edits-need-h = Primeiro posicione os marcadores H (arraste no sonograma).
edits-need-v = Primeiro posicione os marcadores V (arraste no sonograma ou na forma de onda).
edits-need-vh = Primeiro posicione os marcadores V e H (arraste no sonograma).
edits-need-complex = O sinal já é real.
edits-need-baud = Primeiro meça Br (menu Medir) ou posicione os marcadores V sobre um símbolo.

## Signal menu sections and items.

edits-section-spectrum = Espectro (marcadores H)
edits-band-select = Selecionar banda
edits-band-reject = Rejeitar banda
edits-area-delete = Apagar área
edits-shift-to-base = Deslocar banda para a base
edits-shift-to-base-inverted = Deslocar banda para a base, invertida
edits-invert-band = Inversão dentro da banda
edits-section-whole = Sinal inteiro
edits-shift = Deslocar espectro...
edits-invert = Inversão do espectro
edits-resample = Reamostrar...
edits-iq-to-real = I/Q para real
edits-baud-correction = Correção da taxa de baud...
edits-derivative = Primeira derivada
edits-limiter = Limitador rígido
edits-section-isd = Corretor ISD
edits-isd-am = Corretor ISD, modo AM (cego)...
edits-isd-xn = Corretor ISD, modo X^N...
edits-section-detectors = Detectores
edits-am-detector = Detector AM (relógio de PSK/QAM/FSK)
edits-modified-am = Detector AM modificado (relógio de MFSK)
edits-half-wave = Detector de meia onda duplo
edits-phase-detector = Detector de fase (frequência em Hz)
edits-power = Elevar à potência N...
edits-acf = ACF da seleção
edits-ccf = CCF: localizar a seleção em todo o sinal
edits-section-time = Tempo (marcadores V)
edits-copy = Copiar seleção para novo sinal
edits-section-filters = Filtros
edits-lowpass = Passa-baixas...
edits-median = Mediana...
edits-rrc = RRC (filtro casado)...

## Diálogo de parâmetros (pequeno diálogo modal do SA).

# $operation: the dialog's operation; $signal: the window name, e.g. "test(0)(u1)".
edits-dialog-title = SA - { $operation }: { $signal }
edits-title-shift = Deslocar espectro
edits-title-resample = Reamostrar
edits-title-baud = Correção de BR
edits-title-lowpass = Passa-baixas
edits-title-median = Filtro de mediana
edits-title-rrc = Filtro RRC
edits-title-power = Elevar à potência N
edits-title-isd-am = Corretor ISD (modo AM)
edits-title-isd-xn = Corretor ISD (modo X^N)

edits-field-carrier = Portadora (Hz)
edits-field-br = Br, taxa de símbolos (Bd)
edits-field-blocks = Blocos
edits-field-power-psk = Potência N (ordem PSK)
edits-field-window = Janela (amostras, ímpar)
edits-field-shift = Deslocamento (Hz, + para cima)
edits-field-rate = Nova taxa de amostragem (Hz)
edits-field-measured-br = Taxa existente
edits-field-required-br = Taxa desejada
edits-field-cutoff = Corte (Hz)
edits-presets = predefinições
edits-field-power = Potência N
edits-field-alpha = Roll-off alfa
# $lo, $hi: the limits of a field.
edits-range = De { $lo } a { $hi }
# $max: the sample rate in Hz.
edits-hint-shift = Até +/- { $max } Hz, não 0; frações permitidas
# $max: the longest window, in samples.
edits-hint-window = De 1 a { $max }; um valor par vira o ímpar seguinte
edits-ok = Ok
edits-cancel = Cancelar
edits-field-invalid = { $field } "{ $text }" não é válido. { $range }.
edits-range-open = Acima de { $lo } e abaixo de { $hi }
