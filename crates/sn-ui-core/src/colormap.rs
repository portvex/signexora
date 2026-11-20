//! Sonogram colour maps as 256-entry lookup tables.

use crate::color::Rgba8;
use crate::t;

/// Available colour maps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Colormap {
    /// SA-like: dark blue through cyan and yellow to red.
    #[default]
    Classic,
    /// Black to white.
    Gray,
    /// Perceptually uniform blue-green-yellow.
    Viridis,
    /// Perceptually uniform black-purple-orange-yellow.
    Inferno,
}

impl Colormap {
    /// Every colour map, in menu order.
    pub const ALL: [Colormap; 4] = [Self::Classic, Self::Gray, Self::Viridis, Self::Inferno];

    /// Name shown in the colour map menu. Viridis and Inferno are proper names.
    pub fn label(self) -> String {
        match self {
            Self::Classic => t!("colormap-classic"),
            Self::Gray => t!("colormap-grayscale"),
            Self::Viridis => "Viridis".to_owned(),
            Self::Inferno => "Inferno".to_owned(),
        }
    }

    fn stops(self) -> &'static [(f32, [u8; 3])] {
        match self {
            // SA's jet-like map: a dark navy floor through blue, cyan, green and
            // yellow to red and magenta for the strongest components.
            Self::Classic => &[
                (0.00, [0, 0, 24]),
                (0.25, [0, 40, 200]),
                (0.45, [0, 200, 230]),
                (0.60, [40, 220, 60]),
                (0.75, [250, 240, 30]),
                (0.90, [250, 40, 20]),
                (1.00, [220, 0, 160]),
            ],
            Self::Gray => &[(0.0, [0, 0, 0]), (1.0, [255, 255, 255])],
            Self::Viridis => &[
                (0.00, [68, 1, 84]),
                (0.25, [59, 82, 139]),
                (0.50, [33, 145, 140]),
                (0.75, [94, 201, 98]),
                (1.00, [253, 231, 37]),
            ],
            Self::Inferno => &[
                (0.00, [0, 0, 4]),
                (0.25, [87, 16, 110]),
                (0.50, [188, 55, 84]),
                (0.75, [249, 142, 9]),
                (1.00, [252, 255, 164]),
            ],
        }
    }

    /// 256 colours from the lowest to the highest level.
    pub fn lut(self) -> Vec<Rgba8> {
        let stops = self.stops();
        (0..256_u16)
            .map(|i| {
                let x = f32::from(i) / 255.0;
                let upper = stops
                    .iter()
                    .position(|(p, _)| *p >= x)
                    .unwrap_or(stops.len().saturating_sub(1));
                let (p1, c1) = stops.get(upper).copied().unwrap_or((1.0, [255, 255, 255]));
                let (p0, c0) = stops
                    .get(upper.saturating_sub(1))
                    .copied()
                    .unwrap_or((p1, c1));
                let t = if p1 > p0 { (x - p0) / (p1 - p0) } else { 0.0 };
                let mix =
                    |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
                Rgba8::rgb(mix(c0[0], c1[0]), mix(c0[1], c1[1]), mix(c0[2], c1[2]))
            })
            .collect()
    }
}

/// Colour for areas outside the recording in the themes other than Classic (SA): a
/// dark red-brown that is the low end of no colour map. The Classic theme draws them
/// in SA's deep navy (the front end's window chrome).
pub const OUTSIDE: Rgba8 = Rgba8::rgb(52, 18, 14);
/// Colour for columns that are still being computed.
pub const PENDING: Rgba8 = Rgba8::rgb(28, 30, 36);
/// Default of the SigNexora colour map that can replace SA's VMW colours (the choice
/// is remembered in the preferences).
pub const RASTER_DEFAULT: Colormap = Colormap::Gray;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luts_span_their_stops() {
        for c in Colormap::ALL {
            let l = c.lut();
            assert_eq!(l.len(), 256);
            let [r, g, b] = c.stops()[0].1;
            assert_eq!(l[0], Rgba8::rgb(r, g, b), "{c:?} start");
            let [r, g, b] = c.stops()[c.stops().len() - 1].1;
            assert_eq!(l[255], Rgba8::rgb(r, g, b), "{c:?} end");
        }
        assert_eq!(Colormap::Gray.lut()[128], Rgba8::rgb(128, 128, 128));
    }

    #[test]
    fn outside_differs_from_every_low_end() {
        for c in Colormap::ALL {
            for v in c.lut().iter().take(64) {
                let d = (i32::from(v.r) - i32::from(OUTSIDE.r)).abs()
                    + (i32::from(v.g) - i32::from(OUTSIDE.g)).abs()
                    + (i32::from(v.b) - i32::from(OUTSIDE.b)).abs();
                assert!(d > 30, "{c:?} {v:?} too close to OUTSIDE");
            }
        }
    }
}
