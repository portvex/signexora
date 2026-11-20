//! Output forms for demodulated symbols: SA's 32-character map, "x"-separated hex,
//! bits through a selectable symbol-to-bits map, and the APCO-25 helpers from
//! article 412 (dibit maps and inversion detection from long runs).

use std::fmt::Write as _;

use super::{Result, invalid};

/// SA's character map for orders up to 32: symbol `k` is the `k`-th character.
pub const SYMBOL_CHARS: &[u8; 32] = b"0123456789ABCDEFGHIJKLMNOPQRSTUV";

/// Symbols as characters from [`SYMBOL_CHARS`]. Fails for symbols above 31; use
/// [`symbols_to_hex`] for larger orders.
pub fn symbols_to_chars(symbols: &[u8]) -> Result<String> {
    symbols
        .iter()
        .map(|s| {
            SYMBOL_CHARS
                .get(usize::from(*s))
                .map(|c| char::from(*c))
                .ok_or_else(|| {
                    invalid(
                        "format",
                        format!(
                            "symbol {s} does not fit the 32-character map; use the hex output for more than 32 levels"
                        ),
                    )
                })
        })
        .collect()
}

/// Symbols as two-digit hex bytes separated by "x", e.g. `03x00x1F` (SA's hex form).
pub fn symbols_to_hex(symbols: &[u8]) -> String {
    let mut out = String::with_capacity(symbols.len() * 3);
    for (i, s) in symbols.iter().enumerate() {
        if i > 0 {
            out.push('x');
        }
        // Writing to a String cannot fail.
        let _ = write!(out, "{s:02X}");
    }
    out
}

/// Map from level index (0 = lowest level) to a bit group, for orders that are
/// powers of two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitMap {
    bits: u32,
    table: Vec<u16>,
}

impl BitMap {
    fn check_order(order: usize) -> Result<u32> {
        if !(2..=super::MAX_LEVELS).contains(&order) || !order.is_power_of_two() {
            return Err(invalid(
                "order",
                format!(
                    "bit output needs a power-of-two number of levels (2, 4, 8 ... 256), got {order}; use the character or hex output instead"
                ),
            ));
        }
        Ok(order.trailing_zeros())
    }

    /// Natural binary: level `k` carries the bits of `k`.
    pub fn natural(order: usize) -> Result<Self> {
        let bits = Self::check_order(order)?;
        Ok(Self {
            bits,
            table: (0..order).map(|k| k as u16).collect(),
        })
    }

    /// Gray code: adjacent levels differ in one bit.
    pub fn gray(order: usize) -> Result<Self> {
        let bits = Self::check_order(order)?;
        Ok(Self {
            bits,
            table: (0..order).map(|k| (k ^ (k >> 1)) as u16).collect(),
        })
    }

    /// A custom map: `table[k]` is the bit group of level `k`. The table length must
    /// be a power of two and hold every group exactly once.
    pub fn custom(table: Vec<u16>) -> Result<Self> {
        let order = table.len();
        let bits = Self::check_order(order)?;
        let mut seen = vec![false; order];
        for v in &table {
            match seen.get_mut(usize::from(*v)) {
                Some(s) if !*s => *s = true,
                _ => {
                    return Err(invalid(
                        "map",
                        format!(
                            "the map must use every value 0..{order} exactly once; {v} is repeated or too large"
                        ),
                    ));
                }
            }
        }
        Ok(Self { bits, table })
    }

    /// APCO-25 C4FM on frequency levels, lowest first: -1800 Hz = 11, -600 = 10,
    /// +600 = 00, +1800 = 01. This is the map for this demodulator's level order.
    pub fn c4fm() -> Self {
        Self {
            bits: 2,
            table: vec![0b11, 0b10, 0b00, 0b01],
        }
    }

    /// [`BitMap::c4fm`] for a spectrum-inverted signal (levels mirrored).
    pub fn c4fm_inverted() -> Self {
        Self {
            bits: 2,
            table: vec![0b01, 0b00, 0b10, 0b11],
        }
    }

    /// Article 412's table for APCO-25 demodulated in SA's relative (phase) mode:
    /// positions 0..3 carry dibits 10, 00, 01, 11.
    ///
    /// The positions are relative-phase angles (-45, +45, +135, -135 degrees), not
    /// frequency levels. Frequency level `L` (lowest first) is position
    /// [`c4fm_level_to_relative_position`]`(L)`; for this demodulator's output use
    /// [`BitMap::c4fm`], which gives the same dibits.
    pub fn apco25_article412() -> Self {
        Self {
            bits: 2,
            table: vec![0b10, 0b00, 0b01, 0b11],
        }
    }

    /// Article 412's table for an inverted signal: positions 0..3 carry 00, 10, 11, 01.
    pub fn apco25_article412_inverted() -> Self {
        Self {
            bits: 2,
            table: vec![0b00, 0b10, 0b11, 0b01],
        }
    }

    /// Bits per symbol.
    pub fn bits_per_symbol(&self) -> u32 {
        self.bits
    }

    /// Number of levels.
    pub fn order(&self) -> usize {
        self.table.len()
    }

    /// Bit group of level `k`.
    pub fn code(&self, k: usize) -> Option<u16> {
        self.table.get(k).copied()
    }
}

/// SA relative-mode position (article 412 numbering) of C4FM frequency level
/// `level` (0 = -1800 Hz): -1800 Hz is -135 degrees per symbol, position 3; -600 Hz
/// position 0; +600 Hz position 1; +1800 Hz position 2.
pub fn c4fm_level_to_relative_position(level: u8) -> u8 {
    (level.wrapping_add(3)) & 3
}

/// Symbols to bits (one `u8` 0 or 1 per bit, most significant bit of each group
/// first).
pub fn symbols_to_bits(symbols: &[u8], map: &BitMap) -> Result<Vec<u8>> {
    let bits = map.bits_per_symbol();
    let mut out = Vec::with_capacity(symbols.len() * bits as usize);
    for s in symbols {
        let code = map.code(usize::from(*s)).ok_or_else(|| {
            invalid(
                "map",
                format!(
                    "symbol {s} is outside the {}-level bit map; choose a map with as many levels as the demodulator",
                    map.order()
                ),
            )
        })?;
        for i in (0..bits).rev() {
            out.push(((code >> i) & 1) as u8);
        }
    }
    Ok(out)
}

/// Bits as '0'/'1' text with a space after every `group` bits (0 = no spaces).
pub fn bits_to_string(bits: &[u8], group: usize) -> String {
    let mut out = String::with_capacity(bits.len() * 2);
    for (i, b) in bits.iter().enumerate() {
        if group > 0 && i > 0 && i % group == 0 {
            out.push(' ');
        }
        out.push(if *b == 0 { '0' } else { '1' });
    }
    out
}

/// Long runs of one symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunCensus {
    /// The repeated symbol.
    pub symbol: u8,
    /// Number of runs at least `min_run` long.
    pub runs: usize,
    /// Symbols inside those runs.
    pub symbols_in_runs: usize,
    /// Longest run of this symbol.
    pub longest: usize,
}

/// Census of runs of at least `min_run` identical symbols, one entry per symbol that
/// has such a run, most symbols-in-runs first.
pub fn long_runs(symbols: &[u8], min_run: usize) -> Vec<RunCensus> {
    let mut table: Vec<RunCensus> = Vec::new();
    let mut add = |s: u8, len: usize| {
        if len < min_run.max(1) {
            return;
        }
        if let Some(e) = table.iter_mut().find(|e| e.symbol == s) {
            e.runs += 1;
            e.symbols_in_runs += len;
            e.longest = e.longest.max(len);
        } else {
            table.push(RunCensus {
                symbol: s,
                runs: 1,
                symbols_in_runs: len,
                longest: len,
            });
        }
    };
    let mut iter = symbols.iter();
    if let Some(first) = iter.next() {
        let (mut cur, mut len) = (*first, 1_usize);
        for s in iter {
            if *s == cur {
                len += 1;
            } else {
                add(cur, len);
                cur = *s;
                len = 1;
            }
        }
        add(cur, len);
    }
    table.sort_by_key(|e| std::cmp::Reverse(e.symbols_in_runs));
    table
}

/// Polarity of a demodulated stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// As transmitted.
    Normal,
    /// Spectrum inverted: use the inverted bit map.
    Inverted,
    /// Not enough evidence either way.
    Unknown,
}

/// Outcome of an inversion check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InversionReport {
    /// Verdict.
    pub polarity: Polarity,
    /// Symbols in long runs of the symbol expected for a normal stream.
    pub normal_evidence: usize,
    /// Symbols in long runs of the symbol expected for an inverted stream.
    pub inverted_evidence: usize,
}

/// Decide polarity from long runs (article 412): a stream that always holds long
/// runs of one known symbol shows them on `normal_symbol` when upright and on
/// `inverted_symbol` when inverted. Needs at least twice as much evidence for one
/// side as for the other.
pub fn detect_inversion(
    symbols: &[u8],
    normal_symbol: u8,
    inverted_symbol: u8,
    min_run: usize,
) -> InversionReport {
    let census = long_runs(symbols, min_run);
    let count = |s: u8| {
        census
            .iter()
            .find(|e| e.symbol == s)
            .map_or(0, |e| e.symbols_in_runs)
    };
    let n = count(normal_symbol);
    let i = count(inverted_symbol);
    let polarity = if n > 0 && n >= 2 * i {
        Polarity::Normal
    } else if i > 0 && i >= 2 * n {
        Polarity::Inverted
    } else {
        Polarity::Unknown
    };
    InversionReport {
        polarity,
        normal_evidence: n,
        inverted_evidence: i,
    }
}

/// Shortest run counted by [`detect_c4fm_inversion`]: article 412 says APCO-25 always
/// holds runs of at least 15-20 zero bits, that is 7-10 dibits 00.
pub const C4FM_MIN_RUN: usize = 7;

/// APCO-25 inversion check on 4-level output (frequency levels, lowest first). Runs of
/// dibit 00 sit on level 2 (+600 Hz) when upright and on level 1 when inverted.
/// In article 412's relative-mode numbering these are positions 1 and 0.
pub fn detect_c4fm_inversion(symbols: &[u8]) -> InversionReport {
    detect_inversion(symbols, 2, 1, C4FM_MIN_RUN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chars_and_hex() {
        assert_eq!(symbols_to_chars(&[0, 9, 10, 31]).unwrap(), "09AV");
        assert!(symbols_to_chars(&[32]).is_err());
        assert_eq!(symbols_to_hex(&[3, 0, 31, 255]), "03x00x1FxFF");
        assert_eq!(symbols_to_hex(&[]), "");
    }

    #[test]
    fn bit_maps() {
        let n = BitMap::natural(8).unwrap();
        assert_eq!(
            bits_to_string(&symbols_to_bits(&[5, 0], &n).unwrap(), 3),
            "101 000"
        );
        let g = BitMap::gray(4).unwrap();
        assert_eq!(
            bits_to_string(&symbols_to_bits(&[0, 1, 2, 3], &g).unwrap(), 2),
            "00 01 11 10"
        );
        assert!(BitMap::natural(6).is_err());
        assert!(BitMap::custom(vec![0, 1, 1, 3]).is_err());
        assert!(BitMap::custom(vec![3, 2, 1, 0]).is_ok());
        assert!(symbols_to_bits(&[4], &g).is_err());
    }

    #[test]
    fn apco25_maps_agree_with_article_412() {
        let a = BitMap::apco25_article412();
        let ai = BitMap::apco25_article412_inverted();
        let c = BitMap::c4fm();
        let ci = BitMap::c4fm_inverted();
        assert_eq!(
            (0..4).map(|k| a.code(k).unwrap()).collect::<Vec<_>>(),
            vec![0b10, 0b00, 0b01, 0b11]
        );
        assert_eq!(
            (0..4).map(|k| ai.code(k).unwrap()).collect::<Vec<_>>(),
            vec![0b00, 0b10, 0b11, 0b01]
        );
        // Frequency level L and relative position p(L) carry the same dibit, upright
        // and inverted.
        for level in 0..4_u8 {
            let p = usize::from(c4fm_level_to_relative_position(level));
            assert_eq!(c.code(level.into()), a.code(p));
            assert_eq!(ci.code(level.into()), ai.code(p));
        }
    }

    #[test]
    fn runs_and_inversion() {
        let mut s = vec![0_u8, 1, 3, 2, 0];
        s.extend(std::iter::repeat_n(2, 9));
        s.extend([1, 0, 3]);
        s.extend(std::iter::repeat_n(2, 8));
        let census = long_runs(&s, 7);
        assert_eq!(census.len(), 1);
        assert_eq!(census[0].symbol, 2);
        assert_eq!(census[0].runs, 2);
        assert_eq!(census[0].longest, 9);
        assert_eq!(detect_c4fm_inversion(&s).polarity, Polarity::Normal);
        let inv: Vec<u8> = s.iter().map(|x| 3 - x).collect();
        assert_eq!(detect_c4fm_inversion(&inv).polarity, Polarity::Inverted);
        assert_eq!(
            detect_c4fm_inversion(&[0, 1, 2, 3]).polarity,
            Polarity::Unknown
        );
    }
}
