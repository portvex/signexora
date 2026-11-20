//! The standard marks of Windows 95, as [`Glyph`]s drawn pixel by pixel: check
//! marks, arrows, caption-button symbols, the size grip, the message-box icons
//! (the SigNexora brand mark is rendered separately from its PNG asset).

use super::colors::Role;
use super::glyph::{BLACK, Glyph, Ink};

/// The 7x7 check mark of check boxes and menus.
pub(crate) fn check() -> Glyph {
    Glyph::new(7, 7).bitmap(
        0,
        0,
        &[
            "      #", "     ##", "#   ###", "## ### ", "#####  ", " ###   ", "  #    ",
        ],
        BLACK,
    )
}

/// The 6x6 dot of a chosen radio menu item.
pub(crate) fn menu_dot() -> Glyph {
    Glyph::new(6, 6).bitmap(
        0,
        0,
        &[" #### ", "######", "######", "######", "######", " #### "],
        BLACK,
    )
}

/// Direction of an arrow mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dir {
    Up,
    Down,
    Left,
    Right,
}

/// The solid triangle of scroll bar, combo box and spin buttons: `size` is the
/// length of its base (7 for scroll bars, 5 for spin buttons), odd.
pub(crate) fn arrow(dir: Dir, size: i32) -> Glyph {
    let rows = (size + 1) / 2;
    let mut g = match dir {
        Dir::Up | Dir::Down => Glyph::new(size, rows),
        Dir::Left | Dir::Right => Glyph::new(rows, size),
    };
    for i in 0..rows {
        let len = size - 2 * i;
        g = match dir {
            Dir::Down => g.rect(i, i, len, 1, BLACK),
            Dir::Up => g.rect(i, rows - 1 - i, len, 1, BLACK),
            Dir::Right => g.rect(i, i, 1, len, BLACK),
            Dir::Left => g.rect(rows - 1 - i, i, 1, len, BLACK),
        };
    }
    g
}

/// The 4x7 arrow of a menu item that opens a submenu.
pub(crate) fn submenu_arrow() -> Glyph {
    arrow(Dir::Right, 7)
}

/// The close mark of a caption button (8x7).
pub(crate) fn caption_close() -> Glyph {
    Glyph::new(8, 7).bitmap(
        0,
        0,
        &[
            "##    ##", " ##  ## ", "  ####  ", "   ##   ", "  ####  ", " ##  ## ", "##    ##",
        ],
        BLACK,
    )
}

/// The minimize mark of a caption button (6x2, at the bottom).
pub(crate) fn caption_minimize() -> Glyph {
    Glyph::new(6, 2).rect(0, 0, 6, 2, BLACK)
}

/// The maximize mark of a caption button (9x9 window).
pub(crate) fn caption_maximize() -> Glyph {
    Glyph::new(9, 9)
        .rect(0, 0, 9, 2, BLACK)
        .rect(0, 2, 1, 7, BLACK)
        .rect(8, 2, 1, 7, BLACK)
        .rect(1, 8, 7, 1, BLACK)
}

/// The restore mark of a caption button (two overlapping windows, 8x9).
pub(crate) fn caption_restore() -> Glyph {
    Glyph::new(8, 9)
        .rect(2, 0, 6, 2, BLACK)
        .rect(7, 2, 1, 4, BLACK)
        .rect(6, 5, 1, 1, BLACK)
        .rect(2, 2, 1, 1, BLACK)
        .rect(0, 3, 6, 2, BLACK)
        .rect(0, 5, 1, 4, BLACK)
        .rect(5, 5, 1, 4, BLACK)
        .rect(1, 8, 4, 1, BLACK)
}

/// The size grip of a status bar (12x12, at the bottom-right corner).
pub(crate) fn size_grip() -> Glyph {
    let mut g = Glyph::new(12, 12);
    for k in [0, 4, 8] {
        let hi = Ink::Role(Role::Highlight);
        let sh = Ink::Role(Role::Shadow);
        g = g
            .line(k, 11, 11, k, hi)
            .line(k + 1, 11, 11, k + 1, sh)
            .line(k + 2, 11, 11, k + 2, sh);
    }
    g
}

/// The message-box icon kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Icon {
    App,
    Error,
    Warning,
    Info,
    Question,
}

/// A 32x32 system glyph; the application mark uses its PNG instead.
pub(crate) fn icon(kind: Icon) -> Option<Glyph> {
    Some(match kind {
        Icon::App => return None,
        Icon::Error => error_icon(),
        Icon::Warning => warning_icon(),
        Icon::Info => balloon().bitmap_scaled(
            14,
            5,
            2,
            &["##", "##", "  ", "##", "##", "##", "##", "##", "##"],
            Ink::Rgb(0x00_00_FF),
        ),
        Icon::Question => balloon().bitmap_scaled(
            10,
            4,
            2,
            &[
                " #### ", "##  ##", "    ##", "   ## ", "  ##  ", "  ##  ", "      ", "  ##  ",
            ],
            Ink::Rgb(0x00_00_FF),
        ),
    })
}

fn error_icon() -> Glyph {
    let white = Ink::Rgb(0xFF_FF_FF);
    Glyph::new(32, 32)
        .circle(17.0, 17.0, 14.0, Ink::Rgb(0x80_80_80))
        .circle(15.0, 15.0, 14.0, BLACK)
        .circle(15.0, 15.0, 13.0, Ink::Rgb(0xFF_00_00))
        .poly(
            &[(8.0, 10.0), (10.0, 8.0), (22.0, 20.0), (20.0, 22.0)],
            white,
        )
        .poly(
            &[(20.0, 8.0), (22.0, 10.0), (10.0, 22.0), (8.0, 20.0)],
            white,
        )
}

fn warning_icon() -> Glyph {
    Glyph::new(32, 32)
        .poly(
            &[(17.0, 3.0), (32.0, 30.0), (2.0, 30.0)],
            Ink::Rgb(0x80_80_80),
        )
        .poly(&[(15.0, 1.0), (30.0, 28.0), (0.0, 28.0)], BLACK)
        .poly(
            &[(15.0, 3.5), (28.0, 27.0), (2.0, 27.0)],
            Ink::Rgb(0xFF_FF_00),
        )
        .rect(14, 9, 3, 10, BLACK)
        .rect(14, 21, 3, 3, BLACK)
}

/// The white speech balloon of the information and question icons.
fn balloon() -> Glyph {
    let white = Ink::Rgb(0xFF_FF_FF);
    Glyph::new(32, 32)
        .circle(17.0, 15.0, 13.0, Ink::Rgb(0x80_80_80))
        .poly(&[(9.0, 22.0), (15.0, 26.0), (6.0, 31.0)], BLACK)
        .circle(15.0, 13.0, 13.0, BLACK)
        .circle(15.0, 13.0, 12.0, white)
        .poly(&[(10.0, 22.0), (14.0, 24.0), (8.0, 28.0)], white)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::win95::glyph::Op;

    fn pixel_count(g: &Glyph) -> i32 {
        g.ops
            .iter()
            .map(|op| match op {
                Op::Rect(r, _) => r.w * r.h,
                _ => 0,
            })
            .sum()
    }

    #[test]
    fn arrows_are_triangles() {
        let down = arrow(Dir::Down, 7);
        assert_eq!((down.w, down.h), (7, 4));
        assert_eq!(pixel_count(&down), 7 + 5 + 3 + 1);
        let right = arrow(Dir::Right, 7);
        assert_eq!((right.w, right.h), (4, 7));
        assert_eq!(pixel_count(&arrow(Dir::Up, 5)), 5 + 3 + 1);
        assert_eq!(pixel_count(&arrow(Dir::Left, 5)), 9);
    }

    #[test]
    fn marks_have_their_size() {
        assert_eq!(pixel_count(&check()), 1 + 2 + 4 + 5 + 5 + 3 + 1);
        assert_eq!(caption_close().w, 8);
        assert_eq!(pixel_count(&caption_minimize()), 12);
        for k in [Icon::Error, Icon::Warning, Icon::Info, Icon::Question] {
            assert_eq!(icon(k).unwrap().w, 32);
        }
    }
}
