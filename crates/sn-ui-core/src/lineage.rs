//! Pure helpers about how a derived signal relates to its parent: how its frequencies
//! map back to the parent's, how the parent's markers carry over, and how the tab list
//! changes when a tab is closed. Kept free of egui so they can be unit-tested.

use sn_core::{Node, NodeId, SampleKind, SignalGraph};
use sn_dsp::ops::{Edit, effective_band};

use crate::markers::{MarkerId, Markers};

/// The edit that produced `node`, if it was made by the Signal menu.
pub fn edit_of(node: &Node) -> Option<Edit> {
    if node.operation.name != "edit" {
        return None;
    }
    serde_json::from_value::<Edit>(node.operation.params.clone()).ok()
}

/// One step of a frequency map from a child's axis to its parent's.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Step {
    /// `f_parent = scale * f + offset`.
    Affine { scale: f64, offset: f64 },
    /// The band `lo..hi` flipped about its centre; frequencies outside it unchanged.
    /// Its own inverse.
    Mirror { lo: f64, hi: f64 },
}

impl Step {
    fn to_parent(self, f: f64) -> f64 {
        match self {
            Self::Affine { scale, offset } => scale * f + offset,
            Self::Mirror { lo, hi } => mirror(lo, hi, f),
        }
    }

    /// The inverse of [`Self::to_parent`]; `None` when the step collapses the axis.
    fn to_child(self, f: f64) -> Option<f64> {
        match self {
            Self::Affine { scale, offset } => {
                if scale == 0.0 {
                    None
                } else {
                    Some((f - offset) / scale)
                }
            }
            Self::Mirror { lo, hi } => Some(mirror(lo, hi, f)),
        }
    }

    fn is_identity(self) -> bool {
        match self {
            Self::Affine { .. } => {
                self == Self::Affine {
                    scale: 1.0,
                    offset: 0.0,
                }
            }
            // An in-band inverse always has a band (the edit refuses an empty one).
            Self::Mirror { .. } => false,
        }
    }

    /// How much the step stretches a frequency interval.
    fn gain(self) -> f64 {
        match self {
            Self::Affine { scale, .. } => scale.abs(),
            Self::Mirror { .. } => 1.0,
        }
    }
}

/// `f` flipped about the centre of `lo..hi` when it lies inside, else unchanged.
fn mirror(lo: f64, hi: f64, f: f64) -> f64 {
    let (lo, hi) = (lo.min(hi), lo.max(hi));
    if (lo..=hi).contains(&f) {
        lo + hi - f
    } else {
        f
    }
}

/// A frequency map from a derived signal's axis back to an ancestor's: a chain of
/// steps applied in order, nearest edit first.
#[derive(Debug, Clone, PartialEq)]
pub struct FreqMap {
    steps: Vec<Step>,
}

impl FreqMap {
    /// The map of a signal onto itself.
    pub const IDENTITY: Self = Self { steps: Vec::new() };

    /// The parent frequency of the child frequency `f`.
    pub fn apply(&self, f: f64) -> f64 {
        self.steps.iter().fold(f, |f, s| s.to_parent(f))
    }

    /// How much the map stretches a frequency interval (for uncertainties).
    pub fn gain(&self) -> f64 {
        self.steps.iter().map(|s| s.gain()).product()
    }

    /// The map changes no frequency.
    pub fn is_identity(&self) -> bool {
        self.steps.iter().all(|s| s.is_identity())
    }
}

/// How a frequency on the output of `edit` maps back to its input (`parent_fs` and
/// `parent_kind` describe the input). Edits that do not move frequencies give the
/// identity.
fn edit_step(edit: &Edit, parent_fs: f64, parent_kind: SampleKind) -> Step {
    let affine = |scale: f64, offset: f64| Step::Affine { scale, offset };
    match *edit {
        Edit::Power { n } if n > 0 => affine(1.0 / f64::from(n), 0.0),
        Edit::Shift { hz } => affine(1.0, -hz),
        Edit::ShiftToBase { lo, hi, inverted } => {
            let (lo, hi) = effective_band(lo, hi, parent_fs, parent_kind);
            if inverted {
                affine(-1.0, hi)
            } else {
                affine(1.0, lo)
            }
        }
        Edit::IqToReal => affine(1.0, -parent_fs / 2.0),
        Edit::BaudCorrection { existing, required } if existing > 0.0 && required > 0.0 => {
            affine(existing / required, 0.0)
        }
        // Real input is multiplied by (-1)^n, so f becomes fs/2 - f; I/Q is conjugated,
        // so f becomes -f.
        Edit::Invert => match parent_kind {
            SampleKind::Real => affine(-1.0, parent_fs / 2.0),
            SampleKind::Complex => affine(-1.0, 0.0),
        },
        Edit::InvertBand { lo, hi } => {
            let (lo, hi) = effective_band(lo, hi, parent_fs, parent_kind);
            Step::Mirror { lo, hi }
        }
        _ => affine(1.0, 0.0),
    }
}

/// Edits whose output has a frequency axis unrelated to the input's (detector outputs,
/// lag axes, resynthesis, equalisation).
fn breaks_freq_axis(edit: &Edit) -> bool {
    matches!(
        edit,
        Edit::AmDetector
            | Edit::ModifiedAm
            | Edit::HalfWave
            | Edit::PhaseDetector
            | Edit::Acf { .. }
            | Edit::Ccf { .. }
            | Edit::Resynthesize { .. }
            | Edit::Equalize { .. }
    )
}

/// `node` and its ancestors through first inputs, nearest first.
fn ancestry(graph: &SignalGraph, node: NodeId) -> Vec<NodeId> {
    let mut out = vec![node];
    let mut cur = node;
    for _ in 0..10_000 {
        let Some(parent) = graph.get(cur).and_then(|n| n.inputs.first().copied()) else {
            break;
        };
        out.push(parent);
        cur = parent;
    }
    out
}

/// How a frequency on `node` maps onto its parent's axis; `None` when the operation
/// is unknown or has no frequency relation to its input.
fn step_to_parent(graph: &SignalGraph, node: NodeId) -> Option<Step> {
    let n = graph.get(node)?;
    let parent = graph.get(n.inputs.first().copied()?)?;
    let edit = edit_of(n)?;
    if breaks_freq_axis(&edit) {
        return None;
    }
    Some(edit_step(
        &edit,
        parent.signal.sample_rate(),
        parent.signal.kind(),
    ))
}

/// Map frequency `f` on the axis of `from` onto the axis of `to`, through their
/// nearest common ancestor. `None` when they share no ancestor, or a step between
/// them has no frequency relation (a detector, an unknown operation).
pub fn map_freq(graph: &SignalGraph, from: NodeId, to: NodeId, f: f64) -> Option<f64> {
    if from == to {
        return Some(f);
    }
    let up_from = ancestry(graph, from);
    let up_to = ancestry(graph, to);
    let k_from = up_from.iter().position(|n| up_to.contains(n))?;
    let common = *up_from.get(k_from)?;
    let k_to = up_to.iter().position(|n| *n == common)?;
    let mut f = f;
    for n in up_from.iter().take(k_from) {
        f = step_to_parent(graph, *n)?.to_parent(f);
    }
    // Down from the common ancestor: invert each step.
    for n in up_to.iter().take(k_to).rev() {
        f = step_to_parent(graph, *n)?.to_child(f)?;
    }
    f.is_finite().then_some(f)
}

/// The node on whose frequency axis a carrier measured on `node` is expressed: the
/// input of the earliest power-N step when [`carrier_map`] maps through one, else
/// `node` itself.
pub fn carrier_axis(graph: &SignalGraph, node: NodeId) -> NodeId {
    let mut axis = node;
    let mut cur = node;
    for _ in 0..10_000 {
        let Some(n) = graph.get(cur) else {
            break;
        };
        let Some(parent) = n.inputs.first().copied() else {
            break;
        };
        if matches!(edit_of(n), Some(Edit::Power { n }) if n > 1) {
            axis = parent;
        }
        cur = parent;
    }
    axis
}

/// How a carrier line measured on `node` maps back to a real carrier frequency.
///
/// When the lineage contains a power-N step, the line is mapped back through every edit
/// up to the input of the earliest power step (so Power 4 then Band select, or Power 2
/// of Power 2, both divide correctly, and shifts after the power are undone). Without a
/// power step the line is the carrier itself. Returns the map and the total power, or
/// the label of an edit between the line and the power step that has no frequency
/// relation to its input (a detector, a correlation), through which no carrier maps.
pub fn carrier_map(graph: &SignalGraph, node: NodeId) -> Result<(FreqMap, u32), String> {
    // (edit, parent sample rate and kind), from the node up towards its source.
    let mut chain: Vec<(Edit, f64, SampleKind)> = Vec::new();
    let mut cur = node;
    for _ in 0..10_000 {
        let Some(n) = graph.get(cur) else {
            break;
        };
        let Some(parent) = n.inputs.first().copied() else {
            break;
        };
        let (parent_fs, parent_kind) = graph.get(parent).map_or((1.0, SampleKind::Real), |p| {
            (p.signal.sample_rate(), p.signal.kind())
        });
        if let Some(e) = edit_of(n) {
            chain.push((e, parent_fs, parent_kind));
        }
        cur = parent;
    }
    let Some(last_power) = chain
        .iter()
        .rposition(|(e, _, _)| matches!(e, Edit::Power { n } if *n > 1))
    else {
        return Ok((FreqMap::IDENTITY, 1));
    };
    if let Some((e, _, _)) = chain
        .iter()
        .take(last_power + 1)
        .find(|(e, _, _)| breaks_freq_axis(e))
    {
        return Err(e.label());
    }
    let mut map = FreqMap::IDENTITY;
    let mut power = 1_u32;
    for (e, fs, kind) in chain.iter().take(last_power + 1) {
        map.steps.push(edit_step(e, *fs, *kind));
        if let Edit::Power { n } = e {
            power = power.saturating_mul((*n).max(1));
        }
    }
    Ok((map, power))
}

/// What happened to the parent's markers on the child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Carried {
    /// Both pairs map as they were.
    Kept,
    /// At least one pair was moved to the child's axes.
    Mapped,
    /// At least one pair could not be mapped and was turned off.
    Cleared,
}

/// The parent's markers mapped onto a child made by `edit`.
///
/// V markers are sample indices and H markers Hz, so they are moved through the edit:
/// rate changes scale V, Copy subtracts its start, shifts and power N move H. A pair
/// that has no meaning on the child (ACF lag axis, detector outputs, resynthesis) is
/// turned off rather than left pointing at the wrong place.
pub fn inherit_markers(
    m: Markers,
    edit: &Edit,
    parent_fs: f64,
    parent_kind: SampleKind,
    child_fs: f64,
    child_len: usize,
) -> (Markers, Carried) {
    let mut out = m;
    let mut carried = Carried::Kept;
    let mut note = |c: Carried| {
        if c == Carried::Cleared || carried == Carried::Kept {
            carried = c;
        }
    };
    let len = child_len as f64;
    let scale_v = |k: f64| m.v.map(|(a, b)| (a * k, b * k));
    // Time axis.
    let v = match *edit {
        Edit::Copy { start, .. } => {
            let s = start as f64;
            m.v.map(|(a, b)| ((a - s).clamp(0.0, len), (b - s).clamp(0.0, len)))
                .filter(|(a, b)| (b - a).abs() > f64::EPSILON)
        }
        Edit::Acf { .. } | Edit::Ccf { .. } | Edit::Resynthesize { .. } | Edit::Equalize { .. } => {
            None
        }
        Edit::BaudCorrection { existing, required } if existing > 0.0 && required > 0.0 => {
            scale_v(existing / required)
        }
        _ if parent_fs > 0.0 && child_fs > 0.0 => scale_v(child_fs / parent_fs),
        _ => None,
    };
    if v != m.v {
        note(if v.is_none() {
            Carried::Cleared
        } else {
            Carried::Mapped
        });
    }
    out.v = v;
    // Frequency axis.
    let h = match edit {
        e if breaks_freq_axis(e) => None,
        _ => {
            let step = edit_step(edit, parent_fs, parent_kind);
            m.h.and_then(|(a, b)| Some((step.to_child(a)?, step.to_child(b)?)))
        }
    };
    if h != m.h {
        note(if h.is_none() {
            Carried::Cleared
        } else {
            Carried::Mapped
        });
    }
    out.h = h;
    // A lock or nudge target on a pair that is gone means nothing.
    let probe = out;
    let alive = |id: &MarkerId| probe.get(*id).is_some();
    out.locked = out.locked.filter(alive);
    out.active = out.active.filter(alive);
    (out, carried)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use sn_core::{Operation, Signal};

    use super::*;

    fn sig(fs: f64) -> Signal {
        Signal::real(vec![0.0_f32; 64], fs).unwrap()
    }

    fn derive(g: &mut SignalGraph, parent: NodeId, e: &Edit) -> NodeId {
        g.add_derived(
            e.label(),
            Operation::new("edit", serde_json::to_value(e).unwrap()),
            &[parent],
            sig(8000.0),
        )
        .unwrap()
    }

    #[test]
    fn power_is_found_through_the_lineage() {
        let mut g = SignalGraph::new();
        let src = g.add_source("src", Operation::new("load", json!({})), sig(8000.0));
        assert_eq!(carrier_map(&g, src), Ok((FreqMap::IDENTITY, 1)));
        let p4 = derive(&mut g, src, &Edit::Power { n: 4 });
        let bs = derive(
            &mut g,
            p4,
            &Edit::BandSelect {
                lo: 7000.0,
                hi: 7400.0,
            },
        );
        let (m, n) = carrier_map(&g, bs).unwrap();
        assert_eq!(n, 4);
        assert!((m.apply(7201.0) - 1800.25).abs() < 1e-9);

        // Power 2 of Power 2 is power 4.
        let p2 = derive(&mut g, src, &Edit::Power { n: 2 });
        let p22 = derive(&mut g, p2, &Edit::Power { n: 2 });
        let (m, n) = carrier_map(&g, p22).unwrap();
        assert_eq!(n, 4);
        assert!((m.apply(7200.0) - 1800.0).abs() < 1e-9);

        // A shift to base after the power is undone before dividing.
        let base = derive(
            &mut g,
            p4,
            &Edit::ShiftToBase {
                lo: 7000.0,
                hi: 7400.0,
                inverted: false,
            },
        );
        let (m, _) = carrier_map(&g, base).unwrap();
        assert!((m.apply(200.0) - 1800.0).abs() < 1e-9);
    }

    #[test]
    fn carrier_maps_between_branches() {
        let mut g = SignalGraph::new();
        let src = g.add_source("src", Operation::new("load", json!({})), sig(8000.0));
        let base = derive(
            &mut g,
            src,
            &Edit::ShiftToBase {
                lo: 1000.0,
                hi: 3000.0,
                inverted: false,
            },
        );
        // Sc measured on the source, used on the shifted child.
        assert_eq!(map_freq(&g, src, base, 1800.0), Some(800.0));
        assert_eq!(map_freq(&g, base, src, 800.0), Some(1800.0));
        // Sibling branches meet at the source.
        let shifted = derive(&mut g, src, &Edit::Shift { hz: 100.0 });
        let m = map_freq(&g, base, shifted, 800.0).unwrap();
        assert!((m - 1900.0).abs() < 1e-9);
        // Across a detector the axis is lost.
        let am = derive(&mut g, src, &Edit::AmDetector);
        assert_eq!(map_freq(&g, src, am, 1800.0), None);
        // Unrelated recordings share no axis.
        let other = g.add_source("other", Operation::new("load", json!({})), sig(8000.0));
        assert_eq!(map_freq(&g, src, other, 1800.0), None);
        // A carrier measured after power 4 is on the axis of the power step's input.
        let p4 = derive(&mut g, base, &Edit::Power { n: 4 });
        let bs = derive(
            &mut g,
            p4,
            &Edit::BandSelect {
                lo: 3000.0,
                hi: 3400.0,
            },
        );
        assert_eq!(carrier_axis(&g, bs), base);
        assert_eq!(carrier_axis(&g, base), base);
    }

    #[test]
    fn markers_follow_resample_and_copy() {
        let m = Markers {
            v: Some((4800.0, 9600.0)),
            h: Some((1000.0, 2000.0)),
            locked: Some(MarkerId::V1),
            active: Some(MarkerId::H2),
        };
        let (r, c) = inherit_markers(
            m,
            &Edit::Resample { rate: 24_000.0 },
            48_000.0,
            SampleKind::Real,
            24_000.0,
            1,
        );
        assert_eq!(r.v, Some((2400.0, 4800.0)));
        assert_eq!(r.h, m.h);
        assert_eq!(r.locked, Some(MarkerId::V1));
        assert_eq!(c, Carried::Mapped);

        let (r, _) = inherit_markers(
            m,
            &Edit::Copy {
                start: 4000,
                end: 10_000,
            },
            48_000.0,
            SampleKind::Real,
            48_000.0,
            6000,
        );
        assert_eq!(r.v, Some((800.0, 5600.0)));

        // Entirely outside the copied part: the pair turns off, with its lock.
        let (r, c) = inherit_markers(
            m,
            &Edit::Copy { start: 0, end: 100 },
            48_000.0,
            SampleKind::Real,
            48_000.0,
            100,
        );
        assert_eq!(r.v, None);
        assert_eq!(r.locked, None);
        assert_eq!(c, Carried::Cleared);

        let (r, _) = inherit_markers(
            m,
            &Edit::Power { n: 4 },
            48_000.0,
            SampleKind::Real,
            48_000.0,
            1,
        );
        assert_eq!(r.h, Some((4000.0, 8000.0)));
        let (r, _) = inherit_markers(
            m,
            &Edit::ShiftToBase {
                lo: 1000.0,
                hi: 2000.0,
                inverted: false,
            },
            48_000.0,
            SampleKind::Real,
            48_000.0,
            1,
        );
        assert_eq!(r.h, Some((0.0, 1000.0)));
        let (r, c) = inherit_markers(
            m,
            &Edit::Lowpass { cutoff: 100.0 },
            48_000.0,
            SampleKind::Real,
            48_000.0,
            1,
        );
        assert_eq!((r, c), (m, Carried::Kept));
        let (r, _) = inherit_markers(
            m,
            &Edit::AmDetector,
            48_000.0,
            SampleKind::Real,
            48_000.0,
            1,
        );
        assert_eq!(r.h, None);
        assert_eq!(r.active, None);
    }

    fn derive_sig(g: &mut SignalGraph, parent: NodeId, e: &Edit, s: Signal) -> NodeId {
        g.add_derived(
            e.label(),
            Operation::new("edit", serde_json::to_value(e).unwrap()),
            &[parent],
            s,
        )
        .unwrap()
    }

    #[test]
    fn spectrum_inverse_mirrors_the_axis() {
        let mut g = SignalGraph::new();
        // Real parent: f becomes fs/2 - f.
        let src = g.add_source("src", Operation::new("load", json!({})), sig(8000.0));
        let inv = derive(&mut g, src, &Edit::Invert);
        assert_eq!(map_freq(&g, src, inv, 1000.0), Some(3000.0));
        assert_eq!(map_freq(&g, inv, src, 3000.0), Some(1000.0));
        // Complex parent: f becomes -f.
        let iq = Signal::complex(vec![sn_core::Complex32::new(0.0, 0.0); 64], 8000.0).unwrap();
        let isrc = g.add_source("iq", Operation::new("load", json!({})), iq.clone());
        let iinv = derive_sig(&mut g, isrc, &Edit::Invert, iq);
        assert_eq!(map_freq(&g, isrc, iinv, 1000.0), Some(-1000.0));
        // In-band inverse: only frequencies inside the band move.
        let band = Edit::InvertBand {
            lo: 1000.0,
            hi: 2000.0,
        };
        let ib = derive(&mut g, src, &band);
        assert_eq!(map_freq(&g, src, ib, 1200.0), Some(1800.0));
        assert_eq!(map_freq(&g, src, ib, 2500.0), Some(2500.0));
        // A carrier measured after power 2 of an inverted signal maps through both.
        let p2 = derive(&mut g, inv, &Edit::Power { n: 2 });
        let (m, n) = carrier_map(&g, p2).unwrap();
        assert_eq!(n, 2);
        assert!((m.apply(6000.0) - 3000.0).abs() < 1e-9);
        assert!(!m.is_identity());

        let mk = Markers {
            v: None,
            h: Some((1000.0, 1500.0)),
            locked: None,
            active: None,
        };
        let (r, c) = inherit_markers(mk, &Edit::Invert, 8000.0, SampleKind::Real, 8000.0, 64);
        assert_eq!(r.h, Some((3000.0, 2500.0)));
        assert_eq!(c, Carried::Mapped);
        let (r, _) = inherit_markers(mk, &Edit::Invert, 8000.0, SampleKind::Complex, 8000.0, 64);
        assert_eq!(r.h, Some((-1000.0, -1500.0)));
        let (r, _) = inherit_markers(mk, &band, 8000.0, SampleKind::Real, 8000.0, 64);
        assert_eq!(r.h, Some((2000.0, 1500.0)));
    }

    #[test]
    fn band_edits_map_with_the_clamped_band() {
        let mut g = SignalGraph::new();
        let src = g.add_source("src", Operation::new("load", json!({})), sig(8000.0));
        // The band runs past Nyquist (4000 Hz); sn-dsp flips 3000..4000 only.
        let ib = derive(
            &mut g,
            src,
            &Edit::InvertBand {
                lo: 3000.0,
                hi: 5000.0,
            },
        );
        assert_eq!(map_freq(&g, src, ib, 3200.0), Some(3800.0));
        // Shift to base inverted uses the clamped top edge.
        let base = derive(
            &mut g,
            src,
            &Edit::ShiftToBase {
                lo: -500.0,
                hi: 5000.0,
                inverted: true,
            },
        );
        assert_eq!(map_freq(&g, base, src, 1000.0), Some(3000.0));
        let base = derive(
            &mut g,
            src,
            &Edit::ShiftToBase {
                lo: -500.0,
                hi: 1000.0,
                inverted: false,
            },
        );
        assert_eq!(map_freq(&g, base, src, 300.0), Some(300.0));
    }

    #[test]
    fn carrier_does_not_map_through_a_detector() {
        let mut g = SignalGraph::new();
        let src = g.add_source("src", Operation::new("load", json!({})), sig(8000.0));
        let p4 = derive(&mut g, src, &Edit::Power { n: 4 });
        let am = derive(&mut g, p4, &Edit::AmDetector);
        assert_eq!(carrier_map(&g, am), Err(Edit::AmDetector.label()));
        // A detector before the power step is the power step's own input: fine.
        let am0 = derive(&mut g, src, &Edit::AmDetector);
        let p2 = derive(&mut g, am0, &Edit::Power { n: 2 });
        assert!(carrier_map(&g, p2).is_ok());
    }
}
