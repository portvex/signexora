//! Session save/load, replay and report tests.

// Helpers outside #[test] functions are test code too; the no-panic rule is for the library.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::path::{Path, PathBuf};

use serde_json::json;
use sn_core::{
    Complex32, Measurement, MeasurementContext, MeasurementKind, NodeId, Operation, SampleKind,
    Signal, SignalGraph,
};
use sn_dsp::ops::Edit;
use sn_io::{LoadOptions, Scaling, WavFormat};
use sn_session::{
    CaptureOptions, Error, MeasurementRecord, ReplayOptions, SessionFile, SessionNode,
    SignalSummary, UI_KEY_ADP, UI_KEY_TONE_GRID, UI_KEY_TONES, load, replay, report, save,
};

fn chain() -> Vec<Edit> {
    vec![
        Edit::BandSelect {
            lo: 300.0,
            hi: 3300.0,
        },
        Edit::Shift { hz: -1800.0 },
        Edit::Resample { rate: 12_000.0 },
    ]
}

fn write_psk(dir: &Path, name: &str) -> PathBuf {
    let p = sn_synth::PskParams {
        order: 4,
        baud: 2400.0,
        carrier: 1800.0,
        sample_rate: 48_000.0,
        amplitude: 0.5,
    };
    let s = sn_synth::psk(&p, &sn_synth::random_symbols(2400, 4, 7)).unwrap();
    let s = sn_synth::add_awgn_snr(&s, 20.0, 3).unwrap();
    let path = dir.join(name);
    sn_io::write_wav(&path, &s, WavFormat::Pcm16).unwrap();
    path
}

fn load_op(path: &Path) -> Operation {
    let options = LoadOptions::default();
    Operation::new(
        "load",
        json!({
            "path": path.display().to_string(),
            "channels": options.channels,
            "scaling": Scaling::default(),
            "raw": options.raw,
        }),
    )
}

/// Every sample as raw bits, so comparisons are exact.
fn bits(s: &Signal) -> Vec<u32> {
    match s.kind() {
        SampleKind::Real => {
            let mut v = vec![0.0_f32; s.len()];
            s.read_real(0, &mut v);
            v.iter().map(|x| x.to_bits()).collect()
        }
        SampleKind::Complex => {
            let mut v = vec![Complex32::new(0.0, 0.0); s.len()];
            s.read_complex(0, &mut v);
            v.iter()
                .flat_map(|c| [c.re.to_bits(), c.im.to_bits()])
                .collect()
        }
    }
}

/// A session like the app builds: a loaded file, an edit chain, measurements, UI state.
fn interactive_session(path: &Path) -> (SignalGraph, MeasurementContext, Vec<NodeId>) {
    let mut g = SignalGraph::new();
    let src = sn_io::open(path, &LoadOptions::default()).unwrap();
    let mut ids = vec![g.add_source("rec.wav", load_op(path), src.clone())];
    let mut cur = src;
    for e in chain() {
        cur = e.run(&cur, &|| false).unwrap();
        let parent = *ids.last().unwrap();
        let op = Operation::new("edit", serde_json::to_value(&e).unwrap());
        ids.push(
            g.add_derived(e.label(), op, &[parent], cur.clone())
                .unwrap(),
        );
    }
    let mut m = MeasurementContext::new();
    m.record(
        MeasurementKind::BaudRate,
        Measurement {
            value: 2399.98,
            uncertainty: Some(0.01),
            source: Some(ids[1]),
            tool: sn_session::tool_id::MEASURE_LINE.into(),
        },
    )
    .unwrap();
    m.record(
        MeasurementKind::Carrier,
        Measurement {
            value: 1800.0,
            uncertainty: None,
            source: Some(ids[0]),
            tool: "power 4".into(),
        },
    )
    .unwrap();
    (g, m, ids)
}

#[test]
fn round_trip_save_load_is_equal() {
    let dir = tempfile::tempdir().unwrap();
    let wav = write_psk(dir.path(), "rec.wav");
    let (g, m, ids) = interactive_session(&wav);
    let mut s = SessionFile::capture(&g, &m);
    let ui = s.node_ui_mut(ids[1]).unwrap();
    ui.insert("view".into(), json!({"t0": 0.25, "zoom": 3}));
    ui.insert("markers".into(), json!({"v": [100, 2000], "h": null}));
    s.ui.insert("tabs".into(), json!([0, 1]));

    assert_eq!(s.nodes.len(), 4);
    assert_eq!(s.measurements.len(), 2);
    let src = s.nodes[0].source.as_ref().unwrap();
    assert_eq!(src.container, "Wav");

    let file = dir.path().join("session.json");
    save(&file, &s).unwrap();
    let back = load(&file).unwrap();
    assert_eq!(back, s);
    // The measurement context comes back in recording order.
    let ctx = back.measurement_context();
    let kinds: Vec<_> = ctx.iter().map(|(k, _)| k.clone()).collect();
    assert_eq!(
        kinds,
        vec![MeasurementKind::BaudRate, MeasurementKind::Carrier]
    );
}

#[test]
fn replay_is_bit_for_bit_identical_to_running_the_edits() {
    let dir = tempfile::tempdir().unwrap();
    let wav = write_psk(dir.path(), "rec.wav");
    let (g, m, ids) = interactive_session(&wav);
    let file = dir.path().join("session.json");
    save(&file, &SessionFile::capture(&g, &m)).unwrap();

    let session = load(&file).unwrap();
    let mut steps = Vec::new();
    let out = replay(
        &session,
        &ReplayOptions::default(),
        &mut |p| steps.push((p.done, p.total, p.label.to_owned())),
        &|| false,
    )
    .unwrap();
    assert!(out.is_complete(), "{:?}", out.issues);
    assert!(out.issues.is_empty(), "{:?}", out.issues);
    assert_eq!(steps.len(), 4);
    assert_eq!(steps[3].0, 3);
    assert_eq!(steps[3].1, 4);

    // Independent reference: open the file and run the same edits directly.
    let mut reference = vec![sn_io::open(&wav, &LoadOptions::default()).unwrap()];
    for e in chain() {
        let next = e.run(reference.last().unwrap(), &|| false).unwrap();
        reference.push(next);
    }
    for (old, expected) in ids.iter().zip(&reference) {
        let new = out.new_id(*old).unwrap();
        let node = out.graph.get(new).unwrap();
        assert_eq!(node.signal.sample_rate(), expected.sample_rate());
        assert_eq!(bits(&node.signal), bits(expected), "node {old} differs");
        assert!(expected.len() > 1000);
    }
    // Measurements point at the rebuilt nodes.
    let br = out.measurements.latest(&MeasurementKind::BaudRate).unwrap();
    assert_eq!(br.source, out.new_id(ids[1]));
}

#[test]
fn replay_reports_missing_source_and_keeps_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_psk(dir.path(), "a.wav");
    let b = write_psk(dir.path(), "b.wav");
    let mut g = SignalGraph::new();
    let sa = g.add_source(
        "a.wav",
        load_op(&a),
        sn_io::open(&a, &LoadOptions::default()).unwrap(),
    );
    let sb = g.add_source(
        "b.wav",
        load_op(&b),
        sn_io::open(&b, &LoadOptions::default()).unwrap(),
    );
    let e = Edit::Derivative;
    let ga = g.get(sa).unwrap().signal.clone();
    let da = g
        .add_derived(
            "d",
            Operation::new("edit", serde_json::to_value(&e).unwrap()),
            &[sa],
            e.run(&ga, &|| false).unwrap(),
        )
        .unwrap();
    let mut m = MeasurementContext::new();
    m.record(
        MeasurementKind::Carrier,
        Measurement {
            value: 1800.0,
            uncertainty: None,
            source: Some(da),
            tool: "power 4".into(),
        },
    )
    .unwrap();
    let session = SessionFile::capture(&g, &m);
    std::fs::remove_file(&a).unwrap();

    let out = replay(&session, &ReplayOptions::default(), &mut |_| {}, &|| false).unwrap();
    assert!(!out.is_complete());
    assert_eq!(out.failed_count(), 2);
    assert!(out.new_id(sb).is_some());
    assert!(out.new_id(sa).is_none() && out.new_id(da).is_none());
    let first = &out.issues[0];
    assert!(
        first.message.contains("no longer exists"),
        "{}",
        first.message
    );
    assert!(first.message.contains("a.wav"));
    assert!(out.issues[1].message.contains("skipped"));
    // The measurement on the lost node is kept, unlinked, with a warning that says why.
    let carrier = out.measurements.latest(&MeasurementKind::Carrier).unwrap();
    assert_eq!(carrier.source, None);
    let warning = out
        .issues
        .iter()
        .find(|i| !i.failed && i.node == da)
        .unwrap();
    assert!(
        warning.message.contains("power 4") && warning.message.contains("could not be rebuilt"),
        "{}",
        warning.message
    );
}

#[test]
fn replay_resolves_relative_paths_and_reports_bad_operations() {
    let dir = tempfile::tempdir().unwrap();
    write_psk(dir.path(), "rel.wav");
    let mut s = SessionFile::new();
    s.nodes.push(SessionNode {
        id: NodeId::from_raw(0),
        label: "rel.wav".into(),
        inputs: vec![],
        operation: Operation::new("load", json!({"path": "rel.wav"})),
        signal: None,
        source: None,
        ui: serde_json::Map::new(),
    });
    s.nodes.push(SessionNode {
        id: NodeId::from_raw(1),
        label: "mystery".into(),
        inputs: vec![NodeId::from_raw(0)],
        operation: Operation::new("frobnicate", json!({})),
        signal: None,
        source: None,
        ui: serde_json::Map::new(),
    });
    s.nodes.push(SessionNode {
        id: NodeId::from_raw(2),
        label: "bad edit".into(),
        inputs: vec![NodeId::from_raw(0)],
        operation: Operation::new("edit", json!({"op": "band_select", "lo": "x"})),
        signal: None,
        source: None,
        ui: serde_json::Map::new(),
    });
    let opts = ReplayOptions {
        base_dir: Some(dir.path().to_path_buf()),
    };
    let out = replay(&s, &opts, &mut |_| {}, &|| false).unwrap();
    assert!(out.new_id(NodeId::from_raw(0)).is_some());
    assert_eq!(out.failed_count(), 2);
    assert!(out.issues[0].message.contains("frobnicate"));
    assert!(out.issues[1].message.contains("not valid"));
}

#[test]
fn replay_can_be_cancelled() {
    let dir = tempfile::tempdir().unwrap();
    let wav = write_psk(dir.path(), "rec.wav");
    let (g, m, _) = interactive_session(&wav);
    let s = SessionFile::capture(&g, &m);
    let r = replay(&s, &ReplayOptions::default(), &mut |_| {}, &|| true);
    assert!(matches!(r, Err(Error::Cancelled)));
}

#[test]
fn load_rejects_newer_versions_and_other_files() {
    let mut s = SessionFile::new();
    s.version = sn_session::FORMAT_VERSION + 1;
    let text = serde_json::to_string(&s).unwrap();
    let e = SessionFile::from_json(&text).unwrap_err();
    assert!(matches!(e, Error::NewerVersion { .. }));
    assert!(e.to_string().contains("Update SigNexora"));
    assert!(SessionFile::from_json("{\"format\":\"other\",\"version\":1}").is_err());
    assert!(SessionFile::from_json("not json").is_err());
    let dir = tempfile::tempdir().unwrap();
    let e = load(dir.path().join("missing.json")).unwrap_err();
    assert!(e.to_string().contains("missing.json"));
    // Inputs must refer to earlier nodes.
    let bad = json!({"format": "sa-rs-session", "version": 1, "nodes": [
        {"id": 0, "label": "x", "inputs": [5], "operation": {"name": "edit", "params": {}}}
    ]});
    assert!(SessionFile::from_json(&bad.to_string()).is_err());
}

#[test]
fn report_has_all_sections_and_results() {
    let dir = tempfile::tempdir().unwrap();
    let wav = write_psk(dir.path(), "rec.wav");
    let (g, m, ids) = interactive_session(&wav);
    let mut s = SessionFile::capture(&g, &m);
    let ui = s.node_ui_mut(ids[2]).unwrap();
    ui.insert(
        UI_KEY_ADP.into(),
        json!([{"baud": 2400.01, "detector": "Am", "score": 31.5}]),
    );
    ui.insert(
        UI_KEY_TONE_GRID.into(),
        json!({"size": 8, "spacing": 125.0, "lowest": 1000.0, "occupied": 8}),
    );
    ui.insert(UI_KEY_TONES.into(), json!([1000.0, 1125.0]));

    let md = report::markdown(&s);
    let html = report::html(&s);
    for section in report::SECTIONS {
        assert!(
            md.contains(&format!("## {section}")),
            "markdown lacks {section}"
        );
        assert!(
            html.contains(&format!("<h2>{section}</h2>")),
            "html lacks {section}"
        );
    }
    for text in [&md, &html] {
        assert!(text.contains("rec.wav"));
        assert!(text.contains("band select 300.00-3300.00 Hz"));
        assert!(text.contains("2400.010 Bd"));
        assert!(text.contains("125.000 Hz"));
        assert!(text.contains("Br (keying rate)"));
        // A known tool id shows its English name; free text shows as it is.
        assert!(text.contains("Measurement of Freq (measure line)"));
        assert!(!text.contains(sn_session::tool_id::MEASURE_LINE));
        assert!(text.contains("power 4"));
        assert!(text.contains("PCM"));
        assert!(text.is_ascii(), "reports must be plain ASCII");
    }
    // Tree nesting: the resample node is three levels deep.
    assert!(md.contains("      - #3 resample to 12000 Hz"));
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(!html.contains("http"), "no external resources");
    assert_eq!(html.matches("<ul>").count(), html.matches("</ul>").count());
    assert_eq!(html.matches("<li>").count(), html.matches("</li>").count());

    let empty = report::markdown(&SessionFile::new());
    assert!(empty.contains("No measurements were recorded."));
}

fn node(id: u32, inputs: &[u32], operation: Operation) -> SessionNode {
    SessionNode {
        id: NodeId::from_raw(id),
        label: format!("node {id}"),
        inputs: inputs.iter().map(|i| NodeId::from_raw(*i)).collect(),
        operation,
        signal: None,
        source: None,
        ui: serde_json::Map::new(),
    }
}

fn synth_op(extra: &serde_json::Value) -> Operation {
    let mut params = json!({
        "order": 4, "baud": 2400.0, "carrier": 1800.0, "rate": 48000.0,
        "seed": 11, "symbols": 480, "amplitude": 0.5
    });
    for (k, v) in extra.as_object().unwrap() {
        params[k] = v.clone();
    }
    Operation::new("synth_psk", params)
}

fn replay_one(op: Operation) -> sn_session::ReplayOutcome {
    let mut s = SessionFile::new();
    s.nodes.push(node(0, &[], op));
    replay(&s, &ReplayOptions::default(), &mut |_| {}, &|| false).unwrap()
}

#[test]
fn synth_psk_replays_bit_for_bit() {
    let out = replay_one(synth_op(&json!({})));
    assert!(out.issues.is_empty(), "{:?}", out.issues);
    let p = sn_synth::PskParams {
        order: 4,
        baud: 2400.0,
        carrier: 1800.0,
        sample_rate: 48_000.0,
        amplitude: 0.5,
    };
    let expected = sn_synth::psk(&p, &sn_synth::random_symbols(480, 4, 11)).unwrap();
    let got = out
        .graph
        .get(out.new_id(NodeId::from_raw(0)).unwrap())
        .unwrap();
    assert_eq!(bits(&got.signal), bits(&expected));
}

#[test]
fn synth_psk_without_seed_is_reported() {
    let mut op = synth_op(&json!({}));
    op.params.as_object_mut().unwrap().remove("seed");
    let out = replay_one(op);
    assert_eq!(out.failed_count(), 1);
    assert!(
        out.issues[0].message.contains("seed"),
        "{}",
        out.issues[0].message
    );
}

#[test]
fn synth_psk_rejects_huge_or_invalid_sizes_without_allocating() {
    for (extra, field) in [
        (json!({"symbols": 1_000_000_000_000_000_u64}), "symbols"),
        (json!({"symbols": 0}), "symbols"),
        (json!({"baud": 1e-9}), "samples"),
        (json!({"baud": 0.0}), "baud"),
        (json!({"rate": -48000.0}), "rate"),
        (json!({"order": 0}), "order"),
    ] {
        let out = replay_one(synth_op(&extra));
        assert_eq!(out.failed_count(), 1, "{extra}");
        let msg = &out.issues[0].message;
        assert!(msg.contains(field), "{extra}: {msg}");
    }
}

#[test]
fn replay_can_be_cancelled_inside_an_edit() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let dir = tempfile::tempdir().unwrap();
    let wav = write_psk(dir.path(), "rec.wav");
    let (g, m, _) = interactive_session(&wav);
    let s = SessionFile::capture(&g, &m);
    // Calls 1 and 2 are the checks before nodes 0 and 1; call 3 comes from inside
    // the first edit.
    let calls = AtomicUsize::new(0);
    let stop = || calls.fetch_add(1, Ordering::SeqCst) >= 2;
    let mut started = Vec::new();
    let r = replay(
        &s,
        &ReplayOptions::default(),
        &mut |p| started.push(p.done),
        &stop,
    );
    assert!(matches!(r, Err(Error::Cancelled)));
    assert_eq!(started, vec![0, 1], "cancelled before the edit finished");
    assert!(calls.load(Ordering::SeqCst) >= 3);
}

#[test]
fn replay_warns_when_the_source_size_changed() {
    let dir = tempfile::tempdir().unwrap();
    let wav = write_psk(dir.path(), "rec.wav");
    let (g, m, _) = interactive_session(&wav);
    let mut s = SessionFile::capture(&g, &m);
    let real = std::fs::metadata(&wav).unwrap().len();
    s.nodes[0].source.as_mut().unwrap().bytes = Some(real + 10);
    let out = replay(&s, &ReplayOptions::default(), &mut |_| {}, &|| false).unwrap();
    assert!(out.is_complete());
    let w = out
        .issues
        .iter()
        .find(|i| i.message.contains("bytes"))
        .unwrap();
    assert!(!w.failed);
    assert!(
        w.message.contains(&(real + 10).to_string()),
        "{}",
        w.message
    );
}

#[test]
fn replay_warns_when_the_centre_frequency_changed() {
    let mut s = SessionFile::new();
    let mut n = node(0, &[], synth_op(&json!({})));
    let len = (480.0_f64 * 48_000.0 / 2400.0).round() as u64;
    n.signal = Some(SignalSummary {
        sample_rate: 48_000.0,
        len,
        kind: SampleKind::Real,
        center_frequency: 1000.0,
    });
    s.nodes.push(n);
    let out = replay(&s, &ReplayOptions::default(), &mut |_| {}, &|| false).unwrap();
    assert!(out.is_complete());
    assert_eq!(out.issues.len(), 1, "{:?}", out.issues);
    assert!(out.issues[0].message.contains("centre frequency"));
}

#[cfg(unix)]
#[test]
fn replay_says_access_denied_instead_of_missing() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let locked = dir.path().join("locked");
    std::fs::create_dir(&locked).unwrap();
    let wav = write_psk(&locked, "rec.wav");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let readable = std::fs::metadata(&wav).is_ok();
    let mut s = SessionFile::new();
    s.nodes.push(node(0, &[], load_op(&wav)));
    let out = replay(&s, &ReplayOptions::default(), &mut |_| {}, &|| false).unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    if readable {
        return; // Running with privileges that ignore permissions.
    }
    let msg = &out.issues[0].message;
    assert!(msg.contains("cannot access"), "{msg}");
    assert!(!msg.contains("no longer exists"), "{msg}");
}

#[test]
fn non_finite_measurements_are_refused_at_save_and_the_old_file_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("session.json");
    let mut s = SessionFile::new();
    save(&file, &s).unwrap();
    let before = std::fs::read_to_string(&file).unwrap();

    s.measurements.push(MeasurementRecord {
        kind: MeasurementKind::BaudRate,
        measurement: Measurement {
            value: f64::NAN,
            uncertainty: None,
            source: None,
            tool: "measure line".into(),
        },
    });
    let e = save(&file, &s).unwrap_err();
    let msg = e.to_string();
    assert!(msg.contains("measure line") && msg.contains("NaN"), "{msg}");
    assert!(!msg.contains("not a valid"), "{msg}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), before);

    s.measurements[0].measurement.value = 1.0;
    s.measurements[0].measurement.uncertainty = Some(f64::INFINITY);
    assert!(s.to_json().is_err());
}

#[test]
fn save_replaces_the_file_and_leaves_no_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("session.json");
    let mut s = SessionFile::new();
    save(&file, &s).unwrap();
    s.generator = "second".into();
    save(&file, &s).unwrap();
    assert_eq!(load(&file).unwrap().generator, "second");
    let names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, vec![std::ffi::OsString::from("session.json")]);
    // A failed write reports the target path.
    let e = save(dir.path().join("no-such-dir").join("s.json"), &s).unwrap_err();
    assert!(e.to_string().contains("s.json"), "{e}");
}

#[test]
fn newer_version_reports_the_number_in_the_file() {
    let text = json!({"format": "sa-rs-session", "version": 1_u64 << 40, "nodes": []});
    let e = SessionFile::from_json(&text.to_string()).unwrap_err();
    assert!(e.to_string().contains(&(1_u64 << 40).to_string()), "{e}");
}

#[test]
fn capture_can_skip_probing_and_probe_relative_paths_later() {
    let dir = tempfile::tempdir().unwrap();
    write_psk(dir.path(), "rel.wav");
    let mut g = SignalGraph::new();
    let sig = sn_io::open(dir.path().join("rel.wav"), &LoadOptions::default()).unwrap();
    g.add_source(
        "rel.wav",
        Operation::new("load", json!({"path": "rel.wav"})),
        sig,
    );
    let mut s =
        SessionFile::capture_with(&g, &MeasurementContext::new(), &CaptureOptions::default());
    assert!(s.nodes[0].source.is_none());
    s.probe_sources(Some(dir.path()));
    let src = s.nodes[0].source.as_ref().unwrap();
    assert_eq!(src.container, "Wav");
    assert_eq!(
        src.bytes,
        Some(std::fs::metadata(dir.path().join("rel.wav")).unwrap().len())
    );
}

#[test]
fn report_says_when_stored_results_cannot_be_read() {
    let mut s = SessionFile::new();
    let mut n = node(0, &[], synth_op(&json!({})));
    n.ui.insert(UI_KEY_ADP.into(), json!([{"baud": "fast"}]));
    n.ui.insert(UI_KEY_TONE_GRID.into(), json!({"size": "big"}));
    s.nodes.push(n);
    for text in [report::markdown(&s), report::html(&s)] {
        assert!(!text.contains("No ADP results were stored"), "{text}");
        assert!(!text.contains("No tone grids were stored"), "{text}");
        assert!(
            text.contains("ADP results is stored but could not be read"),
            "{text}"
        );
        assert!(
            text.contains("tone grid is stored but could not be read"),
            "{text}"
        );
    }
}

#[test]
fn markdown_escapes_user_text() {
    let mut s = SessionFile::new();
    let mut n = node(0, &[], Operation::new("load", json!({"path": "a`b|c.wav"})));
    n.label = "line one\nline `two` *x*".into();
    s.nodes.push(n);
    s.nodes
        .push(node(1, &[0], Operation::new("edit", json!({"op": "x`y"}))));
    s.measurements.push(MeasurementRecord {
        kind: MeasurementKind::Custom("my\\kind".into()),
        measurement: Measurement {
            value: 1.0,
            uncertainty: None,
            source: None,
            tool: "t`".into(),
        },
    });
    let md = report::markdown(&s);
    assert!(!md.contains("line one\n"), "{md}");
    assert!(md.contains(r"line one line \`two\` \*x\*"), "{md}");
    assert!(md.contains(r"a\`b\|c.wav"), "{md}");
    assert!(md.contains(r"my\\kind"), "{md}");
    assert!(md.contains(r"t\`"), "{md}");
    // Code spans from the report itself stay intact and contain no backtick.
    assert!(md.contains(r#"edit `{"op":"x'y"}`"#), "{md}");
    // Both table rows that hold user text keep their six columns.
    for needle in [r"a\`b", r"my\\kind"] {
        let row = md.lines().find(|l| l.contains(needle)).unwrap();
        assert_eq!(row.replace(r"\|", "").matches('|').count(), 7, "{row}");
    }
}

#[test]
fn save_is_atomic_and_leaves_other_files_alone() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.json");
    // A user file that happens to have the old temporary name must survive.
    let neighbour = dir.path().join("s.json.tmp");
    std::fs::write(&neighbour, b"keep me").unwrap();
    std::fs::write(&path, b"old").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

    let session = SessionFile::new();
    // Several saves at once must each produce a whole, valid file.
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| save(&path, &session).unwrap());
        }
    });
    load(&path).unwrap();
    assert_eq!(std::fs::read(&neighbour).unwrap(), b"keep me");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
    // No temporary files are left behind.
    let names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let mut names = names;
    names.sort();
    assert_eq!(names, ["s.json", "s.json.tmp"]);
}

/// Specs for the generator replay tests: linear, FSK and OFDM, one of them through a
/// channel with every impairment.
fn generator_specs() -> Vec<sn_synth::GeneratorSpec> {
    use sn_synth::{GeneratorSpec, Length, NoiseLevel, SignalKind, Tap};
    let mut linear = GeneratorSpec::default_for(SignalKind::Linear);
    linear.common_mut().length = Length::Seconds(0.5);
    linear.common_mut().seed = 42;
    let mut fsk = GeneratorSpec::preset("mfsk_11").unwrap();
    fsk.common_mut().length = Length::Symbols(40);
    let mut ofdm = GeneratorSpec::preset("ofdm_lu64_lg20_7000").unwrap();
    ofdm.common_mut().length = Length::Seconds(0.4);
    let channel = &mut ofdm.common_mut().channel;
    channel.frequency_offset = 7.5;
    channel.drift = 0.5;
    channel.multipath = vec![
        Tap {
            delay: 0.0,
            gain: 1.0,
            phase: 0.0,
        },
        Tap {
            delay: 0.001,
            gain: 0.3,
            phase: 1.0,
        },
    ];
    channel.clock_ppm = 50.0;
    channel.noise = Some(NoiseLevel::Snr {
        db: 15.0,
        bandwidth: None,
    });
    channel.clip = Some(0.9);
    vec![linear, fsk, ofdm]
}

#[test]
fn generated_signals_replay_bit_for_bit_through_a_saved_session() {
    let dir = tempfile::tempdir().unwrap();
    let mut g = SignalGraph::new();
    let mut expected = Vec::new();
    for spec in generator_specs() {
        let made = sn_synth::generate(&spec).unwrap();
        let op = sn_session::generate_operation(&spec, &made.description).unwrap();
        assert_eq!(op.name, sn_session::OP_GENERATE);
        g.add_source(made.description.clone(), op, made.signal.clone());
        expected.push((made.description, bits(&made.signal)));
    }
    let s = SessionFile::capture(&g, &MeasurementContext::new());
    let path = dir.path().join("gen.sarsession");
    save(&path, &s).unwrap();
    let back = load(&path).unwrap();
    let out = replay(&back, &ReplayOptions::default(), &mut |_| {}, &|| false).unwrap();
    assert!(out.issues.is_empty(), "{:?}", out.issues);
    for (i, (description, want)) in expected.iter().enumerate() {
        let id = out.new_id(NodeId::from_raw(i as u32)).unwrap();
        let got = out.graph.get(id).unwrap();
        assert_eq!(&got.label, description);
        assert_eq!(&bits(&got.signal), want, "{description}");
    }
    // The report names the generator's description.
    let md = report::markdown(&back);
    for (description, _) in &expected {
        assert!(md.contains(&format!("generated: {description}")), "{md}");
    }
}

#[test]
fn bad_generator_specs_are_reported_not_allocated() {
    let mut spec = sn_synth::GeneratorSpec::default_for(sn_synth::SignalKind::Tones);
    spec.common_mut().length = sn_synth::Length::Seconds(1e12);
    let out = replay_one(sn_session::generate_operation(&spec, "huge").unwrap());
    assert_eq!(out.failed_count(), 1);
    assert!(out.issues[0].message.contains("2^28"), "{:?}", out.issues);
    let out = replay_one(Operation::new(
        sn_session::OP_GENERATE,
        json!({"spec": {"kind": "warp"}}),
    ));
    assert_eq!(out.failed_count(), 1);
    assert!(
        out.issues[0].message.contains("generator parameters"),
        "{:?}",
        out.issues
    );
}

/// Values whose shortest decimal form `serde_json` reads back one ULP off unless its
/// `float_roundtrip` feature is on.
#[test]
fn generator_floats_survive_save_and_load_exactly() {
    use sn_synth::{GeneratorSpec, Length, SignalKind, ToneSpec};
    let mut spec = GeneratorSpec::default_for(SignalKind::Tones);
    spec.common_mut().length = Length::Seconds(0.1 + 0.2);
    if let GeneratorSpec::Tones(t) = &mut spec {
        t.tones = vec![ToneSpec {
            frequency: 1_451.186_815_982_315_5,
            amplitude: 0.3,
            phase: 0.1 + 0.7,
        }];
    } else {
        panic!("default_for(Tones) is not a tones spec");
    }
    let made = sn_synth::generate(&spec).unwrap();
    let mut g = SignalGraph::new();
    g.add_source(
        "tone",
        sn_session::generate_operation(&spec, &made.description).unwrap(),
        made.signal.clone(),
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("f.sarsession");
    save(&path, &SessionFile::capture(&g, &MeasurementContext::new())).unwrap();
    let back = load(&path).unwrap();
    let p: sn_session::GenerateParams =
        serde_json::from_value(back.nodes[0].operation.params.clone()).unwrap();
    assert_eq!(p.spec, spec);
    let out = replay(&back, &ReplayOptions::default(), &mut |_| {}, &|| false).unwrap();
    assert!(out.issues.is_empty(), "{:?}", out.issues);
    let id = out.new_id(NodeId::from_raw(0)).unwrap();
    assert_eq!(bits(&out.graph.get(id).unwrap().signal), bits(&made.signal));
}

#[test]
fn non_finite_generator_settings_are_refused_not_saved_as_null() {
    let mut spec = sn_synth::GeneratorSpec::default_for(sn_synth::SignalKind::Tones);
    spec.common_mut().channel.frequency_offset = f64::NAN;
    match sn_session::generate_operation(&spec, "nan") {
        Err(Error::Unsaveable(m)) => assert!(m.contains("cannot be recorded"), "{m}"),
        other => panic!("expected Unsaveable, got {other:?}"),
    }
}

#[test]
fn report_names_generators_without_a_description_or_spec() {
    let mut s = SessionFile::new();
    s.nodes.push(node(
        0,
        &[],
        Operation::new(
            sn_session::OP_GENERATE,
            json!({"spec": {"kind": "tones", "sample_rate": 8000.0}}),
        ),
    ));
    s.nodes.push(node(
        1,
        &[],
        Operation::new(sn_session::OP_GENERATE, json!({})),
    ));
    let md = report::markdown(&s);
    assert!(md.contains("generated: tones signal"), "{md}");
    assert!(md.contains("generated: (no description recorded)"), "{md}");
    assert!(md.contains("(no spec recorded)"), "{md}");
    assert!(!md.contains("``"), "empty code span in {md}");
}
