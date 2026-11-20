//! Rhai scripting over the SigNexora operation API.
//!
//! A [`ScriptEngine`] runs [Rhai](https://rhai.rs) scripts against a private signal
//! graph. Every signal a script opens or derives becomes a node in that graph, so a
//! script run can be exported as a [`SessionFile`] with [`ScriptEngine::session`],
//! replayed, and turned into a report.
//!
//! Functions available to scripts:
//!
//! | Function | Returns |
//! |---|---|
//! | `open(path)` | a signal handle |
//! | `edit(sig, json_or_map)` | a new handle; the argument is an edit, e.g. `#{op: "band_select", lo: 300, hi: 3000}` |
//! | `measure_line(sig, lo, hi)` | map: `frequency`, `level_db`, `snr_db`, `uncertainty` |
//! | `measure_line(sig, lo, hi, start, end)` | the same, over samples `start..end` only |
//! | `adp(sig)` | array of maps: `baud`, `detector`, `score`, `refined` (false: coarse bin), best first |
//! | `adp(sig, start, end)` | the same, over samples `start..end` only |
//! | `record(sig, kind, value)` | stores a measurement (`"Br"`, `"Sc"`, `"dT"`, `"Sh"`, `"K"` or any name) |
//! | `save_wav(sig, path)` | nothing; writes 32-bit float WAV |
//! | `save_sigmf(sig, path, description)` | the metadata file path |
//! | `info(sig)` | map: `id`, `label`, `rate`, `length`, `duration`, `kind` |
//! | `print(x)` | nothing; output goes to the engine's print callback |
//! | `debug(x)` | nothing; like `print`, with a `[debug]` prefix |
//!
//! Relative paths are resolved against [`ScriptOptions::base_dir`]. Scripts cannot
//! import modules or use `eval`, and run under operation, size, depth and output
//! limits.
//! Every failure becomes a [`ScriptError`]; scripts cannot panic the host.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use rhai::{Array, Dynamic, Engine, EvalAltResult, Map, Position};
use sn_core::{
    Measurement, MeasurementContext, MeasurementKind, NodeId, Operation, SampleKind, SignalGraph,
};
use sn_dsp::ops::Edit;
use sn_io::{LoadOptions, Scaling, WavFormat};
use sn_session::{SessionFile, UI_KEY_ADP, tool_id, tool_id::adp_quality};

/// Tool id of a measurement a script stores with `record(sig, kind, value)`.
/// Like every tool id it is stored in the session as it is; the front ends show
/// the name under the Fluent key `measure-tool-script-record` and the report
/// the English name from `sn_session::tool_id::english_name`.
pub const TOOL_SCRIPT_RECORD: &str = tool_id::SCRIPT_RECORD;

/// Default limit on Rhai operations per run.
pub const DEFAULT_MAX_OPERATIONS: u64 = 50_000_000;
/// Default limit on signal nodes a script may create.
pub const DEFAULT_MAX_NODES: usize = 1000;
/// Longest string a script may build, in bytes.
pub const MAX_STRING_SIZE: usize = 1 << 20;
/// Largest array or map a script may build.
pub const MAX_COLLECTION_SIZE: usize = 100_000;
/// Most `print`/`debug` lines kept and forwarded per run. Past this (or past
/// [`MAX_OUTPUT_BYTES`]) one "output truncated" line is added and later output of
/// that run is dropped.
pub const MAX_OUTPUT_LINES: usize = 10_000;
/// Most bytes of `print`/`debug` output kept and forwarded per run.
pub const MAX_OUTPUT_BYTES: usize = 16 << 20;
/// Fewest samples `measure_line` measures.
const LINE_MIN_SAMPLES: usize = 64;
/// Fewest samples `adp` analyses.
const ADP_MIN_SAMPLES: usize = 1024;

/// Errors from running a script.
#[derive(Debug, thiserror::Error)]
pub enum ScriptError {
    /// The script file could not be read.
    #[error("cannot read script {path}: {source}")]
    Io {
        /// Script path.
        path: String,
        /// Underlying error.
        source: std::io::Error,
    },
    /// The script has a syntax error.
    #[error("syntax error in the script: {0}")]
    Syntax(String),
    /// The script failed while running.
    #[error("script error: {0}")]
    Runtime(String),
    /// The script ran longer than the operation limit allows.
    #[error(
        "the script exceeded its limit of {0} operations. Simplify it, or raise the limit \
         with ScriptOptions::max_operations"
    )]
    TooManyOperations(u64),
    /// The run was stopped through the `should_stop` callback.
    #[error("the script was cancelled")]
    Cancelled,
    /// A bug in SigNexora was hit while running the script.
    #[error("internal error while running the script (please report this as a bug)")]
    Internal,
}

type StopFn = Arc<dyn Fn() -> bool + Send + Sync>;
type PrintFn = Rc<dyn Fn(&str)>;

/// Options for a [`ScriptEngine`].
#[derive(Clone)]
pub struct ScriptOptions {
    /// Directory that relative paths in the script are resolved against. `None` uses
    /// the script file's directory in [`ScriptEngine::run_file`], else the current
    /// directory.
    pub base_dir: Option<PathBuf>,
    /// Maximum Rhai operations per run (loops, calls, expressions).
    pub max_operations: u64,
    /// Maximum signal nodes a script may create.
    pub max_nodes: usize,
    /// Receives every `print` and `debug` line from the script, up to the output
    /// limits ([`MAX_OUTPUT_LINES`], [`MAX_OUTPUT_BYTES`]). `None` writes to standard
    /// output.
    pub print: Option<PrintFn>,
    /// Polled during the run and inside long operations; return true to cancel.
    pub should_stop: Option<StopFn>,
}

impl Default for ScriptOptions {
    fn default() -> Self {
        Self {
            base_dir: None,
            max_operations: DEFAULT_MAX_OPERATIONS,
            max_nodes: DEFAULT_MAX_NODES,
            print: None,
            should_stop: None,
        }
    }
}

impl fmt::Debug for ScriptOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScriptOptions")
            .field("base_dir", &self.base_dir)
            .field("max_operations", &self.max_operations)
            .field("max_nodes", &self.max_nodes)
            .field("print", &self.print.is_some())
            .field("should_stop", &self.should_stop.is_some())
            .finish()
    }
}

/// A handle to a signal node, as seen by scripts (type name `Signal`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalHandle {
    /// The node in the engine's graph.
    pub node: NodeId,
}

#[derive(Default)]
struct State {
    graph: SignalGraph,
    measures: MeasurementContext,
    ui: HashMap<NodeId, serde_json::Map<String, serde_json::Value>>,
    output: Vec<String>,
    /// Lines and bytes printed in the current run, for the output limits.
    run_lines: usize,
    run_bytes: usize,
    /// The current run hit an output limit; later output is dropped.
    truncated: bool,
    base_dir: Option<PathBuf>,
}

type Shared = Rc<RefCell<State>>;
type RhaiResult<T> = Result<T, Box<EvalAltResult>>;

/// Runs Rhai scripts against its own signal graph.
pub struct ScriptEngine {
    engine: Engine,
    state: Shared,
    options: ScriptOptions,
}

impl fmt::Debug for ScriptEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScriptEngine")
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

impl ScriptEngine {
    /// A new engine with an empty graph.
    pub fn new(options: ScriptOptions) -> Self {
        let state: Shared = Rc::new(RefCell::new(State {
            base_dir: options.base_dir.clone(),
            ..State::default()
        }));
        let mut engine = Engine::new();
        configure_limits(&mut engine, &options);
        register_api(&mut engine, &state, &options);
        Self {
            engine,
            state,
            options,
        }
    }

    /// Run a script given as text. Relative paths are resolved against
    /// [`ScriptOptions::base_dir`], else the current directory.
    pub fn run(&mut self, script: &str) -> Result<(), ScriptError> {
        if let Ok(mut st) = self.state.try_borrow_mut() {
            st.run_lines = 0;
            st.run_bytes = 0;
            st.truncated = false;
        }
        let ast = self
            .engine
            .compile(script)
            .map_err(|e| ScriptError::Syntax(e.to_string()))?;
        let engine = &self.engine;
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| engine.run_ast(&ast)));
        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(self.map_error(&e)),
            Err(_) => Err(ScriptError::Internal),
        }
    }

    /// Run a script file. Relative paths in the script are resolved against the
    /// script's directory unless [`ScriptOptions::base_dir`] is set.
    pub fn run_file(&mut self, path: impl AsRef<Path>) -> Result<(), ScriptError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| ScriptError::Io {
            path: path.display().to_string(),
            source,
        })?;
        if self.options.base_dir.is_some() {
            return self.run(&text);
        }
        // Use the script's directory for this run only, then put the old value back so
        // a later `run` resolves against the current directory again.
        let script_dir = path.parent().map(Path::to_path_buf);
        let previous = match self.state.try_borrow_mut() {
            Ok(mut st) => std::mem::replace(&mut st.base_dir, script_dir),
            Err(_) => return Err(ScriptError::Internal),
        };
        let result = self.run(&text);
        match self.state.try_borrow_mut() {
            Ok(mut st) => st.base_dir = previous,
            Err(_) => return Err(ScriptError::Internal),
        }
        result
    }

    /// Everything the scripts printed so far, one entry per `print` or `debug` call
    /// (each run limited by [`MAX_OUTPUT_LINES`] and [`MAX_OUTPUT_BYTES`]).
    pub fn output(&self) -> Vec<String> {
        self.state
            .try_borrow()
            .map(|s| s.output.clone())
            .unwrap_or_default()
    }

    /// Number of signal nodes created so far.
    pub fn node_count(&self) -> usize {
        self.state.try_borrow().map_or(0, |s| s.graph.len())
    }

    /// The signal of a node, for callers that want the samples.
    pub fn signal(&self, node: NodeId) -> Option<sn_core::Signal> {
        self.state
            .try_borrow()
            .ok()
            .and_then(|s| s.graph.get(node).map(|n| n.signal.clone()))
    }

    /// The work done so far as a session: the graph, the measurements, and per-node
    /// results (ADP candidates) in the UI state.
    pub fn session(&self) -> SessionFile {
        let Ok(st) = self.state.try_borrow() else {
            return SessionFile::new();
        };
        let mut session = SessionFile::capture(&st.graph, &st.measures);
        for (id, ui) in &st.ui {
            if let Some(map) = session.node_ui_mut(*id) {
                map.extend(ui.clone());
            }
        }
        session
    }

    /// Take the graph and measurements out of the engine, leaving it empty.
    pub fn into_parts(self) -> (SignalGraph, MeasurementContext) {
        match self.state.try_borrow_mut() {
            Ok(mut st) => (
                std::mem::take(&mut st.graph),
                std::mem::take(&mut st.measures),
            ),
            Err(_) => (SignalGraph::new(), MeasurementContext::new()),
        }
    }

    fn map_error(&self, e: &EvalAltResult) -> ScriptError {
        match innermost(e) {
            EvalAltResult::ErrorTerminated(..) => ScriptError::Cancelled,
            EvalAltResult::ErrorTooManyOperations(..) => {
                ScriptError::TooManyOperations(self.options.max_operations)
            }
            EvalAltResult::ErrorTooManyModules(pos) => ScriptError::Runtime(format!(
                "scripts cannot import modules; put all the code in one script (line {})",
                pos.line().unwrap_or(0)
            )),
            inner => {
                let mut msg = match inner {
                    EvalAltResult::ErrorRuntime(v, _) => v.to_string(),
                    other => other.to_string(),
                };
                let pos = e.position();
                if !pos.is_none() && !msg.contains("(line ") {
                    msg = format!("{msg} (line {})", pos.line().unwrap_or(0));
                }
                ScriptError::Runtime(msg)
            }
        }
    }
}

/// The error inside nested function-call errors.
fn innermost(e: &EvalAltResult) -> &EvalAltResult {
    let mut cur = e;
    while let EvalAltResult::ErrorInFunctionCall(_, _, inner, _) = cur {
        cur = inner;
    }
    cur
}

fn configure_limits(engine: &mut Engine, options: &ScriptOptions) {
    engine
        .set_max_operations(options.max_operations)
        .set_max_string_size(MAX_STRING_SIZE)
        .set_max_array_size(MAX_COLLECTION_SIZE)
        .set_max_map_size(MAX_COLLECTION_SIZE)
        .set_max_call_levels(64)
        .set_max_expr_depths(64, 32)
        .set_max_modules(0)
        .set_max_variables(10_000)
        .set_max_functions(1000);
    engine.set_module_resolver(rhai::module_resolvers::DummyModuleResolver::new());
    engine.disable_symbol("eval");
    if let Some(stop) = options.should_stop.clone() {
        engine.on_progress(move |_| {
            if stop() {
                Some(Dynamic::from("cancelled"))
            } else {
                None
            }
        });
    }
}

fn err<T>(msg: impl Into<String>) -> RhaiResult<T> {
    Err(Box::new(EvalAltResult::ErrorRuntime(
        Dynamic::from(msg.into()),
        Position::NONE,
    )))
}

fn borrow(state: &Shared) -> RhaiResult<std::cell::RefMut<'_, State>> {
    state
        .try_borrow_mut()
        .or_else(|_| err("the session is busy; signal functions cannot be nested"))
}

/// A finite number from a script value (integer or float).
fn number(v: &Dynamic, what: &str) -> RhaiResult<f64> {
    let x = if let Some(i) = v.clone().try_cast::<i64>() {
        i as f64
    } else if let Some(f) = v.clone().try_cast::<f64>() {
        f
    } else {
        return err(format!(
            "{what} must be a number, but a {} was given",
            v.type_name()
        ));
    };
    if !x.is_finite() {
        return err(format!("{what} must be a finite number, but {x} was given"));
    }
    Ok(x)
}

/// A sample index from a script value: a whole number, 0 or more.
fn sample_index(v: &Dynamic, what: &str) -> RhaiResult<usize> {
    match v.clone().try_cast::<i64>() {
        Some(i) => usize::try_from(i).or_else(|_| {
            err(format!(
                "{what} must be a sample index of 0 or more, but {i} was given"
            ))
        }),
        None => err(format!(
            "{what} must be a whole number of samples, but a {} was given",
            v.type_name()
        )),
    }
}

/// Check a sample range `start..end` of `signal` for a function that needs at least
/// `min` and at most `max` samples.
fn check_range(
    func: &str,
    signal: &sn_core::Signal,
    start: usize,
    end: usize,
    min: usize,
    max: usize,
) -> RhaiResult<()> {
    let len = signal.len();
    if end > len {
        return err(format!(
            "{func}: end {end} is past the end of the signal, which has {len} samples"
        ));
    }
    if start >= end {
        return err(format!(
            "{func}: start ({start}) must be less than end ({end})"
        ));
    }
    let n = end - start;
    if n < min {
        return err(format!(
            "{func} needs at least {min} samples, but {n} were given. Use a longer signal \
             or a wider start..end range"
        ));
    }
    if n > max {
        return err(format!(
            "{func} handles at most {max} samples, but {n} were given. Pass a start and \
             end sample to measure a part, or cut a shorter piece with \
             edit(sig, #{{op: \"copy\", start: 0, end: {max}}})"
        ));
    }
    Ok(())
}

/// A Rhai error for a failed analysis. A cancellation becomes a termination, so the
/// run ends with [`ScriptError::Cancelled`] rather than a script error.
fn analysis_err<T>(what: &str, e: &sn_analysis::Error) -> RhaiResult<T> {
    match e {
        sn_analysis::Error::Cancelled => terminated(),
        other => err(format!("{what} failed: {other}")),
    }
}

/// Ends the run as cancelled.
fn terminated<T>() -> RhaiResult<T> {
    Err(Box::new(EvalAltResult::ErrorTerminated(
        Dynamic::from("cancelled"),
        Position::NONE,
    )))
}

/// Store one output line and pass it to the print callback, within the per-run
/// output limits.
fn emit(state: &Shared, print: Option<&PrintFn>, line: &str) {
    let forward = match state.try_borrow_mut() {
        Ok(mut st) => {
            if st.truncated {
                None
            } else if st.run_lines >= MAX_OUTPUT_LINES
                || st.run_bytes.saturating_add(line.len()) > MAX_OUTPUT_BYTES
            {
                st.truncated = true;
                let note = format!(
                    "[output truncated: a run may print at most {MAX_OUTPUT_LINES} lines or \
                     {MAX_OUTPUT_BYTES} bytes]"
                );
                st.output.push(note.clone());
                Some(note)
            } else {
                st.run_lines += 1;
                st.run_bytes = st.run_bytes.saturating_add(line.len());
                st.output.push(line.to_owned());
                Some(line.to_owned())
            }
        }
        // Output is never produced while the state is borrowed; forward it anyway.
        Err(_) => Some(line.to_owned()),
    };
    if let Some(text) = forward {
        match print {
            Some(p) => p(&text),
            None => println!("{text}"),
        }
    }
}

impl State {
    fn resolve(&self, path: &str) -> PathBuf {
        let p = PathBuf::from(path);
        match &self.base_dir {
            Some(base) if p.is_relative() => base.join(p),
            _ => p,
        }
    }

    fn signal(&self, h: SignalHandle) -> RhaiResult<sn_core::Signal> {
        match self.graph.get(h.node) {
            Some(n) => Ok(n.signal.clone()),
            None => err(format!("signal {} does not exist in this script", h.node)),
        }
    }
}

fn kind_from_name(name: &str) -> MeasurementKind {
    match name {
        "Br" => MeasurementKind::BaudRate,
        "Sc" => MeasurementKind::Carrier,
        "dT" => MeasurementKind::Period,
        "Sh" => MeasurementKind::Spacing,
        "K" => MeasurementKind::OfdmK,
        other => MeasurementKind::Custom(other.to_owned()),
    }
}

#[allow(clippy::too_many_lines)] // one registration block per script function
fn register_api(engine: &mut Engine, state: &Shared, options: &ScriptOptions) {
    let max_nodes = options.max_nodes;
    let stop: StopFn = options
        .should_stop
        .clone()
        .unwrap_or_else(|| Arc::new(|| false));

    engine.register_type_with_name::<SignalHandle>("Signal");
    {
        let st = state.clone();
        engine.register_fn("to_string", move |h: &mut SignalHandle| -> String {
            describe(&st, *h)
        });
        let st = state.clone();
        engine.register_fn("to_debug", move |h: &mut SignalHandle| -> String {
            describe(&st, *h)
        });
    }

    {
        let st = state.clone();
        let print = options.print.clone();
        engine.on_print(move |s| emit(&st, print.as_ref(), s));
        let st = state.clone();
        let print = options.print.clone();
        engine.on_debug(move |s, _source, _pos| {
            emit(&st, print.as_ref(), &format!("[debug] {s}"));
        });
    }

    // open(path)
    let st = state.clone();
    engine.register_fn("open", move |path: &str| -> RhaiResult<SignalHandle> {
        let mut st = borrow(&st)?;
        if st.graph.len() >= max_nodes {
            return err(format!("a script may create at most {max_nodes} signals"));
        }
        // Record an absolute path, so the exported session replays from anywhere.
        let resolved = st.resolve(path);
        let resolved = std::path::absolute(&resolved).unwrap_or(resolved);
        if !resolved.exists() {
            return err(format!(
                "file {} does not exist. Check the path; relative paths are resolved against \
                 the script's directory",
                resolved.display()
            ));
        }
        let options = LoadOptions::default();
        let signal = match sn_io::open(&resolved, &options) {
            Ok(s) => s,
            Err(e) => return err(format!("cannot open {}: {e}", resolved.display())),
        };
        let label = resolved.file_name().map_or_else(
            || resolved.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let op = Operation::new(
            "load",
            serde_json::json!({
                "path": resolved.display().to_string(),
                "channels": options.channels,
                "scaling": Scaling::default(),
                "raw": options.raw,
            }),
        );
        let node = st.graph.add_source(label, op, signal);
        Ok(SignalHandle { node })
    });

    // edit(sig, json string) and edit(sig, map)
    let apply = {
        let st = state.clone();
        let stop = stop.clone();
        move |h: SignalHandle, params: serde_json::Value| -> RhaiResult<SignalHandle> {
            let edit: Edit = match serde_json::from_value(params.clone()) {
                Ok(e) => e,
                Err(e) => {
                    return err(format!(
                        "not a valid edit ({e}). An edit needs an \"op\" such as \
                         \"band_select\" and that operation's parameters, e.g. \
                         #{{op: \"band_select\", lo: 300, hi: 3000}}"
                    ));
                }
            };
            let input = match st.try_borrow() {
                Ok(s) => {
                    // Check the node limit before doing the (possibly long) edit.
                    if s.graph.len() >= max_nodes {
                        return err(format!("a script may create at most {max_nodes} signals"));
                    }
                    s.signal(h)?
                }
                Err(_) => return err("the session is busy; signal functions cannot be nested"),
            };
            // Run without holding the borrow, so print callbacks stay usable.
            let s = stop.clone();
            let out = match edit.run(&input, &move || s()) {
                Ok(o) => o,
                Err(sn_dsp::Error::Cancelled) => return terminated(),
                Err(e) => return err(format!("'{}' failed: {e}", edit.label())),
            };
            let mut st = borrow(&st)?;
            if st.graph.len() >= max_nodes {
                return err(format!("a script may create at most {max_nodes} signals"));
            }
            let params = serde_json::to_value(&edit).unwrap_or(params);
            match st
                .graph
                .add_derived(edit.label(), Operation::new("edit", params), &[h.node], out)
            {
                Ok(node) => Ok(SignalHandle { node }),
                Err(e) => err(e.to_string()),
            }
        }
    };
    {
        let apply = apply.clone();
        engine.register_fn(
            "edit",
            move |h: SignalHandle, json: &str| -> RhaiResult<SignalHandle> {
                let value: serde_json::Value = match serde_json::from_str(json) {
                    Ok(v) => v,
                    Err(e) => {
                        return err(format!(
                            "the edit is not valid JSON ({e}). Write it like \
                             '{{\"op\":\"band_select\",\"lo\":300,\"hi\":3000}}' or pass a \
                             map instead"
                        ));
                    }
                };
                apply(h, value)
            },
        );
    }
    engine.register_fn(
        "edit",
        move |h: SignalHandle, map: Map| -> RhaiResult<SignalHandle> {
            let value: serde_json::Value = rhai::serde::from_dynamic(&Dynamic::from_map(map))
                .or_else(|e| err(format!("the edit map cannot be read: {e}")))?;
            apply(h, value)
        },
    );

    // measure_line(sig, lo, hi) and measure_line(sig, lo, hi, start, end)
    let measure = {
        let st = state.clone();
        let stop = stop.clone();
        move |h: SignalHandle,
              lo: &Dynamic,
              hi: &Dynamic,
              range: Option<(&Dynamic, &Dynamic)>|
              -> RhaiResult<Map> {
            let lo = number(lo, "lo")?;
            let hi = number(hi, "hi")?;
            if lo.total_cmp(&hi).is_eq() {
                return err(format!(
                    "measure_line: lo and hi bound the band searched for the line, in Hz, so \
                     they must differ (both are {lo})"
                ));
            }
            let signal = borrow(&st)?.signal(h)?;
            let (start, end) = match range {
                Some((a, b)) => (sample_index(a, "start")?, sample_index(b, "end")?),
                None => (0, signal.len()),
            };
            check_range(
                "measure_line",
                &signal,
                start,
                end,
                LINE_MIN_SAMPLES,
                sn_analysis::line::MAX_SAMPLES,
            )?;
            let s = stop.clone();
            let e = sn_analysis::line::measure_line(&signal, start, end, lo, hi, &move || s())
                .or_else(|e| analysis_err("measure_line", &e))?;
            let whole = start == 0 && end == signal.len();
            let tool = if whole {
                format!("measure_line {lo}-{hi} Hz (script)")
            } else {
                format!("measure_line {lo}-{hi} Hz, samples {start}..{end} (script)")
            };
            borrow(&st)?
                .measures
                .record(
                    MeasurementKind::Custom("line frequency".into()),
                    Measurement {
                        value: e.frequency,
                        uncertainty: Some(e.uncertainty),
                        source: Some(h.node),
                        tool,
                    },
                )
                .or_else(|e| err(format!("measure_line gave an unusable result: {e}")))?;
            let mut m = Map::new();
            m.insert("frequency".into(), Dynamic::from(e.frequency));
            m.insert("level_db".into(), Dynamic::from(e.level_db));
            m.insert("snr_db".into(), Dynamic::from(e.snr_db));
            m.insert("uncertainty".into(), Dynamic::from(e.uncertainty));
            Ok(m)
        }
    };
    {
        let measure = measure.clone();
        engine.register_fn(
            "measure_line",
            move |h: SignalHandle, lo: Dynamic, hi: Dynamic| -> RhaiResult<Map> {
                measure(h, &lo, &hi, None)
            },
        );
    }
    engine.register_fn(
        "measure_line",
        move |h: SignalHandle,
              lo: Dynamic,
              hi: Dynamic,
              start: Dynamic,
              end: Dynamic|
              -> RhaiResult<Map> { measure(h, &lo, &hi, Some((&start, &end))) },
    );

    // adp(sig) and adp(sig, start, end)
    let adp = {
        let st = state.clone();
        let stop = stop.clone();
        move |h: SignalHandle, range: Option<(&Dynamic, &Dynamic)>| -> RhaiResult<Array> {
            let signal = borrow(&st)?.signal(h)?;
            let (start, end) = match range {
                Some((a, b)) => (sample_index(a, "start")?, sample_index(b, "end")?),
                None => (0, signal.len()),
            };
            check_range("adp", &signal, start, end, ADP_MIN_SAMPLES, usize::MAX)?;
            let s = stop.clone();
            let cands = sn_analysis::adp::estimate_baud(&signal, start, end, &move || s())
                .or_else(|e| analysis_err("adp", &e))?;
            let mut st = borrow(&st)?;
            if let Some(best) = cands.first() {
                let (uncertainty, tool) = adp_quality(best);
                st.measures
                    .record(
                        MeasurementKind::BaudRate,
                        Measurement {
                            value: best.baud,
                            uncertainty,
                            source: Some(h.node),
                            tool: tool.into(),
                        },
                    )
                    .or_else(|e| err(format!("adp gave an unusable result: {e}")))?;
            }
            if let Ok(v) = serde_json::to_value(&cands) {
                st.ui
                    .entry(h.node)
                    .or_default()
                    .insert(UI_KEY_ADP.to_owned(), v);
            }
            Ok(cands
                .iter()
                .map(|c| {
                    let mut m = Map::new();
                    m.insert("baud".into(), Dynamic::from(c.baud));
                    m.insert(
                        "detector".into(),
                        Dynamic::from(format!("{:?}", c.detector)),
                    );
                    m.insert("score".into(), Dynamic::from(c.score));
                    m.insert("refined".into(), Dynamic::from(c.refined));
                    Dynamic::from_map(m)
                })
                .collect())
        }
    };
    {
        let adp = adp.clone();
        engine.register_fn("adp", move |h: SignalHandle| -> RhaiResult<Array> {
            adp(h, None)
        });
    }
    engine.register_fn(
        "adp",
        move |h: SignalHandle, start: Dynamic, end: Dynamic| -> RhaiResult<Array> {
            adp(h, Some((&start, &end)))
        },
    );

    // record(sig, kind, value)
    let st = state.clone();
    engine.register_fn(
        "record",
        move |h: SignalHandle, kind: &str, value: Dynamic| -> RhaiResult<()> {
            let value = number(&value, "value")?;
            let mut st = borrow(&st)?;
            st.signal(h)?;
            st.measures
                .record(
                    kind_from_name(kind),
                    Measurement {
                        value,
                        uncertainty: None,
                        source: Some(h.node),
                        tool: TOOL_SCRIPT_RECORD.into(),
                    },
                )
                .or_else(|e| err(format!("cannot record the measurement: {e}")))
        },
    );

    // save_wav(sig, path)
    let st = state.clone();
    engine.register_fn(
        "save_wav",
        move |h: SignalHandle, path: &str| -> RhaiResult<()> {
            let st = borrow(&st)?;
            let signal = st.signal(h)?;
            let path = st.resolve(path);
            sn_io::write_wav(&path, &signal, WavFormat::Float32)
                .or_else(|e| err(format!("cannot write {}: {e}", path.display())))
        },
    );

    // save_sigmf(sig, path, description)
    let st = state.clone();
    engine.register_fn(
        "save_sigmf",
        move |h: SignalHandle, path: &str, desc: &str| -> RhaiResult<String> {
            let st = borrow(&st)?;
            let signal = st.signal(h)?;
            let path = st.resolve(path);
            sn_io::write_sigmf(&path, &signal, desc)
                .map(|p| p.display().to_string())
                .or_else(|e| err(format!("cannot write {}: {e}", path.display())))
        },
    );

    // info(sig)
    let st = state.clone();
    engine.register_fn("info", move |h: SignalHandle| -> RhaiResult<Map> {
        let st = borrow(&st)?;
        let Some(n) = st.graph.get(h.node) else {
            return err(format!("signal {} does not exist in this script", h.node));
        };
        let mut m = Map::new();
        m.insert("id".into(), Dynamic::from(i64::from(h.node.raw())));
        m.insert("label".into(), Dynamic::from(n.label.clone()));
        m.insert("rate".into(), Dynamic::from(n.signal.sample_rate()));
        m.insert(
            "length".into(),
            Dynamic::from(i64::try_from(n.signal.len()).unwrap_or(i64::MAX)),
        );
        m.insert("duration".into(), Dynamic::from(n.signal.duration()));
        let kind = match n.signal.kind() {
            SampleKind::Real => "real",
            SampleKind::Complex => "complex",
        };
        m.insert("kind".into(), Dynamic::from(kind.to_owned()));
        Ok(m)
    });
}

fn describe(state: &Shared, h: SignalHandle) -> String {
    let label = state
        .try_borrow()
        .ok()
        .and_then(|s| s.graph.get(h.node).map(|n| n.label.clone()));
    match label {
        Some(l) => format!("Signal {} ({l})", h.node),
        None => format!("Signal {}", h.node),
    }
}
