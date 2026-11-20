use sn_core::{Complex32, SampleKind, Signal};
use sn_dsp::fir::Fir;

use crate::{Error, Result, invalid};

/// Largest span analysed at once, in samples.
pub const MAX_SAMPLES: usize = 1 << 24;

/// A span of a recording as complex samples with the DC removed: the input of every
/// OFDM tool. The removed mean is kept ([`Baseband::dc`]): the CP search works on the
/// DC-free samples, and the channel tools ([`crate::lu_spectrum`],
/// [`crate::occupancy`], [`crate::channel_symbols`]) add it back, so a channel that
/// sits at DC keeps its true values.
#[derive(Debug, Clone)]
pub struct Baseband {
    samples: Vec<Complex32>,
    sample_rate: f64,
    origin: usize,
    dc: Complex32,
}

impl Baseband {
    /// Take samples `start..end` of `signal`. I/Q is used as is; real recordings pass
    /// through an analytic filter (positive frequencies only). The mean is removed,
    /// because a DC offset disturbs the CP correlation (forum tip 14.3); it is kept in
    /// [`Baseband::dc`] for the channel tools.
    pub fn from_signal(
        signal: &Signal,
        start: usize,
        end: usize,
        should_stop: &(dyn Fn() -> bool + Sync),
    ) -> Result<Self> {
        let end = end.min(signal.len());
        let n = end.saturating_sub(start);
        if n < 16 {
            return Err(invalid(
                "the selection holds fewer than 16 samples; select a span that covers \
                 many OFDM symbols",
            ));
        }
        if n > MAX_SAMPLES {
            return Err(invalid(format!(
                "the selection holds {n} samples; select at most {MAX_SAMPLES} \
                 (or resample to a lower rate first)"
            )));
        }
        let fs = signal.sample_rate();
        let samples = match signal.kind() {
            SampleKind::Complex => {
                let mut v = vec![Complex32::new(0.0, 0.0); n];
                let got = signal.read_complex(start, &mut v);
                if got != n {
                    return Err(Error::Invalid(format!(
                        "only {got} of {n} samples could be read from the recording; check \
                         that the file is still readable"
                    )));
                }
                v
            }
            SampleKind::Real => Fir::analytic((fs / 500.0).max(1.0), 70.0, fs)?.apply(
                signal,
                start,
                end,
                should_stop,
            )?,
        };
        let mut b = Self::from_samples(samples, fs)?;
        b.origin = start;
        b.remove_mean();
        Ok(b)
    }

    /// Wrap complex samples as they are (no filtering, no DC removal).
    pub fn from_samples(samples: Vec<Complex32>, sample_rate: f64) -> Result<Self> {
        if !(sample_rate.is_finite() && sample_rate > 0.0) {
            return Err(invalid(format!(
                "sample rate {sample_rate} Hz is invalid; it must be a finite number above 0"
            )));
        }
        if samples.len() > MAX_SAMPLES {
            return Err(invalid(format!(
                "{} samples is too many; use at most {MAX_SAMPLES}",
                samples.len()
            )));
        }
        Ok(Self {
            samples,
            sample_rate,
            origin: 0,
            dc: Complex32::new(0.0, 0.0),
        })
    }

    fn remove_mean(&mut self) {
        if self.samples.is_empty() {
            return;
        }
        let n = self.samples.len() as f64;
        let (re, im) = self.samples.iter().fold((0.0, 0.0), |(r, i), v| {
            (r + f64::from(v.re), i + f64::from(v.im))
        });
        let mean = Complex32::new((re / n) as f32, (im / n) as f32);
        for v in &mut self.samples {
            *v -= mean;
        }
        self.dc += mean;
    }

    /// The mean removed from the samples (zero for [`Baseband::from_samples`]). Add it
    /// back to get the recording's own values.
    pub fn dc(&self) -> Complex32 {
        self.dc
    }

    /// The samples.
    pub fn samples(&self) -> &[Complex32] {
        &self.samples
    }

    /// Sample rate in Hz.
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    /// Index in the source signal of sample 0.
    pub fn origin(&self) -> usize {
        self.origin
    }

    /// Number of samples.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// True when there are no samples.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// The samples (DC removed) as a complex [`Signal`] at the same rate.
    pub fn to_signal(&self) -> Result<Signal> {
        Ok(Signal::complex(self.samples.clone(), self.sample_rate)?)
    }
}
