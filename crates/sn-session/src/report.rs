//! Analysis reports in Markdown and standalone HTML.
//!
//! Both formats contain the same sections: a summary, the sources, the derivation
//! tree, the measurements with their provenance, and the tone grids and ADP
//! candidates stored in the nodes' UI state (see [`crate::UI_KEY_ADP`],
//! [`crate::UI_KEY_TONE_GRID`] and [`crate::UI_KEY_TONES`]). The HTML has inline CSS
//! only and no external resources, so it can be mailed or archived as one file.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

use serde_json::Value;
use sn_analysis::adp::BaudCandidate;
use sn_analysis::tones::ToneGrid;
use sn_core::{MeasurementKind, NodeId};

use crate::file::{SessionFile, SessionNode, UI_KEY_ADP, UI_KEY_TONE_GRID, UI_KEY_TONES};

/// Section titles, in report order.
pub const SECTIONS: [&str; 6] = [
    "Summary",
    "Sources",
    "Derivation tree",
    "Measurements",
    "Tone grids",
    "ADP candidates",
];

/// A piece of inline text. User text (labels, paths, tool names) is always
/// [`Span::Text`] and is escaped as such; only text the report itself puts in code
/// style is [`Span::Code`], so user text can never open or close a code span.
#[derive(Debug, Clone)]
enum Span {
    Text(String),
    Code(String),
}

/// A run of inline text.
#[derive(Debug, Clone, Default)]
struct Inline(Vec<Span>);

impl Inline {
    fn text(s: impl Into<String>) -> Self {
        Self(vec![Span::Text(s.into())])
    }

    fn push_text(&mut self, s: impl Into<String>) -> &mut Self {
        self.0.push(Span::Text(s.into()));
        self
    }

    fn push_code(&mut self, s: impl Into<String>) -> &mut Self {
        self.0.push(Span::Code(s.into()));
        self
    }

    /// The text without any styling.
    fn plain(&self) -> String {
        self.0
            .iter()
            .map(|s| match s {
                Span::Text(t) | Span::Code(t) => t.as_str(),
            })
            .collect()
    }
}

impl From<String> for Inline {
    fn from(s: String) -> Self {
        Self::text(s)
    }
}

impl From<&str> for Inline {
    fn from(s: &str) -> Self {
        Self::text(s)
    }
}

enum Block {
    Heading(u8, Inline),
    Para(Inline),
    /// Header row and body rows.
    Table(Vec<Inline>, Vec<Vec<Inline>>),
    /// Items with their nesting depth (0 = top level), in display order.
    Tree(Vec<(usize, Inline)>),
}

/// Write a report (from [`markdown`] or [`html`]) to `path`.
///
/// Like [`crate::save`], the text goes to a temporary file first and then replaces
/// `path` in one step, so an existing report is never left half-written.
pub fn save_report(path: impl AsRef<std::path::Path>, text: &str) -> crate::Result<()> {
    crate::file::write_atomic(path.as_ref(), text.as_bytes(), "cannot write report")
}

/// Markdown report.
pub fn markdown(session: &SessionFile) -> String {
    let mut out = String::new();
    for block in build(session) {
        match block {
            Block::Heading(level, text) => {
                let _ = writeln!(
                    out,
                    "{} {}\n",
                    "#".repeat(usize::from(level)),
                    md_inline(&text, false)
                );
            }
            Block::Para(text) => {
                let _ = writeln!(out, "{}\n", md_inline(&text, false));
            }
            Block::Table(head, rows) => {
                let cells = |r: &[Inline]| {
                    r.iter()
                        .map(|c| md_inline(c, true))
                        .collect::<Vec<_>>()
                        .join(" | ")
                };
                let _ = writeln!(out, "| {} |", cells(&head));
                let _ = writeln!(out, "|{}", "---|".repeat(head.len()));
                for row in &rows {
                    let _ = writeln!(out, "| {} |", cells(row));
                }
                out.push('\n');
            }
            Block::Tree(items) => {
                for (depth, text) in items {
                    let _ = writeln!(out, "{}- {}", "  ".repeat(depth), md_inline(&text, false));
                }
                out.push('\n');
            }
        }
    }
    out
}

/// Standalone HTML report with inline CSS.
pub fn html(session: &SessionFile) -> String {
    let mut out = String::from(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>SigNexora analysis report</title>\n<style>\n\
         :root { color-scheme: light dark; }\n\
         body { font-family: system-ui, sans-serif; line-height: 1.5; max-width: 60rem; \
         margin: 2rem auto; padding: 0 1rem; background: Canvas; color: CanvasText; }\n\
         table { border-collapse: collapse; margin: 0.5rem 0 1.5rem; }\n\
         th, td { border: 1px solid GrayText; padding: 0.25rem 0.6rem; text-align: left; \
         vertical-align: top; }\n\
         td.num { text-align: right; font-variant-numeric: tabular-nums; }\n\
         code { font-family: ui-monospace, monospace; font-size: 0.9em; }\n\
         ul { margin: 0.25rem 0; }\n\
         h1, h2, h3 { line-height: 1.2; }\n\
         </style>\n</head>\n<body>\n",
    );
    for block in build(session) {
        match block {
            Block::Heading(level, text) => {
                let _ = writeln!(out, "<h{level}>{}</h{level}>", html_inline(&text));
            }
            Block::Para(text) => {
                let _ = writeln!(out, "<p>{}</p>", html_inline(&text));
            }
            Block::Table(head, rows) => {
                out.push_str("<table>\n<thead><tr>");
                for h in &head {
                    let _ = write!(out, "<th>{}</th>", html_inline(h));
                }
                out.push_str("</tr></thead>\n<tbody>\n");
                for row in &rows {
                    out.push_str("<tr>");
                    for c in row {
                        let class = if looks_numeric(c) {
                            " class=\"num\""
                        } else {
                            ""
                        };
                        let _ = write!(out, "<td{class}>{}</td>", html_inline(c));
                    }
                    out.push_str("</tr>\n");
                }
                out.push_str("</tbody>\n</table>\n");
            }
            Block::Tree(items) => {
                let mut open = 0_usize;
                for (depth, text) in items {
                    let depth = depth + 1;
                    if depth > open {
                        for _ in open..depth {
                            out.push_str("<ul>\n");
                        }
                    } else {
                        out.push_str("</li>\n");
                        for _ in depth..open {
                            out.push_str("</ul>\n</li>\n");
                        }
                    }
                    open = depth;
                    let _ = write!(out, "<li>{}", html_inline(&text));
                }
                if open > 0 {
                    out.push_str("</li>\n");
                    for _ in 1..open {
                        out.push_str("</ul>\n</li>\n");
                    }
                    out.push_str("</ul>\n");
                }
            }
        }
    }
    out.push_str("</body>\n</html>\n");
    out
}

fn build(s: &SessionFile) -> Vec<Block> {
    let labels: HashMap<NodeId, &str> = s.nodes.iter().map(|n| (n.id, n.label.as_str())).collect();
    let name = |id: NodeId| match labels.get(&id) {
        Some(l) => format!("{id} {l}"),
        None => format!("{id}"),
    };
    let mut b = vec![
        Block::Heading(1, "SigNexora analysis report".into()),
        Block::Heading(2, "Summary".into()),
        Block::Para(Inline::text(format!(
            "Session format {} version {}, written by {}. {} signal nodes, {} sources, {} \
             measurements.",
            s.format,
            s.version,
            if s.generator.is_empty() {
                "an unknown program"
            } else {
                &s.generator
            },
            s.nodes.len(),
            s.nodes.iter().filter(|n| n.inputs.is_empty()).count(),
            s.measurements.len()
        ))),
    ];

    sources(s, &mut b);
    tree(s, &labels, &name, &mut b);
    measurements(s, &name, &mut b);
    tone_grids(s, &name, &mut b);
    adp(s, &name, &mut b);
    b
}

fn sources(s: &SessionFile, b: &mut Vec<Block>) {
    b.push(Block::Heading(2, "Sources".into()));
    let sources: Vec<&SessionNode> = s.nodes.iter().filter(|n| n.inputs.is_empty()).collect();
    if sources.is_empty() {
        b.push(Block::Para("No sources.".into()));
    } else {
        let rows = sources
            .iter()
            .map(|n| {
                let (origin, format) = match n.operation.name.as_str() {
                    "load" => (
                        n.operation
                            .params
                            .get("path")
                            .and_then(Value::as_str)
                            .unwrap_or("(no path recorded)")
                            .to_owned(),
                        Inline::text(n.source.as_ref().map_or_else(
                            || "unknown (file not readable when saved)".to_owned(),
                            |src| format!("{}, {}", src.container, src.codec),
                        )),
                    ),
                    crate::OP_GENERATE => {
                        let (description, spec) = generator_parts(&n.operation.params);
                        let mut params = Inline::text("SigNexora generator ");
                        match spec {
                            Some(spec) => {
                                params.push_code(spec);
                            }
                            None => {
                                params.push_text("(no spec recorded)");
                            }
                        }
                        (format!("generated: {description}"), params)
                    }
                    other => {
                        let mut params = Inline::default();
                        params.push_code(compact(&n.operation.params));
                        (format!("generated: {other}"), params)
                    }
                };
                vec![
                    n.id.to_string().into(),
                    n.label.as_str().into(),
                    origin.into(),
                    format,
                    n.signal
                        .map_or_else(String::new, |sig| format!("{} Hz", sig.sample_rate))
                        .into(),
                    n.signal
                        .map_or_else(String::new, |sig| {
                            format!("{} samples ({:.3} s)", sig.len, sig.duration())
                        })
                        .into(),
                ]
            })
            .collect();
        b.push(Block::Table(
            ["Node", "Label", "Path", "Format", "Rate", "Length"]
                .map(Inline::from)
                .to_vec(),
            rows,
        ));
    }
}

/// Derivation tree: each node under its first input.
fn tree(
    s: &SessionFile,
    labels: &HashMap<NodeId, &str>,
    name: &dyn Fn(NodeId) -> String,
    b: &mut Vec<Block>,
) {
    b.push(Block::Heading(2, "Derivation tree".into()));
    if s.nodes.is_empty() {
        b.push(Block::Para("The session has no signals.".into()));
    } else {
        let mut children: BTreeMap<NodeId, Vec<&SessionNode>> = BTreeMap::new();
        let mut roots = Vec::new();
        for n in &s.nodes {
            match n.inputs.first() {
                Some(parent) if labels.contains_key(parent) => {
                    children.entry(*parent).or_default().push(n);
                }
                _ => roots.push(n),
            }
        }
        let mut items = Vec::new();
        let mut stack: Vec<(usize, &SessionNode)> =
            roots.into_iter().rev().map(|n| (0, n)).collect();
        while let Some((depth, n)) = stack.pop() {
            items.push((depth, tree_line(n, name)));
            if let Some(kids) = children.get(&n.id) {
                stack.extend(kids.iter().rev().map(|k| (depth + 1, *k)));
            }
        }
        b.push(Block::Tree(items));
    }
}

fn measurements(s: &SessionFile, name: &dyn Fn(NodeId) -> String, b: &mut Vec<Block>) {
    b.push(Block::Heading(2, "Measurements".into()));
    if s.measurements.is_empty() {
        b.push(Block::Para("No measurements were recorded.".into()));
    } else {
        let rows = s
            .measurements
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let m = &r.measurement;
                vec![
                    (i + 1).to_string().into(),
                    kind_name(&r.kind).into(),
                    format!("{} {}", m.value, unit(&r.kind)).trim_end().into(),
                    m.uncertainty
                        .map_or_else(|| "-".to_owned(), |u| format!("{u:.3e}"))
                        .into(),
                    m.source.map_or_else(|| "-".to_owned(), name).into(),
                    crate::tool_id::english_name(&m.tool)
                        .unwrap_or(m.tool.as_str())
                        .into(),
                ]
            })
            .collect();
        b.push(Block::Table(
            [
                "#",
                "Quantity",
                "Value",
                "Uncertainty",
                "Measured on",
                "Tool",
            ]
            .map(Inline::from)
            .to_vec(),
            rows,
        ));
    }
}

/// Message for results that are stored under `key` but do not parse.
fn unreadable(what: &str, e: &serde_json::Error) -> Block {
    Block::Para(Inline::text(format!(
        "{what} is stored but could not be read ({e}); it may come from a different \
         version of SigNexora."
    )))
}

fn tone_grids(s: &SessionFile, name: &dyn Fn(NodeId) -> String, b: &mut Vec<Block>) {
    b.push(Block::Heading(2, "Tone grids".into()));
    let mut any = false;
    for n in &s.nodes {
        let grid =
            n.ui.get(UI_KEY_TONE_GRID)
                .map(|v| serde_json::from_value::<ToneGrid>(v.clone()));
        let tones =
            n.ui.get(UI_KEY_TONES)
                .map(|v| serde_json::from_value::<Vec<f64>>(v.clone()));
        if grid.is_none() && tones.is_none() {
            continue;
        }
        any = true;
        b.push(Block::Heading(3, name(n.id).into()));
        match grid {
            Some(Ok(g)) => b.push(Block::Table(
                ["Grid size", "Spacing", "Lowest tone", "Positions used"]
                    .map(Inline::from)
                    .to_vec(),
                vec![vec![
                    g.size.to_string().into(),
                    format!("{:.3} Hz", g.spacing).into(),
                    format!("{:.3} Hz", g.lowest).into(),
                    g.occupied.to_string().into(),
                ]],
            )),
            Some(Err(e)) => b.push(unreadable("A tone grid", &e)),
            None => {}
        }
        match tones {
            Some(Ok(t)) => {
                let list: Vec<String> = t.iter().map(|f| format!("{f:.2}")).collect();
                b.push(Block::Para(Inline::text(format!(
                    "Tones found ({}): {} Hz",
                    t.len(),
                    list.join(", ")
                ))));
            }
            Some(Err(e)) => b.push(unreadable("A list of tones", &e)),
            None => {}
        }
    }
    if !any {
        b.push(Block::Para(
            "No tone grids were stored in this session.".into(),
        ));
    }
}

fn adp(s: &SessionFile, name: &dyn Fn(NodeId) -> String, b: &mut Vec<Block>) {
    b.push(Block::Heading(2, "ADP candidates".into()));
    let mut any = false;
    for n in &s.nodes {
        let Some(parsed) =
            n.ui.get(UI_KEY_ADP)
                .map(|v| serde_json::from_value::<Vec<BaudCandidate>>(v.clone()))
        else {
            continue;
        };
        any = true;
        b.push(Block::Heading(3, name(n.id).into()));
        let c = match parsed {
            Ok(c) => c,
            Err(e) => {
                b.push(unreadable("A list of ADP results", &e));
                continue;
            }
        };
        let rows = c
            .iter()
            .enumerate()
            .map(|(i, c)| {
                vec![
                    (i + 1).to_string().into(),
                    if c.refined {
                        format!("{:.3} Bd", c.baud)
                    } else {
                        format!("{:.3} Bd (coarse)", c.baud)
                    }
                    .into(),
                    format!("{:?}", c.detector).into(),
                    format!("{:.1}", c.score).into(),
                ]
            })
            .collect();
        b.push(Block::Table(
            ["Rank", "Rate", "Detector", "Score"]
                .map(Inline::from)
                .to_vec(),
            rows,
        ));
    }
    if !any {
        b.push(Block::Para(
            "No ADP results were stored in this session.".into(),
        ));
    }
}

fn tree_line(n: &SessionNode, name: &dyn Fn(NodeId) -> String) -> Inline {
    let op = &n.operation;
    let mut line = Inline::text(format!("{} {}: ", n.id, n.label));
    match op.name.as_str() {
        "edit" => {
            line.push_text("edit ").push_code(compact(&op.params));
        }
        crate::OP_GENERATE => {
            let (description, _) = generator_parts(&op.params);
            line.push_text(format!("generated: {description}"));
        }
        "load" => {
            line.push_text(match &n.source {
                Some(src) => format!("load ({}, {})", src.container, src.codec),
                None => "load".to_owned(),
            });
        }
        other => {
            line.push_text(format!("{other} "))
                .push_code(compact(&op.params));
        }
    }
    if let Some(sig) = n.signal {
        line.push_text(format!(
            "; {} Hz, {} samples, {:?}",
            sig.sample_rate, sig.len, sig.kind
        ));
    }
    if n.inputs.len() > 1 {
        let others: Vec<String> = n.inputs.iter().skip(1).map(|i| name(*i)).collect();
        line.push_text(format!("; also from {}", others.join(", ")));
    }
    line
}

/// The description and the compact spec of a generator operation. An old or edited
/// file without a description shows the kind of signal instead. The spec is `None`
/// when the file does not record one.
fn generator_parts(params: &Value) -> (String, Option<String>) {
    let spec = params.get("spec").map(compact);
    let description = params
        .get("description")
        .and_then(Value::as_str)
        .filter(|d| !d.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            params
                .get("spec")
                .and_then(|s| s.get("kind"))
                .and_then(Value::as_str)
                .map(|k| format!("{k} signal"))
        })
        .unwrap_or_else(|| "(no description recorded)".to_owned());
    (description, spec)
}

fn compact(v: &Value) -> String {
    serde_json::to_string(v)
        .unwrap_or_default()
        .replace('`', "'")
}

fn kind_name(kind: &MeasurementKind) -> String {
    match kind {
        MeasurementKind::BaudRate => "Br (keying rate)".into(),
        MeasurementKind::Carrier => "Sc (carrier)".into(),
        MeasurementKind::Period => "dT (period)".into(),
        MeasurementKind::Spacing => "Sh (spacing)".into(),
        MeasurementKind::OfdmK => "K (OFDM ratio)".into(),
        MeasurementKind::Custom(name) => name.clone(),
    }
}

fn unit(kind: &MeasurementKind) -> &'static str {
    match kind {
        MeasurementKind::BaudRate => "Bd",
        MeasurementKind::Carrier | MeasurementKind::Spacing => "Hz",
        MeasurementKind::Period => "s",
        MeasurementKind::OfdmK | MeasurementKind::Custom(_) => "",
    }
}

fn looks_numeric(cell: &Inline) -> bool {
    let s = cell.plain();
    s.chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit() || c == '-' || c == '+')
}

/// Markdown for a run of inline text. `in_table` also escapes `|` inside code
/// spans, which GitHub-flavoured tables need.
fn md_inline(text: &Inline, in_table: bool) -> String {
    let mut out = String::new();
    for (i, span) in text.0.iter().enumerate() {
        match span {
            Span::Text(t) => out.push_str(&md_text(t, i == 0)),
            Span::Code(c) => {
                let mut code = one_line(c).replace('`', "'");
                if in_table {
                    code = code.replace('|', "\\|");
                }
                let _ = write!(out, "`{code}`");
            }
        }
    }
    out
}

/// Escape plain text for Markdown: newlines become spaces, and every character
/// that could start emphasis, a link, HTML, a code span, a table cell or an escape
/// is backslash-escaped. A `#` is escaped only where it could start a heading.
fn md_text(s: &str, at_start: bool) -> String {
    let s = one_line(s);
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    let mut first = at_start;
    while let Some(c) = chars.next() {
        let heading = first && c == '#' && chars.peek().is_none_or(|n| *n == ' ' || *n == '#');
        if heading
            || matches!(
                c,
                '\\' | '`' | '*' | '_' | '<' | '>' | '[' | ']' | '|' | '&'
            )
        {
            out.push('\\');
        }
        out.push(c);
        first = false;
    }
    out
}

/// Line breaks in user text would split a table row or list item.
fn one_line(s: &str) -> String {
    s.replace("\r\n", " ").replace(['\r', '\n'], " ")
}

/// HTML for a run of inline text.
fn html_inline(text: &Inline) -> String {
    let mut out = String::new();
    for span in &text.0 {
        match span {
            Span::Text(t) => out.push_str(&esc(t)),
            Span::Code(c) => {
                let _ = write!(out, "<code>{}</code>", esc(c));
            }
        }
    }
    out
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}
