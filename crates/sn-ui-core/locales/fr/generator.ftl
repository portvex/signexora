# Outils > Générateur de signaux (OCG) : la fenêtre du générateur, son aperçu
# et les calculateurs OFDM. Ne pas traduire les unités (Hz, Bd, s, ms, dB, ppm),
# les noms de modulation (FSK, MSK, GMSK, PSK, QAM, C4FM, OFDM, BPSK, ...),
# BT, Es/N0, SNR, RC, RRC, LU, LG, LS, K, Br, Fd ni les modèles A, B, C.

gen-menu = Générateur de signaux (OCG)...
    .hover = Créer un signal de test aux paramètres exactement connus : tonalités, bruit, FSK, PSK/QAM, C4FM ou OFDM, avec dégradations du canal
gen-keys-open = Afficher ou masquer le générateur de signaux
gen-title = Générateur de signaux (OCG)
gen-job = Génération de { $name }

gen-kind = Type
gen-kind-tones = Tonalités
gen-kind-noise = Bruit
gen-kind-fsk = FSK/MFSK
gen-kind-linear = PSK/QAM
gen-kind-c4fm = C4FM
gen-kind-ofdm = OFDM
gen-preset = Préréglage
    .hover = Charger un signal tout prêt
gen-preset-defaults = Par défaut
    .hover = Remettre le type actuel à ses valeurs par défaut

gen-section-signal = Signal
gen-section-common = Commun
gen-section-channel = Dégradations du canal
gen-section-derived = Valeurs dérivées
gen-section-preview = Aperçu
gen-section-calc = Calculateurs

gen-complex = Sortie complexe (I/Q)
    .hover = Produire des échantillons I/Q au lieu d'un signal réel ; les fréquences sont alors des décalages par rapport à 0 Hz
gen-amplitude-rms = Amplitude RMS
gen-amplitude-peak = Amplitude crête
gen-phase-rad = Phase (rad)
gen-remove = Supprimer
gen-baud = Rapidité
gen-carrier = Porteuse
gen-center = Centre
gen-alpha = Retombée

gen-tones-frequency = Fréquence
gen-tones-add = Ajouter une tonalité
gen-tones-last = Il faut au moins une tonalité

gen-fsk-order = Tonalités
gen-fsk-spacing = Espacement
gen-fsk-shaping = Mise en forme
gen-fsk-shaping-none = Aucune
gen-fsk-shaping-gaussian = Gaussienne
gen-fsk-bt = BT
gen-fsk-msk = MSK
    .hover = Deux tonalités, espacement égal à la moitié de la rapidité, sans mise en forme
gen-fsk-gmsk = GMSK
    .hover = MSK avec un filtre gaussien de BT 0,3, comme en GSM

gen-lin-constellation = Constellation
gen-lin-shaping = Forme d'impulsion
gen-lin-rect = Rectangulaire
gen-lin-rc = Cosinus surélevé (RC)
gen-lin-rrc = Racine de cosinus surélevé (RRC)
gen-lin-differential = Différentiel
    .hover = Envoyer chaque symbole comme un saut de phase par rapport au précédent
gen-lin-differential-qam = La QAM porte l'amplitude en plus de la phase, elle ne peut donc pas être différentielle
gen-lin-differential-pi4 = La pi/4-DQPSK est toujours différentielle

gen-c4fm-pulse = Impulsion
gen-c4fm-shaped = Mise en forme
    .hover = Désactivé : la fréquence saute entre les quatre niveaux
gen-c4fm-apco = APCO-25
    .hover = L'impulsion TIA-102, prévue pour un récepteur intégrateur-échantillonneur
gen-c4fm-note = Les déviations sont fixées à +-600 et +-1800 Hz ; la sortie C4FM est toujours réelle.

gen-ofdm-lu = LU
    .hover = Longueur utile (taille de FFT) en échantillons à la fréquence d'échantillonnage
gen-ofdm-lg = LG
    .hover = Longueur de garde (préfixe cyclique) en échantillons
gen-ofdm-channels = Canaux
gen-ofdm-first-bin = Premier bin
    .hover = Bin du premier canal ; le bin 0 est le centre
gen-ofdm-modulation = Modulation
gen-ofdm-unused = Positions inutilisées
    .hover = Positions de canal qui ne portent rien, comptées à partir de 0, par exemple 3, 7, 10-12
gen-ofdm-overrides = Exceptions
    .hover = Modulation de positions isolées, par exemple 0: pilot, 5-6: QAM-16
gen-ofdm-model = Modèle
gen-ofdm-model-a = A : chaque canal envoie des valeurs absolues
gen-ofdm-model-b = B : chaque canal est différentiel d'un symbole à l'autre
gen-ofdm-model-c = C : les pilotes portent la référence, les données sont absolues par rapport à eux
gen-ofdm-pilots = Pilotes
gen-ofdm-pilots-none = Aucun
gen-ofdm-pilots-list = Positions
    .hover = Positions des pilotes, par exemple 0, 8, 16
gen-ofdm-pilots-every = Tous les N
gen-ofdm-step = Pas
gen-ofdm-offset = Premier
gen-ofdm-reference = Référence pilote
    .hover = Ce que portent les pilotes dans le modèle C
    .disabled = Seul le modèle C utilise une référence pilote
gen-ofdm-reference-constant = Constante
gen-ofdm-reference-differential = Phase commune aléatoire
gen-ofdm-center = Centre
    .hover = Position du bin 0 ; en sortie complexe, le décalage par rapport à 0 Hz

gen-label = Libellé
    .hover = Nom du nouveau signal ; vide, la description est utilisée
gen-sample-rate = Fréq. d'échantillonnage
gen-length = Durée
gen-length-seconds = Secondes
gen-length-symbols = Symboles
    .disabled = Les tonalités et le bruit n'ont pas de symboles
gen-seed = Graine
gen-new-seed = Nouvelle graine
    .hover = Choisir une graine aléatoire : nouvelles données et nouveau bruit
gen-center-frequency = Fréq. centrale
    .hover = Fréquence RF que représente 0 Hz de la sortie complexe (enregistrée avec le signal)
    .disabled = Seule la sortie complexe a une fréquence centrale

gen-ch-offset = Décalage de fréquence
gen-ch-drift = Dérive
gen-ch-clock = Décalage d'horloge
    .hover = Erreur d'horloge d'échantillonnage ; positive si l'horloge du récepteur avance
gen-ch-noise = Bruit
gen-ch-noise-none = Aucun
gen-ch-noise-full = SNR sur toute la bande
gen-ch-noise-inband = SNR dans la bande
gen-ch-noise-esn0 = Es/N0
gen-ch-bandwidth = Largeur de bande
    .hover = Désactivé : la largeur de bande occupée par le signal
gen-ch-clip = Écrêter à
    .hover = Limiter l'amplitude crête ; les échantillons complexes gardent leur phase
gen-ch-multipath = Trajets multiples
    .hover = Les trajets forment tout le canal : incluez le trajet direct (retard 0, gain 1)
gen-ch-delay = Retard
gen-ch-gain = Gain
gen-ch-add-tap = Ajouter un trajet

gen-d-none = Corrigez les paramètres pour voir les valeurs dérivées.
gen-d-symbol-rate = Rapidité de symbole
gen-d-sps = Échantillons par symbole
gen-d-symbols = Symboles
gen-d-bandwidth = Bande occupée
gen-d-samples = Échantillons
gen-d-duration = Durée
gen-d-index = Indice de modulation h
gen-d-ls = LS
gen-d-k = K = LG/LU
gen-d-spacing = Espacement
gen-d-br = Br

gen-preview = Aperçu du générateur
gen-preview-spectrum = Spectre
gen-preview-wave = Forme d'onde
gen-preview-name = Aperçu du signal généré
gen-preview-building = Création de l'aperçu...
gen-preview-invalid = Pas d'aperçu tant que les paramètres ne sont pas valides
gen-preview-failed = Échec de l'aperçu : { $error }
gen-preview-short = Trop court pour un spectre
gen-preview-axis = Spectre moyen, { $lo } à { $hi } Hz, dynamique 80 dB
gen-preview-wave-axis = Premières { $ms } ms, crête { $peak }

gen-generate = Générer
    .hover = Ajouter le signal à la session comme nouvel enregistrement
gen-generate-save = Générer et enregistrer sous
    .hover = Générer, puis enregistrer le nouveau signal au format choisi
gen-busy = Un signal est en cours de génération
gen-fix-first = Corrigez d'abord les paramètres
gen-invalid = Génération impossible : { $error }
gen-field-error = { $field } : { $error }
gen-ready = Prêt : { $samples } échantillons, { $duration } s
gen-list-bad = "{ $item }" n'est ni une position ni une plage comme 10-12
gen-list-too-long = Trop de positions ({ $max } au plus)
gen-overrides-bad = "{ $item }" n'est pas une position suivie d'une modulation comme 5: QAM-16

gen-calc-cp = OFDM avec CP
gen-calc-cp-lengths = À partir de Fd, LU et LG
gen-calc-spacing = Espacement
gen-calc-symbol = Symbole
gen-calc-useful = Partie utile
gen-calc-guard = Garde
gen-calc-use = Utiliser dans OFDM
    .hover = Copier ces longueurs (et Fd) dans l'éditeur OFDM
    .disabled = LU et LG doivent être des nombres entiers d'échantillons
# Why Use in OFDM is disabled. $max: the generator's limit in samples.
gen-calc-lu-not-whole = LU doit être un nombre entier d'échantillons, au moins 2.
gen-calc-lg-not-whole = LG doit être un nombre entier d'échantillons.
gen-calc-lu-too-large = LU doit valoir au plus { $max } échantillons.
gen-calc-lg-too-large = LG doit valoir au plus { $max } échantillons.
gen-calc-use-k = Utiliser dans OFDM
    .hover = Copier ce K dans l'éditeur OFDM : le plus petit LU qui lui correspond, à partir du LU actuel
gen-calc-inverse = À partir de Br et de l'espacement à Fd : LU et LG entiers les plus proches
gen-calc-exact = exact
gen-calc-error = Erreur
gen-calc-candidates = LU et LG entiers pour Br et K
gen-calc-lu-range = LU à partir de
gen-calc-tolerance = Tolérance
gen-calc-found = { $count ->
    [one] { $count } candidat
   *[other] { $count } candidats, les { $shown } premiers affichés
}
gen-calc-klist = Liste des K
gen-calc-k-from-rates = À partir de Br et de l'espacement
gen-calc-k-direct = À partir de K
gen-calc-uncertainty = Incertitude
    .hover = Écart possible de chaque débit mesuré, en pourcentage du débit. La liste montre les fractions les plus simples dans l'incertitude de K qui en résulte ; pour un K saisi directement, la tolérance est celle d'un K mesuré à partir de deux débits avec cet écart.
gen-calc-uncertainty-bad = L'incertitude doit être comprise entre 0 et { $max } %.
gen-calc-k-none = Aucune fraction ne se trouve dans l'incertitude. Augmentez l'incertitude ou le plus grand dénominateur.
gen-calc-max-den = Plus grand dénominateur
gen-calc-count = Nombre
gen-calc-value = Valeur
gen-calc-fd = Fd
    .hover = Fréquence d'échantillonnage en Hz qui rend ces longueurs exactes pour le Br : Br x (LU + LG)
gen-calc-k = K
    .hover = K = LG/LU, le rapport garde sur longueur utile qu'ont réellement ces longueurs entières
gen-calc-lg-error = Erreur (échantillons)
    .hover = LG moins K x LU : écart entre la longueur de garde entière et la longueur exacte, en échantillons
gen-calc-k-fraction = K
    .hover = K sous forme de fraction p/q irréductible
gen-calc-k-error = Erreur
    .hover = Valeur de la fraction moins le K mesuré (signée ; K n'a pas d'unité, cette valeur non plus)
gen-preview-symbol-too-long = Pas d'aperçu : un symbole fait { $samples } échantillons, plus qu'un aperçu. Augmentez le débit ou baissez la fréquence d'échantillonnage.
gen-calc-fraction-too-big = K = { $k } ne tient pas : LU doit valoir au plus { $lu } et LG au plus { $lg } échantillons. Choisissez un LU ou un K plus petit.
