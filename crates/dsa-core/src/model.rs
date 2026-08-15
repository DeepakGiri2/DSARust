//! The trace model: what a recorded algorithm execution looks like.
//!
//! Everything here is plain data with `serde` impls, deliberately free of any
//! GUI, filesystem or platform dependency. Content packs (`dsa-content`) build
//! these values from scripts, `dsa-viz` renders them, `dsa-app` drives them.
//!
//! The wire format matches the original TypeScript app 1:1 (`kind`-tagged
//! variables, `type`-tagged views) so traces can be exchanged as JSON with the
//! web version and with authoring tools.

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

// ─────────────────────────────────────────────────────────────────────────────
// Scalars
// ─────────────────────────────────────────────────────────────────────────────

/// A scalar rendered inside a container view: an array cell, a map value, a
/// node label. Numbers and strings are the only two shapes any view needs.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Cell {
    Num(f64),
    Str(String),
}

impl Cell {
    /// Stable string form — also the identity used for map/set key lookups.
    pub fn key(&self) -> String {
        self.to_string()
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Cell::Num(n) => Some(*n),
            Cell::Str(s) => s.parse().ok(),
        }
    }

    pub fn is_num(&self) -> bool {
        matches!(self, Cell::Num(_))
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Integers are by far the common case; render them without the
            // trailing `.0` that `{}` on f64 would produce.
            Cell::Num(n) if n.fract() == 0.0 && n.abs() < 1e15 => write!(f, "{}", *n as i64),
            Cell::Num(n) => write!(f, "{n}"),
            Cell::Str(s) => write!(f, "{s}"),
        }
    }
}

impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Cell::Num(a), Cell::Num(b)) => a == b,
            (Cell::Str(a), Cell::Str(b)) => a == b,
            // "3" and 3 compare equal so scripts can highlight by value without
            // caring which side of the map/array the value came from.
            _ => self.to_string() == other.to_string(),
        }
    }
}

impl From<i64> for Cell {
    fn from(v: i64) -> Self {
        Cell::Num(v as f64)
    }
}
impl From<f64> for Cell {
    fn from(v: f64) -> Self {
        Cell::Num(v)
    }
}
impl From<&str> for Cell {
    fn from(v: &str) -> Self {
        Cell::Str(v.to_string())
    }
}
impl From<String> for Cell {
    fn from(v: String) -> Self {
        Cell::Str(v)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Variables
// ─────────────────────────────────────────────────────────────────────────────

/// A runtime value as shown in the Variables / Watch panel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum VarVal {
    Num {
        v: Cell,
    },
    Str {
        v: String,
    },
    Bool {
        v: bool,
    },
    Arr {
        v: Vec<Cell>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        hl: Vec<usize>,
    },
    Map {
        v: Vec<(String, Cell)>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        hl: Vec<String>,
    },
    Set {
        v: Vec<Cell>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        hl: Vec<Cell>,
    },
    /// Rendered as e.g. `-> node(3)`.
    Ptr {
        v: String,
    },
    Null,
}

impl VarVal {
    /// One-line form used by the variables panel when the value is collapsed.
    pub fn summary(&self) -> String {
        match self {
            VarVal::Num { v } => v.to_string(),
            VarVal::Str { v } => format!("\"{v}\""),
            VarVal::Bool { v } => v.to_string(),
            VarVal::Arr { v, .. } => {
                format!(
                    "[{}]",
                    v.iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            VarVal::Map { v, .. } => format!("{{{} entries}}", v.len()),
            VarVal::Set { v, .. } => format!("{{{} items}}", v.len()),
            VarVal::Ptr { v } => v.clone(),
            VarVal::Null => "nil".into(),
        }
    }

    /// True when the panel should offer an expandable child list.
    pub fn is_composite(&self) -> bool {
        matches!(
            self,
            VarVal::Map { .. } | VarVal::Set { .. } | VarVal::Arr { .. }
        )
    }
}

/// An insertion-ordered `name -> value` map.
///
/// Deliberately not a `HashMap`: the variables panel must not reshuffle rows
/// between steps, or the eye loses track of the value it was following.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VarMap(Vec<(String, VarVal)>);

impl VarMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, name: &str) -> Option<&VarVal> {
        self.0.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }

    /// Insert or overwrite, preserving the position of an existing key.
    pub fn set(&mut self, name: impl Into<String>, value: VarVal) {
        let name = name.into();
        match self.0.iter_mut().find(|(k, _)| *k == name) {
            Some(slot) => slot.1 = value,
            None => self.0.push((name, value)),
        }
    }

    pub fn remove(&mut self, name: &str) {
        self.0.retain(|(k, _)| k != name);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &VarVal)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Serialize for VarMap {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut m = ser.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            m.serialize_entry(k, v)?;
        }
        m.end()
    }
}

impl<'de> Deserialize<'de> for VarMap {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = VarMap;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a map of variable name to value")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut m: M) -> Result<VarMap, M::Error> {
                let mut out = Vec::with_capacity(m.size_hint().unwrap_or(8));
                while let Some((k, v)) = m.next_entry::<String, VarVal>()? {
                    out.push((k, v));
                }
                Ok(VarMap(out))
            }
        }
        de.deserialize_map(V)
    }
}

/// One call-stack frame. `fn_name` is a display label, e.g. `climb(n=4)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    #[serde(rename = "fn")]
    pub fn_name: String,
    #[serde(default)]
    pub vars: VarMap,
}

// ─────────────────────────────────────────────────────────────────────────────
// Views — the animated pictures a step renders
// ─────────────────────────────────────────────────────────────────────────────

/// `name -> index` cursor over an array, kept ordered so a pointer keeps its
/// colour and lane across steps (which is what makes the motion readable).
pub type Pointers = Vec<(String, i64)>;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ArrayView {
    pub label: String,
    pub data: Vec<Cell>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pointers: Pointers,
    /// Inclusive `[lo, hi]` highlight band — the sliding window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<(i64, i64)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hl: Vec<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bad: Vec<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub done: Vec<i64>,
    /// Render as a bar chart with heights taken from the values.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bars: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct KvView {
    pub label: String,
    pub entries: Vec<(String, Cell)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "hlKeys")]
    pub hl_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "badKeys")]
    pub bad_keys: Vec<String>,
}

/// Stacks, queues and monotonic deques share one view; only the growth
/// direction and the labels on the ends differ.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StackKind {
    #[default]
    Stack,
    Queue,
    Deque,
    Heap,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StackView {
    pub label: String,
    pub items: Vec<Cell>,
    #[serde(default)]
    pub kind: StackKind,
    /// The top item was just pushed — flashes green and slides in.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pushed: bool,
    /// An item was just removed — drawn fading out past the open end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub popped: Option<Cell>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bad: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ListNode {
    pub id: i64,
    pub val: Cell,
    pub next: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LinkedListView {
    pub label: String,
    pub nodes: Vec<ListNode>,
    /// `name -> node id`, `None` meaning nil.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pointers: Vec<(String, Option<i64>)>,
    /// Ids whose outgoing arrow now points backwards (drawn flipped/green).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reversed: Vec<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TreeNodeV {
    pub id: i64,
    pub val: Cell,
    pub left: Option<i64>,
    pub right: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TreeView {
    pub label: String,
    pub nodes: Vec<TreeNodeV>,
    pub root: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cur: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub done: Vec<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<i64>,
    /// A pair whose subtrees were just swapped — animated along an arc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swap: Option<(i64, i64)>,
}

pub type RC = (i64, i64);

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GridView {
    pub label: String,
    pub data: Vec<Vec<Cell>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hl: Vec<RC>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bad: Vec<RC>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub done: Vec<RC>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<RC>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cur: Option<RC>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "rowLabels")]
    pub row_labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "colLabels")]
    pub col_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphNodeV {
    pub id: i64,
    pub label: String,
    /// Optional layout hint in unit space (0..1). Omit and the view falls back
    /// to a deterministic circular layout, which keeps frames stable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphEdgeV {
    pub from: i64,
    pub to: i64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub directed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<Cell>,
}

/// Adjacency-style view for graph traversal problems (BFS/DFS/topo sort).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GraphView {
    pub label: String,
    pub nodes: Vec<GraphNodeV>,
    pub edges: Vec<GraphEdgeV>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cur: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub done: Vec<i64>,
    /// Discovered but not yet processed — the BFS/DFS frontier.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frontier: Vec<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bad: Vec<i64>,
    /// `(from, to)` pairs currently being traversed — the pulse runs along them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active: Vec<(i64, i64)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BitRow {
    pub label: String,
    pub value: i64,
    #[serde(default = "default_bit_width")]
    pub width: u32,
    /// Bit positions to flash, counted from the least-significant bit.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hl: Vec<u32>,
}

fn default_bit_width() -> u32 {
    32
}

/// Binary view for the Bit Manipulation category: rows of aligned bits with
/// individual positions highlighted as they flip.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BitsView {
    pub label: String,
    pub rows: Vec<BitRow>,
}

/// Free-form annotation panel: derivations, invariants, running formulas.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TextView {
    pub label: String,
    pub lines: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hl: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum VizView {
    Array(ArrayView),
    Kv(KvView),
    Stack(StackView),
    List(LinkedListView),
    Tree(TreeView),
    Grid(GridView),
    Graph(GraphView),
    Bits(BitsView),
    Text(TextView),
}

impl VizView {
    pub fn label(&self) -> &str {
        match self {
            VizView::Array(v) => &v.label,
            VizView::Kv(v) => &v.label,
            VizView::Stack(v) => &v.label,
            VizView::List(v) => &v.label,
            VizView::Tree(v) => &v.label,
            VizView::Grid(v) => &v.label,
            VizView::Graph(v) => &v.label,
            VizView::Bits(v) => &v.label,
            VizView::Text(v) => &v.label,
        }
    }

    /// Discriminator used to pair a view with its previous-step counterpart so
    /// the renderer can tween between them. Views only animate against a view
    /// of the same kind *and* label.
    pub fn anim_key(&self) -> (u8, &str) {
        let tag = match self {
            VizView::Array(_) => 0,
            VizView::Kv(_) => 1,
            VizView::Stack(_) => 2,
            VizView::List(_) => 3,
            VizView::Tree(_) => 4,
            VizView::Grid(_) => 5,
            VizView::Graph(_) => 6,
            VizView::Bits(_) => 7,
            VizView::Text(_) => 8,
        };
        (tag, self.label())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Steps and traces
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepEvent {
    #[default]
    Stmt,
    Call,
    Return,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogKind {
    #[default]
    Log,
    Call,
    Return,
    Result,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    /// Index of the step this line was emitted during.
    pub step: usize,
    pub text: String,
    #[serde(default)]
    pub kind: LogKind,
}

/// One recorded step of execution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Step {
    /// `//@tag` marker naming the source line, resolved per language.
    pub tag: String,
    /// Call depth — this is what makes step-over / step-out possible.
    pub depth: usize,
    #[serde(default)]
    pub event: StepEvent,
    /// Call stack snapshot, innermost **last**.
    pub frames: Vec<Frame>,
    pub views: Vec<VizView>,
    pub note: String,
    /// Number of log lines emitted up to and including this step. The log
    /// itself lives once on the `Trace` — the TS version snapshotted the whole
    /// log per step, which is quadratic on long traces.
    #[serde(default)]
    pub log_len: usize,
}

impl Step {
    pub fn innermost(&self) -> Option<&Frame> {
        self.frames.last()
    }
}

/// A complete recorded execution.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Trace {
    pub steps: Vec<Step>,
    #[serde(default)]
    pub logs: Vec<LogEntry>,
    /// Value the traced function returned, for the result banner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

impl Trace {
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn get(&self, i: usize) -> Option<&Step> {
        self.steps.get(i)
    }

    /// Log lines visible at step `i`.
    pub fn logs_at(&self, i: usize) -> &[LogEntry] {
        let n = self.steps.get(i).map(|s| s.log_len).unwrap_or(0);
        &self.logs[..n.min(self.logs.len())]
    }

    /// Highest call depth reached — used to size the call-stack gutter.
    pub fn max_depth(&self) -> usize {
        self.steps.iter().map(|s| s.depth).max().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_display_drops_float_noise() {
        assert_eq!(Cell::Num(3.0).to_string(), "3");
        assert_eq!(Cell::Num(-12.0).to_string(), "-12");
        assert_eq!(Cell::Num(2.5).to_string(), "2.5");
        assert_eq!(Cell::Str("ab".into()).to_string(), "ab");
    }

    #[test]
    fn cell_equality_bridges_num_and_str() {
        assert_eq!(Cell::Num(3.0), Cell::Str("3".into()));
        assert_ne!(Cell::Num(3.0), Cell::Str("x".into()));
    }

    #[test]
    fn varmap_preserves_insertion_order_on_overwrite() {
        let mut m = VarMap::new();
        m.set("i", VarVal::Num { v: 0.into() });
        m.set("seen", VarVal::Null);
        m.set("i", VarVal::Num { v: 5.into() });
        let names: Vec<_> = m.iter().map(|(k, _)| k.to_string()).collect();
        assert_eq!(names, vec!["i", "seen"]);
        assert_eq!(m.get("i"), Some(&VarVal::Num { v: 5.into() }));
    }

    #[test]
    fn view_json_matches_typescript_shape() {
        let v = VizView::Array(ArrayView {
            label: "nums".into(),
            data: vec![1.into(), 2.into()],
            pointers: vec![("i".into(), 1)],
            hl: vec![0],
            ..Default::default()
        });
        let j = serde_json::to_string(&v).unwrap();
        assert!(j.contains(r#""type":"array""#), "{j}");
        assert!(j.contains(r#""pointers":[["i",1]]"#), "{j}");
        // Empty decoration lists are omitted, keeping serialized traces small.
        assert!(!j.contains("bad"), "{j}");
    }

    #[test]
    fn logs_are_sliced_per_step_not_copied() {
        let trace = Trace {
            steps: vec![
                Step {
                    tag: "a".into(),
                    depth: 1,
                    event: StepEvent::Stmt,
                    frames: vec![],
                    views: vec![],
                    note: String::new(),
                    log_len: 1,
                },
                Step {
                    tag: "b".into(),
                    depth: 1,
                    event: StepEvent::Stmt,
                    frames: vec![],
                    views: vec![],
                    note: String::new(),
                    log_len: 3,
                },
            ],
            logs: (0..3)
                .map(|i| LogEntry {
                    step: i,
                    text: format!("l{i}"),
                    kind: LogKind::Log,
                })
                .collect(),
            result: None,
        };
        assert_eq!(trace.logs_at(0).len(), 1);
        assert_eq!(trace.logs_at(1).len(), 3);
    }
}
