//! Audio playback of a time span (SA: "the part of the recording selected by the V
//! markers"). SA's author notes this is for voice signals and orientation; analysis
//! happens on the screen.

use std::num::NonZero;

use rodio::buffer::SamplesBuffer;
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player};
use sn_core::Signal;
use sn_core::job::JobError;

use crate::t;

/// Longest span played at once, in seconds. Above about 28 kHz [`MAX_SAMPLES`] is
/// reached first and limits the span to fewer seconds.
const MAX_SECONDS: f64 = 600.0;
/// Most samples read for one playback (64 MB of f32), whatever the rate.
const MAX_SAMPLES: usize = 16 * 1024 * 1024;
/// Samples read between cancellation checks (4 MB of f32).
const READ_CHUNK: usize = 1024 * 1024;
/// Highest sample rate sent to the audio device unchanged.
const MAX_RATE: f64 = 384_000.0;

/// Audio ready to hand to the device, prepared off the UI thread.
#[derive(Debug)]
pub struct Prepared {
    buf: Vec<f32>,
    rate: NonZero<u32>,
    /// The playback limit in seconds at this rate, when the requested span was longer.
    truncated: Option<f64>,
    /// The span came from the V markers (rather than the visible view).
    from_markers: bool,
    /// Samples that were not numbers (NaN or infinite), played as silence.
    bad: usize,
}

/// Samples `a..b` to play for a request of `start..end` on a signal of `len` samples
/// at `fs`, and, when the span had to be shortened, the limit that applied in seconds.
pub fn play_span(
    fs: f64,
    len: usize,
    start: f64,
    end: f64,
) -> Result<(usize, usize, Option<f64>), String> {
    if !(fs.is_finite() && fs > 0.0) {
        return Err(t!("playback-rate-too-low"));
    }
    if fs > MAX_RATE {
        return Err(t!("playback-rate-too-high", rate = format!("{fs}")));
    }
    let a = start.max(0.0).floor() as usize;
    let want = (end.max(0.0).ceil() as usize).min(len);
    if want <= a {
        return Err(t!("playback-nothing"));
    }
    let max_len = ((MAX_SECONDS * fs) as usize).clamp(1, MAX_SAMPLES);
    let b = want.min(a.saturating_add(max_len));
    let limit = (b < want).then(|| max_len as f64 / fs);
    Ok((a, b, limit))
}

/// Read and normalise samples `start..end` of `signal` (the real or I part). Runs in
/// a background job: it reads from disk for on-demand signals, in chunks, checking
/// `should_stop` between them. `name` names the signal in messages; `from_markers`
/// says whether the span came from the V markers, for the advice when it is cut short.
pub fn prepare(
    signal: &Signal,
    start: f64,
    end: f64,
    name: &str,
    from_markers: bool,
    should_stop: &dyn Fn() -> bool,
) -> Result<Prepared, JobError> {
    let fs = signal.sample_rate();
    let (a, b, truncated) = play_span(fs, signal.len(), start, end).map_err(JobError::Failed)?;
    let rate = NonZero::new(fs.round() as u32)
        .ok_or_else(|| JobError::Failed(t!("playback-rate-too-low")))?;
    let mut buf = vec![0.0_f32; b - a];
    let mut got = 0;
    for chunk in buf.chunks_mut(READ_CHUNK) {
        if should_stop() {
            return Err(JobError::Cancelled);
        }
        let want = chunk.len();
        let n = signal.read_real(a + got, chunk);
        got += n;
        if n < want {
            break;
        }
    }
    if got < buf.len() {
        return Err(JobError::Failed(t!(
            "playback-short-read",
            got = format!("{:.2}", got as f64 / fs),
            total = format!("{:.2}", buf.len() as f64 / fs),
            name = name
        )));
    }
    let bad = silence_non_finite(&mut buf);
    // Normalise so quiet recordings are audible, without clipping.
    let peak = buf.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
    if peak > 0.0 {
        let g = 0.9 / peak;
        for v in &mut buf {
            *v *= g;
        }
    }
    Ok(Prepared {
        buf,
        rate,
        truncated,
        from_markers,
        bad,
    })
}

/// Replace samples that are not numbers (NaN or infinite) with silence, so they
/// neither reach the device nor upset the normalisation. Returns how many.
fn silence_non_finite(buf: &mut [f32]) -> usize {
    let mut bad = 0;
    for v in buf.iter_mut().filter(|v| !v.is_finite()) {
        *v = 0.0;
        bad += 1;
    }
    bad
}

/// Lazily opened audio output.
#[derive(Default)]
pub struct Playback {
    sink: Option<MixerDeviceSink>,
    player: Option<Player>,
}

impl std::fmt::Debug for Playback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Playback")
            .field("sink", &self.sink.as_ref().map(|_| "open"))
            .field("player", &self.player.as_ref().map(|p| !p.empty()))
            .finish()
    }
}

impl Playback {
    /// True while audio is queued.
    pub fn is_playing(&self) -> bool {
        self.player.as_ref().is_some_and(|p| !p.empty())
    }

    /// Stop playback and close the output, so the next playback opens the
    /// system's current default device (it may have been unplugged or changed).
    pub fn stop(&mut self) {
        if let Some(p) = self.player.take() {
            p.stop();
        }
        self.sink = None;
    }

    /// Play prepared audio. Returns a message for the status bar.
    pub fn start(&mut self, p: Prepared) -> Result<String, String> {
        self.stop();
        if self.sink.is_none() {
            let mut sink = DeviceSinkBuilder::open_default_sink()
                .map_err(|e| t!("playback-no-output", error = e.to_string()))?;
            // Closing it is intended (see `stop`), not a mistake to log.
            sink.log_on_drop(false);
            self.sink = Some(sink);
        }
        let Some(sink) = &self.sink else {
            return Err(t!("playback-output-unavailable"));
        };
        let player = Player::connect_new(sink.mixer());
        let seconds = p.buf.len() as f64 / f64::from(p.rate.get());
        player.append(SamplesBuffer::new(NonZero::<u16>::MIN, p.rate, p.buf));
        self.player = Some(player);
        let message = start_message(seconds, p.truncated, p.from_markers);
        Ok(with_bad_note(message, p.bad))
    }
}

/// Status text for playback of `seconds` of audio, cut to `truncated` seconds if set.
/// `from_markers` says whether the span was set with the V markers or is the view.
fn start_message(seconds: f64, truncated: Option<f64>, from_markers: bool) -> String {
    if let Some(limit) = truncated {
        let key = if from_markers {
            "playback-playing-first-markers"
        } else {
            "playback-playing-first-view"
        };
        // Both values use the same precision so a 349.5 s limit does not read as 350.
        t!(
            key,
            seconds = format!("{seconds:.1}"),
            limit = format!("{limit:.1}")
        )
    } else {
        t!("playback-playing", seconds = format!("{seconds:.2}"))
    }
}

/// `message` with a note on `bad` samples played as silence, if any.
fn with_bad_note(message: String, bad: usize) -> String {
    if bad == 0 {
        return message;
    }
    let note = t!("playback-bad-samples", count = bad.to_string());
    let stop = if message.ends_with('.') { "" } else { "." };
    format!("{message}{stop} {note}")
}

impl Prepared {
    /// A short silent buffer, for tests of code that handles prepared audio
    /// (also the front ends' tests).
    #[doc(hidden)]
    pub fn silent_for_test() -> Self {
        Self {
            buf: vec![0.0; 8],
            rate: NonZero::<u32>::MIN,
            truncated: None,
            from_markers: false,
            bad: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncated_message_rounds_alike_and_says_to_move_v1() {
        let m = start_message(349.5, Some(349.5), true);
        assert!(m.contains("first 349.5 s"), "{m}");
        assert!(m.contains("limited to 349.5 s"), "{m}");
        assert!(
            m.ends_with("move V1 to the end of this part to hear the rest."),
            "{m}"
        );
        let v = start_message(349.5, Some(349.5), false);
        assert!(!v.contains("V1"), "{v}");
        assert!(
            v.ends_with("set the V markers on the rest to hear it."),
            "{v}"
        );
        assert_eq!(start_message(1.0, None, false), "Playing 1.00 s");
    }

    #[test]
    fn span_is_clamped_and_capped() {
        assert_eq!(play_span(8000.0, 100, -5.0, 50.2), Ok((0, 51, None)));
        assert_eq!(play_span(8000.0, 100, 10.0, 1e9), Ok((10, 100, None)));
        assert!(play_span(8000.0, 100, 50.0, 50.0).is_err());
        assert!(play_span(1e6, 100, 0.0, 10.0).is_err());
        assert!(play_span(0.0, 100, 0.0, 10.0).is_err());
        // 600 s at 48 kHz (28.8 M samples) is above the sample cap, so the cap applies.
        let len = 100_000_000;
        assert_eq!(
            play_span(48_000.0, len, 0.0, 1e12),
            Ok((0, MAX_SAMPLES, Some(MAX_SAMPLES as f64 / 48_000.0)))
        );
        // At 8 kHz the 600 s limit comes first.
        assert_eq!(
            play_span(8000.0, len, 0.0, 1e12),
            Ok((0, 4_800_000, Some(600.0)))
        );
        let (a, b, cut) = play_span(384_000.0, len, 1000.0, 9e7).unwrap();
        assert_eq!((a, b - a), (1000, MAX_SAMPLES));
        assert!(cut.is_some());
    }

    #[test]
    fn non_finite_samples_play_as_silence() {
        let s = Signal::real(
            vec![0.5_f32, f32::NAN, f32::INFINITY, -0.25, f32::NEG_INFINITY],
            8000.0,
        )
        .unwrap();
        let p = prepare(&s, 0.0, 5.0, "n0", true, &|| false).unwrap();
        assert_eq!(p.bad, 3);
        assert!(p.buf.iter().all(|v| v.is_finite()));
        assert!((p.buf[0] - 0.9).abs() < 1e-6, "{:?}", p.buf);
        assert!((p.buf[3] + 0.45).abs() < 1e-6);
        assert_eq!(p.buf[1], 0.0);
        let m = with_bad_note("Playing 1.00 s".to_owned(), 3);
        assert!(m.starts_with("Playing 1.00 s. ") && m.contains('3'), "{m}");
        assert_eq!(with_bad_note("x".to_owned(), 0), "x");
    }

    /// A lazy source that claims more samples than it can deliver.
    #[derive(Debug)]
    struct Short;

    impl sn_core::SampleSource for Short {
        fn len(&self) -> usize {
            1000
        }
        fn kind(&self) -> sn_core::SampleKind {
            sn_core::SampleKind::Real
        }
        fn read_complex(&self, _start: usize, out: &mut [sn_core::Complex32]) -> usize {
            out.len().min(10)
        }
        fn read_real(&self, _start: usize, out: &mut [f32]) -> usize {
            out.len().min(10)
        }
    }

    #[test]
    fn short_read_is_an_error() {
        let s = Signal::lazy(std::sync::Arc::new(Short), 8000.0).unwrap();
        let never = || false;
        let e = prepare(&s, 0.0, 500.0, "n0", true, &never)
            .unwrap_err()
            .to_string();
        assert!(e.contains("only 0.00 s of the 0.06 s span of n0"), "{e}");
        let ok = Signal::real(vec![0.5_f32; 500], 8000.0).unwrap();
        assert!(prepare(&ok, 0.0, 500.0, "n0", true, &never).is_ok());
        let stop = || true;
        assert!(matches!(
            prepare(&ok, 0.0, 500.0, "n0", true, &stop),
            Err(JobError::Cancelled)
        ));
    }
}
