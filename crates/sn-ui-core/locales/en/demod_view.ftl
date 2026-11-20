# FSK-2 / universal MFSK demodulator window (SA_UI_SPEC.md section 8).

# Text panel.
demod-heading =
    SA - MFSK dem
    Levels: { $levels }
    Fd: { $fs }
    Br: { $br }
    Data as: { $form }
    Data:
demod-as-default = Default map
demod-as-raw = Raw
demod-as-bits = Bits, natural
demod-as-gray = Bits, Gray
demod-as-c4fm = APCO-25 C4FM dibits
demod-polarity-inverted = [polarity: inverted, bits mapped with the inverted C4FM table]
demod-polarity-normal = [polarity: normal]
demod-polarity-unknown = [polarity: unknown, bits mapped with the normal C4FM table]
demod-truncated = (first 20000 characters; Copy and Save as take everything)
demod-nothing-yet = Nothing demodulated yet: check Br, the levels and the markers.

# Jobs.
demod-job-track = Demodulator: frequency track
demod-job = Demodulating FSK/MFSK

# Top bar.
demod-rrc = RRC
    .hover = Root-raised-cosine filter before the decisions; the field is its cutoff in Hz (Br by default)
demod-cutoff = Cutoff
    .hover = RRC cutoff in Hz: type a value or pick one from the list
demod-alpha = Alpha
    .hover = RRC roll-off, 0,05 to 1 (Enter or leaving the field applies)
demod-br = Br
    .hover = Baud rate; press Enter to apply
demod-br-step = Step
    .hover = Br up or down by { $step }
demod-auto = A
    .hover = Automatic phase and bit synchronisation
demod-auto-button = A
    .hover = Find the phase automatically now and keep it (switches the automatic mode off)
demod-exactly = <= Exactly
    .hover = Fine sampling phase (+-{ $span } % of a symbol)
demod-roughly = <= Roughly
    .hover = Coarse sampling phase across one symbol
demod-levels = Levels
    .hover = Number of levels, 2 to 256
demod-raw = Raw
    .hover = Output the levels as hex bytes separated by x

# Bottom bar.
demod-back-fast = Back 10
    .hover = Back 10 pages
demod-back = Back
    .hover = Previous page
demod-forward = Forward
    .hover = Next page
demod-forward-fast = Forward 10
    .hover = Forward 10 pages
demod-page = Page
    .hover = Page shown ({ $count } in all); Enter goes there
demod-scale-up = Scale up
    .hover = Vertical scale up
demod-scale-down = Scale down
    .hover = Vertical scale down
demod-progress = Progress
    .hover = Progress of the computation (right-click to stop)
demod-stop = Stop
demod-filter = Filter
    .hover = Apply the RRC filter
demod-revert = Revert
    .hover = Back to the unfiltered signal
demod-d = D
    .hover = Output the bitstream to the text panel

# Plot.
demod-plot-name = Demodulator levels
demod-info = Br = { $baud } | clock { $clock } % | quality { $quality } | eye { $eye } | { $count } symbols
demod-menu-tones = Levels at the found tones ({ $count })
    .hover = Use the tones found with WF Sum as the levels instead of the evenly spaced zones
    .disabled = Find the tones first: WF, Sum, Find tones
demod-menu-average = Average filter
    .hover = Integrate over half a symbol when RRC is off (rectangular FSK)
demod-menu-adaptive = Adaptive levels
    .hover = Let the level centres follow a slow drift of the data
demod-menu-info = Info
demod-menu-reset-view = Reset the vertical scale
demod-menu-cancel = Cancel

# Text panel popup.
demod-menu-copy = Copy
demod-menu-save = Save as...
demod-menu-data-as = Data as
demod-save-title = Save the demodulated data
demod-save-filter = Text
demod-save-failed = Could not write { $path }: { $error }
demod-cutoff-invalid = The RRC cutoff "{ $text }" is not a positive number; type it in Hz, such as the Br.
demod-waiting-cutoff = Waiting for a valid RRC cutoff: the result shown is from the earlier settings.
demod-br-invalid = The Br "{ $text }" is not a positive number; the Br in use was kept.
