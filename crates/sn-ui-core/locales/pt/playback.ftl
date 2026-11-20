# Audio playback of a time span: errors and status messages.

playback-rate-too-low = A taxa de amostragem é baixa demais para reproduzir.
# $rate: the sample rate in Hz.
playback-rate-too-high = A reprodução é para sinais em taxa de áudio; este é amostrado a { $rate } Hz. Desloque a banda de interesse para baixo e reamostre-a antes.
playback-nothing = Nada para reproduzir: selecione um intervalo de tempo com os marcadores V.
# $got and $total: seconds; $name: the signal, e.g. "n3".
playback-short-read = Só foi possível ler { $got } s do intervalo de { $total } s de { $name }; o arquivo pode ter mudado no disco. Reabra a gravação.
# $error: the audio system's error text.
playback-no-output = Não é possível abrir a saída de áudio: { $error }
playback-output-unavailable = A saída de áudio não está disponível.
# $seconds: the length played.
playback-playing = Reproduzindo { $seconds } s
# $seconds: the length played; $limit: the playback limit at this rate, in seconds.
playback-playing-first-markers = Reproduzindo só os primeiros { $seconds } s (a reprodução é limitada a { $limit } s nesta taxa); mova V1 para o fim deste trecho para ouvir o resto.
playback-playing-first-view = Reproduzindo só os primeiros { $seconds } s (a reprodução é limitada a { $limit } s nesta taxa); posicione os marcadores V no resto para ouvi-lo.
playback-bad-samples = { $count } amostras que não eram números são reproduzidas como silêncio.
