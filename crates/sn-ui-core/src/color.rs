//! A plain 8-bit RGBA colour, so palettes and colour maps need no UI toolkit.
//!
//! Colours are stored premultiplied by alpha, as GPU compositors (egui, GPUI)
//! blend them: a translucent white of alpha `a` is `(a, a, a, a)`.

/// An 8-bit colour with premultiplied alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rgba8 {
    /// Red, premultiplied by alpha.
    pub r: u8,
    /// Green, premultiplied by alpha.
    pub g: u8,
    /// Blue, premultiplied by alpha.
    pub b: u8,
    /// Alpha (255 is opaque).
    pub a: u8,
}

impl Rgba8 {
    /// Opaque black.
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    /// Opaque white.
    pub const WHITE: Self = Self::rgb(255, 255, 255);

    /// An opaque colour.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// An opaque colour from `0xRRGGBB`; bits above the low 24 are ignored.
    pub const fn from_rgb24(rgb: u32) -> Self {
        Self::rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
    }

    /// A colour whose channels are already premultiplied by `a`.
    pub const fn premultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// A colour from channels not yet multiplied by `a`, rounded exactly as
    /// egui's `Color32::from_rgba_unmultiplied_const`.
    pub const fn unmultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        match a {
            0 => Self::premultiplied(0, 0, 0, 0),
            255 => Self::rgb(r, g, b),
            a => Self::premultiplied(
                mul_frac_round(r, a),
                mul_frac_round(g, a),
                mul_frac_round(b, a),
                a,
            ),
        }
    }

    /// Translucent white of alpha `a`.
    pub const fn white(a: u8) -> Self {
        Self::premultiplied(a, a, a, a)
    }

    /// Translucent black of alpha `a`.
    pub const fn black(a: u8) -> Self {
        Self::premultiplied(0, 0, 0, a)
    }

    /// The colour as `[r, g, b, a]` (premultiplied).
    pub const fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Relative luminance (WCAG 2) of the colour's channels, 0 to 1.
    pub fn luminance(self) -> f64 {
        let lin = |v: u8| {
            let v = f64::from(v) / 255.0;
            if v <= 0.040_45 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(self.r) + 0.7152 * lin(self.g) + 0.0722 * lin(self.b)
    }

    /// This premultiplied colour drawn over opaque `bg` (`c + bg * (1 - a)`).
    #[must_use]
    pub fn over(self, bg: Self) -> Self {
        let k = 1.0 - f64::from(self.a) / 255.0;
        let mix = |a: u8, b: u8| (f64::from(a) + f64::from(b) * k).round().min(255.0) as u8;
        Self::rgb(mix(self.r, bg.r), mix(self.g, bg.g), mix(self.b, bg.b))
    }

    /// WCAG contrast ratio of this colour drawn over opaque `bg`; a translucent
    /// colour is composited first, so the ratio is that of what is on screen.
    pub fn contrast_over(self, bg: Self) -> f64 {
        let (la, lb) = (self.over(bg).luminance(), bg.luminance());
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }
}

/// `val * frac / 255`, rounded, in fixed point (as egui computes it).
const fn mul_frac_round(val: u8, frac: u8) -> u8 {
    let p = (val as u16) * (frac as u16) + 128;
    ((p + (p >> 8)) >> 8) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unmultiplied_rounds_to_the_nearest_value() {
        assert_eq!(Rgba8::unmultiplied(255, 255, 255, 128), Rgba8::white(128));
        assert_eq!(Rgba8::unmultiplied(10, 20, 30, 0), Rgba8::default());
        assert_eq!(Rgba8::unmultiplied(10, 20, 30, 255), Rgba8::rgb(10, 20, 30));
        for v in 0..=255_u8 {
            for a in 1..=254_u8 {
                let want = (f64::from(v) * f64::from(a) / 255.0).round();
                let got = f64::from(Rgba8::unmultiplied(v, 0, 0, a).r);
                assert!((got - want).abs() <= 1.0, "{v} {a}: {got} vs {want}");
            }
        }
    }

    #[test]
    fn from_rgb24_splits_the_channels() {
        assert_eq!(Rgba8::from_rgb24(0xFF12_3456), Rgba8::rgb(0x12, 0x34, 0x56));
    }

    #[test]
    fn contrast_of_black_on_white_is_21() {
        let r = Rgba8::BLACK.contrast_over(Rgba8::WHITE);
        assert!((r - 21.0).abs() < 1e-9, "{r}");
        assert_eq!(
            Rgba8::black(128).over(Rgba8::WHITE),
            Rgba8::rgb(127, 127, 127)
        );
    }
}
