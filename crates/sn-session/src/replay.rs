//! Rebuild a signal graph from a saved session.

use std::collections::BTreeMap;
use std::path::PathBuf;

use sn_core::{MeasurementContext, NodeId, Operation, Signal, SignalGraph};
use sn_dsp::ops::Edit;
use sn_io::{ChannelSelection, LoadOptions, RawFormat, Scaling};

use crate::file::{SessionFile, SessionNode, SignalSummary, resolve};
use crate::{Error, Result};

/// Options for [`replay`].
#[derive(Debug, Clone, Default)]
pub struct ReplayOptions {
    /// Directory that relative source paths are resolved against, usually the
    /// directory of the session file. `None` uses the current directory.
    pub base_dir: Option<PathBuf>,
}

/// Progress of a replay, reported before each node is rebuilt.
#[derive(Debug, Clone, Copy)]
pub struct ReplayProgress<'a> {
    /// Nodes finished so far.
    pub done: usize,
    /// Total nodes in the session.
    pub total: usize,
    /// Label of the node about to be rebuilt.
    pub label: &'a str,
}

/// A problem found while replaying one node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayIssue {
    /// Node id as saved in the session.
    pub node: NodeId,
    /// Node label as saved.
    pub label: String,
    /// What went wrong and how to fix it.
    pub message: String,
    /// True when the node could not be rebuilt; false for a warning.
    pub failed: bool,
}

/// Result of [`replay`]: everything that could be rebuilt, plus the problems.
#[derive(Debug)]
pub struct ReplayOutcome {
    /// The rebuilt graph. Nodes that failed, and their descendants, are missing.
    pub graph: SignalGraph,
    /// Saved node id to rebuilt node id, for every node that was rebuilt.
    pub id_map: BTreeMap<NodeId, NodeId>,
    /// The measurement history, with source nodes mapped to the rebuilt ids.
    pub measurements: MeasurementContext,
    /// Failures and warnings, in session order.
    pub issues: Vec<ReplayIssue>,
}

impl ReplayOutcome {
    /// Rebuilt id of a saved node.
    pub fn new_id(&self, saved: NodeId) -> Option<NodeId> {
        self.id_map.get(&saved).copied()
    }

    /// True when every node was rebuilt.
    pub fn is_complete(&self) -> bool {
        !self.issues.iter().any(|i| i.failed)
    }

    /// Number of nodes that could not be rebuilt.
    pub fn failed_count(&self) -> usize {
        self.issues.iter().filter(|i| i.failed).count()
    }
}

#[derive(serde::Deserialize)]
struct LoadParams {
    path: String,
    #[serde(default)]
    channels: Option<ChannelSelection>,
    #[serde(default)]
    scaling: Option<Scaling>,
    #[serde(default)]
    raw: Option<RawFormat>,
}

/// Name of the operation that records a signal made by the test-signal generator.
pub const OP_GENERATE: &str = "generate";

/// Parameters of an [`OP_GENERATE`] operation: the full generator spec, which replay
/// passes to [`sn_synth::generate`] again, and the description it gave (for reports;
/// replay does not read it).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenerateParams {
    /// What to generate.
    pub spec: sn_synth::GeneratorSpec,
    /// Description from [`sn_synth::Generated::description`].
    #[serde(default)]
    pub description: String,
}

/// The operation that records generating `spec`, whose result was described as
/// `description`. Replaying it gives bit-identical samples on the same platform.
///
/// Fails with [`Error::Unsaveable`] when the spec cannot be recorded exactly, for
/// example when it holds a NaN or infinite number (JSON would store it as null).
pub fn generate_operation(spec: &sn_synth::GeneratorSpec, description: &str) -> Result<Operation> {
    let unsaveable = |why: String| {
        Error::Unsaveable(format!(
            "the generator settings cannot be recorded in the session ({why}); check that \
             every number in the generator is finite"
        ))
    };
    let params = GenerateParams {
        spec: spec.clone(),
        description: description.to_owned(),
    };
    let value = serde_json::to_value(&params).map_err(|e| unsaveable(e.to_string()))?;
    // A non-finite number serialises to null without an error; reading the value back
    // and comparing catches that and any other change, so replay sees the same spec.
    let back: GenerateParams =
        serde_json::from_value(value.clone()).map_err(|e| unsaveable(e.to_string()))?;
    if back.spec != *spec {
        return Err(unsaveable(
            "a value changed when written as JSON".to_owned(),
        ));
    }
    Ok(Operation::new(OP_GENERATE, value))
}

#[derive(serde::Deserialize)]
struct SynthPskParams {
    order: usize,
    baud: f64,
    carrier: f64,
    rate: f64,
    symbols: Option<usize>,
    seed: Option<u64>,
    amplitude: Option<f64>,
}

/// Rebuild the graph of `session` by re-opening its sources and re-running every
/// operation in order.
///
/// Nodes that cannot be rebuilt (a missing source file, an unknown operation) are
/// reported in [`ReplayOutcome::issues`] and skipped together with everything derived
/// from them; the rest of the graph is still rebuilt. `progress` is called before each
/// node. Returns [`Error::Cancelled`] when `should_stop` returns true.
pub fn replay(
    session: &SessionFile,
    options: &ReplayOptions,
    progress: &mut dyn FnMut(ReplayProgress<'_>),
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<ReplayOutcome> {
    let mut out = ReplayOutcome {
        graph: SignalGraph::new(),
        id_map: BTreeMap::new(),
        measurements: MeasurementContext::new(),
        issues: Vec::new(),
    };
    let total = session.nodes.len();
    // Bytes of samples the generate nodes have made so far.
    let mut generated: u64 = 0;
    for (done, node) in session.nodes.iter().enumerate() {
        if should_stop() {
            return Err(Error::Cancelled);
        }
        progress(ReplayProgress {
            done,
            total,
            label: &node.label,
        });
        let issue = |message: String, failed: bool| ReplayIssue {
            node: node.id,
            label: node.label.clone(),
            message,
            failed,
        };

        // Inputs that failed earlier make this node impossible.
        let inputs: Vec<NodeId> = node
            .inputs
            .iter()
            .filter_map(|old| out.id_map.get(old).copied())
            .collect();
        if let Some(old) = node.inputs.iter().find(|old| !out.id_map.contains_key(old)) {
            out.issues.push(issue(
                format!("skipped because its input {old} could not be rebuilt"),
                true,
            ));
            continue;
        }

        let mut warnings = Vec::new();
        let signal = match build(
            node,
            &out.graph,
            &inputs,
            options,
            &mut warnings,
            &mut generated,
            should_stop,
        ) {
            Ok(s) => s,
            Err(BuildError::Cancelled) => return Err(Error::Cancelled),
            Err(BuildError::Failed(message)) => {
                out.issues.push(issue(message, true));
                continue;
            }
        };
        if let Some(saved) = &node.signal
            && let Some(diff) = compare(saved, &SignalSummary::of(&signal))
        {
            warnings.push(format!(
                "the rebuilt signal differs from the saved one ({diff}); the source file \
                 or the program version may have changed"
            ));
        }
        out.issues
            .extend(warnings.into_iter().map(|w| issue(w, false)));

        let added = if inputs.is_empty() {
            Ok(out
                .graph
                .add_source(node.label.clone(), node.operation.clone(), signal))
        } else {
            out.graph
                .add_derived(node.label.clone(), node.operation.clone(), &inputs, signal)
        };
        match added {
            Ok(new) => {
                out.id_map.insert(node.id, new);
            }
            Err(e) => out.issues.push(issue(e.to_string(), true)),
        }
    }

    restore_measurements(session, &mut out);
    Ok(out)
}

/// Copy the measurement history into `out`, pointing each measurement at the
/// rebuilt node. A measurement whose node was not rebuilt is kept without a source,
/// with a warning that says why.
fn restore_measurements(session: &SessionFile, out: &mut ReplayOutcome) {
    for (i, record) in session.measurements.iter().enumerate() {
        let mut m = record.measurement.clone();
        let saved_source = m.source;
        let what = format!(
            "measurement {} ({} from tool '{}')",
            i + 1,
            record.kind,
            m.tool
        );
        let measurement_issue = |old: NodeId, message: String| ReplayIssue {
            node: old,
            label: session
                .node(old)
                .map_or_else(String::new, |n| n.label.clone()),
            message,
            failed: false,
        };
        if let Some(old) = saved_source {
            m.source = out.id_map.get(&old).copied();
            if m.source.is_none() {
                out.issues.push(measurement_issue(
                    old,
                    format!(
                        "{what} was taken on signal {old}, which could not be rebuilt; the \
                         measurement is kept but no longer linked to a signal"
                    ),
                ));
            }
        }
        // Only NaN or infinite values are refused, which a file read with `load`
        // cannot hold. Report it when there is a node to attach the message to.
        if let Err(e) = out.measurements.record(record.kind.clone(), m)
            && let Some(old) = saved_source
        {
            out.issues.push(measurement_issue(
                old,
                format!("{what} was not restored: {e}"),
            ));
        }
    }
}

enum BuildError {
    Cancelled,
    Failed(String),
}

fn failed(message: impl Into<String>) -> BuildError {
    BuildError::Failed(message.into())
}

fn build(
    node: &SessionNode,
    graph: &SignalGraph,
    inputs: &[NodeId],
    options: &ReplayOptions,
    warnings: &mut Vec<String>,
    generated: &mut u64,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Signal, BuildError> {
    let op = &node.operation;
    match op.name.as_str() {
        "load" => load(node, options, warnings),
        "edit" => {
            let edit: Edit = serde_json::from_value(op.params.clone()).map_err(|e| {
                failed(format!(
                    "the parameters of this edit are not valid ({e}); this session may come \
                     from a different version of SigNexora"
                ))
            })?;
            let [input] = inputs else {
                return Err(failed(format!(
                    "an edit needs exactly one input, but this node has {}",
                    inputs.len()
                )));
            };
            let parent = graph
                .get(*input)
                .ok_or_else(|| failed(format!("input {input} is missing")))?;
            edit.run(&parent.signal, should_stop).map_err(|e| match e {
                sn_dsp::Error::Cancelled => BuildError::Cancelled,
                other => failed(format!("'{}' failed: {other}", edit.label())),
            })
        }
        "synth_psk" => synth_psk(&op.params),
        OP_GENERATE => generate(&op.params, generated, should_stop),
        other => Err(failed(format!(
            "operation '{other}' cannot be replayed by this version of SigNexora; update the \
             program or remove this node from the session"
        ))),
    }
}

fn load(
    node: &SessionNode,
    options: &ReplayOptions,
    warnings: &mut Vec<String>,
) -> Result<Signal, BuildError> {
    let p: LoadParams = serde_json::from_value(node.operation.params.clone()).map_err(|e| {
        failed(format!(
            "the load parameters are not valid ({e}); a load node needs at least a \"path\""
        ))
    })?;
    let path = resolve(&p.path, options.base_dir.as_deref());
    let meta = match std::fs::metadata(&path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(failed(format!(
                "source file {} no longer exists. Put the file back at that path, or change \
                 the \"path\" of node {} in the session file",
                path.display(),
                node.id
            )));
        }
        Err(e) => {
            return Err(failed(format!(
                "cannot access source file {}: {e}. Check that the drive is reachable and \
                 that you may read the file",
                path.display()
            )));
        }
    };
    if let Some(saved) = node.source.as_ref().and_then(|s| s.bytes)
        && saved != meta.len()
    {
        warnings.push(format!(
            "source file {} is {} bytes but was {saved} bytes when the session was saved; \
             it has changed, so results may differ",
            path.display(),
            meta.len()
        ));
    }
    let options = LoadOptions {
        channels: p.channels,
        scaling: p.scaling.unwrap_or_default(),
        raw: p.raw,
    };
    sn_io::open(&path, &options)
        .map_err(|e| failed(format!("cannot open source file {}: {e}", path.display())))
}

/// Most symbols a saved test signal may ask for.
const MAX_SYNTH_SYMBOLS: usize = 10_000_000;
/// Most samples a regenerated test signal may have (2 GB of `f32`).
const MAX_SYNTH_SAMPLES: f64 = 500_000_000.0;

fn synth_psk(params: &serde_json::Value) -> Result<Signal, BuildError> {
    let p: SynthPskParams = serde_json::from_value(params.clone())
        .map_err(|e| failed(format!("the test-signal parameters are not valid ({e})")))?;
    let seed = p.seed.ok_or_else(|| {
        failed(
            "this test signal was saved without its random seed, so it cannot be regenerated \
             identically. Generate a new test signal instead",
        )
    })?;
    if !(2..=32).contains(&p.order) {
        return Err(failed(format!(
            "the test-signal \"order\" is {}, but it must be between 2 and 32",
            p.order
        )));
    }
    for (field, v) in [("baud", p.baud), ("rate", p.rate)] {
        if !(v.is_finite() && v > 0.0) {
            return Err(failed(format!(
                "the test-signal \"{field}\" is {v}, but it must be a finite number greater \
                 than 0"
            )));
        }
    }
    let count = p.symbols.unwrap_or(24_000);
    if count == 0 || count > MAX_SYNTH_SYMBOLS {
        return Err(failed(format!(
            "the test-signal \"symbols\" is {count}, but it must be between 1 and \
             {MAX_SYNTH_SYMBOLS}"
        )));
    }
    let samples = count as f64 * p.rate / p.baud;
    if !(samples.is_finite() && samples <= MAX_SYNTH_SAMPLES) {
        return Err(failed(format!(
            "the test signal would have {samples:.0} samples ({count} symbols at {} Bd and \
             {} Hz), more than the limit of {MAX_SYNTH_SAMPLES:.0}; lower \"symbols\" or \
             \"rate\", or raise \"baud\"",
            p.baud, p.rate
        )));
    }
    let psk = sn_synth::PskParams {
        order: p.order,
        baud: p.baud,
        carrier: p.carrier,
        sample_rate: p.rate,
        amplitude: p.amplitude.unwrap_or(0.5),
    };
    let symbols = sn_synth::random_symbols(count, p.order, seed);
    sn_synth::psk(&psk, &symbols).map_err(|e| failed(format!("cannot regenerate: {e}")))
}

/// Most bytes of samples all the generate nodes of one session may make together
/// (4 GiB, two complex signals at the generator's own size limit). Each node is
/// already limited by the generator; this stops a session with many such nodes from
/// exhausting memory.
pub const MAX_GENERATED_BYTES: u64 = 4 << 30;

/// Bytes a generated signal of `samples` samples takes (`f32` or complex `f32`).
fn generated_bytes(samples: usize, complex: bool) -> u64 {
    let per_sample: u64 = if complex { 8 } else { 4 };
    u64::try_from(samples)
        .unwrap_or(u64::MAX)
        .saturating_mul(per_sample)
}

/// Add `bytes` to the running total `used`, or explain why the budget is exceeded.
fn charge_generated(used: &mut u64, bytes: u64) -> Result<(), BuildError> {
    let total = used.saturating_add(bytes);
    if total > MAX_GENERATED_BYTES {
        return Err(failed(format!(
            "the generated signals in this session would need more than {} GiB \
             ({:.2} GiB with this node); remove some generate nodes from the session or \
             shorten them",
            MAX_GENERATED_BYTES >> 30,
            total as f64 / f64::from(1_u32 << 30)
        )));
    }
    *used = total;
    Ok(())
}

fn generate(
    params: &serde_json::Value,
    generated: &mut u64,
    should_stop: &(dyn Fn() -> bool + Sync),
) -> Result<Signal, BuildError> {
    let p: GenerateParams = serde_json::from_value(params.clone()).map_err(|e| {
        failed(format!(
            "the generator parameters are not valid ({e}); this session may come from a \
             different version of SigNexora"
        ))
    })?;
    // Every size is checked before anything is allocated.
    let plan = p
        .spec
        .plan()
        .map_err(|e| failed(format!("cannot regenerate: {e}")))?;
    let mut used = *generated;
    charge_generated(&mut used, generated_bytes(plan.samples, plan.complex))?;
    if should_stop() {
        return Err(BuildError::Cancelled);
    }
    let signal = sn_synth::generate_with_stop(&p.spec, should_stop)
        .map(|g| g.signal)
        .map_err(|e| match e {
            sn_synth::Error::Cancelled => BuildError::Cancelled,
            e @ (sn_synth::Error::InvalidParameter { .. } | sn_synth::Error::Core(_)) => {
                failed(format!("cannot regenerate: {e}"))
            }
        })?;
    *generated = used;
    Ok(signal)
}

fn compare(saved: &SignalSummary, now: &SignalSummary) -> Option<String> {
    if saved.kind != now.kind {
        return Some(format!("{:?} instead of {:?}", now.kind, saved.kind));
    }
    if saved.len != now.len {
        return Some(format!("{} samples instead of {}", now.len, saved.len));
    }
    let tol = 1e-9 * saved.sample_rate.abs().max(1.0);
    if (saved.sample_rate - now.sample_rate).abs() > tol {
        return Some(format!(
            "{} Hz instead of {} Hz",
            now.sample_rate, saved.sample_rate
        ));
    }
    if (saved.center_frequency - now.center_frequency).abs() > tol {
        return Some(format!(
            "centre frequency {} Hz instead of {} Hz",
            now.center_frequency, saved.center_frequency
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_bytes_counts_real_and_complex_samples() {
        assert_eq!(generated_bytes(10, false), 40);
        assert_eq!(generated_bytes(10, true), 80);
        assert_eq!(generated_bytes(usize::MAX, true), u64::MAX);
    }

    #[test]
    fn generate_budget_refuses_the_node_that_would_exceed_it() {
        // Two complex signals at the generator's 2^28-sample limit fit; a third does not.
        let node = generated_bytes(1 << 28, true);
        let mut used = 0;
        assert!(charge_generated(&mut used, node).is_ok());
        assert!(charge_generated(&mut used, node).is_ok());
        assert_eq!(used, MAX_GENERATED_BYTES);
        match charge_generated(&mut used, 1) {
            Err(BuildError::Failed(m)) => {
                assert!(m.contains("more than 4 GiB"), "{m}");
                assert!(m.contains("remove some generate nodes"), "{m}");
            }
            _ => panic!("expected the budget to be exceeded"),
        }
        // A refused node is not charged.
        assert_eq!(used, MAX_GENERATED_BYTES);
    }
    #[test]
    fn generate_stops_half_way_when_asked() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let spec = sn_synth::GeneratorSpec::default_for(sn_synth::SignalKind::Tones);
        let op = generate_operation(&spec, "tones").unwrap();
        // The first check (before generating) passes; the generator's own check stops it.
        let calls = AtomicUsize::new(0);
        let stop = || calls.fetch_add(1, Ordering::Relaxed) >= 1;
        let mut used = 0;
        assert!(matches!(
            generate(&op.params, &mut used, &stop),
            Err(BuildError::Cancelled)
        ));
        assert!(calls.load(Ordering::Relaxed) >= 2);
        // A cancelled node is not charged.
        assert_eq!(used, 0);
    }
}
