# The PhasePlane form (SA_UI_SPEC.md section 6): spectrum mode toolbar, ISD
# corrector, phase viewer, status bar, demodulator output and the LoadFile dialog.
# The control captions are SA's own English names (SA showed them in English under
# every Windows language): keep them as they are; translate the hints and dialogs.

pp-fmin = Fmin
    .hover = Lowest frequency shown, Hz
pp-fmax = Fmax
    .hover = Highest frequency shown, Hz
pp-scale = Scale
    .hover = Part of the band shown; the scroll bar under the spectrum moves it
pp-fft-size = FFTSize
    .hover = FFT size of the averaged spectrum: larger is finer (down to about 1 Hz)
pp-low-level = LowLevel
    .hover = Lowest level shown
pp-stretch = Stretch
    .hover = Stretch the spectrum to the full height
pp-mode = Mode
    .hover = AM: correct on the envelope; X^N: on the Nth power of the signal
pp-blocks = Blocks
    .hover = Number of signal blocks the corrector averages
pp-corr-off = Corr-OFF
    .hover = The ISD corrector is off: press Auto to correct the signal
pp-corr-on = Corr-ON
    .hover = The ISD corrector is on (SaveAs saves the corrected signal)
pp-save-as = SaveAs
    .hover = Save the corrected signal to a file
pp-auto = Auto
    .hover = One step of the corrector over the selected blocks
pp-break = Break
pp-reset = Reset
    .hover = Reset the corrector (switch it off)
pp-carrier = Carrier
    .hover = Carrier frequency, Hz (from the last Sc measurement)
pp-br = BR
    .hover = Keying rate, Bd (from the last Br measurement)
pp-nary = n-Ary
    .hover = Order of the modulation, 2-32 (1: no carrier tracking)
pp-synch-br = Synch BR
    .hover = Clock synchronisation
pp-nonstop = NonStop
    .hover = Run continuously: start over at the end of the file
pp-rrc = RRC filter
    .hover = Root-raised-cosine filter; the slider sets alpha
pp-pll = PLL
    .hover = Carrier loop speed
pp-pll-slow = Slow
pp-pll-medium = Medium
pp-pll-quick = Quick
pp-pll-hard = Hard
pp-auto-reset = AutoReset
    .hover = With NonStop: reset the generators and PLL loops at the end of the file (for drifting signals)
pp-transitions = Transitions
    .hover = Draw the transitions between the points (the trajectory)
pp-life = Life
    .hover = Dynamic display: only the last Points... symbols stay
pp-diff = Diff
    .hover = 0: absolute angles; 1-3: phase differences (relative modes such as pi/4 DQPSK)
pp-offset-detected = Offset mode detected
pp-angles = Angles
    .hover = Build the angles histogram in the corrector plot (needs Synch BR)
pp-vectors = Vectors of dem
    .hover = Show the demodulator's positions; the zero vector is light blue
pp-output-dem = Output Dem as
    .hover = Write the demodulated stream as codes of absolute angles to the text panel
pp-trimming = Trimming { $value }
    .hover = Fine phase trim, degrees
pp-delay = Delay { $value }
    .hover = Real delay per symbol, seconds (for very short signals)
pp-points = Points... { $value }
    .hover = Points shown at once with Life on
pp-contrast = Contrast { $value }
    .hover = Brightness of the points
pp-start = Start
    .hover = Start or stop the constellation; after a stop it starts again from the beginning of the file
pp-clear = Clear
    .hover = Clear the display
pp-load-file = LoadFile
    .hover = Load one of the open files into the phase plane
pp-status-f = F = { $value }
pp-status-fl = Fl = { $value }
pp-status-fu = Fu = { $value }
pp-status-df = dF = { $value }
pp-status-angle-d = Angle_D = { $value }
pp-status-angle-i = Angle_I = { $value }
pp-dem-view-mode = View mode: diff { $diff }
pp-dem-mode = Dem mode: { $mode }
pp-dem-absolute = Data presented as absolute angles!
pp-dem-coding = The coding positions of angles:
pp-dem-data = Data:

pp-isd-n = Harmonic of the signal the corrector works with
    .hover = Harmonic of the signal the corrector works with
pp-isd-b = On: the better algorithm; off: a slightly worse, faster one (no effect in AM mode)
    .hover = On: the better algorithm; off: a slightly worse, faster one (no effect in AM mode)
pp-isd-break = Stop the current step
    .hover = Stop the current step
pp-alpha = Roll-off alpha of the RRC filter
    .hover = Roll-off alpha of the RRC filter
pp-quality = Conventional signal-to-noise ratio of the constellation, dB
    .hover = Conventional signal-to-noise ratio of the constellation, dB
pp-load-title = SA - select file for PhasePlane
pp-ok = Ok
pp-cancel = Cancel
pp-check-settings = Please check settings!
pp-run-limited = Only the first { $symbols } symbols are demodulated. To analyse a later part, set the V markers on it on the sonogram, use { sw-hint-copy } and open { sw-hint-phase-plane } on the copy.
pp-copy-panel = Copy the text
pp-copy-codes = Copy all codes since Start
pp-eye = Eye diagram
    .hover = Show the eye diagram (I, two symbols) instead of the constellation
pp-job-corrector = ISD corrector spectrum
pp-job-isd = ISD corrector
pp-job-spectrum = PhasePlane spectrum
# $count symbols, $offset Hz, $rings radii, $iq_offset symbols.
pp-quality-details = { $count ->
        [one] { $count } symbol
       *[other] { $count } symbols
    }, loop offset { $offset } Hz, rings [{ $rings }], I/Q offset { $iq_offset } symbols
pp-source-gone = The loaded file is no longer in the session; load a working file again with LoadFile.
pp-load-gone = The selected file was closed; pick another working file.
