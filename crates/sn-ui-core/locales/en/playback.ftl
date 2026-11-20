# Audio playback of a time span: errors and status messages.

playback-rate-too-low = The sample rate is too low to play.
# $rate: the sample rate in Hz.
playback-rate-too-high = Playback is for audio-rate signals; this one is sampled at { $rate } Hz. Shift the band of interest down and resample it first.
playback-nothing = Nothing to play: select a time span with the V markers.
# $got and $total: seconds; $name: the signal, e.g. "n3".
playback-short-read = Could read only { $got } s of the { $total } s span of { $name }; the file may have changed on disk. Reopen the recording.
# $error: the audio system's error text.
playback-no-output = Cannot open the audio output: { $error }
playback-output-unavailable = The audio output is not available.
# $seconds: the length played.
playback-playing = Playing { $seconds } s
# $seconds: the length played; $limit: the playback limit at this rate, in seconds.
playback-playing-first-markers = Playing the first { $seconds } s only (playback is limited to { $limit } s at this rate); move V1 to the end of this part to hear the rest.
playback-playing-first-view = Playing the first { $seconds } s only (playback is limited to { $limit } s at this rate); set the V markers on the rest to hear it.
playback-bad-samples = { $count } samples that were not numbers play as silence.
