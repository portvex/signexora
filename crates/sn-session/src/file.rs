//! The session file format.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use sn_core::{
    Measurement, MeasurementContext, MeasurementKind, NodeId, Operation, SampleKind, SignalGraph,
};

use crate::{Error, Result};

/// Value of [`SessionFile::format`]; identifies the file type.
pub const FORMAT_NAME: &str = "sa-rs-session";
/// Current format version. Files with a higher version are rejected.
pub const FORMAT_VERSION: u32 = 1;

/// Conventional key in a node's UI state for ADP candidates (an array of
/// [`sn_analysis::adp::BaudCandidate`]). The report shows them when present.
pub const UI_KEY_ADP: &str = "adp_candidates";
/// Conventional key in a node's UI state for a tone grid
/// ([`sn_analysis::tones::ToneGrid`]). The report shows it when present.
pub const UI_KEY_TONE_GRID: &str = "tone_grid";
/// Conventional key in a node's UI state for the tone frequencies found (array of Hz).
pub const UI_KEY_TONES: &str = "tones";
/// Conventional key in a node's UI state for the tone search tolerance in Hz.
pub const UI_KEY_TONE_TOL: &str = "tone_tolerance_hz";
/// Conventional key in a node's UI state for its window name.
pub const UI_KEY_NAME: &str = "window_name";
/// Conventional key in a node's UI state for its Info check box.
pub const UI_KEY_INFO: &str = "info";
/// Conventional key in a node's UI state for its markers.
pub const UI_KEY_MARKERS: &str = "markers";
/// Conventional session-wide UI key for the open windows.
pub const UI_KEY_WINDOWS: &str = "windows";
/// Conventional session-wide UI key for the window name counters.
pub const UI_KEY_COUNTERS: &str = "name_counters";

/// A saved session: the signal graph without samples, UI state and measurements.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SessionFile {
    /// Always [`FORMAT_NAME`].
    pub format: String,
    /// Format version, [`FORMAT_VERSION`] when written by this program.
    pub version: u32,
    /// Program that wrote the file, e.g. `"SigNexora 0.1.0"`.
    #[serde(default)]
    pub generator: String,
    /// Every node, in creation order (inputs always come before their outputs).
    pub nodes: Vec<SessionNode>,
    /// Every measurement, in the order it was recorded.
    #[serde(default)]
    pub measurements: Vec<MeasurementRecord>,
    /// Session-wide UI state (open tabs, layout, ...). Opaque to this crate.
    #[serde(default)]
    pub ui: Map<String, Value>,
}

/// One signal node: how it was made, not its samples.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SessionNode {
    /// Node id in the session that was saved. Replay maps it to a new id.
    pub id: NodeId,
    /// Label shown in the session tree.
    pub label: String,
    /// Parent nodes (empty for sources).
    #[serde(default)]
    pub inputs: Vec<NodeId>,
    /// Operation name and parameters.
    pub operation: Operation,
    /// Shape of the signal when it was saved, used to check a replay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal: Option<SignalSummary>,
    /// File details for `load` nodes, as found when the session was saved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceInfo>,
    /// Per-node UI state (view, markers, settings, results). Opaque to this crate,
    /// except for the conventional keys [`UI_KEY_ADP`], [`UI_KEY_TONE_GRID`] and
    /// [`UI_KEY_TONES`] that the report reads.
    #[serde(default)]
    pub ui: Map<String, Value>,
}

/// Shape of a signal.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SignalSummary {
    /// Sample rate in Hz.
    pub sample_rate: f64,
    /// Number of samples.
    pub len: u64,
    /// Real or complex.
    pub kind: SampleKind,
    /// Centre frequency offset in Hz.
    #[serde(default)]
    pub center_frequency: f64,
}

impl SignalSummary {
    /// Summary of a signal.
    pub fn of(signal: &sn_core::Signal) -> Self {
        Self {
            sample_rate: signal.sample_rate(),
            len: signal.len() as u64,
            kind: signal.kind(),
            center_frequency: signal.center_frequency(),
        }
    }

    /// Duration in seconds.
    pub fn duration(&self) -> f64 {
        if self.sample_rate > 0.0 {
            self.len as f64 / self.sample_rate
        } else {
            0.0
        }
    }
}

/// File details of a source.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SourceInfo {
    /// Container, e.g. `"Wav"`, `"Sigmf"`, `"Raw"`.
    pub container: String,
    /// Codec or sample format, e.g. `"PCM 16-bit"`.
    pub codec: String,
    /// File size in bytes, to notice a changed file on replay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
}

/// Options for [`SessionFile::capture_with`].
#[derive(Debug, Clone, Default)]
pub struct CaptureOptions {
    /// Directory that relative source paths are resolved against when probing, the
    /// same way [`crate::ReplayOptions::base_dir`] resolves them on replay. `None`
    /// uses the current directory.
    pub base_dir: Option<PathBuf>,
    /// Open every `load` source to record its format and size ([`SourceInfo`]).
    /// This reads the files, so it can be slow on a network drive; leave it off on a
    /// UI thread and call [`SessionFile::probe_sources`] from a background job instead.
    pub probe_sources: bool,
}

/// One measurement with its kind.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MeasurementRecord {
    /// What was measured.
    pub kind: MeasurementKind,
    /// Value and provenance.
    #[serde(flatten)]
    pub measurement: Measurement,
}

impl SessionFile {
    /// An empty session.
    pub fn new() -> Self {
        Self {
            format: FORMAT_NAME.to_owned(),
            version: FORMAT_VERSION,
            generator: format!("SigNexora {}", env!("CARGO_PKG_VERSION")),
            nodes: Vec::new(),
            measurements: Vec::new(),
            ui: Map::new(),
        }
    }

    /// Record a graph and its measurements. UI state starts empty; fill it with
    /// [`SessionFile::node_ui_mut`] and [`SessionFile::ui`].
    ///
    /// Sources loaded from files are probed for their format, resolving relative
    /// paths against the current directory; a file that can no longer be read simply
    /// has no [`SourceInfo`]. This does file I/O: on a UI thread use
    /// [`SessionFile::capture_with`] without probing and probe in a background job.
    pub fn capture(graph: &SignalGraph, measurements: &MeasurementContext) -> Self {
        Self::capture_with(
            graph,
            measurements,
            &CaptureOptions {
                base_dir: None,
                probe_sources: true,
            },
        )
    }

    /// Like [`SessionFile::capture`], with control over source probing. Without
    /// [`CaptureOptions::probe_sources`] no file is touched.
    pub fn capture_with(
        graph: &SignalGraph,
        measurements: &MeasurementContext,
        options: &CaptureOptions,
    ) -> Self {
        let mut session = Self::new();
        for node in graph.nodes() {
            session.nodes.push(SessionNode {
                id: node.id,
                label: node.label.clone(),
                inputs: node.inputs.clone(),
                operation: node.operation.clone(),
                signal: Some(SignalSummary::of(&node.signal)),
                source: None,
                ui: Map::new(),
            });
        }
        if options.probe_sources {
            session.probe_sources(options.base_dir.as_deref());
        }
        session.measurements = measurements
            .iter()
            .map(|(kind, m)| MeasurementRecord {
                kind: kind.clone(),
                measurement: m.clone(),
            })
            .collect();
        session
    }

    /// Fill in [`SessionNode::source`] for every `load` node by probing its file.
    /// Relative paths are resolved against `base_dir` (the current directory when
    /// `None`), exactly as [`crate::replay`] resolves them. Does file I/O.
    pub fn probe_sources(&mut self, base_dir: Option<&Path>) {
        for node in &mut self.nodes {
            if node.operation.name == "load" {
                node.source = source_info(&node.operation, base_dir);
            }
        }
    }

    /// The node saved with id `id`.
    pub fn node(&self, id: NodeId) -> Option<&SessionNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// UI state of node `id`, for writing.
    pub fn node_ui_mut(&mut self, id: NodeId) -> Option<&mut Map<String, Value>> {
        self.nodes
            .iter_mut()
            .find(|n| n.id == id)
            .map(|n| &mut n.ui)
    }

    /// The measurement history as a context, with sources as saved (not remapped).
    ///
    /// Records the context refuses (NaN or infinite values) are left out; a file
    /// read with [`load`] cannot contain them, because JSON has no such numbers.
    pub fn measurement_context(&self) -> MeasurementContext {
        let mut ctx = MeasurementContext::new();
        for r in &self.measurements {
            // Only non-finite values are refused, and those are documented as skipped.
            let _ = ctx.record(r.kind.clone(), r.measurement.clone());
        }
        ctx
    }

    /// Serialise as pretty-printed JSON.
    ///
    /// Fails when a measurement or signal summary holds NaN or infinity: JSON cannot
    /// store those, so the file would be written but could never be read back.
    pub fn to_json(&self) -> Result<String> {
        self.check_finite()?;
        serde_json::to_string_pretty(self)
            .map_err(|e| Error::Unsaveable(format!("cannot write the session as JSON: {e}")))
    }

    /// Reject values that JSON turns into `null`.
    fn check_finite(&self) -> Result<()> {
        for (i, r) in self.measurements.iter().enumerate() {
            let m = &r.measurement;
            let bad = if m.value.is_finite() {
                m.uncertainty
                    .filter(|u| !u.is_finite())
                    .map(|u| ("uncertainty", u))
            } else {
                Some(("value", m.value))
            };
            if let Some((field, v)) = bad {
                return Err(Error::Unsaveable(format!(
                    "cannot save the session: measurement {} ({} from tool '{}') has {field} \
                     {v}, which a session file cannot store. Delete or redo that measurement \
                     and save again",
                    i + 1,
                    r.kind,
                    m.tool
                )));
            }
        }
        for n in &self.nodes {
            let Some(sig) = &n.signal else { continue };
            for (field, v) in [
                ("sample rate", sig.sample_rate),
                ("centre frequency", sig.center_frequency),
            ] {
                if !v.is_finite() {
                    return Err(Error::Unsaveable(format!(
                        "cannot save the session: signal {} ('{}') has {field} {v}, which a \
                         session file cannot store. Delete that signal and save again",
                        n.id, n.label
                    )));
                }
            }
        }
        Ok(())
    }

    /// Parse JSON, checking the format name and version first so a file from a
    /// newer program gets a clear message instead of a schema error.
    pub fn from_json(text: &str) -> Result<Self> {
        let value: Value = serde_json::from_str(text)
            .map_err(|e| Error::Invalid(format!("not valid JSON ({e})")))?;
        match value.get("format").and_then(Value::as_str) {
            Some(FORMAT_NAME) => {}
            Some(other) => {
                return Err(Error::Invalid(format!(
                    "the \"format\" field is \"{other}\", expected \"{FORMAT_NAME}\""
                )));
            }
            None => {
                return Err(Error::Invalid(format!(
                    "the \"format\" field is missing (expected \"{FORMAT_NAME}\")"
                )));
            }
        }
        let version = value
            .get("version")
            .and_then(Value::as_u64)
            .ok_or_else(|| Error::Invalid("the \"version\" field is missing".into()))?;
        if version > u64::from(FORMAT_VERSION) {
            return Err(Error::NewerVersion {
                found: version,
                supported: FORMAT_VERSION,
            });
        }
        let session: Self =
            serde_json::from_value(value).map_err(|e| Error::Invalid(e.to_string()))?;
        session.check()?;
        Ok(session)
    }

    /// Check that ids are unique and every input refers to an earlier node.
    fn check(&self) -> Result<()> {
        let mut seen = std::collections::HashSet::new();
        for n in &self.nodes {
            for input in &n.inputs {
                if !seen.contains(input) {
                    return Err(Error::Invalid(format!(
                        "node {} ('{}') refers to input {input}, which is not an earlier node",
                        n.id, n.label
                    )));
                }
            }
            if !seen.insert(n.id) {
                return Err(Error::Invalid(format!("node id {} appears twice", n.id)));
            }
        }
        Ok(())
    }
}

impl Default for SessionFile {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve a source path from a session: relative paths are taken relative to
/// `base`, or to the current directory when `base` is `None`.
pub(crate) fn resolve(path: &str, base: Option<&Path>) -> PathBuf {
    let p = PathBuf::from(path);
    match base {
        Some(base) if p.is_relative() => base.join(p),
        _ => p,
    }
}

fn source_info(op: &Operation, base_dir: Option<&Path>) -> Option<SourceInfo> {
    let path = resolve(op.params.get("path")?.as_str()?, base_dir);
    let raw = op
        .params
        .get("raw")
        .cloned()
        .and_then(|v| serde_json::from_value::<Option<sn_io::RawFormat>>(v).ok())
        .flatten();
    let bytes = std::fs::metadata(&path).ok().map(|m| m.len());
    if let Some(raw) = raw {
        return Some(SourceInfo {
            container: "Raw".to_owned(),
            codec: raw.format.label().to_owned(),
            bytes,
        });
    }
    let info = sn_io::probe(&path).ok()?;
    Some(SourceInfo {
        container: format!("{:?}", info.container),
        codec: info.codec,
        bytes,
    })
}

/// Write a session as pretty-printed JSON.
///
/// The text goes to a temporary file next to `path` first, which then replaces
/// `path` in one step, so an existing session is never left empty or half-written
/// when writing fails.
pub fn save(path: impl AsRef<Path>, session: &SessionFile) -> Result<()> {
    let text = session.to_json()?;
    write_atomic(path.as_ref(), text.as_bytes(), "cannot write session file")
}

/// Write `bytes` to a uniquely named temporary file next to `path` and move it over
/// `path` in one step, so an existing file is never left empty or half-written, two
/// saves at once never share a temporary file, and no other file is touched. On
/// Unix the saved file keeps the mode of the file it replaces. `action` heads the
/// error.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8], action: &'static str) -> Result<()> {
    sn_io::write_atomically(path, |f| {
        f.write_all(bytes).map_err(|source| sn_io::Error::Io {
            path: path.to_owned(),
            source,
        })
    })
    .map_err(|e| {
        let source = match e {
            sn_io::Error::Io { source, .. } => source,
            // For example `Unsupported` when the target is the memory-mapped file
            // that is open; its message already explains what to do.
            other => std::io::Error::other(other.to_string()),
        };
        Error::Io {
            action,
            path: path.display().to_string(),
            source,
        }
    })
}

/// Read a session file.
pub fn load(path: impl AsRef<Path>) -> Result<SessionFile> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path).map_err(|source| Error::Io {
        action: "cannot read session file",
        path: path.display().to_string(),
        source,
    })?;
    SessionFile::from_json(&text)
}
