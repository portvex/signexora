# Main window (MDI frame): messages, File menu extras, the WF window's toolbar
# and popup menu, and the confirmation dialogs.

## View kinds (child windows)

app-tab-sonogram = Sonograma
app-tab-waveform = Forma de onda
app-tab-raster = Raster (VMW)
app-tab-phase = PhasePlane
app-tab-demod = Demodulador FSK/MFSK
app-tab-ofdm = OFDM

## Lists of names: "A", "A and B", "A, B and C"

app-list-separator = { ", " }
app-list-and = { $rest } e { $last }

## Background jobs (shown in the status bar while they run)

app-job-loading = Carregando { $name }
app-job-overview = Criando visão geral da forma de onda
app-job-saving = Salvando { $path }
app-job-measure-line = Medindo a frequência da linha
app-job-adp = Estimando a taxa de baud (ADP)
app-job-writing = Gravando { $path }
app-job-replay = Reproduzindo a sessão
app-job-script = Executando script
app-job-find-tones = Localizando tons
app-job-playback = Preparando a reprodução
app-job-demodulator = Demodulador

## Status messages

app-ready = Pronto
app-signal-gone = Esse sinal não existe mais.
app-set-h-markers-first = Primeiro posicione os marcadores H em torno da linha.
app-sc-unmappable = Sc não pode ser mapeado através de { $edit }: a saída não tem relação de frequência com a entrada. Meça a portadora antes da etapa { $edit }.
app-recorded = { $kind } = { $value } de { $node } ({ $tool })
app-recorded-uncertain = { $kind } = { $value } +/- { $uncertainty } de { $node } ({ $tool })
app-not-recorded = { $kind } não registrado: { $error }
app-mapped-onto = { $from }, mapeado sobre { $node }
app-session-save-running = Já há um salvamento de sessão em andamento
app-export-report-failed = Exportar relatório: { $error }
app-save-session-failed = Salvar sessão: { $error }
app-open-session-failed = Abrir sessão: { $error }
app-script-error = Erro: { $error }
app-replay-skipped = { $what ->
    [markers] { $node } ({ $label }): os marcadores salvos não puderam ser lidos e foram ignorados: { $error }
    [tones] { $node } ({ $label }): os tons salvos não puderam ser lidos e foram ignorados: { $error }
    [grid] { $node } ({ $label }): a grade de tons salva não pôde ser lida e foi ignorada: { $error }
   *[adp] { $node } ({ $label }): os candidatos ADP salvos não puderam ser lidos e foram ignorados: { $error }
}
app-session-restored = Sessão restaurada
app-session-restored-problems = { $count ->
    [one] Sessão restaurada, { $count } problema (veja a janela)
   *[other] Sessão restaurada, { $count } problemas (veja a janela)
}
app-wrote-report = Relatório gravado em { $path }
app-saved = Salvo em { $path }
app-saved-session = Sessão salva em { $path }
app-saved-session-replaced = Sessão salva em { $path } (a sessão que estava nele foi substituída)
app-cancel-too-late = { $name } terminou antes de o cancelamento ter efeito; o resultado foi mantido.
app-job-cancelled = { $name }: cancelado
app-job-failed = { $name }: { $error }
app-playback-cancelled = Reprodução cancelada
app-derived-opened = { $label }: novo sinal { $node } aberto em { $view }
app-derived-opened-mapped = { $label }: novo sinal { $node } aberto em { $view }; marcadores movidos para os eixos dele
app-derived-opened-cleared = { $label }: novo sinal { $node } aberto em { $view }; alguns marcadores foram removidos: o novo sinal tem outro eixo de tempo ou de frequência
app-derive-failed = { $label }: { $error }
app-measured-line = { $kind } = { $value } +/- { $uncertainty } de { $node } (medição de linha; { $level } dBFS, { $snr } dB acima da banda)
app-measured-line-mapped = { $kind } = { $value } +/- { $uncertainty } de { $node } (medição de linha; { $level } dBFS, { $snr } dB acima da banda), linha em { $line } Hz mapeada de volta pela potência { $power }
app-loaded = { $label } carregado: { $samples } amostras a { $rate } Hz
app-loaded-notes = { $label } carregado: { $samples } amostras a { $rate } Hz. { $notes }
app-script-no-signals = O script { $path } não gerou sinais
app-script-finished = Script { $path } concluído
app-adp-none = O ADP não encontrou taxa de manipulação na seleção
app-tones-found = { $count ->
    [one] { $count } tom encontrado em { $node }
   *[other] { $count } tons encontrados em { $node }
}
app-markers-restored = Marcadores restaurados em { $node }
app-closed-tab-stopped = { $count ->
    [one] { $name } fechada; a tarefa { $jobs } em execução foi parada. Abra a janela de novo e execute-a outra vez.
   *[other] { $name } fechada; as tarefas { $jobs } em execução foram paradas. Abra a janela de novo e execute-as outra vez.
}
app-nothing-to-play = Nada para reproduzir: abra o sonograma ou posicione os marcadores V.
app-find-tones-failed = Localizar tons: { $error }

## Confirmation before work is lost

app-confirm-open-title = Abrir sessão
app-confirm-open-question = Substituir a sessão atual?
app-confirm-replace = Substituir
app-confirm-script-title = Executar script
app-confirm-script-question = Os sinais do script substituem a sessão atual.
app-confirm-quit-title = Sair
app-confirm-quit-unsaved = Salvar a sessão antes de sair?
app-confirm-quit-without-saving = Sair sem salvar
app-confirm-quit-writing = Um arquivo ainda está sendo gravado.
app-confirm-quit-anyway = Sair mesmo assim
app-confirm-lost = { $signals ->
    [one] { $signals } sinal
   *[other] { $signals } sinais
} e { $measurements ->
    [one] { $measurements } medição
   *[other] { $measurements } medições
} não salvos serão fechados. Isso não pode ser desfeito.
app-confirm-write-running = { $name } ainda está em execução; sair agora o abandona.
app-confirm-save-first = Salvar sessão antes
app-confirm-save-first-ask = Salvar sessão antes...
app-confirm-wait = Aguardar
app-confirm-keep-open = Manter o app aberto
app-cancel = Cancelar

## File dialogs

app-dialog-save-as = Salvar como { $format }
app-dialog-save-session-as = Salvar sessão como
app-filter-session = Sessão SigNexora
app-dialog-export-report = Exportar relatório de análise
app-filter-report = Relatório
app-dialog-open-session = Abrir sessão
app-dialog-run-script = Executar script
app-filter-rhai = Script Rhai

## File menu

app-menu-file = Arquivo
app-menu-open-session = Abrir sessão...
    .hover = Os sinais dela substituem a sessão atual
app-nothing-to-save = Nada para salvar
app-menu-save-session = Salvar sessão
app-menu-save-session-ask = Salvar sessão...
app-menu-save-session-choose = Escolha um arquivo para a sessão
app-menu-save-session-to = Salvar em { $path }
app-menu-save-session-as = Salvar sessão como...
app-menu-export-report = Exportar relatório de análise
app-menu-report-html = HTML...
app-menu-report-markdown = Markdown...

## File menu extras and the WF popup menu

app-menu-run-script = Executar script (Rhai)...
    .hover = Executa um script; os sinais dele substituem a sessão atual
app-resynth = Ressintetizar FSK/MFSK
    .hover = Gera um sinal limpo nos tons de Localizar tons, a Br
    .disabled = Primeiro localize os tons (Soma da forma de onda) e meça Br.
app-menu-measure = Medir
app-measure-hint = Posicione os marcadores H em torno da linha (e os marcadores V para o intervalo).
app-measure-br = Medir linha como taxa de baud (Br)
app-measure-sc = Medir linha como portadora (Sc)
    .hover = Após a potência N, a linha é mapeada de volta à portadora (dividida por N, deslocamentos desfeitos)
app-measure-adp = Estimar taxa de baud automaticamente (ADP)
    .hover = Usa o intervalo dos marcadores V ou o sinal inteiro
app-measure-dt-disabled = Primeiro posicione os marcadores V.

## Hints

app-no-signals-hint = Abra uma gravação (Arquivo > Abrir arquivo... ({ $shortcut }), ou solte um arquivo na janela).
app-open-in = Abrir...
    .hover = Abrir este sinal em outra visualização ou ferramenta

## Sonogram toolbar

app-sono-shear = Shear
    .hover = Passo entre quadros FFT, como fração do tamanho da FFT. Um passo menor sobrepõe mais os quadros: mais detalhe no tempo, cálculo mais lento
app-sono-image = Image
    .hover = Faixa dinâmica exibida abaixo da componente mais forte
app-sono-hres-hover = Afina as linhas espectrais por reatribuição de frequência (ajuda a achar linhas de relógio fracas). Os tons mantêm o nível; o ruído aparece cerca de 2 a 6 dB mais baixo do que com HRes desligado (conforme a janela), pois é exibido por largura de bin.
app-sono-alc-hover = Normaliza cada coluna pelo seu componente mais forte (só exibição)
app-sono-combine = Zoom afastado
    .hover = Como se combinam, com zoom afastado, os quadros FFT que partilham uma coluna da imagem
app-sono-peak = Pico
    .hover = Com zoom afastado: mantém visíveis as rajadas curtas
app-sono-mean = Média
    .hover = Com zoom afastado: média, ruído mais suave

## Waveform toolbar

app-wave-show-q = Mostrar Q
app-find-tones = Localizar tons
    .hover = Picos do histograma: frequências dos tons na saída do detector de fase

## Zoom and playback

app-play = Reproduzir
    .hover = Reproduz o intervalo dos marcadores V ou o intervalo visível ({ $shortcut })
app-play-stop = Parar
    .hover = Parar a reprodução ({ $shortcut })

## Child windows

app-tab-signal-gone = Este sinal não existe mais.

## Windows

app-replay-problems-title = Sessão restaurada com problemas
app-script-output = Saída do script: { $path }
app-script-no-output = (sem saída)
# $lines: the lines kept; $dropped: the lines left out.
app-script-output-truncated = Saída truncada após { $lines } linhas; { $dropped } linhas a mais não foram mantidas.

## Closing a window, the sonogram toolbar

app-confirm-close-tab-title = Fechar a janela?
# $name: the window title; $jobs: the job names, joined.
app-confirm-close-tab-question = { $count ->
    [one] Fechar { $name } interrompe a tarefa { $jobs } em execução. Fechar mesmo assim?
   *[other] Fechar { $name } interrompe as tarefas { $jobs } em execução. Fechar mesmo assim?
}
app-confirm-close-and-stop = Fechar e interromper
app-confirm-keep-tab = Manter aberta
# $prev, $next: the keys that show the previous and next signal.
app-select-signal-first = Traga primeiro uma janela para a frente ({ $prev } ou { $next } mostra os sinais um a um).
app-sono-fft = SizeFFT
    .hover = Tamanho da FFT: maior dá resolução de frequência mais fina e resolução de tempo mais grosseira
app-sono-window = WinF
    .hover = Função de janela de cada FFT (WinF no SA)
app-sono-colormap = Cores
    .hover = Mapa de cores da imagem do sonograma
app-confirm-close-all-title = Fechar todas as janelas
app-confirm-close-all-question = Isto remove todos os sinais e medições da sessão.
app-confirm-close-all = Fechar tudo
app-confirm-close-all-stops = { $name } ainda está em execução; fechar todas as janelas o interrompe.
app-replay-session-skipped = { $what ->
    [windows] O layout de janelas salvo não pôde ser lido; abre-se uma janela por resultado: { $error }
   *[counters] Os contadores de nomes salvos não puderam ser lidos; novos nomes podem repetir números: { $error }
}
app-replay-windows-missing = { $count ->
    [one] 1 janela salva não foi aberta porque seu sinal não foi refeito
   *[other] { $count } janelas salvas não foram abertas porque seus sinais não foram refeitos
}
app-drop-loader-open = A caixa de arquivo está aberta: conclua ou cancele e solte o arquivo de novo
app-drop-first-only = { $count } arquivos soltos; só o primeiro, { $name }, é aberto
app-sono-target = { $active ->
    [yes] Altera a janela de sonograma ativa.
   *[no] Altera o padrão dos novos sonogramas.
}
app-edit-unrecordable = não foi possível registrá-lo na sessão para refazer ({ $error })
app-empty-open = Abrir uma gravação... ({ $key })
app-empty-drop = ou solte um arquivo aqui (File > Open file... também abre um).
app-empty-guide = O guia ({ $key }) mostra a análise passo a passo.
# $action: what the chooser was for, e.g. "Open session"; $error: why it failed.
app-chooser-failed = { $action }: não foi possível mostrar o seletor de arquivos ({ $error }). Tente novamente ou verifique se o serviço de diálogos de arquivos do sistema (portal da área de trabalho) está em execução.

## The tools that record a measurement, by the id stored in the session.

measure-tool-adp = Definir parâmetros (ADP)
measure-tool-adp-coarse = Definir parâmetros, aproximado (dentro de meio bin de FFT)
measure-tool-measure-line = Medição de frequência (linha de medição)
measure-tool-v-markers = Marcadores V (dT)
measure-tool-ofdm-adp = Ferramenta OFDM, Get Br (ADP)
measure-tool-script-record = Script, record()
