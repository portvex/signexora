# The WF (waveform) window: SA's top toolbar, bottom toolbar hints, popup menu and
# result box. SA's control names (LPF, RRC, Sum, Info, VMW, Br, Z+, Z-) and the
# result box text are names: keep them.

## Top toolbar, left to right.

wf-lpf = LPF
    .hover = Low-pass cutoff in Hz, or the baud rate of the RRC filter (type a value or pick one)
wf-copy-br = <==
    .hover = Copy the measured Br into the LPF field
wf-rrc = RRC
    .hover = Switch the RRC filter on or off (the LPF button then runs it)
wf-rrc-alpha = RRC
    .hover = RRC roll-off
wf-freq = 0-0
    .hover = Click: frequency of the short fragment between the V markers, and its whole periods
wf-vmw = VMW
    .hover = Switch to VMW (the signal folded at a line period)
wf-sum = Sum
    .hover = Show the histogram of the values (then Count freqs in the popup menu)
wf-info = Info
    .hover = Show the results overlay (t1, t2, dT, S, Br)

## Bottom toolbar hints, left to right.

wf-hint-zoom-in = Zoom in (time)
wf-hint-zoom-out = Zoom out (time)
wf-hint-scale-up = Larger vertical scale
wf-hint-scale-down = Smaller vertical scale
wf-hint-zoom-plus = Z+: Zoom between the V markers
wf-hint-zoom-minus = Z-: Zoom back one step
wf-hint-lpf = Run the LPF filter (the RRC filter while RRC is on)
wf-hint-revert = Revert to original
wf-hint-synth = Reverse FSK/MFSK synthesis
wf-hint-save = Save as 16-bit WAV (right-click for float WAV, DataFile or SigMF)
wf-hint-copy = Copy the fragment between the V markers to a new window
wf-hint-demod = Send the fragment to the FSK/MFSK demodulator
wf-hint-fit = Fit the vertical scale to the signal
wf-hint-acf = Autocorrelation of the fragment (new window)
wf-hint-ccf = Cross-correlation of the fragment with the whole signal (drawn here)

## Why a button is disabled.

wf-need-revert = Nothing to revert: no filter run and no CCF shown.
wf-need-derived = This window shows the opened file, which is already saved. Save is for results made here.

## Popup menu.

wf-popup-count-freqs = Count freqs
wf-popup-more = SigNexora tools
    .hover = Tools of SigNexora that SA's WF window did not have
wf-popup-operations = Operations

## The Count freqs result box (SA's text, not translated). $size: a whole number;
## $value: a number with two decimals.

wf-label-grid = Freq grid: { $size }
wf-label-delta = Delta: { $value } Hz
wf-count-needs-h = Set the H markers around the tones first.
wf-count-tones-busy = Find tones is still running; try Count freqs again when it finishes.
wf-count-freqs-failed = No frequency grid was found ({ $error }). Widen the H markers around the tones or zoom out so more tones are in view.
wf-copy-br-value = Copy Br = { $br } Bd into the LPF field
wf-copy-br-missing = Measure Br first (sonogram popup: Auto define params)
