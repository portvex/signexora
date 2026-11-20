use std::collections::HashMap;
use std::fmt;

use crate::{Error, NodeId, Result};

/// The named quantities SA carried between tools (report section 6), plus custom ones.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MeasurementKind {
    /// Keying rate (baud), Hz. Pre-fills PhasePlane, filters and demodulators.
    BaudRate,
    /// Carrier frequency, Hz. Pre-fills PhasePlane.
    Carrier,
    /// A measured time span, seconds. Pre-fills the VMW line period.
    Period,
    /// Tone or OFDM channel spacing, Hz.
    Spacing,
    /// OFDM ratio K = LG / LU.
    OfdmK,
    /// Anything else, by name.
    Custom(String),
}

impl fmt::Display for MeasurementKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BaudRate => f.write_str("Br"),
            Self::Carrier => f.write_str("Sc"),
            Self::Period => f.write_str("dT"),
            Self::Spacing => f.write_str("Sh"),
            Self::OfdmK => f.write_str("K"),
            Self::Custom(name) => f.write_str(name),
        }
    }
}

/// One measured value with where it came from.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Measurement {
    /// The value, in the unit implied by the kind (Hz, s, or dimensionless).
    pub value: f64,
    /// One-sigma uncertainty, when the estimator provides one.
    pub uncertainty: Option<f64>,
    /// Signal node the measurement was taken on.
    pub source: Option<NodeId>,
    /// Tool that produced it, e.g. `"measure_line_frequency"`.
    pub tool: String,
}

/// Session-wide store of the latest measurement of each kind, with full history.
///
/// Tools read [`MeasurementContext::latest`] to pre-fill their parameters, and the UI
/// shows the provenance so pre-filled values are never a mystery.
#[derive(Debug, Default)]
pub struct MeasurementContext {
    history: HashMap<MeasurementKind, Vec<Measurement>>,
    /// Recording order across kinds: the kind and the index in its history.
    order: Vec<(MeasurementKind, usize)>,
}

impl MeasurementContext {
    /// Empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new measurement; it becomes the latest of its kind.
    ///
    /// A NaN or infinite value or uncertainty is rejected and not stored, so tools are
    /// never pre-filled from a degenerate estimate.
    pub fn record(&mut self, kind: MeasurementKind, measurement: Measurement) -> Result<()> {
        let bad = if measurement.value.is_finite() {
            measurement
                .uncertainty
                .filter(|u| !u.is_finite())
                .map(|u| format!("uncertainty {u}"))
        } else {
            Some(measurement.value.to_string())
        };
        if let Some(value) = bad {
            return Err(Error::InvalidMeasurement {
                kind: kind.to_string(),
                tool: measurement.tool,
                value,
            });
        }
        let list = self.history.entry(kind.clone()).or_default();
        self.order.push((kind, list.len()));
        list.push(measurement);
        Ok(())
    }

    /// Every measurement of every kind, in the order they were recorded.
    pub fn iter(&self) -> impl Iterator<Item = (&MeasurementKind, &Measurement)> {
        self.order.iter().filter_map(|(kind, i)| {
            self.history
                .get(kind)
                .and_then(|v| v.get(*i))
                .map(|m| (kind, m))
        })
    }

    /// Total number of measurements recorded.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// True when nothing has been measured.
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// The latest measurement of a kind.
    pub fn latest(&self, kind: &MeasurementKind) -> Option<&Measurement> {
        self.history.get(kind).and_then(|v| v.last())
    }

    /// The latest value of a kind, or `default` when nothing was measured yet.
    pub fn value_or(&self, kind: &MeasurementKind, default: f64) -> f64 {
        self.latest(kind).map_or(default, |m| m.value)
    }

    /// Every measurement of a kind, oldest first.
    pub fn history(&self, kind: &MeasurementKind) -> &[Measurement] {
        self.history.get(kind).map_or(&[], Vec::as_slice)
    }

    /// Forget everything (SA reset its defaults when the analysis session closed).
    pub fn clear(&mut self) {
        self.history.clear();
        self.order.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(value: f64) -> Measurement {
        Measurement {
            value,
            uncertainty: Some(0.01),
            source: None,
            tool: "test".into(),
        }
    }

    #[test]
    fn latest_wins_and_history_is_kept() {
        let mut ctx = MeasurementContext::new();
        assert!((ctx.value_or(&MeasurementKind::BaudRate, 33.0) - 33.0).abs() < f64::EPSILON);
        ctx.record(MeasurementKind::BaudRate, m(2373.0)).unwrap();
        ctx.record(MeasurementKind::BaudRate, m(2399.76)).unwrap();
        let latest = ctx.latest(&MeasurementKind::BaudRate).unwrap();
        assert!((latest.value - 2399.76).abs() < f64::EPSILON);
        assert_eq!(ctx.history(&MeasurementKind::BaudRate).len(), 2);
        assert!(ctx.latest(&MeasurementKind::Carrier).is_none());
        ctx.record(MeasurementKind::Carrier, m(1800.0)).unwrap();
        let order: Vec<f64> = ctx.iter().map(|(_, m)| m.value).collect();
        assert_eq!(order, vec![2373.0, 2399.76, 1800.0]);
        assert_eq!(ctx.len(), 3);
        ctx.clear();
        assert!(ctx.is_empty());
        assert_eq!(ctx.history(&MeasurementKind::BaudRate), []);
    }

    #[test]
    fn non_finite_measurements_are_rejected() {
        let mut ctx = MeasurementContext::new();
        for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let err = ctx.record(MeasurementKind::BaudRate, m(v)).unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains("'test'") && msg.contains("Br"), "{msg}");
        }
        let mut bad_sigma = m(100.0);
        bad_sigma.uncertainty = Some(f64::NAN);
        assert!(ctx.record(MeasurementKind::Carrier, bad_sigma).is_err());
        assert!(ctx.is_empty());
        assert!((ctx.value_or(&MeasurementKind::BaudRate, 50.0) - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn display_uses_sa_names() {
        assert_eq!(MeasurementKind::BaudRate.to_string(), "Br");
        assert_eq!(MeasurementKind::Carrier.to_string(), "Sc");
        assert_eq!(MeasurementKind::Custom("Fc".into()).to_string(), "Fc");
    }
}
