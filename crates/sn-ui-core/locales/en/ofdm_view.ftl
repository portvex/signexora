# OFDM module window, SA-OFDM (SA_UI_SPEC.md section 10).

# Top bar (SA's control names).
ofdm-min-br = Min Br-LS
    .hover = Lowest symbol rate searched, Bd (an integer); the field after it is the longest LS
ofdm-max-br = Max Br-LS
    .hover = Highest symbol rate searched, Bd (an integer); the field after it is the shortest LS
ofdm-total-ls = Total LS
    .hover = Symbols of the longest length (Min Br-LS) in the selection
ofdm-current-ls = Current LS
    .hover = Symbol shown (0 while Find CT runs)
ofdm-survey = Survey
    .hover = Overview: a 1024-point spectrum window moved with the scroll bar below sets where Find CT starts
ofdm-list-k = List of K
    .hover = Magic K calculator: the most likely K values and native sample rates
    .disabled = Run Find CT and ADP first

# Controls (SA's control names).
ofdm-symbols = Symbols
    .hover = Symbols folded into the correlation triangle (and ADP's block); 20 to 40 is good, never below 6
ofdm-scale-fft = Scale FFT
    .hover = Width of the channel bars of the LU spectrum
ofdm-shift = Shift Frequency
    .hover = Frequency removed before the FFT, Hz; Enter applies
ofdm-vshift = VShift
    .hover = Virtual shift by one channel (exact grid steps, no limit)
ofdm-shift-step = Step
    .hover = Shift Frequency up or down by { $step } Hz
ofdm-br = Br
    .hover = Symbol rate: ADP's value, or type one and press Enter
ofdm-adp = ADP
    .hover = Precise symbol rate and frequency position (SA's Get Br)
    .disabled = Run Find CT first
ofdm-show = Show
    .hover = Show the marked channels symbol by symbol with symbol sync (Break stops)
ofdm-break = Break
ofdm-find-ct = Find CT
    .hover = Find the correlation triangle: LS, LG and the symbol start
ofdm-mode = Mode
    .hover = Signal model of the constellations
ofdm-model-a = A
    .hover = Model A: every channel as generated (absolute values)
ofdm-model-b = B
    .hover = Model B: every channel is its own pilot (relative values)
ofdm-model-c = C
    .hover = Model C: data referenced to the pilot at the red marker
ofdm-each = Each
    .hover = Show every Nth symbol
ofdm-start-plus = Start+
    .hover = First symbol shown by Show

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
ofdm-finish = Normal finish, time { $secs } sec.
ofdm-stopped = Stopped by user, time { $secs } sec.
ofdm-wait = Don't panic! Please wait...
ofdm-wait-ct = Searching the correlation triangle
ofdm-wait-adp = Calculating the precise Br
ofdm-wait-k = Calculating "Magic K"
ofdm-wait-left = Remained Time: { $secs } sec.

# Plots.
ofdm-survey-plot = Survey spectrum
ofdm-triangle-plot = Correlation triangle
ofdm-triangle = Triangle
    .hover = Click the good (left) slope to place the LU window; at the peak it is the parent LU
ofdm-spectrum-plot = LU spectrum
ofdm-spectrum = Spectrum
    .hover = Click a channel to move the nearer marker, double-click to place both
ofdm-pane = Channel
    .hover = Channel { $ch }, { $count } symbols
ofdm-k = K
    .hover = K = { $k }, LU error { $err }, ICI { $ici } dB, { $lu }
ofdm-k-lu-even = even LU
ofdm-k-lu-odd = odd LU

# Popup menus.
ofdm-sort-sr = Sort by SR
ofdm-sort-k = Sort by K
ofdm-sort-am = Sort by AM
ofdm-k-close = Close
ofdm-menu-cancel = Cancel
ofdm-menu-reset = Clear the results
ofdm-menu-around = Bounds around Br = { $rate }
    .hover = Set Min Br-LS and Max Br-LS to 0,8 and 1,25 times the measured Br
ofdm-resample = Resample to { $rate } Hz
    .hover = Resample the signal to this native rate (a new window)

# Jobs and messages.
ofdm-job-ct = OFDM: Find CT
ofdm-job-br = OFDM: ADP
ofdm-job-k = OFDM: List of K
ofdm-job-show = OFDM: Show
ofdm-needs-ct = Run Find CT first.
ofdm-needs-br = Run ADP first.
ofdm-baseband-freed = The window was closed meanwhile: run Find CT again.
ofdm-check-settings = Please check settings! Show needs Symbols of at least 6 and 10 symbols after Start+.
ofdm-failed = Search failed, time { $secs } sec.
ofdm-pick-busy = Wait for the running job, or press Break, then pick a line.
ofdm-br-invalid = Br "{ $text }" is not usable: enter a number above 0 and up to { $max } Bd (fs/LU).
ofdm-br-invalid-nogrid = Br "{ $text }" is not usable: enter a number above 0.
ofdm-plot-empty = No data: run Find CT.
ofdm-shift-invalid = Shift Frequency "{ $text }" is not a number; type it in Hz, such as 12.5.
