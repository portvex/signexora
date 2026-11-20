# The workflow guide: SA's analysis method as a checklist with buttons.

guide-title = Guide de la méthode
guide-no-signal = Ouvrez d'abord un enregistrement (Fichier > Ouvrir un fichier...).
# $signal: the active window's signal, which the buttons act on.
guide-acts-on = Les boutons agissent sur : { $signal }
# $key: a key name. Shown after a menu path only when that key acts on the guide's signal.
guide-key = , touche { $key }

## Steps: the message is the step's title, .text its explanation.

guide-step-record = 1. Enregistrer correctement
    .text = WAV PCM 16 bits (ou I/Q), AGC du récepteur désactivé, sans écrêtage ni retouche. Les fichiers compressés ou 8 bits sont signalés à l'ouverture.
guide-step-quality = 2. Vérifier la qualité de l'enregistrement
    .text = Ouvrez la fenêtre WF (clic droit sur le sonagramme > { sw-popup-waveform }). Cherchez l'écrêtage, les marches 8 bits, la composante continue, le pompage de l'AGC et les coupures. Activez Somme pour voir la distribution des amplitudes.
guide-step-isolate = 3. Isoler le signal
    .text = Sur le sonagramme, placez les marqueurs H autour du signal. Utilisez ensuite { sw-hint-band-select } et { sw-hint-shift-base } de la barre d'outils, puis R pour rééchantillonner plus bas s'il est large.
guide-step-rate = 4. Trouver la rapidité de modulation
    .text = Lancez un détecteur, encadrez sa raie d'horloge avec les marqueurs et choisissez { sw-popup-measure } dans le menu contextuel{ $br }. { sw-hint-am } pour PSK/QAM/FSK, { sw-hint-modified-am } pour MFSK, ou essayez { sw-popup-adp }.
guide-step-family = 5. Étudier la famille de modulation
    .text = FSK/MFSK : { sw-hint-phase-detector }, puis dans la fenêtre WF { edits-median } sur 1/3 de symbole, Somme, Trouver les tons, Compter les fréquences. PSK/QAM : { sw-hint-power } pour l'ordre et la porteuse ({ sw-popup-measure } dans sa fenêtre{ $sc }), puis PhasePlane ; sur un signal déformé, utilisez d'abord le correcteur ISD. OFDM : Trouver CT, Obtenir Br, Canaux, puis la liste des K pour le débit natif.
guide-step-structure = 6. Trouver la structure
    .text = { sw-hint-acf } pour les périodes de trame, { sw-hint-ccf } pour retrouver un fragment partout, puis placez les marqueurs V sur une trame et choisissez { sw-popup-measure } > { raster-dt-menu-item } dans le menu contextuel pour relever dT (la période de trame), et ouvrez { guide-btn-raster } à cette période pour voir trames et mots de synchro.

## Step buttons.

guide-btn-waveform = Forme d'onde
guide-btn-sonogram = Sonagramme
guide-btn-am-detector = Détecteur AM
guide-btn-modified-am = AM modifié
guide-btn-phase-detector = Détecteur de phase
guide-btn-demod = Démodulateur FSK/MFSK
# $n: the power, a number.
guide-btn-power = Élever à la puissance { $n }
    .hover = Crée un nouveau signal élevé à la puissance { $n } ; un signal PSK-{ $n } montre alors une raie à { $n } fois la porteuse
guide-btn-phase-plane = PhasePlane
guide-btn-raster = Raster (VMW)
guide-btn-adp = ADP
    .hover = Estimer automatiquement la rapidité de modulation (ADP) sur l'intervalle des marqueurs V, ou sur tout le signal
guide-btn-ofdm = OFDM
    .hover = Ouvrir l'outil OFDM (Find CT, Get Br, Liste des K)
guide-open-view = Ouvrir cette vue du signal de la fenêtre active
guide-new-signal = Crée un nouveau signal à partir de celui de la fenêtre active (dans une nouvelle fenêtre)
# $kind: Br, Sc or dT (untranslated).
guide-not-measured = { $kind } : pas encore mesuré
