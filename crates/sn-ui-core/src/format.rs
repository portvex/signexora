//! Text formatting for readouts and axes.

/// SA-style time: `MM:SS.mmm.uuu` (minutes, seconds, milliseconds, microseconds).
/// Negative values get a leading minus sign; hours roll into minutes.
pub fn time_long(seconds: f64) -> String {
    if !seconds.is_finite() {
        return "--".to_owned();
    }
    let sign = if seconds < 0.0 { "-" } else { "" };
    let total_us = (seconds.abs() * 1e6).round() as u64;
    let us = total_us % 1000;
    let ms = (total_us / 1000) % 1000;
    let s = (total_us / 1_000_000) % 60;
    let m = total_us / 60_000_000;
    format!("{sign}{m:02}:{s:02}.{ms:03}.{us:03}")
}

/// Frequency with units: Hz below 10 kHz, kHz below 10 MHz, otherwise MHz, with
/// the interface language's decimal separator.
pub fn freq(hz: f64) -> String {
    freq_with(hz, crate::i18n::decimal_separator())
}

fn freq_with(hz: f64, separator: char) -> String {
    if !hz.is_finite() {
        return "--".to_owned();
    }
    let a = hz.abs();
    let text = if a >= 10e6 {
        format!("{:.6} MHz", hz / 1e6)
    } else if a >= 10e3 {
        format!("{:.4} kHz", hz / 1e3)
    } else {
        format!("{hz:.2} Hz")
    };
    with_separator(&text, separator)
}

/// A rate in Bd / Hz with enough digits for baud-rate work, with the interface
/// language's decimal separator.
pub fn rate(hz: f64) -> String {
    rate_with(hz, crate::i18n::decimal_separator())
}

fn rate_with(hz: f64, separator: char) -> String {
    if hz.is_finite() {
        with_separator(&format!("{hz:.3}"), separator)
    } else {
        "--".to_owned()
    }
}

/// "1,234,567" style grouping for sample counts.
pub fn count(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// SA-style short time: `MM:SS.mmm` (the Info overlay's t1 and t2).
pub fn time_short(seconds: f64) -> String {
    if !seconds.is_finite() {
        return "--".to_owned();
    }
    let sign = if seconds < 0.0 { "-" } else { "" };
    let total_ms = (seconds.abs() * 1e3).round() as u64;
    let ms = total_ms % 1000;
    let s = (total_ms / 1000) % 60;
    let m = total_ms / 60_000;
    format!("{sign}{m:02}:{s:02}.{ms:03}")
}

/// `v` with `decimals` decimals and the decimal separator of the interface
/// language, as SA showed numbers (comma in a Russian Windows).
pub fn dec(v: f64, decimals: usize) -> String {
    dec_with(v, decimals, crate::i18n::decimal_separator())
}

/// [`dec`] with an explicit separator.
fn dec_with(v: f64, decimals: usize, separator: char) -> String {
    let text = format!("{v:.decimals$}");
    // `-0.001` with 2 decimals is "-0.00": show it as zero, as `dec_trim` does.
    let text = match text.strip_prefix('-') {
        Some(rest) if rest.chars().all(|c| c == '0' || c == '.') => rest.to_owned(),
        _ => text,
    };
    with_separator(&text, separator)
}

/// `v` rounded to at most `decimals` decimals, trailing zeros dropped (SA's
/// `Fc = 3963,5`, `Fu = 6411`).
pub fn dec_trim(v: f64, decimals: usize) -> String {
    dec_trim_with(v, decimals, crate::i18n::decimal_separator())
}

/// [`dec_trim`] with an explicit separator.
fn dec_trim_with(v: f64, decimals: usize, separator: char) -> String {
    let text = format!("{v:.decimals$}");
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        text
    };
    let text = if text == "-0" { "0".to_owned() } else { text };
    with_separator(&text, separator)
}

/// `text` (formatted with a point) with `separator` as the decimal separator.
fn with_separator(text: &str, separator: char) -> String {
    if separator == '.' {
        text.to_owned()
    } else {
        text.replacen('.', &separator.to_string(), 1)
    }
}

/// Read a number typed by the user: a comma is accepted as the decimal separator
/// (SA 6.2.6.5 takes `7777,77`), surrounding spaces are ignored.
pub fn parse_number(text: &str) -> Option<f64> {
    let t = text.trim().replace(',', ".");
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_time_matches_sa_format() {
        assert_eq!(time_long(0.0), "00:00.000.000");
        assert_eq!(time_long(18.751_999), "00:18.751.999");
        assert_eq!(time_long(83.000_25), "01:23.000.250");
        assert_eq!(time_long(-0.5), "-00:00.500.000");
        assert_eq!(time_long(f64::NAN), "--");
    }

    #[test]
    fn ticks_and_frequencies() {
        assert_eq!(freq_with(1800.284, '.'), "1800.28 Hz");
        assert_eq!(freq_with(433_920_000.0, '.'), "433.920000 MHz");
        assert_eq!(freq_with(48_000.0, '.'), "48.0000 kHz");
        assert_eq!(count(1_234_567), "1,234,567");
        assert_eq!(count(12), "12");
        assert_eq!(rate_with(2399.756, '.'), "2399.756");
    }

    #[test]
    fn sa_numbers_and_times() {
        assert_eq!(time_short(57.664), "00:57.664");
        assert_eq!(time_short(0.0), "00:00.000");
        assert_eq!(time_short(83.5), "01:23.500");
        assert_eq!(with_separator("3963.5", ','), "3963,5");
        assert_eq!(with_separator("3963.5", '.'), "3963.5");
        assert_eq!(dec_with(-0.001, 2, '.'), "0.00");
        assert_eq!(dec_with(-1.5, 1, '.'), "-1.5");
        assert_eq!(dec_trim(6411.2, 0), "6411");
        assert_eq!(dec_trim(-0.01, 0), "0");
        assert_eq!(parse_number(" 7777,77 "), Some(7777.77));
        assert_eq!(parse_number("8000"), Some(8000.0));
        assert_eq!(parse_number("-150.5"), Some(-150.5));
        assert_eq!(parse_number("abc"), None);
        assert_eq!(parse_number("inf"), None);
    }

    #[test]
    fn numbers_follow_the_language_separator() {
        use crate::i18n::{Lang, decimal_separator_of};
        let ru = decimal_separator_of(Lang::Ru);
        assert_eq!(ru, ',');
        assert_eq!(decimal_separator_of(Lang::En), '.');
        assert_eq!(dec_with(3963.5, 1, ru), "3963,5");
        assert_eq!(dec_trim_with(3963.50, 2, ru), "3963,5");
        // Frequencies and rates use the same separator as the other numbers.
        assert_eq!(freq_with(1800.284, ru), "1800,28 Hz");
        assert_eq!(freq_with(48_000.0, ru), "48,0000 kHz");
        assert_eq!(rate_with(2399.756, ru), "2399,756");
        assert_eq!(
            dec_with(3963.5, 1, decimal_separator_of(Lang::En)),
            "3963.5"
        );
    }
}
