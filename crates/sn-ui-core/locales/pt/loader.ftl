# SA's file loader, the "SA Open file" window, and its "SA - select channel"
# question. Keep the window titles, the SA control captions (Use options, Data
# start, Channels, Bits, Freq rate, Wave tag, Bits per sample, Data size, Ext.
# Info) and the list entries (All - I/Q, Ch - 1, 16, 24-A, ...) untranslated.

loader-title = SA Open file
loader-ok = OK
loader-cancel = Cancelar

## Toolbar buttons.

loader-back = Voltar
    .hover = Voltar à pasta anterior
loader-up = Acima
    .hover = Subir uma pasta (Backspace)
loader-view = Modo de exibição
    .hover = Mudar o modo de exibição: lista ou detalhes
loader-refresh = Atualizar
    .hover = Atualizar a lista

## The list.

loader-col-name = Nome
loader-col-type = Tipo/Ext
loader-col-size = Tamanho
loader-col-date = Data
loader-col-time = Hora
loader-type-folder = Pasta
loader-type-link = F-Link
loader-popup-update = Atualizar
loader-popup-auto-preview = Pré-visualização automática
loader-popup-copy = Copiar
loader-popup-paste = Colar
loader-popup-cancel = Cancelar
loader-search = Busca rápida
    .hover = Digite parte de um nome para selecionar o primeiro arquivo que o contém
loader-filter-wav-mp3 = wav & mp3
loader-filter-wav = wav
loader-filter-mp3 = mp3
loader-filter-supported = todos suportados
loader-filter-all = *.*
# $path: what was typed or pasted.
loader-path-not-found = Caminho não encontrado: { $path }

## The info panel and the options.

loader-folder-link = Link de pasta
# $value: a number from the file header.
loader-info-tag = Wave tag = { $value }
loader-info-channels = Channels = { $value }
loader-info-rate = Freq rate = { $value }
loader-info-bits = Bits per sample = { $value }
loader-info-size = Data size = { $value }
loader-info-no-header = Sem cabeçalho
loader-info-unreadable = Formato desconhecido
# $codec: the codec name of the file, e.g. "Intel DVI ADPCM (IMA ADPCM)".
loader-ext-info = Ext. Info: { $codec }
loader-use-options = Use options
    .hover = Ler o arquivo com estas opções em vez do cabeçalho (necessário sem cabeçalho)
loader-use-options-compressed = Os arquivos comprimidos são decodificados pelo cabeçalho; opções brutas leriam ruído.
loader-use-options-headerless = Dados sem cabeçalho só podem ser lidos com estas opções.
loader-data-start = Data start
    .hover = Bytes a saltar antes das amostras
loader-channels = Channels
    .hover = O canal a carregar, ou All para os dois primeiros como I/Q
loader-bits = Bits
    .hover = Formato da amostra e ordem dos bytes
loader-freq-rate = Freq rate
    .hover = Taxa de amostragem, Hz
loader-in-file = Canais no arquivo
    .hover = Canais intercalados neste arquivo sem cabeçalho
# $error: why the file cannot be read.
loader-cannot-read = Não é possível ler este arquivo: { $error }
loader-check-settings = Verifique as configurações!

## The preview.

loader-preview-building = Criando a pré-visualização...
loader-prescaler = Pré-escalador
    .hover = Pré-escalador: { $gain } dB. Amostras acima da escala completa (linhas vermelhas) são cortadas.

## SA - select channel.

loader-select-channel-title = SA - select channel
loader-select-channel-group = Este arquivo é estéreo, selecione
loader-channel-1 = Canal - 1
loader-channel-2 = Canal - 2
loader-all-samples = Todas as amostras
loader-break = Interromper
loader-needs-options = Este arquivo não tem cabeçalho legível: marque Use options e defina o formato.
loader-no-file = Selecione antes um arquivo.
loader-rate-invalid = Freq rate "{ $text }" não é válido: digite uma taxa acima de 0 e até 1000000000 Hz, por exemplo 48000.
loader-data-start-invalid = Data start "{ $text }" não é válido: digite um número inteiro de bytes, por exemplo 0 ou 44.
loader-list-error = Não é possível ler a pasta { $path }: { $error }. Verifique as permissões ou escolha outra pasta.
