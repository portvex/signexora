# File > Signal generator (OCG): the generator window, its preview and the
# OFDM calculators. Keep units (Hz, Bd, s, ms, dB, ppm), modulation names
# (FSK, MSK, GMSK, PSK, QAM, C4FM, OFDM, BPSK, ...), BT, Es/N0, SNR, RC, RRC,
# LU, LG, LS, K, Br, Fd and the model letters A, B, C untranslated.

gen-menu = Signal generator (OCG)...
    .hover = Make a test signal with exactly known parameters: tones, noise, FSK, PSK/QAM, C4FM or OFDM, with channel impairments
gen-keys-open = Show or hide the signal generator
gen-title = Signal generator (OCG)
gen-job = Generating { $name }

gen-kind = Kind
gen-kind-tones = Tones
gen-kind-noise = Noise
gen-kind-fsk = FSK/MFSK
gen-kind-linear = PSK/QAM
gen-kind-c4fm = C4FM
gen-kind-ofdm = OFDM
gen-preset = Preset
    .hover = Load a ready-made signal
gen-preset-defaults = Defaults
    .hover = Reset the current kind to its default values

gen-section-signal = Signal
gen-section-common = Common
gen-section-channel = Channel impairments
gen-section-derived = Derived values
gen-section-preview = Preview
gen-section-calc = Calculators

gen-complex = Complex (I/Q) output
    .hover = Make I/Q samples instead of a real signal; frequencies are then offsets from 0 Hz
gen-amplitude-rms = RMS amplitude
gen-amplitude-peak = Peak amplitude
gen-phase-rad = Phase (rad)
gen-remove = Remove
gen-baud = Baud rate
gen-carrier = Carrier
gen-center = Centre
gen-alpha = Roll-off

gen-tones-frequency = Frequency
gen-tones-add = Add tone
gen-tones-last = At least one tone is needed

gen-fsk-order = Tones
gen-fsk-spacing = Spacing
gen-fsk-shaping = Shaping
gen-fsk-shaping-none = None
gen-fsk-shaping-gaussian = Gaussian
gen-fsk-bt = BT
gen-fsk-msk = MSK
    .hover = Two tones, spacing half the baud rate, no shaping
gen-fsk-gmsk = GMSK
    .hover = MSK with a Gaussian filter of BT 0.3, as in GSM

gen-lin-constellation = Constellation
gen-lin-shaping = Pulse shape
gen-lin-rect = Rectangular
gen-lin-rc = Raised cosine (RC)
gen-lin-rrc = Root raised cosine (RRC)
gen-lin-differential = Differential
    .hover = Send each symbol as a phase step from the previous one
gen-lin-differential-qam = QAM carries amplitude as well as phase, so it cannot be differential
gen-lin-differential-pi4 = pi/4-DQPSK is always differential

gen-c4fm-pulse = Pulse
gen-c4fm-shaped = Shaped
    .hover = Off: the frequency jumps between the four levels
gen-c4fm-apco = APCO-25
    .hover = The TIA-102 pulse, meant for an integrate-and-dump receiver
gen-c4fm-note = The deviations are fixed at +-600 and +-1800 Hz; C4FM output is always real.

gen-ofdm-lu = LU
    .hover = Useful length (FFT size) in samples at the sample rate
gen-ofdm-lg = LG
    .hover = Guard (cyclic prefix) length in samples
gen-ofdm-channels = Channels
gen-ofdm-first-bin = First bin
    .hover = Bin of the first channel; bin 0 is the centre
gen-ofdm-modulation = Modulation
gen-ofdm-unused = Unused positions
    .hover = Channel positions that carry nothing, counted from 0, for example 3, 7, 10-12
gen-ofdm-overrides = Overrides
    .hover = Modulation of single positions, for example 0: pilot, 5-6: QAM-16
gen-ofdm-model = Model
gen-ofdm-model-a = A: every channel sends absolute values
gen-ofdm-model-b = B: every channel is differential from symbol to symbol
gen-ofdm-model-c = C: pilots carry the reference, data are absolute relative to them
gen-ofdm-pilots = Pilots
gen-ofdm-pilots-none = None
gen-ofdm-pilots-list = Positions
    .hover = Pilot positions, for example 0, 8, 16
gen-ofdm-pilots-every = Every N
gen-ofdm-step = Step
gen-ofdm-offset = First
gen-ofdm-reference = Pilot reference
    .hover = What the pilots carry in model C
    .disabled = Only model C uses a pilot reference
gen-ofdm-reference-constant = Constant
gen-ofdm-reference-differential = Random common phase
gen-ofdm-center = Centre
    .hover = Where bin 0 lands; for complex output, the offset from 0 Hz

gen-label = Label
    .hover = Name of the new signal; empty uses the description
gen-sample-rate = Sample rate
gen-length = Length
gen-length-seconds = Seconds
gen-length-symbols = Symbols
    .disabled = Tones and noise have no symbols
gen-seed = Seed
gen-new-seed = New seed
    .hover = Pick a random seed: new data and new noise
gen-center-frequency = Centre frequency
    .hover = RF frequency that 0 Hz of the complex output stands for (stored with the signal)
    .disabled = Only complex output has a centre frequency

gen-ch-offset = Frequency offset
gen-ch-drift = Drift
gen-ch-clock = Clock offset
    .hover = Sample-clock error; positive means the receiver clock runs fast
gen-ch-noise = Noise
gen-ch-noise-none = None
gen-ch-noise-full = SNR over the full band
gen-ch-noise-inband = SNR in band
gen-ch-noise-esn0 = Es/N0
gen-ch-bandwidth = Bandwidth
    .hover = Off: the bandwidth the signal occupies
gen-ch-clip = Clip at
    .hover = Limit the peak amplitude; complex samples keep their phase
gen-ch-multipath = Multipath taps
    .hover = The taps are the whole channel: include the direct path (delay 0, gain 1)
gen-ch-delay = Delay
gen-ch-gain = Gain
gen-ch-add-tap = Add tap

gen-d-none = Fix the parameters to see the derived values.
gen-d-symbol-rate = Symbol rate
gen-d-sps = Samples per symbol
gen-d-symbols = Symbols
gen-d-bandwidth = Occupied bandwidth
gen-d-samples = Samples
gen-d-duration = Duration
gen-d-index = Modulation index h
gen-d-ls = LS
gen-d-k = K = LG/LU
gen-d-spacing = Spacing
gen-d-br = Br

gen-preview = Generator preview
gen-preview-spectrum = Spectrum
gen-preview-wave = Waveform
gen-preview-name = Preview of the generated signal
gen-preview-building = Making the preview...
gen-preview-invalid = No preview until the parameters are valid
gen-preview-failed = Preview failed: { $error }
gen-preview-short = Too short for a spectrum
gen-preview-axis = Mean spectrum, { $lo } to { $hi } Hz, 80 dB range
gen-preview-wave-axis = First { $ms } ms, peak { $peak }

gen-generate = Generate
    .hover = Add the signal to the session as a new recording
gen-generate-save = Generate and save as
    .hover = Generate, then save the new signal in the chosen format
gen-busy = A signal is being generated
gen-fix-first = Fix the parameters first
gen-invalid = Cannot generate: { $error }
gen-field-error = { $field }: { $error }
gen-ready = Ready: { $samples } samples, { $duration } s
gen-list-bad = "{ $item }" is not a position or a range such as 10-12
gen-list-too-long = Too many positions (at most { $max })
gen-overrides-bad = "{ $item }" is not a position and a modulation such as 5: QAM-16

gen-calc-cp = OFDM with CP
gen-calc-cp-lengths = From Fd, LU and LG
gen-calc-spacing = Spacing
gen-calc-symbol = Symbol
gen-calc-useful = Useful part
gen-calc-guard = Guard
gen-calc-use = Use in OFDM
    .hover = Copy these lengths (and Fd) into the OFDM editor
    .disabled = LU and LG must be whole numbers of samples
# Why Use in OFDM is disabled. $max: the generator's limit in samples.
gen-calc-lu-not-whole = LU must be a whole number of samples, at least 2.
gen-calc-lg-not-whole = LG must be a whole number of samples.
gen-calc-lu-too-large = LU must be at most { $max } samples.
gen-calc-lg-too-large = LG must be at most { $max } samples.
gen-calc-use-k = Use in OFDM
    .hover = Copy this K into the OFDM editor: the smallest LU that fits it, from the current LU up
gen-calc-inverse = From Br and spacing at Fd: nearest whole LU and LG
gen-calc-exact = exact
gen-calc-error = Error
gen-calc-candidates = Whole LU and LG for Br and K
gen-calc-lu-range = LU from
gen-calc-tolerance = Tolerance
gen-calc-found = { $count ->
    [one] { $count } candidate
   *[other] { $count } candidates, the first { $shown } shown
}
gen-calc-klist = List of K
gen-calc-k-from-rates = From Br and spacing
gen-calc-k-direct = From K
gen-calc-uncertainty = Uncertainty
    .hover = How far off each measured rate may be, in percent of the rate. The list shows the simplest fractions within the K uncertainty this gives; for K entered directly, the tolerance is that of K measured from two rates this far off.
gen-calc-uncertainty-bad = The uncertainty must be between 0 and { $max } %.
gen-calc-k-none = No fraction lies within the uncertainty. Raise the uncertainty or the largest denominator.
gen-calc-max-den = Largest denominator
gen-calc-count = Count
gen-calc-value = Value
gen-calc-fd = Fd
    .hover = Sample rate in Hz that makes these lengths exact for the Br: Br x (LU + LG)
gen-calc-k = K
    .hover = K = LG/LU, the guard to useful length ratio these whole lengths really have
gen-calc-lg-error = Error (samples)
    .hover = LG minus K x LU: how far the whole guard length is from the exact one, in samples
gen-calc-k-fraction = K
    .hover = K as a fraction p/q in lowest terms
gen-calc-k-error = Error
    .hover = The fraction's value minus the measured K (signed; K has no unit, so neither does this)
gen-preview-symbol-too-long = No preview: one symbol is { $samples } samples, longer than a preview. Raise the baud rate or lower the sample rate to see one.
gen-calc-fraction-too-big = K = { $k } does not fit: LU must be at most { $lu } and LG at most { $lg } samples. Choose a smaller LU or K.
