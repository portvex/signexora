# The Signal menu, the parameter dialog of its operations and the save formats.

edits-save-wav-pcm16 = WAV, PCM 16 bits
edits-save-wav-float = WAV, flottant 32 bits
edits-save-sigmf = SigMF (garde les fréquences fractionnaires)
edits-save-raw-f32 = Brut, flottant 32 bits

# $from: where a measured value came from, e.g. "n4, measure line".
edits-measured-other-axis = { $from } (autre axe, à vérifier)

## Notes under the dialog fields: where a pre-filled value came from.

edits-note-cutoff-raised = Coupure : Br de { $from } (relevée au minimum de 30 Hz ; Br vaut { $br } Bd)
edits-note-cutoff-lowered = Coupure : Br de { $from } (abaissée sous la moitié de la fréquence d'échantillonnage ; Br vaut { $br } Bd)
edits-note-cutoff-br = Coupure : Br de { $from }
edits-note-cutoff-guessed = Coupure : estimée (fs/8), mesurez d'abord Br
# $count: the longest window offered, in samples.
edits-note-window-capped = { $count ->
    [one] Fenêtre : 1/3 de symbole à Br de { $from } (limitée à { $count } échantillon ; rééchantillonnez plus bas pour un vrai 1/3 de symbole)
   *[other] Fenêtre : 1/3 de symbole à Br de { $from } (limitée à { $count } échantillons ; rééchantillonnez plus bas pour un vrai 1/3 de symbole)
}
edits-note-window-br = Fenêtre : 1/3 de symbole à Br de { $from }
edits-note-window-default = Fenêtre : 5 échantillons par défaut ; mesurez Br pour obtenir 1/3 de symbole
edits-note-measured-br = Br mesuré de { $from }
edits-note-measured-s = Mesuré : S = 1/dT des marqueurs V
edits-note-carrier-sc = Porteuse : Sc de { $from }
edits-note-carrier-h = Porteuse : centre des marqueurs H
edits-note-carrier-guessed = Porteuse : estimée (fs/4), mesurez d'abord Sc
edits-note-br-from = Br : de { $from }
edits-note-br-guessed-8 = Br : estimé (fs/8), mesurez d'abord Br
edits-note-br-s = Br : S = 1/dT des marqueurs V
edits-note-br-guessed-10 = Br : estimé (fs/10), mesurez d'abord Br

## Why a menu item is disabled.

edits-need-h = Placez d'abord les marqueurs H (glisser sur le sonagramme).
edits-need-v = Placez d'abord les marqueurs V (glisser sur le sonagramme ou la forme d'onde).
edits-need-vh = Placez d'abord les marqueurs V et H (glisser sur le sonagramme).
edits-need-complex = Le signal est déjà réel.
edits-need-baud = Mesurez d'abord Br (menu Mesure) ou placez les marqueurs V sur un symbole.

## Signal menu sections and items.

edits-section-spectrum = Spectre (marqueurs H)
edits-band-select = Sélection de bande
edits-band-reject = Réjection de bande
edits-area-delete = Suppression de zone
edits-shift-to-base = Ramener la bande en bande de base
edits-shift-to-base-inverted = Ramener la bande en bande de base, inversée
edits-invert-band = Inversion dans la bande
edits-section-whole = Signal entier
edits-shift = Décaler le spectre...
edits-invert = Inversion du spectre
edits-resample = Rééchantillonner...
edits-iq-to-real = I/Q vers réel
edits-baud-correction = Correction de rapidité...
edits-derivative = Dérivée première
edits-limiter = Limiteur dur
edits-section-isd = Correcteur ISD
edits-isd-am = Correcteur ISD, mode AM (aveugle)...
edits-isd-xn = Correcteur ISD, mode X^N...
edits-section-detectors = Détecteurs
edits-am-detector = Détecteur AM (horloge PSK/QAM/FSK)
edits-modified-am = Détecteur AM modifié (horloge MFSK)
edits-half-wave = Détecteur double demi-onde
edits-phase-detector = Détecteur de phase (fréquence en Hz)
edits-power = Élever à la puissance N...
edits-acf = ACF de la sélection
edits-ccf = CCF : retrouver la sélection partout
edits-section-time = Temps (marqueurs V)
edits-copy = Copier la sélection vers un nouveau signal
edits-section-filters = Filtres
edits-lowpass = Passe-bas...
edits-median = Médian...
edits-rrc = RRC (filtre adapté)...

## Boîte de paramètres (petite boîte modale de SA).

# $operation: the dialog's operation; $signal: the window name, e.g. "test(0)(u1)".
edits-dialog-title = SA - { $operation } : { $signal }
edits-title-shift = Décaler le spectre
edits-title-resample = Rééchantillonner
edits-title-baud = Correction de BR
edits-title-lowpass = Passe-bas
edits-title-median = Filtre médian
edits-title-rrc = Filtre RRC
edits-title-power = Élever à la puissance N
edits-title-isd-am = Correcteur ISD (mode AM)
edits-title-isd-xn = Correcteur ISD (mode X^N)

edits-field-carrier = Porteuse (Hz)
edits-field-br = Br, rapidité (Bd)
edits-field-blocks = Blocs
edits-field-power-psk = Puissance N (ordre PSK)
edits-field-window = Fenêtre (échantillons, impaire)
edits-field-shift = Décalage (Hz, + vers le haut)
edits-field-rate = Nouvelle fréq. d'échantillonnage (Hz)
edits-field-measured-br = Débit existant
edits-field-required-br = Débit voulu
edits-field-cutoff = Coupure (Hz)
edits-presets = préréglages
edits-field-power = Puissance N
edits-field-alpha = Retombée alpha
# $lo, $hi: the limits of a field.
edits-range = De { $lo } à { $hi }
# $max: the sample rate in Hz.
edits-hint-shift = Jusqu'à +/- { $max } Hz, pas 0 ; décimales acceptées
# $max: the longest window, in samples.
edits-hint-window = De 1 à { $max } ; une valeur paire devient l'impaire suivante
edits-ok = Ok
edits-cancel = Annuler
edits-field-invalid = { $field } "{ $text }" n'est pas valide. { $range }.
edits-range-open = Au-dessus de { $lo } et en dessous de { $hi }
