# Texts of the Windows 95 front end (sn-gui) that the egui app does not have:
# message box buttons, the appearance schemes, the system menu of a child
# window, credits and notices. An & marks the mnemonic letter (Alt+letter).

## Message box buttons.

gui-button-yes = &Oui
gui-button-no = &Non

## Options > Theme: SigNexora's themes as Windows appearance schemes.

gui-scheme-standard = Standard Windows
gui-scheme-high-contrast-black = Contraste élevé noir
gui-scheme-high-contrast-white = Contraste élevé blanc

## The system menu of a child window.

gui-sys-restore = &Restaurer
gui-sys-move = &Déplacer
gui-sys-size = &Taille
gui-sys-minimize = Réd&uire
gui-sys-maximize = A&grandir
gui-sys-close = &Fermer
gui-sys-next = Sui&vant

## Help > About: credits of the Windows 95 front end.

gui-about-credit-gpui = GPUI de Zed Industries (Apache-2.0).
gui-about-credit-w95fa = Police W95FA d'Alina Sava (FontsArena), SIL Open Font License 1.1.
gui-about-credit-selawik = Police Selawik de Microsoft, SIL Open Font License 1.1 (quand Segoe UI, Tahoma ou Trebuchet MS n'est pas installée).
gui-about-credit-linux-fonts = Polices Cantarell et Noto Sans, SIL Open Font License 1.1 ; police DejaVu Sans, licence Bitstream Vera (styles Linux).

## Notices.

# $path: the settings file; $error: why it could not be written.
gui-prefs-not-saved = Les réglages n'ont pas pu être enregistrés dans { $path } : { $error }. Vérifiez que le dossier est accessible en écriture.
# $path: the settings file.
gui-prefs-not-encoded = Les réglages n'ont pas pu être enregistrés dans { $path } : ils n'ont pas pu être convertis au format du fichier. C'est une erreur du programme ; veuillez la signaler. L'ancien fichier de réglages est conservé.

## The signal generator (OCG) window and its calculators.

gui-gen-preset = &Préréglage...
gui-gen-generate = &Générer
gui-gen-generate-save = Générer et &enregistrer sous...
gui-gen-calculators = &Calculateurs...
gui-gen-calc-title = Calculateurs OCG
# $text: what was typed in a number field.
gui-gen-not-number = "{ $text }" n'est pas un nombre
gui-gen-not-whole = "{ $text }" n'est pas un nombre entier
# $min, $max: the limits of the field.
gui-gen-range = doit être compris entre { $min } et { $max }

## Help > Use help: the Windows 95 keys.

gui-keys-menu = Sélectionner la barre de menus
gui-keys-menu-letter = Ouvrir le menu, ou appuyer sur le bouton, qui a cette lettre soulignée
gui-keys-next-window = Mettre au premier plan la fenêtre suivante (avec Maj : la précédente)
