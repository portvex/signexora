# Main window (MDI frame): messages, File menu extras, the WF window's toolbar
# and popup menu, and the confirmation dialogs.

## View kinds (child windows)

app-tab-sonogram = Sonogram
app-tab-waveform = Waveform
app-tab-raster = Raster (VMW)
app-tab-phase = PhasePlane
app-tab-demod = FSK/MFSK demodulator
app-tab-ofdm = OFDM

## Lists of names: "A", "A and B", "A, B and C"

app-list-separator = { ", " }
app-list-and = { $rest } and { $last }

## Background jobs (shown in the status bar while they run)

app-job-loading = Loading { $name }
app-job-overview = Building waveform overview
app-job-saving = Saving { $path }
app-job-measure-line = Measuring line frequency
app-job-adp = Estimating baud rate (ADP)
app-job-writing = Writing { $path }
app-job-replay = Replaying session
app-job-script = Running script
app-job-find-tones = Finding tones
app-job-playback = Preparing playback
app-job-demodulator = Demodulator

## Status messages

app-ready = Ready
app-signal-gone = That signal no longer exists.
app-set-h-markers-first = Set the H markers around the line first.
app-sc-unmappable = Sc cannot be mapped through { $edit }: its output has no frequency relation to its input. Measure the carrier before the { $edit } step.
app-recorded = { $kind } = { $value } from { $node } ({ $tool })
app-recorded-uncertain = { $kind } = { $value } +/- { $uncertainty } from { $node } ({ $tool })
app-not-recorded = { $kind } not recorded: { $error }
app-mapped-onto = { $from }, mapped onto { $node }
app-session-save-running = A session save is already running
app-export-report-failed = Export report: { $error }
app-save-session-failed = Save session: { $error }
app-open-session-failed = Open session: { $error }
app-script-error = Error: { $error }
app-replay-skipped = { $what ->
    [markers] { $node } ({ $label }): saved markers could not be read and were skipped: { $error }
    [tones] { $node } ({ $label }): saved tones could not be read and were skipped: { $error }
    [grid] { $node } ({ $label }): saved tone grid could not be read and was skipped: { $error }
   *[adp] { $node } ({ $label }): saved ADP candidates could not be read and were skipped: { $error }
}
app-session-restored = Session restored
app-session-restored-problems = { $count ->
    [one] Session restored, 1 problem (see the window)
   *[other] Session restored, { $count } problems (see the window)
}
app-wrote-report = Wrote report { $path }
app-saved = Saved { $path }
app-saved-session = Saved session { $path }
app-saved-session-replaced = Saved session { $path } (the session it held was replaced)
app-cancel-too-late = { $name } finished before the cancel took effect; the result was kept.
app-job-cancelled = { $name }: cancelled
app-job-failed = { $name }: { $error }
app-playback-cancelled = Playback cancelled
app-derived-opened = { $label }: new signal { $node } opened in { $view }
app-derived-opened-mapped = { $label }: new signal { $node } opened in { $view }; markers moved to its axes
app-derived-opened-cleared = { $label }: new signal { $node } opened in { $view }; some markers were cleared: the new signal has a different time or frequency axis
app-derive-failed = { $label }: { $error }
app-measured-line = { $kind } = { $value } +/- { $uncertainty } from { $node } (measure line; { $level } dBFS, { $snr } dB above the band)
app-measured-line-mapped = { $kind } = { $value } +/- { $uncertainty } from { $node } (measure line; { $level } dBFS, { $snr } dB above the band), line at { $line } Hz mapped back through power { $power }
app-loaded = Loaded { $label }: { $samples } samples at { $rate } Hz
app-loaded-notes = Loaded { $label }: { $samples } samples at { $rate } Hz. { $notes }
app-script-no-signals = Script { $path } produced no signals
app-script-finished = Script { $path } finished
app-adp-none = ADP found no keying rate in the selection
app-tones-found = { $count ->
    [one] Found 1 tone on { $node }
   *[other] Found { $count } tones on { $node }
}
app-markers-restored = Markers restored on { $node }
app-closed-tab-stopped = { $count ->
    [one] Closed { $name }; its running { $jobs } job was stopped. Open the window again and run it again.
   *[other] Closed { $name }; its running { $jobs } jobs were stopped. Open the window again and run them again.
}
app-nothing-to-play = Nothing to play: open the sonogram or set the V markers.
app-find-tones-failed = Find tones: { $error }

## Confirmation before work is lost

app-confirm-open-title = Open session
app-confirm-open-question = Replace the current session?
app-confirm-replace = Replace
app-confirm-script-title = Run script
app-confirm-script-question = The script's signals replace the current session.
app-confirm-quit-title = Quit
app-confirm-quit-unsaved = Save the session before quitting?
app-confirm-quit-without-saving = Quit without saving
app-confirm-quit-writing = A file is still being written.
app-confirm-quit-anyway = Quit anyway
app-confirm-lost = { $signals ->
    [one] 1 signal
   *[other] { $signals } signals
} and { $measurements ->
    [one] 1 measurement
   *[other] { $measurements } measurements
} that are not saved will be closed. This cannot be undone.
app-confirm-write-running = { $name } is still running; quitting now abandons it.
app-confirm-save-first = Save session first
app-confirm-save-first-ask = Save session first...
app-confirm-wait = Wait
app-confirm-keep-open = Keep the app open
app-cancel = Cancel

## File dialogs

app-dialog-save-as = Save as { $format }
app-dialog-save-session-as = Save session as
app-filter-session = SigNexora session
app-dialog-export-report = Export analysis report
app-filter-report = Report
app-dialog-open-session = Open session
app-dialog-run-script = Run script
app-filter-rhai = Rhai script

## File menu

app-menu-file = File
app-menu-open-session = Open session...
    .hover = Its signals replace the current session
app-nothing-to-save = Nothing to save
app-menu-save-session = Save session
app-menu-save-session-ask = Save session...
app-menu-save-session-choose = Choose a file for the session
app-menu-save-session-to = Save to { $path }
app-menu-save-session-as = Save session as...
app-menu-export-report = Export analysis report
app-menu-report-html = HTML...
app-menu-report-markdown = Markdown...

## File menu extras and the WF popup menu

app-menu-run-script = Run script (Rhai)...
    .hover = Runs a script; its signals replace the current session
app-resynth = Resynthesize FSK/MFSK
    .hover = Regenerate a clean signal on the tones from Find tones, at Br
    .disabled = Find the tones (waveform Sum) and measure Br first.
app-menu-measure = Measure
app-measure-hint = Set the H markers around the line (and V markers for the span).
app-measure-br = Measure line as baud rate (Br)
app-measure-sc = Measure line as carrier (Sc)
    .hover = After power N the line is mapped back to the carrier (divided by N, shifts undone)
app-measure-adp = Estimate baud rate automatically (ADP)
    .hover = Uses the V-marker span, or the whole signal
app-measure-dt-disabled = Set the V markers first.

## Hints

app-no-signals-hint = Open a recording (File > Open file... ({ $shortcut }), or drop a file on the window).
app-open-in = Open...
    .hover = Open this signal in another view or tool

## Sonogram toolbar

app-sono-shear = Shear
    .hover = Step between FFT frames, as a fraction of the FFT size. A smaller step overlaps the frames more: finer time detail, slower to compute
app-sono-image = Image
    .hover = Dynamic range shown below the strongest component
app-sono-hres-hover = Sharpen spectral lines by frequency reassignment (helps find faint clock lines). Tones keep their level; noise reads about 2 to 6 dB lower than with HRes off (depending on the window), because it is shown per bin width.
app-sono-alc-hover = Normalise each column to its strongest component (display only)
app-sono-combine = Zoomed out
    .hover = How the FFT frames that share one image column are merged when zoomed out
app-sono-peak = Peak
    .hover = Zoomed out: keep short bursts visible
app-sono-mean = Mean
    .hover = Zoomed out: average, smoother noise

## Waveform toolbar

app-wave-show-q = Show Q
app-find-tones = Find tones
    .hover = Peaks of the histogram: tone frequencies on phase-detector output

## Zoom and playback

app-play = Play
    .hover = Play the V-marker span, or the visible span ({ $shortcut })
app-play-stop = Stop
    .hover = Stop playback ({ $shortcut })

## Child windows

app-tab-signal-gone = This signal no longer exists.

## Windows

app-replay-problems-title = Session restored with problems
app-script-output = Script output: { $path }
app-script-no-output = (no output)
# $lines: the lines kept; $dropped: the lines left out.
app-script-output-truncated = Output truncated after { $lines } lines; { $dropped } more lines were not kept.

## Closing a window, the sonogram toolbar

app-confirm-close-tab-title = Close window?
# $name: the window title; $jobs: the job names, joined.
app-confirm-close-tab-question = { $count ->
    [one] Closing { $name } stops its running { $jobs } job. Close anyway?
   *[other] Closing { $name } stops its running { $jobs } jobs. Close anyway?
}
app-confirm-close-and-stop = Close and stop
app-confirm-keep-tab = Keep open
# $prev, $next: the keys that show the previous and next signal.
app-select-signal-first = Bring a window to the front first ({ $prev } or { $next } shows the signals in turn).
app-sono-fft = SizeFFT
    .hover = FFT size: larger gives finer frequency resolution and coarser time resolution
app-sono-window = WinF
    .hover = Window function of each FFT (WinF in SA)
app-sono-colormap = Colours
    .hover = Colour map of the sonogram image
app-confirm-close-all-title = Close all windows
app-confirm-close-all-question = This removes every signal and measurement from the session.
app-confirm-close-all = Close all
app-confirm-close-all-stops = { $name } is still running; closing all windows stops it.
app-replay-session-skipped = { $what ->
    [windows] The saved window layout could not be read, so one window opens per result: { $error }
   *[counters] The saved name counters could not be read, so new names may repeat numbers in use: { $error }
}
app-replay-windows-missing = { $count ->
    [one] 1 saved window was not opened because its signal did not replay
   *[other] { $count } saved windows were not opened because their signals did not replay
}
app-drop-loader-open = The file dialog is open: finish or cancel it, then drop the file again
app-drop-first-only = { $count } files dropped; only the first, { $name }, is opened
app-sono-target = { $active ->
    [yes] Changes the active sonogram window.
   *[no] Changes the default for new sonogram windows.
}
app-edit-unrecordable = it could not be recorded in the session for replay ({ $error })
app-empty-open = Open a recording... ({ $key })
app-empty-drop = or drop a file here (File > Open file... also opens one).
app-empty-guide = The workflow guide ({ $key }) shows the analysis step by step.
# $action: what the chooser was for, e.g. "Open session"; $error: why it failed.
app-chooser-failed = { $action }: the file chooser could not be shown ({ $error }). Try again, or check that the system file dialog service (desktop portal) is running.

## The tools that record a measurement, by the id stored in the session.

measure-tool-adp = Auto define params (ADP)
measure-tool-adp-coarse = Auto define params, coarse (within half an FFT bin)
measure-tool-measure-line = Measurement of Freq (measure line)
measure-tool-v-markers = V markers (dT)
measure-tool-ofdm-adp = OFDM tool, Get Br (ADP)
measure-tool-script-record = Script, record()
