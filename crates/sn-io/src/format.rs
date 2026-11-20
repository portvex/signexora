//! Sample formats, channel selection and pre-load scaling.

/// How one sample is stored on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SampleFormat {
    /// Unsigned 8-bit, offset by 128 (standard WAV 8-bit).
    U8,
    /// Signed 8-bit (common for SDR captures).
    I8,
    /// Signed 16-bit.
    I16,
    /// Signed 24-bit, packed in 3 bytes.
    I24,
    /// Signed 32-bit.
    I32,
    /// IEEE 754 32-bit float.
    F32,
    /// IEEE 754 64-bit float.
    F64,
    /// G.711 A-law, 8-bit.
    ALaw,
    /// G.711 mu-law, 8-bit.
    MuLaw,
}

impl SampleFormat {
    /// Bytes per sample.
    pub fn bytes(self) -> usize {
        match self {
            Self::U8 | Self::I8 | Self::ALaw | Self::MuLaw => 1,
            Self::I16 => 2,
            Self::I24 => 3,
            Self::I32 | Self::F32 => 4,
            Self::F64 => 8,
        }
    }

    /// Bits per sample as shown to the user.
    pub fn bits(self) -> u16 {
        (self.bytes() * 8) as u16
    }

    /// True for floating-point formats.
    pub fn is_float(self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }

    /// Short name for the UI, for example `"16-bit PCM"`.
    pub fn label(self) -> &'static str {
        match self {
            Self::U8 => "8-bit PCM (unsigned)",
            Self::I8 => "8-bit PCM (signed)",
            Self::I16 => "16-bit PCM",
            Self::I24 => "24-bit PCM",
            Self::I32 => "32-bit PCM",
            Self::F32 => "32-bit float",
            Self::F64 => "64-bit float",
            Self::ALaw => "A-law",
            Self::MuLaw => "mu-law",
        }
    }

    /// Decode one sample from the start of `b` to roughly `-1.0..=1.0` (floats are
    /// passed through unscaled). Returns `None` when `b` is shorter than
    /// [`Self::bytes`].
    #[inline]
    pub fn decode(self, b: &[u8], endian: Endian) -> Option<f32> {
        let le = endian == Endian::Little;
        let v = match self {
            Self::U8 => (f32::from(*b.first()?) - 128.0) / 128.0,
            Self::I8 => f32::from(i8::from_ne_bytes(take::<1>(b)?)) / 128.0,
            Self::I16 => {
                let a = take::<2>(b)?;
                let v = if le {
                    i16::from_le_bytes(a)
                } else {
                    i16::from_be_bytes(a)
                };
                f32::from(v) / 32_768.0
            }
            Self::I24 => {
                let [x, y, z] = take::<3>(b)?;
                let (lo, mid, hi) = if le { (x, y, z) } else { (z, y, x) };
                // Place in the top 24 bits of an i32 to sign-extend.
                let v = i32::from_le_bytes([0, lo, mid, hi]) >> 8;
                v as f32 / 8_388_608.0
            }
            Self::I32 => {
                let a = take::<4>(b)?;
                let v = if le {
                    i32::from_le_bytes(a)
                } else {
                    i32::from_be_bytes(a)
                };
                v as f32 / 2_147_483_648.0
            }
            Self::F32 => {
                let a = take::<4>(b)?;
                if le {
                    f32::from_le_bytes(a)
                } else {
                    f32::from_be_bytes(a)
                }
            }
            Self::F64 => {
                let a = take::<8>(b)?;
                (if le {
                    f64::from_le_bytes(a)
                } else {
                    f64::from_be_bytes(a)
                }) as f32
            }
            Self::ALaw => f32::from(alaw_to_i16(*b.first()?)) / 32_768.0,
            Self::MuLaw => f32::from(mulaw_to_i16(*b.first()?)) / 32_768.0,
        };
        Some(v)
    }
}

/// The first `N` bytes of `b` as an array, or `None` if `b` is too short.
#[inline]
fn take<const N: usize>(b: &[u8]) -> Option<[u8; N]> {
    b.get(..N)?.try_into().ok()
}

/// Byte order of multi-byte samples.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum Endian {
    /// Least significant byte first (WAV, most SDR files).
    #[default]
    Little,
    /// Most significant byte first.
    Big,
}

/// G.711 A-law expansion.
fn alaw_to_i16(a: u8) -> i16 {
    let a = a ^ 0x55;
    let sign = a & 0x80;
    let exponent = i32::from((a >> 4) & 0x07);
    let mantissa = i32::from(a & 0x0F);
    let mut v = (mantissa << 4) + 8;
    if exponent != 0 {
        v = (v + 0x100) << (exponent - 1);
    }
    let v = v as i16;
    if sign != 0 { v } else { -v }
}

/// G.711 mu-law expansion.
fn mulaw_to_i16(u: u8) -> i16 {
    let u = !u;
    let sign = u & 0x80;
    let exponent = i32::from((u >> 4) & 0x07);
    let mantissa = i32::from(u & 0x0F);
    let v = (((mantissa << 3) + 0x84) << exponent) - 0x84;
    let v = v as i16;
    if sign != 0 { -v } else { v }
}

/// Which channels of the file make up the signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChannelSelection {
    /// One channel as a real signal (0 = first/left).
    Mono(u16),
    /// Two channels as a complex I/Q signal.
    IQ {
        /// Channel holding the in-phase (I) component.
        i: u16,
        /// Channel holding the quadrature (Q) component.
        q: u16,
    },
}

impl ChannelSelection {
    /// The usual choice for a file with `channels` channels: mono for one channel,
    /// I/Q for two, the first channel otherwise.
    pub fn default_for(channels: u16) -> Self {
        if channels == 2 {
            Self::IQ { i: 0, q: 1 }
        } else {
            Self::Mono(0)
        }
    }

    /// Highest channel index used.
    pub fn max_channel(self) -> u16 {
        match self {
            Self::Mono(c) => c,
            Self::IQ { i, q } => i.max(q),
        }
    }
}

/// Pre-load scaling, as in SA's loader (report 11.1).
///
/// With a clip threshold, samples are first limited to `-clip..=clip` and then
/// rescaled so the threshold maps to full scale. This rescues recordings where a few
/// impulses used up most of the range. `gain` is applied last.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Scaling {
    /// Linear gain applied after clipping.
    pub gain: f32,
    /// Clip threshold in full-scale units (`0 < clip`), or none.
    pub clip: Option<f32>,
}

impl Default for Scaling {
    fn default() -> Self {
        Self {
            gain: 1.0,
            clip: None,
        }
    }
}

impl Scaling {
    /// True when the scaling changes nothing.
    pub fn is_identity(&self) -> bool {
        self.clip.is_none() && (self.gain - 1.0).abs() < f32::EPSILON
    }

    /// Check that the gain is finite and the clip threshold, when set, is finite and
    /// above 0. [`open`](crate::open) calls this before loading anything.
    pub fn validate(&self) -> crate::Result<()> {
        if !self.gain.is_finite() {
            return Err(crate::Error::Unsupported(format!(
                "the gain must be a finite number, not {}",
                self.gain
            )));
        }
        if let Some(c) = self.clip
            && !(c.is_finite() && c > 0.0)
        {
            return Err(crate::Error::Unsupported(format!(
                "the clip threshold must be above 0 (got {c}); turn clipping off or \
                 choose a positive value"
            )));
        }
        Ok(())
    }

    /// Apply to one sample.
    #[inline]
    pub fn apply(&self, v: f32) -> f32 {
        let v = match self.clip {
            Some(c) if c > 0.0 => v.clamp(-c, c) / c,
            _ => v,
        };
        v * self.gain
    }
}

/// Where and how sample data is laid out in a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DataLayout {
    /// Sample format.
    pub format: SampleFormat,
    /// Byte order.
    pub endian: Endian,
    /// Interleaved channels per frame.
    pub channels: u16,
    /// Byte offset of the first sample.
    pub data_offset: u64,
    /// Length of the sample data in bytes.
    pub data_len: u64,
}

impl DataLayout {
    /// Bytes per frame (one sample of every channel).
    pub fn frame_bytes(&self) -> usize {
        self.format.bytes() * usize::from(self.channels)
    }

    /// Number of whole frames.
    pub fn frames(&self) -> u64 {
        match self.frame_bytes() {
            0 => 0,
            fb => self.data_len / fb as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_decoding_and_endianness() {
        assert!((SampleFormat::U8.decode(&[0], Endian::Little).unwrap() + 1.0).abs() < 1e-6);
        assert!(
            SampleFormat::U8
                .decode(&[128], Endian::Little)
                .unwrap()
                .abs()
                < 1e-6
        );
        assert!((SampleFormat::I8.decode(&[0x80], Endian::Little).unwrap() + 1.0).abs() < 1e-6);
        let half = 16_384_i16;
        let le = half.to_le_bytes();
        let be = half.to_be_bytes();
        assert!((SampleFormat::I16.decode(&le, Endian::Little).unwrap() - 0.5).abs() < 1e-6);
        assert!((SampleFormat::I16.decode(&be, Endian::Big).unwrap() - 0.5).abs() < 1e-6);
        // -0.5 in 24-bit = -4194304 = 0xC00000
        assert!(
            (SampleFormat::I24
                .decode(&[0x00, 0x00, 0xC0], Endian::Little)
                .unwrap()
                + 0.5)
                .abs()
                < 1e-6
        );
        assert!(
            (SampleFormat::I24
                .decode(&[0xC0, 0x00, 0x00], Endian::Big)
                .unwrap()
                + 0.5)
                .abs()
                < 1e-6
        );
        let q = (i32::MAX / 4).to_le_bytes();
        assert!((SampleFormat::I32.decode(&q, Endian::Little).unwrap() - 0.25).abs() < 1e-6);
        let f = 0.125_f64.to_be_bytes();
        assert!((SampleFormat::F64.decode(&f, Endian::Big).unwrap() - 0.125).abs() < 1e-9);
    }

    #[test]
    fn short_input_is_not_a_panic() {
        assert_eq!(SampleFormat::I24.decode(&[1, 2], Endian::Little), None);
        assert_eq!(SampleFormat::U8.decode(&[], Endian::Little), None);
    }

    #[test]
    fn g711_matches_reference_points() {
        // Reference values from the ITU-T G.711 tables.
        assert_eq!(alaw_to_i16(0xD5), 8);
        assert_eq!(alaw_to_i16(0x55), -8);
        assert_eq!(alaw_to_i16(0xAA), 32_256);
        assert_eq!(mulaw_to_i16(0xFF), 0);
        assert_eq!(mulaw_to_i16(0x80), 32_124);
        assert_eq!(mulaw_to_i16(0x00), -32_124);
    }

    #[test]
    fn scaling_clips_then_rescales() {
        let s = Scaling {
            gain: 1.0,
            clip: Some(0.25),
        };
        assert!((s.apply(0.9) - 1.0).abs() < 1e-6);
        assert!((s.apply(0.125) - 0.5).abs() < 1e-6);
        assert!(Scaling::default().is_identity());
    }

    #[test]
    fn scaling_rejects_bad_parameters() {
        assert!(Scaling::default().validate().is_ok());
        let ok = Scaling {
            gain: 0.5,
            clip: Some(0.1),
        };
        assert!(ok.validate().is_ok());
        for bad in [
            Scaling {
                gain: f32::NAN,
                clip: None,
            },
            Scaling {
                gain: f32::INFINITY,
                clip: None,
            },
            Scaling {
                gain: 1.0,
                clip: Some(0.0),
            },
            Scaling {
                gain: 1.0,
                clip: Some(-0.5),
            },
            Scaling {
                gain: 1.0,
                clip: Some(f32::NAN),
            },
        ] {
            assert!(bad.validate().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn layout_frames() {
        let l = DataLayout {
            format: SampleFormat::I16,
            endian: Endian::Little,
            channels: 2,
            data_offset: 44,
            data_len: 4002,
        };
        assert_eq!(l.frame_bytes(), 4);
        assert_eq!(l.frames(), 1000);
    }
}
