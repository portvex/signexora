# Main window (MDI frame): messages, File menu extras, the WF window's toolbar
# and popup menu, and the confirmation dialogs.

## View kinds (child windows)

app-tab-sonogram = Sonagramme
app-tab-waveform = Forme d'onde
app-tab-raster = Raster (VMW)
app-tab-phase = PhasePlane
app-tab-demod = Démodulateur FSK/MFSK
app-tab-ofdm = OFDM

## Lists of names: "A", "A and B", "A, B and C"

app-list-separator = { ", " }
app-list-and = { $rest } et { $last }

## Background jobs (shown in the status bar while they run)

app-job-loading = Chargement de { $name }
app-job-overview = Calcul de l'aperçu de la forme d'onde
app-job-saving = Enregistrement de { $path }
app-job-measure-line = Mesure de la fréquence de la raie
app-job-adp = Estimation de la rapidité (ADP)
app-job-writing = Écriture de { $path }
app-job-replay = Relecture de la session
app-job-script = Exécution du script
app-job-find-tones = Recherche des tonalités
app-job-playback = Préparation de la lecture
app-job-demodulator = Démodulateur

## Status messages

app-ready = Prêt
app-signal-gone = Ce signal n'existe plus.
app-set-h-markers-first = Placez d'abord les marqueurs H autour de la raie.
app-sc-unmappable = Sc ne peut pas être reporté à travers { $edit } : sa sortie n'a pas de relation de fréquence avec son entrée. Mesurez la porteuse avant l'étape { $edit }.
app-recorded = { $kind } = { $value } sur { $node } ({ $tool })
app-recorded-uncertain = { $kind } = { $value } +/- { $uncertainty } sur { $node } ({ $tool })
app-not-recorded = { $kind } non enregistré : { $error }
app-mapped-onto = { $from }, reporté sur { $node }
app-session-save-running = Un enregistrement de session est déjà en cours
app-export-report-failed = Export du rapport : { $error }
app-save-session-failed = Enregistrement de la session : { $error }
app-open-session-failed = Ouverture de la session : { $error }
app-script-error = Erreur : { $error }
app-replay-skipped = { $what ->
    [markers] { $node } ({ $label }) : marqueurs enregistrés illisibles, ignorés : { $error }
    [tones] { $node } ({ $label }) : tonalités enregistrées illisibles, ignorées : { $error }
    [grid] { $node } ({ $label }) : grille de tonalités enregistrée illisible, ignorée : { $error }
   *[adp] { $node } ({ $label }) : candidats ADP enregistrés illisibles, ignorés : { $error }
}
app-session-restored = Session restaurée
app-session-restored-problems = { $count ->
    [one] Session restaurée, { $count } problème (voir la fenêtre)
   *[other] Session restaurée, { $count } problèmes (voir la fenêtre)
}
app-wrote-report = Rapport écrit : { $path }
app-saved = Enregistré : { $path }
app-saved-session = Session enregistrée : { $path }
app-saved-session-replaced = Session enregistrée : { $path } (la session qu'il contenait a été remplacée)
app-cancel-too-late = { $name } s'est terminé avant que l'annulation prenne effet ; le résultat est conservé.
app-job-cancelled = { $name } : annulé
app-job-failed = { $name } : { $error }
app-playback-cancelled = Lecture annulée
app-derived-opened = { $label } : nouveau signal { $node } ouvert dans { $view }
app-derived-opened-mapped = { $label } : nouveau signal { $node } ouvert dans { $view } ; marqueurs reportés sur ses axes
app-derived-opened-cleared = { $label } : nouveau signal { $node } ouvert dans { $view } ; certains marqueurs ont été effacés : le nouveau signal a un autre axe de temps ou de fréquence
app-derive-failed = { $label } : { $error }
app-measured-line = { $kind } = { $value } +/- { $uncertainty } sur { $node } (mesure de raie ; { $level } dBFS, { $snr } dB au-dessus de la bande)
app-measured-line-mapped = { $kind } = { $value } +/- { $uncertainty } sur { $node } (mesure de raie ; { $level } dBFS, { $snr } dB au-dessus de la bande), raie à { $line } Hz reportée via la puissance { $power }
app-loaded = Chargé { $label } : { $samples } échantillons à { $rate } Hz
app-loaded-notes = Chargé { $label } : { $samples } échantillons à { $rate } Hz. { $notes }
app-script-no-signals = Le script { $path } n'a produit aucun signal
app-script-finished = Script { $path } terminé
app-adp-none = ADP n'a trouvé aucune rapidité dans la sélection
app-tones-found = { $count ->
    [one] { $count } tonalité trouvée sur { $node }
   *[other] { $count } tonalités trouvées sur { $node }
}
app-markers-restored = Marqueurs restaurés sur { $node }
app-closed-tab-stopped = { $count ->
    [one] { $name } fermée ; sa tâche { $jobs } en cours a été arrêtée. Rouvrez la fenêtre et relancez-la.
   *[other] { $name } fermée ; ses tâches { $jobs } en cours ont été arrêtées. Rouvrez la fenêtre et relancez-les.
}
app-nothing-to-play = Rien à lire : ouvrez le sonagramme ou placez les marqueurs V.
app-find-tones-failed = Trouver les tonalités : { $error }

## Confirmation before work is lost

app-confirm-open-title = Ouvrir une session
app-confirm-open-question = Remplacer la session actuelle ?
app-confirm-replace = Remplacer
app-confirm-script-title = Exécuter un script
app-confirm-script-question = Les signaux du script remplacent la session actuelle.
app-confirm-quit-title = Quitter
app-confirm-quit-unsaved = Enregistrer la session avant de quitter ?
app-confirm-quit-without-saving = Quitter sans enregistrer
app-confirm-quit-writing = Un fichier est encore en cours d'écriture.
app-confirm-quit-anyway = Quitter quand même
app-confirm-lost = { $signals ->
    [one] { $signals } signal
   *[other] { $signals } signaux
} et { $measurements ->
    [one] { $measurements } mesure
   *[other] { $measurements } mesures
} non enregistrés seront fermés. Cette action est irréversible.
app-confirm-write-running = { $name } est encore en cours ; quitter maintenant l'abandonne.
app-confirm-save-first = Enregistrer d'abord la session
app-confirm-save-first-ask = Enregistrer d'abord la session...
app-confirm-wait = Attendre
app-confirm-keep-open = Garder l'application ouverte
app-cancel = Annuler

## File dialogs

app-dialog-save-as = Enregistrer en { $format }
app-dialog-save-session-as = Enregistrer la session sous
app-filter-session = Session SigNexora
app-dialog-export-report = Exporter le rapport d'analyse
app-filter-report = Rapport
app-dialog-open-session = Ouvrir une session
app-dialog-run-script = Exécuter un script
app-filter-rhai = Script Rhai

## File menu

app-menu-file = Fichier
app-menu-open-session = Ouvrir une session...
    .hover = Ses signaux remplacent la session actuelle
app-nothing-to-save = Rien à enregistrer
app-menu-save-session = Enregistrer la session
app-menu-save-session-ask = Enregistrer la session...
app-menu-save-session-choose = Choisir un fichier pour la session
app-menu-save-session-to = Enregistrer dans { $path }
app-menu-save-session-as = Enregistrer la session sous...
app-menu-export-report = Exporter le rapport d'analyse
app-menu-report-html = HTML...
app-menu-report-markdown = Markdown...

## File menu extras and the WF popup menu

app-menu-run-script = Exécuter un script (Rhai)...
    .hover = Exécute un script ; ses signaux remplacent la session actuelle
app-resynth = Resynthétiser FSK/MFSK
    .hover = Régénérer un signal propre sur les tonalités de Trouver les tonalités, à Br
    .disabled = Trouvez d'abord les tonalités (Sum de la forme d'onde) et mesurez Br.
app-menu-measure = Mesure
app-measure-hint = Placez les marqueurs H autour de la raie (et les marqueurs V pour l'intervalle).
app-measure-br = Mesurer la raie comme rapidité (Br)
app-measure-sc = Mesurer la raie comme porteuse (Sc)
    .hover = Après la puissance N, la raie est ramenée à la porteuse (divisée par N, décalages annulés)
app-measure-adp = Estimer la rapidité automatiquement (ADP)
    .hover = Utilise l'intervalle des marqueurs V, ou tout le signal
app-measure-dt-disabled = Placez d'abord les marqueurs V.

## Hints

app-no-signals-hint = Ouvrez un enregistrement (Fichier > Ouvrir un fichier... ({ $shortcut }), ou déposez un fichier sur la fenêtre).
app-open-in = Ouvrir...
    .hover = Ouvrir ce signal dans une autre vue ou un autre outil

## Sonogram toolbar

app-sono-shear = Shear
    .hover = Pas entre trames FFT, en fraction de la taille de la FFT. Un pas plus petit fait davantage chevaucher les trames : détail temporel plus fin, calcul plus lent
app-sono-image = Image
    .hover = Dynamique affichée sous la composante la plus forte
app-sono-hres-hover = Affine les raies spectrales par réallocation en fréquence (aide à trouver les raies d'horloge faibles). Les tonalités gardent leur niveau ; le bruit apparaît environ 2 à 6 dB plus bas que sans HRes (selon la fenêtre), car il est affiché par largeur de bin.
app-sono-alc-hover = Normaliser chaque colonne sur sa composante la plus forte (affichage seulement)
app-sono-combine = Vue dézoomée
    .hover = Comment les trames FFT qui partagent une colonne de l'image sont combinées en vue dézoomée
app-sono-peak = Crête
    .hover = Vue dézoomée : garder visibles les salves courtes
app-sono-mean = Moyenne
    .hover = Vue dézoomée : moyenne, bruit plus lisse

## Waveform toolbar

app-wave-show-q = Afficher Q
app-find-tones = Trouver les tonalités
    .hover = Pics de l'histogramme : fréquences des tonalités sur la sortie du détecteur de phase

## Zoom and playback

app-play = Lire
    .hover = Lire l'intervalle des marqueurs V, ou l'intervalle visible ({ $shortcut })
app-play-stop = Arrêter
    .hover = Arrêter la lecture ({ $shortcut })

## Child windows

app-tab-signal-gone = Ce signal n'existe plus.

## Windows

app-replay-problems-title = Session restaurée avec des problèmes
app-script-output = Sortie du script : { $path }
app-script-no-output = (aucune sortie)
# $lines: the lines kept; $dropped: the lines left out.
app-script-output-truncated = Sortie tronquée après { $lines } lignes ; { $dropped } lignes de plus n'ont pas été conservées.

## Closing a window, the sonogram toolbar

app-confirm-close-tab-title = Fermer la fenêtre ?
# $name: the window title; $jobs: the job names, joined.
app-confirm-close-tab-question = { $count ->
    [one] Fermer { $name } arrête son traitement { $jobs } en cours. Fermer quand même ?
   *[other] Fermer { $name } arrête ses traitements { $jobs } en cours. Fermer quand même ?
}
app-confirm-close-and-stop = Fermer et arrêter
app-confirm-keep-tab = Garder ouvert
# $prev, $next: the keys that show the previous and next signal.
app-select-signal-first = Mettez d'abord une fenêtre au premier plan ({ $prev } ou { $next } affiche les signaux tour à tour).
app-sono-fft = SizeFFT
    .hover = Taille de la FFT : plus grande, elle affine la résolution en fréquence et grossit la résolution temporelle
app-sono-window = WinF
    .hover = Fonction de fenêtre de chaque FFT (WinF dans SA)
app-sono-colormap = Couleurs
    .hover = Palette de couleurs de l'image du sonagramme
app-confirm-close-all-title = Fermer toutes les fenêtres
app-confirm-close-all-question = Cela retire tous les signaux et mesures de la session.
app-confirm-close-all = Tout fermer
app-confirm-close-all-stops = { $name } est encore en cours ; fermer toutes les fenêtres l'arrête.
app-replay-session-skipped = { $what ->
    [windows] La disposition des fenêtres enregistrée est illisible, une fenêtre s'ouvre par résultat : { $error }
   *[counters] Les compteurs de noms enregistrés sont illisibles, les nouveaux noms peuvent répéter des numéros : { $error }
}
app-replay-windows-missing = { $count ->
    [one] 1 fenêtre enregistrée n'a pas été ouverte, son signal n'a pas été rejoué
   *[other] { $count } fenêtres enregistrées n'ont pas été ouvertes, leurs signaux n'ont pas été rejoués
}
app-drop-loader-open = La boîte de fichier est ouverte : terminez-la ou annulez-la, puis déposez de nouveau le fichier
app-drop-first-only = { $count } fichiers déposés ; seul le premier, { $name }, est ouvert
app-sono-target = { $active ->
    [yes] Modifie la fenêtre de sonogramme active.
   *[no] Modifie la valeur par défaut des nouveaux sonogrammes.
}
app-edit-unrecordable = impossible de l'enregistrer dans la session pour la rejouer ({ $error })
app-empty-open = Ouvrir un enregistrement... ({ $key })
app-empty-drop = ou déposez un fichier ici (File > Open file... en ouvre aussi un).
app-empty-guide = Le guide ({ $key }) montre l'analyse pas à pas.
# $action: what the chooser was for, e.g. "Open session"; $error: why it failed.
app-chooser-failed = { $action } : le sélecteur de fichiers n'a pas pu s'afficher ({ $error }). Réessayez, ou vérifiez que le service de dialogue de fichiers du système (portail de bureau) est lancé.

## The tools that record a measurement, by the id stored in the session.

measure-tool-adp = Définir les paramètres (ADP)
measure-tool-adp-coarse = Définir les paramètres, grossier (à une demi-case FFT près)
measure-tool-measure-line = Mesure de fréquence (ligne de mesure)
measure-tool-v-markers = Marqueurs V (dT)
measure-tool-ofdm-adp = Outil OFDM, Get Br (ADP)
measure-tool-script-record = Script, record()
