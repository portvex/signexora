# The PhasePlane form (SA_UI_SPEC.md section 6): spectrum mode toolbar, ISD
# corrector, phase viewer, status bar, demodulator output and the LoadFile dialog.
# The control captions are SA's own English names (SA showed them in English under
# every Windows language): keep them as they are; translate the hints and dialogs.

pp-fmin = Fmin
    .hover = Fréquence la plus basse affichée, Hz
pp-fmax = Fmax
    .hover = Fréquence la plus haute affichée, Hz
pp-scale = Scale
    .hover = Partie de la bande affichée ; la barre sous le spectre la déplace
pp-fft-size = FFTSize
    .hover = Taille FFT du spectre moyenné : plus grande, plus fine (jusqu'à 1 Hz environ)
pp-low-level = LowLevel
    .hover = Niveau le plus bas affiché
pp-stretch = Stretch
    .hover = Étirer le spectre sur toute la hauteur
pp-mode = Mode
    .hover = AM : correction sur l'enveloppe ; X^N : sur la puissance N du signal
pp-blocks = Blocks
    .hover = Nombre de blocs du signal moyennés par le correcteur
pp-corr-off = Corr-OFF
    .hover = Le correcteur ISD est inactif : appuyez sur Auto pour corriger le signal
pp-corr-on = Corr-ON
    .hover = Le correcteur ISD est actif (SaveAs enregistre le signal corrigé)
pp-save-as = SaveAs
    .hover = Enregistrer le signal corrigé dans un fichier
pp-auto = Auto
    .hover = Une étape du correcteur sur les blocs choisis
pp-break = Break
pp-reset = Reset
    .hover = Réinitialiser le correcteur (le couper)
pp-carrier = Carrier
    .hover = Fréquence porteuse, Hz (dernière mesure Sc)
pp-br = BR
    .hover = Rapidité de modulation, Bd (dernière mesure Br)
pp-nary = n-Ary
    .hover = Ordre de la modulation, 2-32 (1 : sans suivi de porteuse)
pp-synch-br = Synch BR
    .hover = Synchronisation d'horloge
pp-nonstop = NonStop
    .hover = Fonctionnement continu : reprendre à la fin du fichier
pp-rrc = RRC filter
    .hover = Filtre en racine de cosinus surélevé ; le curseur règle alpha
pp-pll = PLL
    .hover = Vitesse de la boucle de porteuse
pp-pll-slow = Slow
pp-pll-medium = Medium
pp-pll-quick = Quick
pp-pll-hard = Hard
pp-auto-reset = AutoReset
    .hover = Avec NonStop : réinitialiser générateurs et boucles PLL en fin de fichier (signaux qui dérivent)
pp-transitions = Transitions
    .hover = Tracer les transitions entre les points (la trajectoire)
pp-life = Life
    .hover = Affichage dynamique : seuls les derniers symboles (Points...) restent
pp-diff = Diff
    .hover = 0 : angles absolus ; 1-3 : différences de phase (modes relatifs comme pi/4 DQPSK)
pp-offset-detected = Offset mode detected
pp-angles = Angles
    .hover = Construire l'histogramme des angles dans le tracé du correcteur (Synch BR requis)
pp-vectors = Vectors of dem
    .hover = Afficher les positions du démodulateur ; le vecteur zéro est bleu clair
pp-output-dem = Output Dem as
    .hover = Écrire le flux démodulé en codes d'angles absolus dans le panneau de texte
pp-trimming = Trimming { $value }
    .hover = Réglage fin de la phase, degrés
pp-delay = Delay { $value }
    .hover = Délai réel par symbole, secondes (signaux très courts)
pp-points = Points... { $value }
    .hover = Points affichés à la fois avec Life
pp-contrast = Contrast { $value }
    .hover = Luminosité des points
pp-start = Start
    .hover = Démarrer ou arrêter la constellation ; après un arrêt elle repart du début du fichier
pp-clear = Clear
    .hover = Effacer l'affichage
pp-load-file = LoadFile
    .hover = Charger un des fichiers ouverts dans le plan de phase
pp-status-f = F = { $value }
pp-status-fl = Fl = { $value }
pp-status-fu = Fu = { $value }
pp-status-df = dF = { $value }
pp-status-angle-d = Angle_D = { $value }
pp-status-angle-i = Angle_I = { $value }
pp-dem-view-mode = View mode: diff { $diff }
pp-dem-mode = Dem mode: { $mode }
pp-dem-absolute = Data presented as absolute angles!
pp-dem-coding = The coding positions of angles:
pp-dem-data = Data:

pp-isd-n = Harmonique du signal utilisée par le correcteur
    .hover = Harmonique du signal utilisée par le correcteur
pp-isd-b = Activé : le meilleur algorithme ; désactivé : un peu moins bon, plus rapide (sans effet en mode AM)
    .hover = Activé : le meilleur algorithme ; désactivé : un peu moins bon, plus rapide (sans effet en mode AM)
pp-isd-break = Arrêter l'étape en cours
    .hover = Arrêter l'étape en cours
pp-alpha = Facteur alpha du filtre RRC
    .hover = Facteur alpha du filtre RRC
pp-quality = Rapport signal sur bruit conventionnel de la constellation, dB
    .hover = Rapport signal sur bruit conventionnel de la constellation, dB
pp-load-title = SA - choisir le fichier pour PhasePlane
pp-ok = Ok
pp-cancel = Annuler
pp-check-settings = Vérifiez les réglages !
pp-run-limited = Seuls les { $symbols } premiers symboles sont démodulés. Pour analyser une partie plus loin, placez les marqueurs V dessus dans le sonogramme, utilisez { sw-hint-copy } et ouvrez { sw-hint-phase-plane } sur la copie.
pp-copy-panel = Copier le texte
pp-copy-codes = Copier tous les codes depuis Start
pp-eye = Diagramme de l'œil
    .hover = Afficher le diagramme de l'œil (I, deux symboles) au lieu de la constellation
pp-job-corrector = Spectre du correcteur ISD
pp-job-isd = Correcteur ISD
pp-job-spectrum = Spectre PhasePlane
# $count symbols, $offset Hz, $rings radii, $iq_offset symbols.
pp-quality-details = { $count ->
        [one] { $count } symbole
       *[other] { $count } symboles
    }, écart de boucle { $offset } Hz, anneaux [{ $rings }], décalage I/Q { $iq_offset } symbole
pp-source-gone = Le fichier chargé n'est plus dans la session ; rechargez un fichier de travail avec LoadFile.
pp-load-gone = Le fichier choisi a été fermé ; choisissez un autre fichier de travail.
