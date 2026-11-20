# Texts of the Windows 95 front end (sn-gui) that the egui app does not have:
# message box buttons, the appearance schemes, the system menu of a child
# window, credits and notices. An & marks the mnemonic letter (Alt+letter).

## Message box buttons.

gui-button-yes = &Sí
gui-button-no = &No

## Options > Theme: SigNexora's themes as Windows appearance schemes.

gui-scheme-standard = Estándar de Windows
gui-scheme-high-contrast-black = Contraste alto negro
gui-scheme-high-contrast-white = Contraste alto blanco

## The system menu of a child window.

gui-sys-restore = &Restaurar
gui-sys-move = &Mover
gui-sys-size = &Tamaño
gui-sys-minimize = Mi&nimizar
gui-sys-maximize = Ma&ximizar
gui-sys-close = &Cerrar
gui-sys-next = Si&guiente

## Help > About: credits of the Windows 95 front end.

gui-about-credit-gpui = GPUI de Zed Industries (Apache-2.0).
gui-about-credit-w95fa = Fuente W95FA de Alina Sava (FontsArena), SIL Open Font License 1.1.
gui-about-credit-selawik = Fuente Selawik de Microsoft, SIL Open Font License 1.1 (cuando Segoe UI, Tahoma o Trebuchet MS no está instalada).
gui-about-credit-linux-fonts = Fuentes Cantarell y Noto Sans, SIL Open Font License 1.1; fuente DejaVu Sans, licencia Bitstream Vera (estilos de Linux).

## Notices.

# $path: the settings file; $error: why it could not be written.
gui-prefs-not-saved = No se pudo guardar la configuración en { $path }: { $error }. Compruebe que la carpeta admite escritura.
# $path: the settings file.
gui-prefs-not-encoded = No se pudo guardar la configuración en { $path }: no se pudo convertir al formato del archivo. Es un error del programa; infórmelo. Se conserva el archivo de configuración anterior.

## The signal generator (OCG) window and its calculators.

gui-gen-preset = &Preajuste...
gui-gen-generate = &Generar
gui-gen-generate-save = Generar y guardar &como...
gui-gen-calculators = Ca&lculadoras...
gui-gen-calc-title = Calculadoras OCG
# $text: what was typed in a number field.
gui-gen-not-number = "{ $text }" no es un número
gui-gen-not-whole = "{ $text }" no es un número entero
# $min, $max: the limits of the field.
gui-gen-range = debe estar entre { $min } y { $max }

## Help > Use help: the Windows 95 keys.

gui-keys-menu = Seleccionar la barra de menús
gui-keys-menu-letter = Abrir el menú, o pulsar el botón, que tiene esa letra subrayada
gui-keys-next-window = Traer al frente la ventana siguiente (con Mayús: la anterior)
