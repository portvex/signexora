//! Script engine tests: the Phase 8 exit criterion and error handling.

// Helpers outside #[test] functions are test code too; the no-panic rule is for the library.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use sn_core::{Complex32, NodeId, SampleKind, Signal};
use sn_dsp::ops::Edit;
use sn_io::{LoadOptions, WavFormat};
use sn_script::{ScriptEngine, ScriptError, ScriptOptions};
use sn_session::{ReplayOptions, replay};

fn write_psk(dir: &Path) -> PathBuf {
    let p = sn_synth::PskParams {
        order: 2,
        baud: 1200.0,
        carrier: 1800.0,
        sample_rate: 48_000.0,
        amplitude: 0.5,
    };
    let s = sn_synth::psk(&p, &sn_synth::random_symbols(2400, 2, 11)).unwrap();
    let path = dir.join("rec.wav");
    sn_io::write_wav(&path, &s, WavFormat::Pcm16).unwrap();
    path
}

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

fn quiet(dir: &Path) -> (ScriptOptions, Rc<RefCell<Vec<String>>>) {
    let printed = Rc::new(RefCell::new(Vec::new()));
    let sink = printed.clone();
    let options = ScriptOptions {
        base_dir: Some(dir.to_path_buf()),
        print: Some(Rc::new(move |s: &str| sink.borrow_mut().push(s.to_owned()))),
        ..Default::default()
    };
    (options, printed)
}

const CHAIN: &str = r#"
let s = open("rec.wav");
let a = edit(s, #{op: "band_select", lo: 600, hi: 3000});
let b = edit(a, `{"op":"shift","hz":-1800}`);
let c = edit(b, #{op: "resample", rate: 12000.0});
let i = info(c);
print(`rate ${i.rate} length ${i.length} kind ${i.kind}`);
save_wav(c, "out.wav");
"#;

#[test]
fn script_chain_matches_direct_edits_and_replay() {
    let dir = tempfile::tempdir().unwrap();
    let wav = write_psk(dir.path());
    let (options, printed) = quiet(dir.path());
    let mut engine = ScriptEngine::new(options);
    engine.run(CHAIN).unwrap();
    assert_eq!(engine.node_count(), 4);
    assert_eq!(printed.borrow().len(), 1);
    assert!(printed.borrow()[0].starts_with("rate 12000.0 length"));
    assert_eq!(engine.output(), *printed.borrow());

    // Reference: the same edits run directly.
    let mut sig = sn_io::open(&wav, &LoadOptions::default()).unwrap();
    for e in [
        Edit::BandSelect {
            lo: 600.0,
            hi: 3000.0,
        },
        Edit::Shift { hz: -1800.0 },
        Edit::Resample { rate: 12_000.0 },
    ] {
        sig = e.run(&sig, &|| false).unwrap();
    }
    let scripted = engine.signal(NodeId::from_raw(3)).unwrap();
    assert_eq!(bits(&scripted), bits(&sig));

    // The saved WAV is 32-bit float, so it holds the same samples.
    let saved = sn_io::open(dir.path().join("out.wav"), &LoadOptions::default()).unwrap();
    assert_eq!(bits(&saved), bits(&sig));

    // The script exported as a session replays to the same samples.
    let session = engine.session();
    assert_eq!(session.nodes.len(), 4);
    let out = replay(&session, &ReplayOptions::default(), &mut |_| {}, &|| false).unwrap();
    assert!(out.issues.is_empty(), "{:?}", out.issues);
    let replayed = &out
        .graph
        .get(out.new_id(NodeId::from_raw(3)).unwrap())
        .unwrap()
        .signal;
    assert_eq!(bits(replayed), bits(&sig));
}

#[test]
fn analysis_functions_record_measurements() {
    let dir = tempfile::tempdir().unwrap();
    write_psk(dir.path());
    let (options, printed) = quiet(dir.path());
    let mut engine = ScriptEngine::new(options);
    engine
        .run(
            r#"
            let s = open("rec.wav");
            let c = adp(s);
            print(c[0].baud);
            let sq = edit(s, #{op: "power", n: 2});
            let line = measure_line(sq, 3500, 3700);
            print(line.frequency);
            record(s, "Sc", line.frequency / 2);
            let meta = save_sigmf(s, "copy", "test copy");
            print(meta);
            print(s);
            "#,
        )
        .unwrap();
    let out = printed.borrow();
    let baud: f64 = out[0].parse().unwrap();
    assert!((baud - 1200.0).abs() < 1.0, "baud {baud}");
    let line: f64 = out[1].parse().unwrap();
    assert!((line - 3600.0).abs() < 0.5, "line {line}");
    assert!(out[2].ends_with("copy.sigmf-meta"));
    assert_eq!(out[3], "Signal #0 (rec.wav)");
    let session = engine.session();
    assert_eq!(session.measurements.len(), 3);
    assert!(session.nodes[0].ui.contains_key(sn_session::UI_KEY_ADP));
    // Measurements carry stable tool ids, not free text, so they read the same
    // in every interface language.
    let tools: Vec<&str> = session
        .measurements
        .iter()
        .map(|m| m.measurement.tool.as_str())
        .collect();
    assert!(
        tools
            .iter()
            .any(|t| *t == sn_session::tool_id::ADP || *t == sn_session::tool_id::ADP_COARSE),
        "{tools:?}"
    );
    assert!(tools.contains(&sn_script::TOOL_SCRIPT_RECORD), "{tools:?}");
    let adp = session
        .measurements
        .iter()
        .find(|m| m.measurement.tool.starts_with("adp"))
        .unwrap();
    if adp.measurement.tool == sn_session::tool_id::ADP_COARSE {
        assert!(adp.measurement.uncertainty.is_some_and(|u| u > 0.0));
    }
    let md = sn_session::report::markdown(&session);
    assert!(md.contains("Auto define params"), "{md}");
}

fn run_err(script: &str) -> ScriptError {
    let dir = tempfile::tempdir().unwrap();
    write_psk(dir.path());
    let (mut options, _) = quiet(dir.path());
    options.max_operations = 100_000;
    ScriptEngine::new(options).run(script).unwrap_err()
}

#[test]
fn script_errors_are_errors_not_panics() {
    let e = run_err(r#"let s = open("rec.wav"); edit(s, "{not json");"#);
    assert!(matches!(e, ScriptError::Runtime(_)), "{e:?}");
    assert!(e.to_string().contains("not valid JSON"), "{e}");

    let e = run_err(r#"open("missing.wav");"#);
    assert!(e.to_string().contains("does not exist"), "{e}");
    assert!(e.to_string().contains("missing.wav"), "{e}");

    let e = run_err(r#"let s = open("rec.wav"); edit(s, #{op: "no_such_op"});"#);
    assert!(e.to_string().contains("not a valid edit"), "{e}");

    let e = run_err(r#"let s = open("rec.wav"); edit(s, #{op: "resample", rate: -5});"#);
    assert!(e.to_string().contains("failed"), "{e}");

    let e = run_err(r#"let s = open("rec.wav"); measure_line(s, "a", 3);"#);
    assert!(e.to_string().contains("must be a number"), "{e}");

    let e = run_err(r#"let s = open("rec.wav"); adp(edit(s, #{op: "copy", start: 0, end: 100}));"#);
    assert!(matches!(e, ScriptError::Runtime(_)), "{e:?}");
    assert!(e.to_string().contains("at least 1024 samples"), "{e}");
    assert!(!e.to_string().contains("marker"), "{e}");

    let e = run_err(r#"let s = open("rec.wav"); measure_line(s, 3000, 3000);"#);
    assert!(e.to_string().contains("must differ"), "{e}");
    assert!(!e.to_string().contains("marker"), "{e}");

    let e = run_err(r#"let s = open("rec.wav"); measure_line(s, 1000, 2000, 0, 10);"#);
    assert!(e.to_string().contains("at least 64 samples"), "{e}");

    let e = run_err(r#"let s = open("rec.wav"); adp(s, 100, 50);"#);
    assert!(e.to_string().contains("must be less than end"), "{e}");

    let e = run_err(r#"let s = open("rec.wav"); adp(s, -1, 5000);"#);
    assert!(e.to_string().contains("0 or more"), "{e}");

    let e = run_err(r#"let s = open("rec.wav"); adp(s, 0, 100000000);"#);
    assert!(e.to_string().contains("past the end"), "{e}");

    let e = run_err(r#"let s = open("rec.wav"); record(s, "Br", 0.0 / 0.0);"#);
    assert!(e.to_string().contains("finite number"), "{e}");

    let e = run_err(r#"let s = open("rec.wav"); save_wav(s, "no_such_dir/out.wav");"#);
    assert!(matches!(e, ScriptError::Runtime(_)), "{e:?}");
    assert!(e.to_string().contains("cannot write"), "{e}");
    assert!(e.to_string().contains("no_such_dir"), "{e}");

    let e = run_err("let x = ;");
    assert!(matches!(e, ScriptError::Syntax(_)), "{e:?}");

    let e = run_err("loop { }");
    assert!(
        matches!(e, ScriptError::TooManyOperations(100_000)),
        "{e:?}"
    );

    let e = run_err(r#"eval("1 + 1")"#);
    assert!(
        matches!(e, ScriptError::Syntax(_)),
        "eval is disabled: {e:?}"
    );

    let e = run_err(r#"import "x" as y;"#);
    assert!(matches!(e, ScriptError::Runtime(_)), "{e:?}");
    assert!(e.to_string().contains("cannot import modules"), "{e}");

    let e = run_err(r#"let s = "x"; loop { s += s; }"#);
    assert!(matches!(e, ScriptError::Runtime(_)), "string limit: {e:?}");

    let e = run_err("throw \"custom stop\";");
    assert!(e.to_string().contains("custom stop"), "{e}");

    let e = run_err("info(42)");
    assert!(matches!(e, ScriptError::Runtime(_)), "{e:?}");
}

#[test]
fn scripts_can_be_cancelled() {
    let dir = tempfile::tempdir().unwrap();
    let stop = Arc::new(AtomicBool::new(true));
    let flag = stop.clone();
    let (mut options, _) = quiet(dir.path());
    options.should_stop = Some(Arc::new(move || flag.load(Ordering::Relaxed)));
    let e = ScriptEngine::new(options).run("loop { }").unwrap_err();
    assert!(matches!(e, ScriptError::Cancelled), "{e:?}");
}

#[test]
fn run_file_resolves_paths_against_the_script() {
    let dir = tempfile::tempdir().unwrap();
    write_psk(dir.path());
    let script = dir.path().join("chain.rhai");
    std::fs::write(&script, CHAIN).unwrap();
    let mut engine = ScriptEngine::new(ScriptOptions {
        print: Some(Rc::new(|_: &str| {})),
        ..Default::default()
    });
    engine.run_file(&script).unwrap();
    assert!(dir.path().join("out.wav").exists());
    let e = engine.run_file(dir.path().join("nope.rhai")).unwrap_err();
    assert!(matches!(e, ScriptError::Io { .. }));
}

#[test]
fn measure_line_and_adp_take_a_sample_range() {
    let dir = tempfile::tempdir().unwrap();
    write_psk(dir.path());
    let (options, printed) = quiet(dir.path());
    let mut engine = ScriptEngine::new(options);
    engine
        .run(
            r#"
            let s = open("rec.wav");
            print(adp(s, 0, 48000)[0].baud);
            let sq = edit(s, #{op: "power", n: 2});
            print(measure_line(sq, 3500, 3700, 1000, 30000).frequency);
            "#,
        )
        .unwrap();
    let out = printed.borrow();
    let baud: f64 = out[0].parse().unwrap();
    assert!((baud - 1200.0).abs() < 1.0, "baud {baud}");
    let line: f64 = out[1].parse().unwrap();
    assert!((line - 3600.0).abs() < 1.0, "line {line}");
    let session = engine.session();
    assert!(
        session
            .measurements
            .iter()
            .any(|m| m.measurement.tool.contains("samples 1000..30000")),
    );
}

#[test]
fn node_limit_is_enforced() {
    let dir = tempfile::tempdir().unwrap();
    write_psk(dir.path());
    let (mut options, _) = quiet(dir.path());
    options.max_nodes = 2;
    let mut engine = ScriptEngine::new(options);
    let e = engine
        .run(
            r#"
            let s = open("rec.wav");
            let a = edit(s, #{op: "shift", hz: 10});
            let b = edit(a, #{op: "shift", hz: 10});
            "#,
        )
        .unwrap_err();
    assert!(e.to_string().contains("at most 2 signals"), "{e}");
    assert_eq!(engine.node_count(), 2);
}

/// A stop flag armed by the script's first `print`, that turns true on the
/// `after`-th poll since then.
fn stop_after_print(after: usize, options: &mut ScriptOptions) -> Arc<AtomicUsize> {
    let armed = Arc::new(AtomicBool::new(false));
    let polls = Arc::new(AtomicUsize::new(0));
    let arm = armed.clone();
    options.print = Some(Rc::new(move |_: &str| arm.store(true, Ordering::Relaxed)));
    let p = polls.clone();
    options.should_stop = Some(Arc::new(move || {
        armed.load(Ordering::Relaxed) && p.fetch_add(1, Ordering::Relaxed) + 1 >= after
    }));
    polls
}

#[test]
fn cancel_inside_adp_is_a_cancellation() {
    let dir = tempfile::tempdir().unwrap();
    write_psk(dir.path());
    let (mut options, _) = quiet(dir.path());
    // Reaching adp takes a handful of polls after the print; adp itself polls hundreds
    // of times, so the 50th poll lands inside it.
    let polls = stop_after_print(50, &mut options);
    let mut engine = ScriptEngine::new(options);
    let e = engine
        .run(r#"let s = open("rec.wav"); print("go"); adp(s); print("done");"#)
        .unwrap_err();
    assert!(matches!(e, ScriptError::Cancelled), "{e:?}");
    assert!(polls.load(Ordering::Relaxed) >= 50);
    // adp did not finish, so nothing was recorded.
    assert_eq!(engine.session().measurements.len(), 0);
}

#[test]
fn cancel_inside_measure_line_and_edit_is_a_cancellation() {
    let dir = tempfile::tempdir().unwrap();
    write_psk(dir.path());
    for script in [
        r#"let s = open("rec.wav"); print("go"); measure_line(s, 1000, 2000);"#,
        r#"let s = open("rec.wav"); print("go"); edit(s, #{op: "band_select", lo: 300, hi: 3000});"#,
    ] {
        let (mut options, _) = quiet(dir.path());
        // Reaching the call takes a few polls after the print; both calls poll about
        // 20 to 30 times, so the 10th poll lands inside them.
        stop_after_print(10, &mut options);
        let e = ScriptEngine::new(options).run(script).unwrap_err();
        assert!(matches!(e, ScriptError::Cancelled), "{script}: {e:?}");
    }
}

#[test]
fn print_output_is_capped() {
    let dir = tempfile::tempdir().unwrap();
    let (options, printed) = quiet(dir.path());
    let mut engine = ScriptEngine::new(options);
    let n = sn_script::MAX_OUTPUT_LINES + 500;
    engine
        .run(&format!("for i in 0..{n} {{ print(i); }}"))
        .unwrap();
    let out = engine.output();
    assert_eq!(out.len(), sn_script::MAX_OUTPUT_LINES + 1);
    assert!(
        out.last().unwrap().contains("output truncated"),
        "{:?}",
        out.last()
    );
    assert_eq!(*printed.borrow(), out);

    // The limit is per run.
    engine.run(r#"print("again");"#).unwrap();
    assert_eq!(engine.output().last().unwrap(), "again");

    // Large strings hit the byte limit long before the line limit.
    let mut engine = ScriptEngine::new(quiet(dir.path()).0);
    engine
        .run(r#"let s = "x"; while s.len() < 1000000 { s += s; } for i in 0..100 { print(s); }"#)
        .unwrap();
    let out = engine.output();
    let bytes: usize = out.iter().map(String::len).sum();
    assert!(bytes <= sn_script::MAX_OUTPUT_BYTES + 200, "{bytes}");
    assert!(out.last().unwrap().contains("output truncated"));
}

#[test]
fn debug_goes_to_the_print_callback() {
    let dir = tempfile::tempdir().unwrap();
    let (options, printed) = quiet(dir.path());
    let mut engine = ScriptEngine::new(options);
    engine.run(r#"debug("hello"); debug(42);"#).unwrap();
    assert_eq!(
        *printed.borrow(),
        vec![r#"[debug] "hello""#.to_owned(), "[debug] 42".to_owned()]
    );
    assert_eq!(engine.output(), *printed.borrow());
}

#[test]
fn run_after_run_file_does_not_keep_the_script_directory() {
    let dir = tempfile::tempdir().unwrap();
    write_psk(dir.path());
    let script = dir.path().join("chain.rhai");
    std::fs::write(&script, CHAIN).unwrap();
    let mut engine = ScriptEngine::new(ScriptOptions {
        print: Some(Rc::new(|_: &str| {})),
        ..Default::default()
    });
    engine.run_file(&script).unwrap();
    // `run` resolves against the current directory (the crate root under cargo test),
    // where rec.wav does not exist.
    let e = engine.run(r#"open("rec.wav");"#).unwrap_err();
    let cwd = std::env::current_dir().unwrap();
    assert!(
        e.to_string()
            .contains(&cwd.join("rec.wav").display().to_string()),
        "{e}"
    );
}
