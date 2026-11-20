# Audio playback of a time span: errors and status messages.

playback-rate-too-low = La fréquence d'échantillonnage est trop basse pour la lecture.
# $rate: the sample rate in Hz.
playback-rate-too-high = La lecture concerne les signaux à fréquence audio ; celui-ci est échantillonné à { $rate } Hz. Ramenez d'abord la bande utile vers le bas et rééchantillonnez-la.
playback-nothing = Rien à lire : sélectionnez un intervalle de temps avec les marqueurs V.
# $got and $total: seconds; $name: the signal, e.g. "n3".
playback-short-read = Seulement { $got } s lues sur les { $total } s de l'intervalle de { $name } ; le fichier a peut-être changé sur le disque. Rouvrez l'enregistrement.
# $error: the audio system's error text.
playback-no-output = Impossible d'ouvrir la sortie audio : { $error }
playback-output-unavailable = La sortie audio n'est pas disponible.
# $seconds: the length played.
playback-playing = Lecture de { $seconds } s
# $seconds: the length played; $limit: the playback limit at this rate, in seconds.
playback-playing-first-markers = Lecture des { $seconds } premières s seulement (la lecture est limitée à { $limit } s à cette fréquence) ; placez V1 à la fin de cette partie pour entendre la suite.
playback-playing-first-view = Lecture des { $seconds } premières s seulement (la lecture est limitée à { $limit } s à cette fréquence) ; placez les marqueurs V sur la suite pour l'entendre.
playback-bad-samples = { $count } échantillons qui n'étaient pas des nombres sont joués comme du silence.
