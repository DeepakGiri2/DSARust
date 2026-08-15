//! Core model for **DSA Visualized** — the trace format, the debugger
//! timeline, problem metadata and the source-annotation parser.
//!
//! This crate is deliberately dependency-light and platform-free: no GUI, no
//! filesystem, no scripting engine. Everything above it (content loading,
//! rendering, the harness, the app shell) builds on these types, which is what
//! keeps the Windows, Linux and macOS builds byte-identical in behaviour.

pub mod code;
pub mod diff;
pub mod model;
pub mod practice;
pub mod problem;
pub mod synth;
pub mod timeline;

pub use code::{parse_code, ParsedCode};
pub use model::{
    ArrayView, BitRow, BitsView, Cell, Frame, GraphEdgeV, GraphNodeV, GraphView, GridView, KvView,
    LinkedListView, ListNode, LogEntry, LogKind, StackKind, StackView, Step, StepEvent, TextView,
    Trace, TreeNodeV, TreeView, VarMap, VarVal, VizView,
};
pub use problem::{
    validate_inputs, CatalogItem, Difficulty, InputField, InputMap, InputType, InputValue, LangId,
    LanguageDef, ProblemMeta, TestCase, Tier, Toolchain,
};
pub use timeline::{StepMode, Timeline};

/// Hard ceiling on recorded steps. A runaway script is a bug in the content,
/// not a reason to hang the UI, so the tracer stops and reports instead.
pub const MAX_STEPS: usize = 4000;

/// Name of the function a trace script must define.
pub const TRACE_ENTRY: &str = "trace";
