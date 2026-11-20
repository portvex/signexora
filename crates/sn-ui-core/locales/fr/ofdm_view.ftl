# OFDM module window, SA-OFDM (SA_UI_SPEC.md section 10).

# Top bar (SA's control names).
ofdm-min-br = Min Br-LS
    .hover = Rapidité minimale cherchée, Bd (entier) ; le champ suivant est le LS le plus long
ofdm-max-br = Max Br-LS
    .hover = Rapidité maximale cherchée, Bd (entier) ; le champ suivant est le LS le plus court
ofdm-total-ls = Total LS
    .hover = Symboles de la plus grande longueur (Min Br-LS) dans la sélection
ofdm-current-ls = Current LS
    .hover = Symbole affiché (0 pendant Find CT)
ofdm-survey = Survey
    .hover = Aperçu : une fenêtre spectrale de 1024 points déplacée avec la barre du dessous fixe le départ de Find CT
ofdm-list-k = List of K
    .hover = Calculateur Magic K : les valeurs de K et les fréquences natives les plus probables
    .disabled = Lancez d'abord Find CT et ADP

# Controls (SA's control names).
ofdm-symbols = Symbols
    .hover = Symboles repliés dans le triangle de corrélation (et bloc d'ADP) ; 20 à 40 est bien, jamais moins de 6
ofdm-scale-fft = Scale FFT
    .hover = Largeur des barres de canaux du spectre LU
ofdm-shift = Shift Frequency
    .hover = Fréquence retirée avant la FFT, Hz ; Entrée pour appliquer
ofdm-vshift = VShift
    .hover = Décalage virtuel d'un canal (pas exacts de la grille, sans limite)
ofdm-shift-step = Pas
    .hover = Shift Frequency plus ou moins { $step } Hz
ofdm-br = Br
    .hover = Rapidité des symboles : valeur d'ADP, ou saisissez-en une et appuyez sur Entrée
ofdm-adp = ADP
    .hover = Rapidité précise et position en fréquence (Get Br de SA)
    .disabled = Lancez d'abord Find CT
ofdm-show = Show
    .hover = Afficher les canaux marqués symbole par symbole avec synchronisation (Break arrête)
ofdm-break = Break
ofdm-find-ct = Find CT
    .hover = Trouver le triangle de corrélation : LS, LG et le début des symboles
ofdm-mode = Mode
    .hover = Modèle de signal des constellations
ofdm-model-a = A
    .hover = Modèle A : chaque canal tel que généré (valeurs absolues)
ofdm-model-b = B
    .hover = Modèle B : chaque canal est son propre pilote (valeurs relatives)
ofdm-model-c = C
    .hover = Modèle C : données rapportées au pilote du marqueur rouge
ofdm-each = Each
    .hover = Afficher un symbole sur N
ofdm-start-plus = Start+
    .hover = Premier symbole affiché par Show

# Plot texts (SA's formats).
ofdm-info-br = Br = { $br }
ofdm-info-sh = Sh = { $sh }
ofdm-info-magic = Magic K = { $num }/{ $den }
ofdm-info-k = * { $k } *
ofdm-info-lg = LG(CP) = { $lg }
ofdm-info-lu = LU = { $lu }
ofdm-info-up = CH_Up = { $ch }, Freq = { $freq }
ofdm-info-down = CH_Down = { $ch }, Freq = { $freq }
ofdm-info-sel = CH_Sel = { $count }, dFreqs = { $df }
ofdm-info-vshift = VShift = { $bins }
ofdm-line = LS={ $ls }; LG={ $lg }; Br={ $br }; Sh={ $sh }
ofdm-k-line = SR={ $sr }; LS={ $ls }; LU={ $lu }; LG={ $lg }; K={ $num }/{ $den }
ofdm-finish = Fin normale, durée { $secs } s.
ofdm-stopped = Arrêté par l'utilisateur, durée { $secs } s.
ofdm-wait = Pas de panique ! Veuillez patienter...
ofdm-wait-ct = Recherche du triangle de corrélation
ofdm-wait-adp = Calcul du Br précis
ofdm-wait-k = Calcul de "Magic K"
ofdm-wait-left = Temps restant : { $secs } s.

# Plots.
ofdm-survey-plot = Spectre d'aperçu
ofdm-triangle-plot = Triangle de corrélation
ofdm-triangle = Triangle
    .hover = Cliquez sur la bonne pente (gauche) pour placer la fenêtre LU ; au sommet c'est le LU parent
ofdm-spectrum-plot = Spectre LU
ofdm-spectrum = Spectre
    .hover = Cliquez sur un canal pour déplacer le marqueur le plus proche, double-cliquez pour placer les deux
ofdm-pane = Canal
    .hover = Canal { $ch }, symboles : { $count }
ofdm-k = K
    .hover = K = { $k }, erreur LU { $err }, ICI { $ici } dB, { $lu }
ofdm-k-lu-even = LU pair
ofdm-k-lu-odd = LU impair

# Popup menus.
ofdm-sort-sr = Trier par SR
ofdm-sort-k = Trier par K
ofdm-sort-am = Trier par AM
ofdm-k-close = Fermer
ofdm-menu-cancel = Annuler
ofdm-menu-reset = Effacer les résultats
ofdm-menu-around = Bornes autour de Br = { $rate }
    .hover = Mettre Min Br-LS et Max Br-LS à 0,8 et 1,25 fois le Br mesuré
ofdm-resample = Rééchantillonner à { $rate } Hz
    .hover = Rééchantillonner le signal à cette fréquence native (nouvelle fenêtre)

# Jobs and messages.
ofdm-job-ct = OFDM : Find CT
ofdm-job-br = OFDM : ADP
ofdm-job-k = OFDM : List of K
ofdm-job-show = OFDM : Show
ofdm-needs-ct = Lancez d'abord Find CT.
ofdm-needs-br = Lancez d'abord ADP.
ofdm-baseband-freed = La fenêtre a été fermée entre-temps : relancez Find CT.
ofdm-check-settings = Vérifiez les réglages ! Show demande Symbols d'au moins 6 et 10 symboles après Start+.
ofdm-failed = Recherche échouée, durée { $secs } s.
ofdm-pick-busy = Attendez la fin du travail en cours ou appuyez sur Break, puis choisissez une ligne.
ofdm-br-invalid = Br "{ $text }" est inutilisable : entrez un nombre supérieur à 0 et jusqu'à { $max } Bd (fs/LU).
ofdm-br-invalid-nogrid = Br "{ $text }" est inutilisable : entrez un nombre supérieur à 0.
ofdm-plot-empty = Pas de données : lancez Find CT.
ofdm-shift-invalid = Shift Frequency "{ $text }" n'est pas un nombre ; tapez-la en Hz, par exemple 12.5.
