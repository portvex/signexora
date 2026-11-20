# The VMW mode of the WF window. SA's control names (Y scale, As time, Bright,
# WaveForm, TS, Info) and the units S, ms, us, ns are names: keep them.

vmw-strobe = Strobe
    .hover = Strobe : une période sur N (1 à 1000) ; amplifie les petites erreurs de période en pentes
vmw-y-scale = Y scale
    .hover = Hauteur d'une ligne du raster en pixels
vmw-as-time = As time
    .hover = La période de ligne suit l'intervalle des marqueurs V de WF
    .disabled = Placez d'abord les marqueurs V dans WF
vmw-waveform = WaveForm
    .hover = Retour à WF
vmw-info = Info
    .hover = Afficher dT, S et la cadence de ligne TSs
vmw-ts = TS
    .hover = Période de ligne S:ms.us.ns. Clic gauche sur un chiffre : +1, clic droit : -1
vmw-period-slider = TS
    .hover = Période de ligne (grossière, logarithmique)
vmw-status = dT
    .hover = Intervalle des marqueurs V et sa cadence

## Popup menu (SigNexora tools).

vmw-popup-start-v1 = Commencer le raster au premier marqueur V de WF
vmw-popup-from-dt = Période de ligne = dernier dT mesuré
vmw-popup-colours = Palette
vmw-popup-sa-colours = Signals Analyzer
