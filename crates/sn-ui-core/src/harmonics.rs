//! SA's harmonics (power-of-N) module (`SA_UI_SPEC.md` section 5, M-p12,
//! M-p13): the controls of a `(q)` window's bottom toolbar, independent of the
//! UI toolkit (moved from `sa-app`'s `harmonics_window.rs`, which keeps its own
//! copy until the egui app is removed).
//!
//! The toolbar keeps the zoom buttons, the progress field and Stop, then `<`
//! and `<<` (carrier down by 1 and 10 Hz), the carrier field, `>>` and `>`,
//! `St^n` (compute with the current settings), `<`, the power field (2 to
//! start), `>`, and the `Control` check box that draws a marker line at the
//! carrier.

use crate::format;

/// Powers the module offers.
pub const POWERS: std::ops::RangeInclusive<u32> = 2..=32;

/// The module's controls.
#[derive(Debug, Clone, PartialEq)]
pub struct HarmonicsState {
    /// The carrier field (whole Hz).
    pub carrier: String,
    /// The power in use.
    pub power: u32,
    /// The power field as typed; committed to `power` on a valid value, and
    /// rewritten when the buttons change `power`.
    pub power_text: String,
    /// The Control check box.
    pub control: bool,
    /// `St^n` was pressed for this power: its result replaces this window's signal.
    pub pending: Option<u32>,
    /// Where the Control line goes on this window's frequency axis, Hz (set by the
    /// app from the carrier each frame).
    pub marker_hz: Option<f64>,
}

impl Default for HarmonicsState {
    /// The lowest power and a 0 Hz carrier, keeping the 2..=32 invariant.
    fn default() -> Self {
        Self::new(*POWERS.start(), 0.0)
    }
}

impl HarmonicsState {
    /// The controls of a window showing power `power`, carrier `carrier` Hz.
    pub fn new(power: u32, carrier: f64) -> Self {
        let power = power.clamp(*POWERS.start(), *POWERS.end());
        Self {
            carrier: format!("{:.0}", carrier.max(0.0)),
            power,
            power_text: power.to_string(),
            control: false,
            pending: None,
            marker_hz: None,
        }
    }

    /// The carrier in Hz, if the field holds a number.
    pub fn carrier_hz(&self) -> Option<f64> {
        format::parse_number(&self.carrier)
    }

    /// Move the carrier by `hz` (the `<`, `<<`, `>>`, `>` buttons), in whole Hz.
    /// A field that is not a number is left as typed (the field shows it is
    /// invalid) rather than silently replaced. Returns true when the value
    /// changed (false for a field that is not a number, or a carrier held at 0).
    pub fn step_carrier(&mut self, hz: i64) -> bool {
        let Some(now) = self.carrier_hz() else {
            return false;
        };
        let new = (now.round() as i64).saturating_add(hz).max(0);
        self.carrier = new.to_string();
        (new as f64 - now).abs() > 0.0
    }

    /// Move the power by `d`, within 2..=32.
    pub fn step_power(&mut self, d: i32) {
        let p = i64::from(self.power) + i64::from(d);
        self.power = p.clamp(i64::from(*POWERS.start()), i64::from(*POWERS.end())) as u32;
        self.power_text = self.power.to_string();
    }

    /// Take the typed power if it is a whole number in 2..=32. Returns false
    /// (leaving `power` unchanged) otherwise.
    pub fn commit_power_text(&mut self) -> bool {
        match self.power_text.trim().parse::<u32>() {
            Ok(p) if POWERS.contains(&p) => {
                self.power = p;
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carrier_steps_by_whole_hertz_and_power_stays_in_range() {
        let mut h = HarmonicsState::new(2, 1701.4);
        assert_eq!(h.carrier, "1701");
        h.step_carrier(-1);
        assert_eq!(h.carrier, "1700");
        h.step_carrier(10);
        assert_eq!(h.carrier, "1710");
        h.carrier = "abc".to_owned();
        assert!(!h.step_carrier(1));
        assert_eq!(h.carrier, "abc", "a typo is not replaced");
        h.carrier = "5".to_owned();
        assert!(h.step_carrier(-10));
        assert_eq!(h.carrier, "0", "never below 0 Hz");
        assert!(!h.step_carrier(-10), "held at 0 Hz: it did not move");
        let d = HarmonicsState::default();
        assert_eq!((d.power, d.power_text.as_str()), (2, "2"));
        h.step_power(-1);
        assert_eq!(h.power, 2, "2 at least");
        h.step_power(3);
        assert_eq!(h.power, 5);
        h.power = 32;
        h.step_power(1);
        assert_eq!(h.power, 32);
        assert_eq!(h.power_text, "32");
        // Typing 16: the "1" on the way is kept, not thrown away.
        h.power_text = "1".to_owned();
        assert!(!h.commit_power_text());
        assert_eq!(h.power, 32);
        h.power_text = "16".to_owned();
        assert!(h.commit_power_text());
        assert_eq!(h.power, 16);
        assert_eq!(HarmonicsState::new(99, 0.0).power, 32);
    }
}
