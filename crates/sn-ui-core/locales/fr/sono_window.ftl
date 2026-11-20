# The sonogram work window: SA's top toolbar captions, the bottom toolbar hints,
# the right-click popup menu and the Correction of BR panel. SA's control names
# (Fl, Fu, ALC, HRes, Info, WaveFile, DataFile) are names: keep them.

## Top toolbar (SizeFFT, Shear, Image and WinF are in app.ftl).

sw-fl = Fl
    .hover = Fréquence la plus basse affichée, Hz (bleu pendant le zoom sur la bande)
sw-fu = Fu
    .hover = Fréquence la plus haute affichée, Hz (bleu pendant le zoom sur la bande)
sw-alc = ALC
sw-hres = HRes
sw-info = Info
    .hover = Afficher le panneau des résultats (Fu, Fc, Fl, dF, t1, t2, dT, S, Br, Sc)

## Bottom toolbar hints, left to right.

sw-hint-zoom-in = Étendre la bande entre les marqueurs H à la fenêtre
sw-hint-zoom-back = Zoom arrière d'un pas
sw-hint-zoom-reset = Annuler le zoom
sw-hint-abort = Arrêter
sw-hint-play = Lire
sw-hint-stop-play = Arrêter la lecture
sw-hint-band-select = Sélection de bande
sw-hint-area-delete = Suppression de zone
sw-hint-band-reject = Réjection de bande
sw-hint-am = Détecteur AM en quadrature
sw-hint-modified-am = Détecteur AM modifié
sw-hint-half-wave = Détecteur double demi-onde
sw-hint-derivative = Dérivée première
sw-hint-power = Module de puissance N
sw-hint-phase-plane = PhasePlane
sw-hint-acf = Autocorrélation
sw-hint-ccf = Intercorrélation
sw-hint-ofdm = Module OFDM
sw-hint-phase-detector = Détecteur de phase
sw-hint-limiter = Limiteur dur
sw-hint-iq = I/Q vers réel
sw-hint-shift-base = Ramener la bande en bande de base
sw-hint-shift-base-inverted = Ramener la bande en bande de base, inversée
sw-hint-inverse = Inversion dans la bande (tout le spectre si les marqueurs H sont confondus)
sw-hint-copy = Copier le fragment
sw-hint-resample = Rééchantillonner à la fréquence du champ
sw-hint-shift = Décaler le spectre de la valeur du champ (Hz ; négatif vers le bas)
sw-value = R / S
    .hover = Nouvelle fréquence pour R, ou décalage pour S, en Hz (virgule ou point décimal)
sw-value-presets = Valeurs prédéfinies

## Why a button is disabled.

sw-need-history = Aucun pas de zoom auquel revenir.
sw-need-running = Rien ne tourne pour cette fenêtre.
sw-need-playing = Aucune lecture en cours.

## Popup menu.

sw-popup-save-as = Enregistrer sous
sw-popup-wave-file = WaveFile
sw-popup-data-file = DataFile
sw-popup-wave-file-float = WaveFile (flottant 32 bits)
sw-popup-sigmf = SigMF
sw-popup-waveform = Module Wave Form
sw-popup-measure = Mesure de fréquence
sw-popup-adp = Définir les paramètres (ADP)
sw-popup-brc = Correction de BR
sw-popup-cancel = Annuler

## Correction of BR panel.

sw-brc-existing = Débit existant
sw-brc-required = Débit voulu
sw-brc-max-delta = Écart max. : +/- 30%
sw-brc-yes = Oui

## Messages.

sw-check-settings = Vérifiez les réglages !
# Result boxes in the plot (SA's own text). $value: a number with two decimals.
sw-label-freq = Freq: { $value } Hz
sw-label-brate = Brate: { $value } Hz
sw-rs-resample-invalid = La valeur R/S "{ $text }" n'est pas une fréquence d'échantillonnage positive ; tapez une valeur en Hz, par exemple 8000.
sw-rs-shift-invalid = La valeur R/S "{ $text }" n'est pas un nombre ; tapez un décalage en Hz, par exemple -150.
sw-brc-existing-invalid = Existent Baud Rate "{ $text }" n'est pas un nombre ; tapez le Br mesuré en Bd ou lancez d'abord ADP.
sw-brc-required-invalid = Required Baud Rate "{ $text }" n'est pas un nombre ; tapez le Br voulu en Bd.
sw-brc-not-positive = Les deux débits doivent être supérieurs à 0 Bd.
sw-brc-too-far = Le Br requis doit être entre { $lo } et { $hi } Bd (au plus 30% de l'existant).
sw-brc-measured = Prérempli avec le Br mesuré par ADP
sw-brc-not-measured = Aucun Br mesuré : tapez-le ou lancez d'abord ADP
