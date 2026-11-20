# The Signal menu, the parameter dialog of its operations and the save formats.

edits-save-wav-pcm16 = WAV, 16-bit PCM
edits-save-wav-float = WAV, 32-bit float
edits-save-sigmf = SigMF (keeps fractional rates)
edits-save-raw-f32 = Raw, 32-bit float

# $from: where a measured value came from, e.g. "n4, measure line".
edits-measured-other-axis = { $from } (other axis, check)

## Notes under the dialog fields: where a pre-filled value came from.

edits-note-cutoff-raised = Cutoff: Br from { $from } (raised to the 30 Hz minimum; Br is { $br } Bd)
edits-note-cutoff-lowered = Cutoff: Br from { $from } (lowered below half the sample rate; Br is { $br } Bd)
edits-note-cutoff-br = Cutoff: Br from { $from }
edits-note-cutoff-guessed = Cutoff: guessed (fs/8), measure Br first
# $count: the longest window offered, in samples.
edits-note-window-capped = { $count ->
    [one] Window: 1/3 symbol at Br from { $from } (capped at { $count } sample; resample down for a true 1/3 symbol)
   *[other] Window: 1/3 symbol at Br from { $from } (capped at { $count } samples; resample down for a true 1/3 symbol)
}
edits-note-window-br = Window: 1/3 symbol at Br from { $from }
edits-note-window-default = Window: default 5 samples; measure Br to get 1/3 symbol
edits-note-measured-br = Measured Br from { $from }
edits-note-measured-s = Measured: S = 1/dT of the V markers
edits-note-carrier-sc = Carrier: Sc from { $from }
edits-note-carrier-h = Carrier: centre of the H markers
edits-note-carrier-guessed = Carrier: guessed (fs/4), measure Sc first
edits-note-br-from = Br: from { $from }
edits-note-br-guessed-8 = Br: guessed (fs/8), measure Br first
edits-note-br-s = Br: S = 1/dT of the V markers
edits-note-br-guessed-10 = Br: guessed (fs/10), measure Br first

## Why a menu item is disabled.

edits-need-h = Set the H markers first (drag on the sonogram).
edits-need-v = Set the V markers first (drag on the sonogram or waveform).
edits-need-vh = Set the V and H markers first (drag on the sonogram).
edits-need-complex = The signal is already real.
edits-need-baud = Measure Br (Measure menu) or set V markers over one symbol first.

## Signal menu sections and items.

edits-section-spectrum = Spectrum (H markers)
edits-band-select = Band select
edits-band-reject = Band reject
edits-area-delete = Area delete
edits-shift-to-base = Shift band to base
edits-shift-to-base-inverted = Shift band to base, inverted
edits-invert-band = In-band inverse
edits-section-whole = Whole signal
edits-shift = Shift spectrum...
edits-invert = Spectrum inverse
edits-resample = Resample...
edits-iq-to-real = I/Q to real
edits-baud-correction = Baud-rate correction...
edits-derivative = First derivative
edits-limiter = Hard limiter
edits-section-isd = ISD corrector
edits-isd-am = ISD corrector, AM mode (blind)...
edits-isd-xn = ISD corrector, X^N mode...
edits-section-detectors = Detectors
edits-am-detector = AM detector (clock of PSK/QAM/FSK)
edits-modified-am = Modified AM detector (clock of MFSK)
edits-half-wave = Double half-wave detector
edits-phase-detector = Phase detector (frequency in Hz)
edits-power = Raise to power N...
edits-acf = ACF of selection
edits-ccf = CCF: find the selection everywhere
edits-section-time = Time (V markers)
edits-copy = Copy selection to new signal
edits-section-filters = Filters
edits-lowpass = Lowpass...
edits-median = Median...
edits-rrc = RRC (matched filter)...

## Parameter dialog (SA's small modal dialog).

# $operation: the dialog's operation; $signal: the window name, e.g. "test(0)(u1)".
edits-dialog-title = SA - { $operation }: { $signal }
edits-title-shift = Shift spectrum
edits-title-resample = Resample
edits-title-baud = Correction of BR
edits-title-lowpass = Lowpass
edits-title-median = Median filter
edits-title-rrc = RRC filter
edits-title-power = Raise to power N
edits-title-isd-am = ISD corrector (AM mode)
edits-title-isd-xn = ISD corrector (X^N mode)

edits-field-carrier = Carrier (Hz)
edits-field-br = Br, symbol rate (Bd)
edits-field-blocks = Blocks
edits-field-power-psk = Power N (PSK order)
edits-field-window = Window (samples, odd)
edits-field-shift = Shift (Hz, + is up)
edits-field-rate = New sample rate (Hz)
edits-field-measured-br = Existent Baud Rate
edits-field-required-br = Required Baud Rate
edits-field-cutoff = Cutoff (Hz)
edits-presets = presets
edits-field-power = Power N
edits-field-alpha = Roll-off alpha
# $lo, $hi: the limits of a field.
edits-range = From { $lo } to { $hi }
# $max: the sample rate in Hz.
edits-hint-shift = Up to +/- { $max } Hz, not 0; fractions allowed
# $max: the longest window, in samples.
edits-hint-window = From 1 to { $max }; an even value is used as the next odd one
edits-ok = Ok
edits-cancel = Cancel
edits-field-invalid = { $field } "{ $text }" is not valid. { $range }.
edits-range-open = Above { $lo } and below { $hi }
