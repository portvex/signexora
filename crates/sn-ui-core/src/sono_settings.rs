//! The sonogram toolbar settings (SA's top toolbar): FFT size, Shear, window,
//! dynamic range, ALC, colour map, column combining and HRes.

use sn_dsp::spectrum::Combine;
use sn_dsp::window::WindowKind;

use crate::colormap::Colormap;
use crate::t;

/// SA's "Shear": hop between FFT frames as a fraction of the FFT size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Shear {
    /// Hop = FFT size (no overlap). SA's default.
    #[default]
    Size,
    /// Hop = size / 2.
    Half,
    /// Hop = size / 4.
    Quarter,
    /// Hop = size / 8.
    Eighth,
    /// Hop = size / 16.
    Sixteenth,
}

impl Shear {
    /// Every choice, in menu order.
    pub const ALL: [Shear; 5] = [
        Self::Size,
        Self::Half,
        Self::Quarter,
        Self::Eighth,
        Self::Sixteenth,
    ];

    /// Frame size divided by the hop.
    pub fn divisor(self) -> usize {
        match self {
            Self::Size => 1,
            Self::Half => 2,
            Self::Quarter => 4,
            Self::Eighth => 8,
            Self::Sixteenth => 16,
        }
    }

    /// Name shown in the Shear menu.
    pub fn label(self) -> String {
        match self {
            Self::Size => t!("sono-shear-size"),
            Self::Half | Self::Quarter | Self::Eighth | Self::Sixteenth => {
                t!("sono-shear-fraction", divisor = self.divisor().to_string())
            }
        }
    }
}

/// FFT sizes offered in the toolbar.
pub const FFT_SIZES: [usize; 13] = [
    64, 128, 256, 512, 1024, 2048, 4096, 8192, 16_384, 32_768, 65_536, 131_072, 262_144,
];

/// User settings of one sonogram (SA's top toolbar). Remembered across runs (see
/// [`crate::prefs`]); missing fields take their defaults.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    /// SA "`SizeFFT`" (default 1024).
    pub fft_size: usize,
    /// SA "Shear".
    pub shear: Shear,
    /// SA "`WinF`" (default Kaiser, b=8).
    pub window: WindowKind,
    /// SA "Image": dynamic range below the reference level, dB (default 50).
    pub range_db: f32,
    /// SA "ALC": normalise each column to its strongest bin (display only).
    pub alc: bool,
    /// Colour map.
    pub colormap: Colormap,
    /// How frames inside one pixel column are combined.
    pub combine: Combine,
    /// SA "HRes": sharper lines by frequency reassignment.
    pub hres: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            fft_size: 1024,
            shear: Shear::Size,
            window: WindowKind::default(),
            range_db: 50.0,
            alc: false,
            colormap: Colormap::Classic,
            combine: Combine::PeakHold,
            hres: false,
        }
    }
}

impl Settings {
    /// Samples between FFT frames.
    pub fn hop(&self) -> usize {
        (self.fft_size / self.shear.divisor()).max(1)
    }
}
