# The sonogram work window: SA's top toolbar captions, the bottom toolbar hints,
# the right-click popup menu and the Correction of BR panel. SA's control names
# (Fl, Fu, ALC, HRes, Info, WaveFile, DataFile) are names: keep them.

## Top toolbar (SizeFFT, Shear, Image and WinF are in app.ftl).

sw-fl = Fl
    .hover = Lowest frequency shown, Hz (blue while the band is zoomed)
sw-fu = Fu
    .hover = Highest frequency shown, Hz (blue while the band is zoomed)
sw-alc = ALC
sw-hres = HRes
sw-info = Info
    .hover = Show the results overlay (Fu, Fc, Fl, dF, t1, t2, dT, S, Br, Sc)

## Bottom toolbar hints, left to right.

sw-hint-zoom-in = Zoom band between H markers to window
sw-hint-zoom-back = Zoom back one step
sw-hint-zoom-reset = Reset zoom
sw-hint-abort = Abort
sw-hint-play = Play
sw-hint-stop-play = Stop playback
sw-hint-band-select = Band select
sw-hint-area-delete = Delete area
sw-hint-band-reject = Band reject
sw-hint-am = Quadrature AM detector
sw-hint-modified-am = Modified AM detector
sw-hint-half-wave = Double half-wave detector
sw-hint-derivative = First derivative
sw-hint-power = Power-of-N module
sw-hint-phase-plane = PhasePlane
sw-hint-acf = Autocorrelation
sw-hint-ccf = Cross-correlation
sw-hint-ofdm = OFDM module
sw-hint-phase-detector = Phase detector
sw-hint-limiter = Hard limiter
sw-hint-iq = I/Q to real
sw-hint-shift-base = Shift band to base
sw-hint-shift-base-inverted = Shift band to base, inverted
sw-hint-inverse = In-band inverse (the whole spectrum while the H markers are collapsed)
sw-hint-copy = Copy fragment
sw-hint-resample = Resample to the rate in the field
sw-hint-shift = Shift spectrum by the value in the field (Hz; negative moves it down)
sw-value = R / S
    .hover = New rate for R, or shift for S, in Hz (a comma or a point for decimals)
sw-value-presets = Preset values

## Why a button is disabled.

sw-need-history = There is no zoom step to go back to.
sw-need-running = Nothing is running for this window.
sw-need-playing = Nothing is playing.

## Popup menu.

sw-popup-save-as = Save as
sw-popup-wave-file = WaveFile
sw-popup-data-file = DataFile
sw-popup-wave-file-float = WaveFile (32-bit float)
sw-popup-sigmf = SigMF
sw-popup-waveform = Wave Form module
sw-popup-measure = Measurement of Freq
sw-popup-adp = Auto define params
sw-popup-brc = Correction of BR
sw-popup-cancel = Cancel

## Correction of BR panel.

sw-brc-existing = Existent Baud Rate
sw-brc-required = Required Baud Rate
sw-brc-max-delta = Max delta change: +/- 30%
sw-brc-yes = Yes

## Messages.

sw-check-settings = Please check settings!
# Result boxes in the plot. $value: a number with two decimals.
sw-label-freq = Freq: { $value } Hz
sw-label-brate = Brate: { $value } Hz
sw-rs-resample-invalid = The R/S value "{ $text }" is not a positive sample rate; type a rate in Hz such as 8000.
sw-rs-shift-invalid = The R/S value "{ $text }" is not a number; type a shift in Hz such as -150.
sw-brc-existing-invalid = Existent Baud Rate "{ $text }" is not a number; type the measured Br in Bd, or run Auto define params first.
sw-brc-required-invalid = Required Baud Rate "{ $text }" is not a number; type the Br you want in Bd.
sw-brc-not-positive = Both baud rates must be above 0 Bd.
sw-brc-too-far = Required Br must be between { $lo } and { $hi } Bd (at most 30% from the existent one).
sw-brc-measured = Pre-filled with the Br measured by Auto define params
sw-brc-not-measured = No Br measured yet: type it, or run Auto define params first
