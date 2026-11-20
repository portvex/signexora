# The workflow guide: SA's analysis method as a checklist with buttons.

guide-title = Guia de trabalho
guide-no-signal = Abra primeiro uma gravação (Arquivo > Abrir arquivo...).
# $signal: the active window's signal, which the buttons act on.
guide-acts-on = Os botões agem sobre: { $signal }
# $key: a key name. Shown after a menu path only when that key acts on the guide's signal.
guide-key = , tecla { $key }

## Steps: the message is the step's title, .text its explanation.

guide-step-record = 1. Grave corretamente
    .text = WAV PCM de 16 bits (ou I/Q) com o AGC do receptor desligado, sem saturação, sem edição. Arquivos comprimidos ou de 8 bits são sinalizados ao abrir.
guide-step-quality = 2. Verificar a qualidade da gravação
    .text = Abra a janela WF (clique direito no sonograma > { sw-popup-waveform }). Procure ceifamento, degraus de 8 bits, componente contínua, bombeamento do AGC e falhas. Ative Soma para ver a distribuição de amplitudes.
guide-step-isolate = 3. Isolar o sinal
    .text = No sonograma, posicione os marcadores H em volta do sinal. Depois use { sw-hint-band-select } e { sw-hint-shift-base } da barra, e R para reamostrar mais baixo se ele for largo.
guide-step-rate = 4. Encontrar a taxa de manipulação
    .text = Aplique um detector, enquadre a sua linha de relógio com os marcadores e use { sw-popup-measure } do menu de contexto{ $br }. { sw-hint-am } para PSK/QAM/FSK, { sw-hint-modified-am } para MFSK, ou tente { sw-popup-adp }.
guide-step-family = 5. Estudar a família de modulação
    .text = FSK/MFSK: { sw-hint-phase-detector }, depois na janela WF { edits-median } sobre 1/3 de símbolo, Soma, Encontrar tons, Contar frequências. PSK/QAM: { sw-hint-power } para ordem e portadora ({ sw-popup-measure } na sua janela{ $sc }), depois PhasePlane; em sinais distorcidos use antes o corretor ISD. OFDM: Encontrar CT, Obter Br, Canais e a lista de K para a taxa nativa.
guide-step-structure = 6. Encontrar a estrutura
    .text = { sw-hint-acf } para períodos de quadro, { sw-hint-ccf } para achar um fragmento em todo o sinal, depois posicione os marcadores V sobre um quadro e escolha { sw-popup-measure } > { raster-dt-menu-item } no menu de contexto para registar dT (o período de quadro) e abra { guide-btn-raster } com esse período para ver quadros e palavras de sincronismo.

## Step buttons.

guide-btn-waveform = Forma de onda
guide-btn-sonogram = Sonograma
guide-btn-am-detector = Detector AM
guide-btn-modified-am = AM modificado
guide-btn-phase-detector = Detector de fase
guide-btn-demod = Demodulador FSK/MFSK
# $n: the power, a number.
guide-btn-power = Elevar à potência { $n }
    .hover = Cria um novo sinal elevado à potência { $n }; um sinal PSK-{ $n } mostra então uma linha em { $n } vezes a portadora
guide-btn-phase-plane = PhasePlane
guide-btn-raster = Raster (VMW)
guide-btn-adp = ADP
    .hover = Estimar automaticamente a taxa de manipulação (ADP) no intervalo dos marcadores V ou no sinal inteiro
guide-btn-ofdm = OFDM
    .hover = Abrir a ferramenta OFDM (Localizar CT, Obter Br, Lista de K)
guide-open-view = Abrir esta vista do sinal da janela ativa
guide-new-signal = Cria um sinal novo a partir do da janela ativa (abre numa nova janela)
# $kind: Br, Sc or dT (untranslated).
guide-not-measured = { $kind }: ainda não medido
