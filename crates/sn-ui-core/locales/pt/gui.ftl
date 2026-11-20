# Texts of the Windows 95 front end (sn-gui) that the egui app does not have:
# message box buttons, the appearance schemes, the system menu of a child
# window, credits and notices. An & marks the mnemonic letter (Alt+letter).

## Message box buttons.

gui-button-yes = &Sim
gui-button-no = &Não

## Options > Theme: SigNexora's themes as Windows appearance schemes.

gui-scheme-standard = Padrão do Windows
gui-scheme-high-contrast-black = Alto contraste preto
gui-scheme-high-contrast-white = Alto contraste branco

## The system menu of a child window.

gui-sys-restore = &Restaurar
gui-sys-move = &Mover
gui-sys-size = &Tamanho
gui-sys-minimize = Mi&nimizar
gui-sys-maximize = Ma&ximizar
gui-sys-close = &Fechar
gui-sys-next = &Seguinte

## Help > About: credits of the Windows 95 front end.

gui-about-credit-gpui = GPUI da Zed Industries (Apache-2.0).
gui-about-credit-w95fa = Fonte W95FA de Alina Sava (FontsArena), SIL Open Font License 1.1.
gui-about-credit-selawik = Fonte Selawik da Microsoft, SIL Open Font License 1.1 (quando Segoe UI, Tahoma ou Trebuchet MS não está instalada).
gui-about-credit-linux-fonts = Fontes Cantarell e Noto Sans, SIL Open Font License 1.1; fonte DejaVu Sans, licença Bitstream Vera (estilos Linux).

## Notices.

# $path: the settings file; $error: why it could not be written.
gui-prefs-not-saved = Não foi possível guardar as configurações em { $path }: { $error }. Verifique se a pasta permite gravação.
# $path: the settings file.
gui-prefs-not-encoded = Não foi possível guardar as configurações em { $path }: não foi possível convertê-las para o formato do arquivo. Este é um erro do programa; informe-o. O arquivo de configurações anterior é mantido.

## The signal generator (OCG) window and its calculators.

gui-gen-preset = &Predefinição...
gui-gen-generate = &Gerar
gui-gen-generate-save = Gerar e &salvar como...
gui-gen-calculators = &Calculadoras...
gui-gen-calc-title = Calculadoras OCG
# $text: what was typed in a number field.
gui-gen-not-number = "{ $text }" não é um número
gui-gen-not-whole = "{ $text }" não é um número inteiro
# $min, $max: the limits of the field.
gui-gen-range = deve estar entre { $min } e { $max }

## Help > Use help: the Windows 95 keys.

gui-keys-menu = Selecionar a barra de menus
gui-keys-menu-letter = Abrir o menu, ou pressionar o botão, que tem essa letra sublinhada
gui-keys-next-window = Trazer para a frente a janela seguinte (com Shift: a anterior)
