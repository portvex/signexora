# Fenêtre du démodulateur FSK-2 / MFSK universel (SA_UI_SPEC.md section 8).

# Text panel.
demod-heading =
    SA - MFSK dem
    Levels: { $levels }
    Fd: { $fs }
    Br: { $br }
    Data as: { $form }
    Data:
demod-as-default = Table par défaut
demod-as-raw = Brut
demod-as-bits = Bits, naturel
demod-as-gray = Bits, Gray
demod-as-c4fm = Dibits APCO-25 C4FM
demod-polarity-inverted = [polarité : inversée, bits selon la table C4FM inversée]
demod-polarity-normal = [polarité : normale]
demod-polarity-unknown = [polarité : inconnue, bits selon la table C4FM normale]
demod-truncated = (20000 premiers caractères ; Copier et Enregistrer prennent tout)
demod-nothing-yet = Rien n'est encore démodulé : vérifiez Br, les niveaux et les marqueurs.

# Jobs.
demod-job-track = Démodulateur : piste de fréquence
demod-job = Démodulation FSK/MFSK

# Top bar.
demod-rrc = RRC
    .hover = Filtre en racine de cosinus surélevé avant les décisions ; le champ est sa coupure en Hz (Br par défaut)
demod-cutoff = Coupure
    .hover = Coupure du RRC en Hz : saisissez une valeur ou choisissez-la dans la liste
demod-alpha = Alpha
    .hover = Retombée du RRC, 0,05 à 1 (Entrée ou sortie du champ pour appliquer)
demod-br = Br
    .hover = Rapidité de modulation ; Entrée pour appliquer
demod-br-step = Pas
    .hover = Br plus ou moins { $step }
demod-auto = A
    .hover = Synchronisation automatique de phase et de bits
demod-auto-button = A
    .hover = Trouver la phase automatiquement maintenant et la garder (désactive le mode automatique)
demod-exactly = <= Exactly
    .hover = Phase d'échantillonnage fine (+-{ $span } % d'un symbole)
demod-roughly = <= Roughly
    .hover = Phase d'échantillonnage grossière sur un symbole
demod-levels = Levels
    .hover = Nombre de niveaux, 2 à 256
demod-raw = Raw
    .hover = Écrire les niveaux en octets hexadécimaux séparés par x

# Bottom bar.
demod-back-fast = Arrière 10
    .hover = Reculer de 10 pages
demod-back = Arrière
    .hover = Page précédente
demod-forward = Avant
    .hover = Page suivante
demod-forward-fast = Avant 10
    .hover = Avancer de 10 pages
demod-page = Page
    .hover = Page affichée (sur { $count }) ; Entrée pour y aller
demod-scale-up = Agrandir
    .hover = Agrandir l'échelle verticale
demod-scale-down = Réduire
    .hover = Réduire l'échelle verticale
demod-progress = Progression
    .hover = Progression du calcul (clic droit pour arrêter)
demod-stop = Arrêter
demod-filter = Filtre
    .hover = Appliquer le filtre RRC
demod-revert = Retour
    .hover = Revenir au signal non filtré
demod-d = D
    .hover = Écrire le train de bits dans le panneau de texte

# Plot.
demod-plot-name = Niveaux du démodulateur
demod-info = Br = { $baud } | horloge { $clock } % | qualité { $quality } | oeil { $eye } | symboles : { $count }
demod-menu-tones = Niveaux sur les tons trouvés ({ $count })
    .hover = Utiliser comme niveaux les tons trouvés avec Sum dans WF au lieu des zones régulières
    .disabled = Trouvez d'abord les tons : WF, Sum, Trouver les tons
demod-menu-average = Filtre moyenneur
    .hover = Intégrer sur un demi-symbole quand RRC est désactivé (FSK rectangulaire)
demod-menu-adaptive = Niveaux adaptatifs
    .hover = Laisser les centres des niveaux suivre une dérive lente des données
demod-menu-info = Info
demod-menu-reset-view = Réinitialiser l'échelle verticale
demod-menu-cancel = Annuler

# Text panel popup.
demod-menu-copy = Copier
demod-menu-save = Enregistrer sous...
demod-menu-data-as = Données en
demod-save-title = Enregistrer les données démodulées
demod-save-filter = Texte
demod-save-failed = Impossible d'écrire { $path } : { $error }
demod-cutoff-invalid = La coupure RRC "{ $text }" n'est pas un nombre positif ; tapez-la en Hz, par exemple le Br.
demod-waiting-cutoff = En attente d'une coupure RRC valide : le résultat affiché vient des réglages précédents.
demod-br-invalid = Le Br "{ $text }" n'est pas un nombre positif ; le Br utilisé est conservé.
