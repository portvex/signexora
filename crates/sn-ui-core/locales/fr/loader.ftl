# SA's file loader, the "SA Open file" window, and its "SA - select channel"
# question. Keep the window titles, the SA control captions (Use options, Data
# start, Channels, Bits, Freq rate, Wave tag, Bits per sample, Data size, Ext.
# Info) and the list entries (All - I/Q, Ch - 1, 16, 24-A, ...) untranslated.

loader-title = SA Open file
loader-ok = Ok
loader-cancel = Annuler

## Toolbar buttons.

loader-back = Précédent
    .hover = Revenir au dossier précédent
loader-up = Monter
    .hover = Monter d'un dossier (Retour arrière)
loader-view = Mode d'affichage
    .hover = Changer le mode d'affichage : liste ou détails
loader-refresh = Actualiser
    .hover = Actualiser la liste

## The list.

loader-col-name = Nom
loader-col-type = Type/Ext
loader-col-size = Taille
loader-col-date = Date
loader-col-time = Heure
loader-type-folder = Dossier
loader-type-link = F-Link
loader-popup-update = Actualiser
loader-popup-auto-preview = Aperçu automatique
loader-popup-copy = Copier
loader-popup-paste = Coller
loader-popup-cancel = Annuler
loader-search = Recherche rapide
    .hover = Tapez une partie d'un nom pour sélectionner le premier fichier qui la contient
loader-filter-wav-mp3 = wav & mp3
loader-filter-wav = wav
loader-filter-mp3 = mp3
loader-filter-supported = tous gérés
loader-filter-all = *.*
# $path: what was typed or pasted.
loader-path-not-found = Chemin introuvable : { $path }

## The info panel and the options.

loader-folder-link = Lien de dossier
# $value: a number from the file header.
loader-info-tag = Wave tag = { $value }
loader-info-channels = Channels = { $value }
loader-info-rate = Freq rate = { $value }
loader-info-bits = Bits per sample = { $value }
loader-info-size = Data size = { $value }
loader-info-no-header = Pas d'en-tête
loader-info-unreadable = Format inconnu
# $codec: the codec name of the file, e.g. "Intel DVI ADPCM (IMA ADPCM)".
loader-ext-info = Ext. Info: { $codec }
loader-use-options = Use options
    .hover = Lire le fichier avec ces réglages au lieu de son en-tête (nécessaire sans en-tête)
loader-use-options-compressed = Les fichiers compressés sont décodés d'après leur en-tête ; des options brutes liraient du bruit.
loader-use-options-headerless = Les données sans en-tête ne peuvent être lues qu'avec ces options.
loader-data-start = Data start
    .hover = Octets à sauter avant les échantillons
loader-channels = Channels
    .hover = Le canal à charger, ou All pour les deux premiers en I/Q
loader-bits = Bits
    .hover = Format des échantillons et ordre des octets
loader-freq-rate = Freq rate
    .hover = Fréquence d'échantillonnage, Hz
loader-in-file = Voies du fichier
    .hover = Voies entrelacées dans ce fichier sans en-tête
# $error: why the file cannot be read.
loader-cannot-read = Impossible de lire ce fichier : { $error }
loader-check-settings = Vérifiez les réglages !

## The preview.

loader-preview-building = Construction de l'aperçu...
loader-prescaler = Préamplification
    .hover = Préamplification : { $gain } dB. Les échantillons au-delà de la pleine échelle (lignes rouges) sont écrêtés.

## SA - select channel.

loader-select-channel-title = SA - select channel
loader-select-channel-group = Ce fichier est stéréo, choisissez
loader-channel-1 = Voie - 1
loader-channel-2 = Voie - 2
loader-all-samples = Tous les échantillons
loader-break = Interrompre
loader-needs-options = Ce fichier n'a pas d'en-tête lisible : cochez Use options et indiquez son format.
loader-no-file = Sélectionnez d'abord un fichier.
loader-rate-invalid = Freq rate "{ $text }" n'est pas valide : entrez une fréquence entre 0 (exclu) et 1000000000 Hz, par exemple 48000.
loader-data-start-invalid = Data start "{ $text }" n'est pas valide : entrez un nombre entier d'octets, par exemple 0 ou 44.
loader-list-error = Impossible de lire le dossier { $path } : { $error }. Vérifiez ses droits ou choisissez un autre dossier.
