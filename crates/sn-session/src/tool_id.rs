//! Stable ids of the tools that record measurements. They are stored in the
//! session, so a session reads the same in every interface language. The front
//! ends show a translated name (Fluent key `measure-tool-<id>`); the report,
//! which is in English, uses [`english_name`].

/// Auto define params, refined rate.
pub const ADP: &str = "adp";
/// Auto define params, unrefined rate (centre of a spectrum bin).
pub const ADP_COARSE: &str = "adp-coarse";
/// Measurement of Freq on a spectral line.
pub const MEASURE_LINE: &str = "measure-line";
/// dT between the V markers.
pub const V_MARKERS: &str = "v-markers";
/// The OFDM tool's Br estimate.
pub const OFDM_ADP: &str = "ofdm-adp";
/// A value stored by a script's `record(sig, kind, value)`.
pub const SCRIPT_RECORD: &str = "script-record";
/// Every id, for checks that each has a shown name.
pub const ALL: [&str; 6] = [
    ADP,
    ADP_COARSE,
    MEASURE_LINE,
    V_MARKERS,
    OFDM_ADP,
    SCRIPT_RECORD,
];

/// The English name of a known tool id, or `None` for anything else (such as
/// the free text name a session from before the ids holds).
pub fn english_name(id: &str) -> Option<&'static str> {
    match id {
        ADP => Some("Auto define params (ADP)"),
        ADP_COARSE => Some("Auto define params, coarse (within half an FFT bin)"),
        MEASURE_LINE => Some("Measurement of Freq (measure line)"),
        V_MARKERS => Some("V markers (dT)"),
        OFDM_ADP => Some("OFDM tool, Get Br (ADP)"),
        SCRIPT_RECORD => Some("Script, record()"),
        _ => None,
    }
}

/// The uncertainty and tool id to record an ADP candidate with, the same in
/// every front end and in scripts: an unrefined rate is the centre of a coarse
/// spectrum bin, so it is within half the bin width and the tool is
/// [`ADP_COARSE`] (its name shows wherever Br pre-fills a tool).
pub fn adp_quality(c: &sn_analysis::adp::BaudCandidate) -> (Option<f64>, &'static str) {
    if c.refined {
        (None, ADP)
    } else if c.resolution > 0.0 {
        (Some(c.resolution / 2.0), ADP_COARSE)
    } else {
        (None, ADP_COARSE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_id_has_an_english_name() {
        for id in ALL {
            assert!(english_name(id).is_some(), "{id}");
        }
        assert_eq!(english_name("ADP, coarse"), None);
    }

    #[test]
    fn adp_candidates_record_their_quality() {
        let mut c = sn_analysis::adp::BaudCandidate {
            baud: 75.0,
            detector: sn_analysis::adp::Detector::Am,
            score: 20.0,
            refined: true,
            resolution: 2.0,
        };
        assert_eq!(adp_quality(&c), (None, ADP));
        c.refined = false;
        assert_eq!(adp_quality(&c), (Some(1.0), ADP_COARSE));
        c.resolution = 0.0;
        assert_eq!(adp_quality(&c), (None, ADP_COARSE));
    }
}
