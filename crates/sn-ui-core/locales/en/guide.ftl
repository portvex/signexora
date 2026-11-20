# The workflow guide: SA's analysis method as a checklist with buttons.

guide-title = Workflow guide
guide-no-signal = Open a recording first (File > Open file...).
# $signal: the active window's signal, which the buttons act on.
guide-acts-on = Buttons act on: { $signal }
# $key: a key name. Shown after a menu path only when that key acts on the guide's signal.
guide-key = , key { $key }

## Steps: the message is the step's title, .text its explanation.

guide-step-record = 1. Record properly
    .text = 16-bit PCM WAV (or I/Q) with the receiver's AGC off, no clipping, no editing. Compressed or 8-bit files are flagged when you open them.
guide-step-quality = 2. Check recording quality
    .text = Open the WF window (right-click the sonogram > { sw-popup-waveform }). Look for clipping, 8-bit steps, DC offset, AGC pumping and dropouts. Turn on Sum to see the amplitude distribution.
guide-step-isolate = 3. Isolate the signal
    .text = On the sonogram, drag the H markers around the signal. Then use the toolbar's { sw-hint-band-select } and { sw-hint-shift-base }, and R to resample to a lower rate if it is wide.
guide-step-rate = 4. Find the keying rate
    .text = Run a detector, frame its clock line with the markers and use the popup menu's { sw-popup-measure }{ $br }. { sw-hint-am } for PSK/QAM/FSK, { sw-hint-modified-am } for MFSK, or try { sw-popup-adp } (ADP).
guide-step-family = 5. Work the modulation family
    .text = FSK/MFSK: { sw-hint-phase-detector }, then in the WF window { edits-median } over 1/3 symbol, Sum, Find tones, Count freqs. PSK/QAM: { sw-hint-power } to find order and carrier ({ sw-popup-measure } in its window{ $sc }), then PhasePlane; on distorted signals use the ISD corrector first. OFDM: Find CT, Get Br, Channels, then the List of K for the native rate.
guide-step-structure = 6. Find the structure
    .text = { sw-hint-acf } for frame periods, { sw-hint-ccf } to find a fragment everywhere, then place the V markers on one frame and choose { sw-popup-measure } > { raster-dt-menu-item } in the popup menu to record dT (the frame period), and open { guide-btn-raster } at that period to see frames and sync words.

## Step buttons.

guide-btn-waveform = Waveform
guide-btn-sonogram = Sonogram
guide-btn-am-detector = AM detector
guide-btn-modified-am = Modified AM
guide-btn-phase-detector = Phase detector
guide-btn-demod = FSK/MFSK demodulator
# $n: the power, a number.
guide-btn-power = Raise to power { $n }
    .hover = Makes a new signal raised to the power { $n }; a PSK-{ $n } signal then shows a line at { $n } times the carrier
guide-btn-phase-plane = PhasePlane
guide-btn-raster = Raster (VMW)
guide-btn-adp = ADP
    .hover = Estimate the keying rate automatically (ADP) from the V-marker span, or the whole signal
guide-btn-ofdm = OFDM
    .hover = Open the OFDM tool (Find CT, Get Br, List of K)
guide-open-view = Open this view of the active window's signal
guide-new-signal = Makes a new signal from the active window's signal (opens in a new window)
# $kind: Br, Sc or dT (untranslated).
guide-not-measured = { $kind }: not measured yet
