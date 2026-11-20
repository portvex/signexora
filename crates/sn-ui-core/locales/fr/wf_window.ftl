# The WF (waveform) window: SA's top toolbar, bottom toolbar hints, popup menu and
# result box. SA's control names (LPF, RRC, Sum, Info, VMW, Br, Z+, Z-) and the
# result box text are names: keep them.

## Top toolbar, left to right.

wf-lpf = LPF
    .hover = Coupure du passe-bas en Hz, ou débit du filtre RRC (saisir ou choisir une valeur)
wf-copy-br = <==
    .hover = Copier le Br mesuré dans le champ LPF
wf-rrc = RRC
    .hover = Activer ou désactiver le filtre RRC (le bouton LPF l'exécute alors)
wf-rrc-alpha = RRC
    .hover = Retombée du RRC
wf-freq = 0-0
    .hover = Clic : fréquence du court fragment entre les marqueurs V, et ses périodes entières
wf-vmw = VMW
    .hover = Passer en VMW (le signal replié sur une période de ligne)
wf-sum = Sum
    .hover = Afficher l'histogramme des valeurs (puis Compter les fréquences dans le menu)
wf-info = Info
    .hover = Afficher les résultats (t1, t2, dT, S, Br)

## Bottom toolbar hints, left to right.

wf-hint-zoom-in = Zoom avant (temps)
wf-hint-zoom-out = Zoom arrière (temps)
wf-hint-scale-up = Échelle verticale plus grande
wf-hint-scale-down = Échelle verticale plus petite
wf-hint-zoom-plus = Z+: Zoom entre les marqueurs V
wf-hint-zoom-minus = Z-: Revenir d'un zoom
wf-hint-lpf = Appliquer le filtre LPF (le filtre RRC si RRC est actif)
wf-hint-revert = Revenir à l'original
wf-hint-synth = Synthèse FSK/MFSK inverse
wf-hint-save = Enregistrer en WAV 16 bits (clic droit : WAV flottant, DataFile ou SigMF)
wf-hint-copy = Copier le fragment entre les marqueurs V dans une nouvelle fenêtre
wf-hint-demod = Envoyer le fragment au démodulateur FSK/MFSK
wf-hint-fit = Ajuster l'échelle verticale au signal
wf-hint-acf = Autocorrélation du fragment (nouvelle fenêtre)
wf-hint-ccf = Corrélation croisée du fragment avec tout le signal (tracée ici)

## Why a button is disabled.

wf-need-revert = Rien à annuler : aucun filtre ni CCF.
wf-need-derived = Cette fenêtre montre le fichier ouvert, déjà enregistré. Enregistrer sert aux résultats faits ici.

## Popup menu.

wf-popup-count-freqs = Compter les fréquences
wf-popup-more = Outils SigNexora
    .hover = Outils de SigNexora absents de la fenêtre WF de SA
wf-popup-operations = Opérations

## The Count freqs result box (SA's text, not translated). $size: a whole number;
## $value: a number with two decimals.

wf-label-grid = Freq grid: { $size }
wf-label-delta = Delta: { $value } Hz
wf-count-needs-h = Placez d'abord les marqueurs H autour des tons.
wf-count-tones-busy = Find tones est encore en cours ; relancez Count freqs quand il aura fini.
wf-count-freqs-failed = Aucune grille de fréquences trouvée ({ $error }). Élargissez les marqueurs H autour des tons ou dézoomez pour voir plus de tons.
wf-copy-br-value = Copier Br = { $br } Bd dans le champ LPF
wf-copy-br-missing = Mesurez d'abord Br (menu du sonogramme : Auto define params)
