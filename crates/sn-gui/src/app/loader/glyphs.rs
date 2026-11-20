//! The 16x16 pictures of `SA Open file`: the list's file and folder icons and
//! the toolbar's back, folder up, view mode and refresh buttons.

use crate::win95::glyph::{BLACK, Glyph, Ink};

const YELLOW: Ink = Ink::Rgb(0xFF_FF_00);
const DARK_YELLOW: Ink = Ink::Rgb(0x80_80_00);
const GREEN: Ink = Ink::Rgb(0x00_A0_00);
const DARK_GREEN: Ink = Ink::Rgb(0x00_60_00);
const WHITE: Ink = Ink::Rgb(0xFF_FF_FF);
const BLUE: Ink = Ink::Rgb(0x00_40_FF);
const RED: Ink = Ink::Rgb(0xFF_00_00);

/// A page with a folded corner and a coloured band (green: wav).
pub(crate) fn file_icon(wav: bool) -> Glyph {
    let band = if wav { GREEN } else { BLUE };
    Glyph::new(16, 16)
        .poly(
            &[
                (3.0, 1.0),
                (10.0, 1.0),
                (13.0, 4.0),
                (13.0, 15.0),
                (3.0, 15.0),
            ],
            BLACK,
        )
        .poly(
            &[
                (4.0, 2.0),
                (9.5, 2.0),
                (12.0, 4.5),
                (12.0, 14.0),
                (4.0, 14.0),
            ],
            WHITE,
        )
        .rect(9, 2, 1, 3, BLACK)
        .rect(9, 4, 3, 1, BLACK)
        .rect(5, 7, 6, 2, band)
        .rect(5, 10, 6, 2, band)
}

/// A yellow folder; `link` adds SA's red arrow (`F-Link`).
pub(crate) fn folder_icon(link: bool) -> Glyph {
    let mut g = Glyph::new(16, 16)
        .rect(1, 3, 6, 2, BLACK)
        .rect(2, 4, 4, 1, YELLOW)
        .rect(1, 4, 14, 11, BLACK)
        .rect(2, 5, 12, 9, YELLOW)
        .rect(2, 13, 12, 1, DARK_YELLOW)
        .rect(13, 5, 1, 9, DARK_YELLOW);
    if link {
        g = g
            .poly(&[(4.0, 12.0), (9.0, 7.0), (10.0, 8.0), (5.0, 13.0)], RED)
            .poly(&[(7.0, 6.0), (11.0, 6.0), (11.0, 10.0)], RED);
    }
    g
}

/// SA's green back arrow.
pub(crate) fn back() -> Glyph {
    Glyph::new(16, 16)
        .poly(
            &[
                (1.0, 8.0),
                (7.0, 2.0),
                (7.0, 5.0),
                (14.0, 5.0),
                (14.0, 11.0),
                (7.0, 11.0),
                (7.0, 14.0),
            ],
            DARK_GREEN,
        )
        .poly(
            &[
                (2.5, 8.0),
                (6.0, 4.5),
                (6.0, 6.0),
                (13.0, 6.0),
                (13.0, 10.0),
                (6.0, 10.0),
                (6.0, 11.5),
            ],
            GREEN,
        )
}

/// A yellow folder with a green up arrow.
pub(crate) fn folder_up() -> Glyph {
    folder_icon(false)
        .poly(
            &[
                (9.0, 2.0),
                (14.0, 7.0),
                (11.0, 7.0),
                (11.0, 11.0),
                (7.0, 11.0),
                (7.0, 7.0),
                (4.0, 7.0),
            ],
            DARK_GREEN,
        )
        .poly(
            &[
                (9.0, 3.5),
                (12.0, 6.5),
                (10.0, 6.5),
                (10.0, 10.0),
                (8.0, 10.0),
                (8.0, 6.5),
                (6.0, 6.5),
            ],
            GREEN,
        )
}

/// The view mode button: list (columns of small rows) or details (rows with
/// columns).
pub(crate) fn view_mode(details: bool) -> Glyph {
    let mut g = Glyph::new(16, 16);
    if details {
        for y in [2, 6, 10] {
            g = g.rect(1, y, 3, 3, BLUE).rect(5, y + 1, 10, 1, BLACK);
        }
    } else {
        for x in [1, 9] {
            for y in [2, 7, 12] {
                g = g.rect(x, y, 2, 2, BLUE).rect(x + 3, y, 3, 1, BLACK);
            }
        }
    }
    g
}

/// SA's green refresh arrows (6.2.6).
pub(crate) fn refresh() -> Glyph {
    Glyph::new(16, 16)
        .circle(8.0, 8.0, 6.5, GREEN)
        .circle(8.0, 8.0, 4.0, Ink::Role(crate::win95::colors::Role::Face))
        .rect(1, 7, 7, 2, Ink::Role(crate::win95::colors::Role::Face))
        .poly(&[(0.0, 7.0), (6.0, 7.0), (3.0, 11.0)], GREEN)
        .rect(8, 7, 7, 2, Ink::Role(crate::win95::colors::Role::Face))
        .poly(&[(10.0, 9.0), (16.0, 9.0), (13.0, 5.0)], GREEN)
}
