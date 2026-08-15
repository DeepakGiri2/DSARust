//! The Rhai host: the authoring API that turns a `trace.rhai` file into a
//! recorded [`Trace`].
//!
//! This module *is* the extension point. A new problem — or a redesigned
//! animation for an existing one — is a script on disk plus a manifest; the
//! binary never changes. Scripts are sandboxed (no filesystem, no network, no
//! process spawning; bounded operations, recursion, allocation and steps), so
//! a downloaded content pack cannot do anything worse than fail to trace.
//!
//! The API mirrors the vocabulary the TypeScript version used — call-stack
//! bookkeeping, variable updates, a step recorder and view builders — so
//! existing problems port across almost mechanically. The two renames are
//! `enter`/`leave` for what TS called `push`/`pop`, because Rhai already
//! defines `push` and `pop` on arrays and strings.
//!
//! ```text
//! fn trace(input) {
//!     let nums = input.nums;
//!     enter("twoSum", #{ nums: A(nums), target: N(input.target) });
//!     step("init", "Start with an empty map.", [ array("nums", nums), kv("seen", []) ]);
//!     ...
//! }
//! ```

use crate::convert::*;
use dsa_core::model::*;
use dsa_core::problem::{InputMap, InputValue};
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map, Scope, AST};
use std::sync::{Arc, Mutex};

/// Wrapper so view builders can hang chainable methods off a single Rhai type.
#[derive(Clone)]
pub struct View(pub VizView);

#[derive(Debug, thiserror::Error)]
pub enum ScriptError {
    #[error("{0}")]
    Parse(String),
    #[error("{0}")]
    Runtime(String),
    #[error("script has no `fn {0}(input)`")]
    MissingEntry(&'static str),
}

/// Mutable state shared between the script and the host for one run.
#[derive(Default)]
struct Recorder {
    steps: Vec<Step>,
    stack: Vec<Frame>,
    logs: Vec<LogEntry>,
    result: Option<String>,
}

type Shared = Arc<Mutex<Recorder>>;

// ─────────────────────────────────────────────────────────────────────────────
// Engine construction
// ─────────────────────────────────────────────────────────────────────────────

fn base_engine() -> Engine {
    let mut engine = Engine::new();
    // Sandbox + runaway protection. These numbers are generous for an
    // algorithm animation and still fail fast on an accidental infinite loop.
    engine.set_max_operations(40_000_000);
    engine.set_max_call_levels(256);
    engine.set_max_expr_depths(128, 64);
    engine.set_max_array_size(200_000);
    engine.set_max_map_size(200_000);
    engine.set_max_string_size(200_000);
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});
    engine
}

/// Build an engine wired to `rec`. One engine per run keeps runs independent
/// and makes the recorder impossible to leak between problems.
fn engine_with(rec: Shared) -> Engine {
    let mut engine = base_engine();
    register_values(&mut engine);
    register_views(&mut engine);
    register_helpers(&mut engine);
    register_recorder(&mut engine, rec);
    engine
}

// ── variables ───────────────────────────────────────────────────────────────

fn register_values(engine: &mut Engine) {
    engine.register_type_with_name::<VarVal>("Var");

    engine.register_fn("N", |d: Dynamic| -> Result<VarVal, Box<EvalAltResult>> {
        Ok(VarVal::Num { v: to_cell(&d)? })
    });
    engine.register_fn("S", |d: Dynamic| VarVal::Str { v: d.to_string() });
    engine.register_fn("B", |b: bool| VarVal::Bool { v: b });
    engine.register_fn("PTR", |d: Dynamic| VarVal::Ptr { v: d.to_string() });
    engine.register_fn("NIL", || VarVal::Null);

    engine.register_fn("A", |d: Dynamic| -> Result<VarVal, Box<EvalAltResult>> {
        Ok(VarVal::Arr {
            v: to_cells(&d)?,
            hl: vec![],
        })
    });
    engine.register_fn(
        "A",
        |d: Dynamic, hl: Dynamic| -> Result<VarVal, Box<EvalAltResult>> {
            Ok(VarVal::Arr {
                v: to_cells(&d)?,
                hl: to_i64_list(&hl)?
                    .into_iter()
                    .map(|i| i.max(0) as usize)
                    .collect(),
            })
        },
    );
    engine.register_fn("MAP", |d: Dynamic| -> Result<VarVal, Box<EvalAltResult>> {
        Ok(VarVal::Map {
            v: to_entries(&d)?,
            hl: vec![],
        })
    });
    engine.register_fn(
        "MAP",
        |d: Dynamic, hl: Dynamic| -> Result<VarVal, Box<EvalAltResult>> {
            Ok(VarVal::Map {
                v: to_entries(&d)?,
                hl: to_string_list(&hl)?,
            })
        },
    );
    engine.register_fn("SET", |d: Dynamic| -> Result<VarVal, Box<EvalAltResult>> {
        Ok(VarVal::Set {
            v: to_cells(&d)?,
            hl: vec![],
        })
    });
    engine.register_fn(
        "SET",
        |d: Dynamic, hl: Dynamic| -> Result<VarVal, Box<EvalAltResult>> {
            Ok(VarVal::Set {
                v: to_cells(&d)?,
                hl: to_cells(&hl)?,
            })
        },
    );
}

// ── views ───────────────────────────────────────────────────────────────────

macro_rules! view_fn {
    ($engine:expr, $name:literal, |$($arg:ident : $ty:ty),*| $body:expr) => {
        $engine.register_fn($name, |$($arg: $ty),*| -> Result<View, Box<EvalAltResult>> { $body });
    };
}

fn register_views(engine: &mut Engine) {
    engine.register_type_with_name::<View>("View");

    view_fn!(engine, "array", |label: &str, data: Dynamic| Ok(View(
        VizView::Array(ArrayView {
            label: label.into(),
            data: to_cells(&data)?,
            ..Default::default()
        })
    )));
    view_fn!(engine, "bars", |label: &str, data: Dynamic| Ok(View(
        VizView::Array(ArrayView {
            label: label.into(),
            data: to_cells(&data)?,
            bars: true,
            ..Default::default()
        })
    )));
    view_fn!(engine, "kv", |label: &str, entries: Dynamic| Ok(View(
        VizView::Kv(KvView {
            label: label.into(),
            entries: to_entries(&entries)?,
            ..Default::default()
        })
    )));
    view_fn!(engine, "text", |label: &str, lines: Dynamic| Ok(View(
        VizView::Text(TextView {
            label: label.into(),
            lines: to_string_list(&lines)?,
            ..Default::default()
        })
    )));

    for (name, kind) in [
        ("stack", StackKind::Stack),
        ("queue", StackKind::Queue),
        ("deque", StackKind::Deque),
        ("heap", StackKind::Heap),
    ] {
        engine.register_fn(
            name,
            move |label: &str, items: Dynamic| -> Result<View, Box<EvalAltResult>> {
                Ok(View(VizView::Stack(StackView {
                    label: label.into(),
                    items: to_cells(&items)?,
                    kind,
                    ..Default::default()
                })))
            },
        );
    }

    // Linked list from plain values, or from explicit #{id, val, next} nodes.
    view_fn!(engine, "list", |label: &str, data: Dynamic| {
        let nodes = if data.is_array() {
            let arr = data.clone().into_array().unwrap_or_default();
            if arr.first().map(|x| x.is_map()).unwrap_or(false) {
                arr.iter()
                    .map(|n| {
                        let m = n.clone().try_cast::<Map>().unwrap_or_default();
                        Ok(ListNode {
                            id: to_i64(m.get("id").unwrap_or(&Dynamic::from(0_i64)))?,
                            val: to_cell(m.get("val").unwrap_or(&Dynamic::UNIT))?,
                            next: match m.get("next") {
                                Some(d) => to_opt_i64(d)?,
                                None => None,
                            },
                        })
                    })
                    .collect::<Result<Vec<_>, Box<EvalAltResult>>>()?
            } else {
                list_from_values(&to_cells(&data)?)
            }
        } else {
            list_from_values(&to_cells(&data)?)
        };
        Ok(View(VizView::List(LinkedListView {
            label: label.into(),
            nodes,
            ..Default::default()
        })))
    });

    // Tree from level-order text, or from explicit #{id, val, left, right}.
    view_fn!(engine, "tree", |label: &str, spec: Dynamic| {
        let (nodes, root) = if spec.is_string() {
            parse_tree(&spec.clone().into_string().unwrap_or_default())
        } else {
            let arr = spec.clone().into_array().map_err(|_| {
                rt_err("tree() wants level-order text or a list of #{id, val, left, right}")
            })?;
            let nodes = arr
                .iter()
                .map(|n| {
                    let m = n.clone().try_cast::<Map>().unwrap_or_default();
                    Ok(TreeNodeV {
                        id: to_i64(m.get("id").unwrap_or(&Dynamic::from(0_i64)))?,
                        val: to_cell(m.get("val").unwrap_or(&Dynamic::UNIT))?,
                        left: match m.get("left") {
                            Some(d) => to_opt_i64(d)?,
                            None => None,
                        },
                        right: match m.get("right") {
                            Some(d) => to_opt_i64(d)?,
                            None => None,
                        },
                    })
                })
                .collect::<Result<Vec<_>, Box<EvalAltResult>>>()?;
            let root = nodes.first().map(|n| n.id);
            (nodes, root)
        };
        Ok(View(VizView::Tree(TreeView {
            label: label.into(),
            nodes,
            root,
            ..Default::default()
        })))
    });

    view_fn!(engine, "grid", |label: &str, spec: Dynamic| Ok(View(
        VizView::Grid(GridView {
            label: label.into(),
            data: parse_grid(&spec)?,
            ..Default::default()
        })
    )));

    view_fn!(
        engine,
        "graph",
        |label: &str, nodes: Dynamic, edges: Dynamic| {
            let ns = nodes
                .clone()
                .into_array()
                .map_err(|_| rt_err("graph() wants a node list"))?;
            let nodes = ns
                .iter()
                .enumerate()
                .map(|(i, n)| {
                    if let Some(m) = n.clone().try_cast::<Map>() {
                        Ok(GraphNodeV {
                            id: match m.get("id") {
                                Some(d) => to_i64(d)?,
                                None => i as i64,
                            },
                            label: m
                                .get("label")
                                .map(|d| d.to_string())
                                .unwrap_or_else(|| i.to_string()),
                            x: m.get("x")
                                .and_then(|d| d.clone().try_cast::<f64>())
                                .map(|v| v as f32),
                            y: m.get("y")
                                .and_then(|d| d.clone().try_cast::<f64>())
                                .map(|v| v as f32),
                        })
                    } else {
                        Ok(GraphNodeV {
                            id: i as i64,
                            label: to_cell(n)?.to_string(),
                            x: None,
                            y: None,
                        })
                    }
                })
                .collect::<Result<Vec<_>, Box<EvalAltResult>>>()?;
            let es = edges.clone().into_array().unwrap_or_default();
            let edges = es
                .iter()
                .map(|e| {
                    let p = e
                        .clone()
                        .into_array()
                        .map_err(|_| rt_err("edges are [from, to]"))?;
                    if p.len() < 2 {
                        return Err(rt_err("edges are [from, to]"));
                    }
                    Ok(GraphEdgeV {
                        from: to_i64(&p[0])?,
                        to: to_i64(&p[1])?,
                        directed: p
                            .get(2)
                            .map(|d| d.as_bool().unwrap_or(false))
                            .unwrap_or(false),
                        weight: match p.get(3) {
                            Some(d) if !d.is_unit() => Some(to_cell(d)?),
                            _ => None,
                        },
                    })
                })
                .collect::<Result<Vec<_>, Box<EvalAltResult>>>()?;
            Ok(View(VizView::Graph(GraphView {
                label: label.into(),
                nodes,
                edges,
                ..Default::default()
            })))
        }
    );

    view_fn!(engine, "bits", |label: &str, rows: Dynamic| {
        let arr = rows
            .clone()
            .into_array()
            .map_err(|_| rt_err("bits() wants [[label, value], ...]"))?;
        let rows = arr
            .iter()
            .map(|r| {
                if let Some(m) = r.clone().try_cast::<Map>() {
                    return Ok(BitRow {
                        label: m.get("label").map(|d| d.to_string()).unwrap_or_default(),
                        value: to_i64(m.get("value").unwrap_or(&Dynamic::from(0_i64)))?,
                        width: m.get("width").map(to_i64).transpose()?.unwrap_or(32) as u32,
                        hl: match m.get("hl") {
                            Some(d) => to_i64_list(d)?
                                .into_iter()
                                .map(|v| v.max(0) as u32)
                                .collect(),
                            None => vec![],
                        },
                    });
                }
                let p = r
                    .clone()
                    .into_array()
                    .map_err(|_| rt_err("bits rows are [label, value]"))?;
                Ok(BitRow {
                    label: p.first().map(|d| d.to_string()).unwrap_or_default(),
                    value: p.get(1).map(to_i64).transpose()?.unwrap_or(0),
                    width: p.get(2).map(to_i64).transpose()?.unwrap_or(32) as u32,
                    hl: match p.get(3) {
                        Some(d) => to_i64_list(d)?
                            .into_iter()
                            .map(|v| v.max(0) as u32)
                            .collect(),
                        None => vec![],
                    },
                })
            })
            .collect::<Result<Vec<_>, Box<EvalAltResult>>>()?;
        Ok(View(VizView::Bits(BitsView {
            label: label.into(),
            rows,
        })))
    });

    register_view_setters(engine);
}

fn rt_err(msg: &str) -> Box<EvalAltResult> {
    Box::new(EvalAltResult::ErrorRuntime(
        msg.to_string().into(),
        rhai::Position::NONE,
    ))
}

/// Chainable decorations. One name works across view kinds — `.hl(…)` means
/// "highlight these", whether *these* are array indices, grid cells, tree ids
/// or map keys. Applying a decoration a view does not support is a no-op
/// rather than an error, so shared helper functions stay simple.
fn register_view_setters(engine: &mut Engine) {
    engine.register_fn(
        "ptr",
        |v: View, name: &str, idx: Dynamic| -> Result<View, Box<EvalAltResult>> {
            let mut v = v;
            match &mut v.0 {
                VizView::Array(a) => {
                    let i = to_i64(&idx)?;
                    set_pointer(&mut a.pointers, name, i);
                }
                VizView::List(l) => {
                    let id = to_opt_i64(&idx)?;
                    match l.pointers.iter_mut().find(|(n, _)| n == name) {
                        Some(slot) => slot.1 = id,
                        None => l.pointers.push((name.to_string(), id)),
                    }
                }
                _ => {}
            }
            Ok(v)
        },
    );

    engine.register_fn(
        "pointers",
        |v: View, p: Dynamic| -> Result<View, Box<EvalAltResult>> {
            let mut v = v;
            match &mut v.0 {
                VizView::Array(a) => a.pointers = to_pointers(&p)?,
                VizView::List(l) => l.pointers = to_opt_pointers(&p)?,
                _ => {}
            }
            Ok(v)
        },
    );

    engine.register_fn("window", |v: View, lo: i64, hi: i64| {
        let mut v = v;
        if let VizView::Array(a) = &mut v.0 {
            a.window = Some((lo, hi));
        }
        v
    });

    engine.register_fn("bars", |v: View, on: bool| {
        let mut v = v;
        if let VizView::Array(a) = &mut v.0 {
            a.bars = on;
        }
        v
    });

    // hl / bad / done / path all share the same shape-dispatch.
    for name in ["hl", "bad", "done", "path"] {
        engine.register_fn(
            name,
            move |v: View, d: Dynamic| -> Result<View, Box<EvalAltResult>> {
                let mut v = v;
                match &mut v.0 {
                    VizView::Array(a) => {
                        let idx = to_i64_list(&d)?;
                        match name {
                            "hl" => a.hl = idx,
                            "bad" => a.bad = idx,
                            _ => a.done = idx,
                        }
                    }
                    VizView::Grid(g) => {
                        let rc = to_rc_list(&d)?;
                        match name {
                            "hl" => g.hl = rc,
                            "bad" => g.bad = rc,
                            "done" => g.done = rc,
                            _ => g.path = rc,
                        }
                    }
                    VizView::Tree(t) => {
                        let ids = to_i64_list(&d)?;
                        match name {
                            "done" => t.done = ids,
                            "path" => t.path = ids,
                            "hl" => t.path = ids,
                            _ => {}
                        }
                    }
                    VizView::Graph(g) => {
                        let ids = to_i64_list(&d)?;
                        match name {
                            "done" => g.done = ids,
                            "bad" => g.bad = ids,
                            _ => g.frontier = ids,
                        }
                    }
                    VizView::Kv(k) => {
                        let keys = to_string_list(&d)?;
                        match name {
                            "bad" => k.bad_keys = keys,
                            _ => k.hl_keys = keys,
                        }
                    }
                    VizView::Text(t) => {
                        t.hl = to_i64_list(&d)?
                            .into_iter()
                            .map(|i| i.max(0) as usize)
                            .collect();
                    }
                    VizView::Stack(s) => {
                        if name == "bad" {
                            s.bad = true;
                        }
                    }
                    VizView::Bits(_) | VizView::List(_) => {}
                }
                Ok(v)
            },
        );
    }

    engine.register_fn(
        "keys",
        |v: View, d: Dynamic| -> Result<View, Box<EvalAltResult>> {
            let mut v = v;
            if let VizView::Kv(k) = &mut v.0 {
                k.hl_keys = to_string_list(&d)?;
            }
            Ok(v)
        },
    );

    engine.register_fn(
        "cur",
        |v: View, d: Dynamic| -> Result<View, Box<EvalAltResult>> {
            let mut v = v;
            match &mut v.0 {
                VizView::Grid(g) => g.cur = to_rc(&d)?,
                VizView::Tree(t) => t.cur = to_opt_i64(&d)?,
                VizView::Graph(g) => g.cur = to_opt_i64(&d)?,
                VizView::Array(a) => {
                    if let Some(i) = to_opt_i64(&d)? {
                        set_pointer(&mut a.pointers, "cur", i);
                    }
                }
                _ => {}
            }
            Ok(v)
        },
    );

    engine.register_fn(
        "root",
        |v: View, d: Dynamic| -> Result<View, Box<EvalAltResult>> {
            let mut v = v;
            if let VizView::Tree(t) = &mut v.0 {
                t.root = to_opt_i64(&d)?;
            }
            Ok(v)
        },
    );

    engine.register_fn("swap", |v: View, a: i64, b: i64| {
        let mut v = v;
        if let VizView::Tree(t) = &mut v.0 {
            t.swap = Some((a, b));
        }
        v
    });

    engine.register_fn("pushed", |v: View, on: bool| {
        let mut v = v;
        if let VizView::Stack(s) = &mut v.0 {
            s.pushed = on;
        }
        v
    });

    engine.register_fn(
        "popped",
        |v: View, d: Dynamic| -> Result<View, Box<EvalAltResult>> {
            let mut v = v;
            if let VizView::Stack(s) = &mut v.0 {
                s.popped = if d.is_unit() {
                    None
                } else {
                    Some(to_cell(&d)?)
                };
            }
            Ok(v)
        },
    );

    engine.register_fn(
        "reversed",
        |v: View, d: Dynamic| -> Result<View, Box<EvalAltResult>> {
            let mut v = v;
            if let VizView::List(l) = &mut v.0 {
                l.reversed = to_i64_list(&d)?;
            }
            Ok(v)
        },
    );

    engine.register_fn(
        "frontier",
        |v: View, d: Dynamic| -> Result<View, Box<EvalAltResult>> {
            let mut v = v;
            if let VizView::Graph(g) = &mut v.0 {
                g.frontier = to_i64_list(&d)?;
            }
            Ok(v)
        },
    );

    engine.register_fn(
        "active",
        |v: View, d: Dynamic| -> Result<View, Box<EvalAltResult>> {
            let mut v = v;
            if let VizView::Graph(g) = &mut v.0 {
                g.active = to_rc_list(&d)?;
            }
            Ok(v)
        },
    );

    engine.register_fn(
        "labels",
        |v: View, rows: Dynamic, cols: Dynamic| -> Result<View, Box<EvalAltResult>> {
            let mut v = v;
            if let VizView::Grid(g) = &mut v.0 {
                g.row_labels = to_string_list(&rows)?;
                g.col_labels = to_string_list(&cols)?;
            }
            Ok(v)
        },
    );

    engine.register_fn("label", |v: View, s: &str| {
        let mut v = v;
        match &mut v.0 {
            VizView::Array(x) => x.label = s.into(),
            VizView::Kv(x) => x.label = s.into(),
            VizView::Stack(x) => x.label = s.into(),
            VizView::List(x) => x.label = s.into(),
            VizView::Tree(x) => x.label = s.into(),
            VizView::Grid(x) => x.label = s.into(),
            VizView::Graph(x) => x.label = s.into(),
            VizView::Bits(x) => x.label = s.into(),
            VizView::Text(x) => x.label = s.into(),
        }
        v
    });
}

fn set_pointer(ptrs: &mut Vec<(String, i64)>, name: &str, idx: i64) {
    match ptrs.iter_mut().find(|(n, _)| n == name) {
        Some(slot) => slot.1 = idx,
        None => ptrs.push((name.to_string(), idx)),
    }
}

// ── recorder ────────────────────────────────────────────────────────────────

fn register_recorder(engine: &mut Engine, rec: Shared) {
    // Deliberately *not* named `push`/`pop`: Rhai already defines those for
    // arrays and strings, and its overload resolution prefers the concrete
    // built-in over a `Dynamic` parameter — `pop("x")` would silently pop a
    // character off a string instead of unwinding the call stack.
    let r = rec.clone();
    engine.register_fn("enter", move |name: &str| {
        let mut g = r.lock().unwrap();
        let n = g.steps.len();
        g.stack.push(Frame {
            fn_name: name.into(),
            vars: VarMap::new(),
        });
        g.logs.push(LogEntry {
            step: n,
            text: format!("-> enter {name}"),
            kind: LogKind::Call,
        });
    });

    let r = rec.clone();
    engine.register_fn(
        "enter",
        move |name: &str, vars: Dynamic| -> Result<(), Box<EvalAltResult>> {
            let mut frame = Frame {
                fn_name: name.into(),
                vars: VarMap::new(),
            };
            for (k, v) in to_vars(&vars)? {
                frame.vars.set(k, v);
            }
            let mut g = r.lock().unwrap();
            let n = g.steps.len();
            g.stack.push(frame);
            g.logs.push(LogEntry {
                step: n,
                text: format!("-> enter {name}"),
                kind: LogKind::Call,
            });
            Ok(())
        },
    );

    let r = rec.clone();
    engine.register_fn("leave", move || {
        r.lock().unwrap().stack.pop();
    });

    let r = rec.clone();
    engine.register_fn("leave", move |ret: Dynamic| {
        let mut g = r.lock().unwrap();
        let n = g.steps.len();
        if let Some(f) = g.stack.pop() {
            let text = format!("<- {} returns {}", f.fn_name, ret);
            g.logs.push(LogEntry {
                step: n,
                text,
                kind: LogKind::Return,
            });
        }
    });

    let r = rec.clone();
    engine.register_fn(
        "set",
        move |vars: Dynamic| -> Result<(), Box<EvalAltResult>> {
            let vars = to_vars(&vars)?;
            let mut g = r.lock().unwrap();
            if let Some(top) = g.stack.last_mut() {
                for (k, v) in vars {
                    top.vars.set(k, v);
                }
            }
            Ok(())
        },
    );

    let r = rec.clone();
    engine.register_fn(
        "setv",
        move |name: &str, v: Dynamic| -> Result<(), Box<EvalAltResult>> {
            let v = to_var(&v)?;
            let mut g = r.lock().unwrap();
            if let Some(top) = g.stack.last_mut() {
                top.vars.set(name, v);
            }
            Ok(())
        },
    );

    let r = rec.clone();
    engine.register_fn("unset", move |name: &str| {
        let mut g = r.lock().unwrap();
        if let Some(top) = g.stack.last_mut() {
            top.vars.remove(name);
        }
    });

    let r = rec.clone();
    engine.register_fn("log", move |text: &str| {
        let mut g = r.lock().unwrap();
        let n = g.steps.len();
        g.logs.push(LogEntry {
            step: n,
            text: text.into(),
            kind: LogKind::Log,
        });
    });

    let r = rec.clone();
    engine.register_fn("log", move |text: &str, kind: &str| {
        let kind = match kind {
            "call" => LogKind::Call,
            "return" => LogKind::Return,
            "result" => LogKind::Result,
            _ => LogKind::Log,
        };
        let mut g = r.lock().unwrap();
        let n = g.steps.len();
        g.logs.push(LogEntry {
            step: n,
            text: text.into(),
            kind,
        });
    });

    let r = rec.clone();
    engine.register_fn("result", move |text: &str| {
        r.lock().unwrap().result = Some(text.into());
    });

    let r = rec.clone();
    engine.register_fn("steps_taken", move || r.lock().unwrap().steps.len() as i64);

    let r = rec.clone();
    engine.register_fn("step", move |tag: &str, note: &str, views: Dynamic| {
        record_step(&r, tag, note, views, StepEvent::Stmt)
    });

    let r = rec.clone();
    engine.register_fn(
        "step",
        move |tag: &str, note: &str, views: Dynamic, event: &str| {
            let ev = match event {
                "call" => StepEvent::Call,
                "return" => StepEvent::Return,
                _ => StepEvent::Stmt,
            };
            record_step(&r, tag, note, views, ev)
        },
    );
}

fn record_step(
    rec: &Shared,
    tag: &str,
    note: &str,
    views: Dynamic,
    event: StepEvent,
) -> Result<(), Box<EvalAltResult>> {
    let views = collect_views(&views)?;
    let mut g = rec.lock().unwrap();
    if g.steps.len() >= dsa_core::MAX_STEPS {
        return Err(rt_err(&format!(
            "trace exceeded {} steps - try a smaller input",
            dsa_core::MAX_STEPS
        )));
    }
    let depth = g.stack.len();
    let frames = g.stack.clone();
    let log_len = g.logs.len();
    g.steps.push(Step {
        tag: tag.to_string(),
        depth,
        event,
        frames,
        views,
        note: note.to_string(),
        log_len,
    });
    Ok(())
}

fn collect_views(d: &Dynamic) -> Result<Vec<VizView>, Box<EvalAltResult>> {
    if d.is_unit() {
        return Ok(vec![]);
    }
    if let Some(v) = d.clone().try_cast::<View>() {
        return Ok(vec![v.0]);
    }
    let arr: Array = d
        .clone()
        .into_array()
        .map_err(|_| rt_err("step() wants a view or a list of views"))?;
    arr.into_iter()
        .map(|x| {
            x.try_cast::<View>()
                .map(|v| v.0)
                .ok_or_else(|| rt_err("step() view list contains something that is not a view"))
        })
        .collect()
}

// ── small helpers scripts keep needing ──────────────────────────────────────

fn register_helpers(engine: &mut Engine) {
    register_ordered_collections(engine);

    engine.register_fn("chars", |s: &str| {
        s.chars()
            .map(|c| Dynamic::from(c.to_string()))
            .collect::<Array>()
    });

    // Small array utilities that every second script would otherwise reinvent.
    engine.register_fn("join", |a: Array, sep: &str| {
        a.iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join(sep)
    });
    engine.register_fn("seq", |from: i64, to: i64| {
        // Inclusive, and empty rather than reversed when to < from.
        (from..=to.max(from - 1))
            .map(Dynamic::from)
            .collect::<Array>()
    });
    engine.register_fn("sum_of", |a: Array| {
        a.iter().filter_map(|d| to_i64(d).ok()).sum::<i64>()
    });
    engine.register_fn("max_of", |a: Array| {
        a.iter().filter_map(|d| to_i64(d).ok()).max().unwrap_or(0)
    });
    engine.register_fn("min_of", |a: Array| {
        a.iter().filter_map(|d| to_i64(d).ok()).min().unwrap_or(0)
    });
    engine.register_fn("count_of", |a: Array, v: Dynamic| {
        let key = to_cell(&v).map(|c| c.key()).unwrap_or_default();
        a.iter()
            .filter(|d| to_cell(d).map(|c| c.key()).unwrap_or_default() == key)
            .count() as i64
    });
    engine.register_fn("swap_at", |a: &mut Array, i: i64, j: i64| {
        let (i, j) = (i.max(0) as usize, j.max(0) as usize);
        if i < a.len() && j < a.len() {
            a.swap(i, j);
        }
    });
    // 2-D grids are arrays of arrays; mutating a cell through Rhai's indexing
    // rules is fiddly enough that a helper is worth it.
    engine.register_fn("grid_set", |g: &mut Array, r: i64, c: i64, v: Dynamic| {
        if let Some(row) = g.get_mut(r.max(0) as usize) {
            if let Some(mut arr) = row.clone().try_cast::<Array>() {
                let c = c.max(0) as usize;
                if c < arr.len() {
                    arr[c] = v;
                    *row = Dynamic::from(arr);
                }
            }
        }
    });
    engine.register_fn("grid_get", |g: Array, r: i64, c: i64| -> Dynamic {
        g.get(r.max(0) as usize)
            .and_then(|row| row.clone().try_cast::<Array>())
            .and_then(|row| row.get(c.max(0) as usize).cloned())
            .unwrap_or(Dynamic::UNIT)
    });
    engine.register_fn("ord", |s: &str| {
        s.chars().next().map(|c| c as i64).unwrap_or(0)
    });
    engine.register_fn("chr", |i: i64| {
        char::from_u32(i.clamp(0, 0x10_FFFF) as u32)
            .map(String::from)
            .unwrap_or_default()
    });
    engine.register_fn("num", |s: &str| s.trim().parse::<i64>().unwrap_or(0));
    engine.register_fn("words", |s: &str| {
        s.split_whitespace()
            .map(|w| Dynamic::from(w.to_string()))
            .collect::<Array>()
    });
    engine.register_fn("repeat", |s: &str, n: i64| {
        s.repeat(n.clamp(0, 4096) as usize)
    });
    // Level-order text -> the node list `tree()` also accepts, so scripts can
    // inspect structure (child ids, values) before rendering.
    engine.register_fn("tree_nodes", |spec: &str| {
        let (nodes, _) = parse_tree(spec);
        nodes
            .into_iter()
            .map(|n| {
                let mut m = Map::new();
                m.insert("id".into(), Dynamic::from(n.id));
                m.insert("val".into(), cell_to_dynamic(&n.val));
                m.insert(
                    "left".into(),
                    n.left.map(Dynamic::from).unwrap_or(Dynamic::UNIT),
                );
                m.insert(
                    "right".into(),
                    n.right.map(Dynamic::from).unwrap_or(Dynamic::UNIT),
                );
                Dynamic::from(m)
            })
            .collect::<Array>()
    });
    engine.register_fn("grid_rows", |spec: &str| {
        spec.split_whitespace()
            .map(|row| {
                row.chars()
                    .map(|c| Dynamic::from(c.to_string()))
                    .collect::<Array>()
            })
            .map(Dynamic::from)
            .collect::<Array>()
    });
}

/// Insertion-ordered maps and sets, represented as an array of `[key, value]`
/// pairs (a set stores `[value, value]`).
///
/// Rhai's own object maps are sorted by key, which would be actively wrong
/// here: watching `seen[x] = i` land at the *bottom* of the hash map is how a
/// viewer follows a one-pass scan. These also drop straight into `kv(...)`,
/// since the pair-array is exactly the view's entry format.
fn register_ordered_collections(engine: &mut Engine) {
    fn key_of(d: &Dynamic) -> String {
        to_cell(d)
            .map(|c| c.key())
            .unwrap_or_else(|_| d.to_string())
    }
    fn find(arr: &Array, key: &str) -> Option<usize> {
        arr.iter().position(|e| {
            e.clone()
                .try_cast::<Array>()
                .and_then(|p| p.first().map(key_of))
                .is_some_and(|k| k == key)
        })
    }
    fn pair(k: Dynamic, v: Dynamic) -> Dynamic {
        Dynamic::from(vec![k, v])
    }

    engine.register_fn("map_new", Array::new);
    engine.register_fn("set_new", Array::new);

    engine.register_fn(
        "map_set",
        |m: &mut Array, k: Dynamic, v: Dynamic| match find(m, &key_of(&k)) {
            Some(i) => m[i] = pair(k, v),
            None => m.push(pair(k, v)),
        },
    );
    engine.register_fn("map_get", |m: Array, k: Dynamic| -> Dynamic {
        find(&m, &key_of(&k))
            .and_then(|i| m[i].clone().try_cast::<Array>())
            .and_then(|p| p.get(1).cloned())
            .unwrap_or(Dynamic::UNIT)
    });
    engine.register_fn("map_has", |m: Array, k: Dynamic| {
        find(&m, &key_of(&k)).is_some()
    });
    engine.register_fn("map_del", |m: &mut Array, k: Dynamic| {
        if let Some(i) = find(m, &key_of(&k)) {
            let _ = m.remove(i);
        }
    });
    engine.register_fn("map_len", |m: Array| m.len() as i64);
    engine.register_fn("map_keys", |m: Array| {
        m.iter()
            .filter_map(|e| e.clone().try_cast::<Array>())
            .filter_map(|p| p.first().cloned())
            .collect::<Array>()
    });
    engine.register_fn("map_values", |m: Array| {
        m.iter()
            .filter_map(|e| e.clone().try_cast::<Array>())
            .filter_map(|p| p.get(1).cloned())
            .collect::<Array>()
    });
    // Counting is so common (anagrams, frequencies, character windows) that
    // doing it by hand in every script is just noise.
    engine.register_fn("map_inc", |m: &mut Array, k: Dynamic, by: i64| -> i64 {
        let key = key_of(&k);
        match find(m, &key) {
            Some(i) => {
                let cur = m[i]
                    .clone()
                    .try_cast::<Array>()
                    .and_then(|p| p.get(1).and_then(|d| d.clone().try_cast::<i64>()))
                    .unwrap_or(0);
                let next = cur + by;
                m[i] = pair(k, Dynamic::from(next));
                next
            }
            None => {
                m.push(pair(k, Dynamic::from(by)));
                by
            }
        }
    });

    engine.register_fn("set_add", |s: &mut Array, v: Dynamic| {
        if find(s, &key_of(&v)).is_none() {
            s.push(pair(v.clone(), v));
        }
    });
    engine.register_fn("set_has", |s: Array, v: Dynamic| {
        find(&s, &key_of(&v)).is_some()
    });
    engine.register_fn("set_del", |s: &mut Array, v: Dynamic| {
        if let Some(i) = find(s, &key_of(&v)) {
            let _ = s.remove(i);
        }
    });
    engine.register_fn("set_len", |s: Array| s.len() as i64);
    // A set renders as a plain list of its members.
    engine.register_fn("set_items", |s: Array| {
        s.iter()
            .filter_map(|e| e.clone().try_cast::<Array>())
            .filter_map(|p| p.first().cloned())
            .collect::<Array>()
    });
}

fn cell_to_dynamic(c: &Cell) -> Dynamic {
    match c {
        Cell::Num(n) if n.fract() == 0.0 => Dynamic::from(*n as i64),
        Cell::Num(n) => Dynamic::from(*n),
        Cell::Str(s) => Dynamic::from(s.clone()),
    }
}

fn input_to_dynamic(v: &InputValue) -> Dynamic {
    match v {
        InputValue::Bool(b) => Dynamic::from(*b),
        InputValue::Int(i) => Dynamic::from(*i),
        InputValue::Float(f) => Dynamic::from(*f),
        InputValue::Str(s) => Dynamic::from(s.clone()),
        InputValue::Null => Dynamic::UNIT,
        InputValue::List(v) => Dynamic::from(v.iter().map(input_to_dynamic).collect::<Array>()),
    }
}

pub fn input_map_to_rhai(input: &InputMap) -> Map {
    input
        .iter()
        .map(|(k, v)| (k.into(), input_to_dynamic(v)))
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Public entry points
// ─────────────────────────────────────────────────────────────────────────────

/// Compiles trace scripts and runs them. Holding one of these keeps the
/// (cheap) compile-only engine warm; each *run* gets its own engine and
/// recorder so traces can never bleed into each other.
pub struct ScriptHost {
    compiler: Engine,
}

impl Default for ScriptHost {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptHost {
    pub fn new() -> Self {
        // The compile engine still needs the registrations so that the
        // optimizer sees the same function namespace as the run engine.
        Self {
            compiler: engine_with(Shared::default()),
        }
    }

    /// `prelude` is the shared `content/lib/*.rhai` code, concatenated ahead of
    /// the problem script so helpers are plain global functions.
    pub fn compile(&self, prelude: &str, source: &str) -> Result<AST, ScriptError> {
        let combined = if prelude.is_empty() {
            source.to_string()
        } else {
            format!("{prelude}\n{source}")
        };
        self.compiler
            .compile(&combined)
            .map_err(|e| ScriptError::Parse(e.to_string()))
    }

    /// Run `fn trace(input)` and collect what it recorded.
    pub fn run_trace(&self, ast: &AST, input: &InputMap) -> Result<Trace, ScriptError> {
        let rec: Shared = Shared::default();
        let engine = engine_with(rec.clone());
        let mut scope = Scope::new();
        let args = (input_map_to_rhai(input),);

        match engine.call_fn::<Dynamic>(&mut scope, ast, dsa_core::TRACE_ENTRY, args) {
            Ok(_) => {}
            Err(e) => {
                if let EvalAltResult::ErrorFunctionNotFound(name, _) = &*e {
                    if name.starts_with(dsa_core::TRACE_ENTRY) {
                        return Err(ScriptError::MissingEntry(dsa_core::TRACE_ENTRY));
                    }
                }
                // Keep whatever was recorded before the failure: a half-built
                // trace plus the error is far more useful when authoring than
                // an empty screen.
                let partial = rec.lock().unwrap().steps.len();
                return Err(ScriptError::Runtime(if partial > 0 {
                    format!("{e} (after {partial} steps)")
                } else {
                    e.to_string()
                }));
            }
        }

        let g = rec.lock().unwrap();
        Ok(Trace {
            steps: g.steps.clone(),
            logs: g.logs.clone(),
            result: g.result.clone(),
        })
    }

    /// Optional `fn validate(input) -> string?`. Returning a string, or a
    /// non-empty unit, marks the input invalid.
    pub fn run_validate(&self, ast: &AST, input: &InputMap) -> Option<String> {
        if !ast
            .iter_functions()
            .any(|f| f.name == "validate" && f.params.len() == 1)
        {
            return None;
        }
        let engine = engine_with(Shared::default());
        let mut scope = Scope::new();
        match engine.call_fn::<Dynamic>(&mut scope, ast, "validate", (input_map_to_rhai(input),)) {
            Ok(d) if d.is_unit() => None,
            Ok(d) => {
                let s = d.to_string();
                if s.trim().is_empty() || s == "()" {
                    None
                } else {
                    Some(s)
                }
            }
            Err(e) => Some(format!("validate() failed: {e}")),
        }
    }

    /// Tags a script mentions in `step(...)` calls, for the content linter.
    /// A static scan is enough: tags are string literals by convention.
    pub fn declared_tags(source: &str) -> Vec<String> {
        let mut out = Vec::new();
        let bytes = source.as_bytes();
        let mut i = 0;
        while let Some(pos) = source[i..].find("step(") {
            let start = i + pos + 5;
            let rest = &source[start..];
            let rest_trim = rest.trim_start();
            let pad = rest.len() - rest_trim.len();
            if let Some(after_quote) = rest_trim.strip_prefix('"') {
                if let Some(end) = after_quote.find('"') {
                    out.push(after_quote[..end].to_string());
                }
            }
            i = start + pad;
            if i >= bytes.len() {
                break;
            }
        }
        out.sort();
        out.dedup();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dsa_core::problem::InputValue;

    fn run(src: &str) -> Trace {
        let host = ScriptHost::new();
        let ast = host.compile("", src).expect("compiles");
        let mut input = InputMap::new();
        input.insert(
            "nums".into(),
            InputValue::List(vec![InputValue::Int(2), InputValue::Int(7)]),
        );
        input.insert("target".into(), InputValue::Int(9));
        host.run_trace(&ast, &input).expect("runs")
    }

    #[test]
    fn records_steps_frames_and_depth() {
        let t = run(r#"
            fn trace(input) {
                enter("twoSum", #{ target: N(input.target) });
                step("init", "start", [ array("nums", input.nums) ]);
                enter("inner");
                step("deep", "nested", []);
                leave("1");
                step("out", "back", []);
                leave();
            }
        "#);
        assert_eq!(t.len(), 3);
        assert_eq!(t.steps[0].depth, 1);
        assert_eq!(t.steps[1].depth, 2);
        assert_eq!(t.steps[2].depth, 1);
        assert_eq!(t.steps[0].frames[0].fn_name, "twoSum");
        assert_eq!(
            t.steps[0].frames[0].vars.get("target"),
            Some(&VarVal::Num { v: 9.into() })
        );
    }

    #[test]
    fn array_view_decorations_land_on_the_right_fields() {
        let t = run(r#"
            fn trace(input) {
                enter("f");
                step("a", "n", [ array("nums", input.nums).ptr("i", 1).hl([0]).window(0, 1) ]);
            }
        "#);
        match &t.steps[0].views[0] {
            VizView::Array(a) => {
                assert_eq!(a.data.len(), 2);
                assert_eq!(a.pointers, vec![("i".to_string(), 1)]);
                assert_eq!(a.hl, vec![0]);
                assert_eq!(a.window, Some((0, 1)));
            }
            v => panic!("expected array view, got {v:?}"),
        }
    }

    #[test]
    fn hl_dispatches_by_view_kind() {
        let t = run(r#"
            fn trace(input) {
                enter("f");
                step("a", "n", [
                    grid("g", "110 011").hl([[0, 1]]).cur([1, 2]),
                    kv("m", [["a", 1]]).hl(["a"]),
                    tree("t", "4 2 7").done([1]).cur(0),
                ]);
            }
        "#);
        let views = &t.steps[0].views;
        match &views[0] {
            VizView::Grid(g) => {
                assert_eq!(g.hl, vec![(0, 1)]);
                assert_eq!(g.cur, Some((1, 2)));
            }
            _ => panic!("grid"),
        }
        match &views[1] {
            VizView::Kv(k) => assert_eq!(k.hl_keys, vec!["a".to_string()]),
            _ => panic!("kv"),
        }
        match &views[2] {
            VizView::Tree(t) => {
                assert_eq!(t.done, vec![1]);
                assert_eq!(t.cur, Some(0));
                assert_eq!(t.nodes.len(), 3);
            }
            _ => panic!("tree"),
        }
    }

    #[test]
    fn logs_attach_to_the_step_that_follows_them() {
        let t = run(r#"
            fn trace(input) {
                enter("f");
                log("before first");
                step("a", "n", []);
                log("between", "result");
                step("b", "n", []);
            }
        "#);
        assert_eq!(t.steps[0].log_len, 2, "enter log + explicit log");
        assert_eq!(t.logs_at(0).len(), 2);
        assert_eq!(t.logs_at(1).len(), 3);
        assert_eq!(t.logs[2].kind, LogKind::Result);
    }

    #[test]
    fn runtime_errors_report_the_partial_step_count() {
        let host = ScriptHost::new();
        let ast = host
            .compile(
                "",
                r#"fn trace(input) { enter("f"); step("a", "n", []); explode(); }"#,
            )
            .unwrap();
        let e = host.run_trace(&ast, &InputMap::new()).unwrap_err();
        let msg = e.to_string();
        assert!(msg.contains("after 1 steps"), "{msg}");
    }

    #[test]
    fn step_limit_is_enforced() {
        let host = ScriptHost::new();
        let ast = host
            .compile(
                "",
                r#"fn trace(input) { enter("f"); loop { step("a", "n", []); } }"#,
            )
            .unwrap();
        let e = host
            .run_trace(&ast, &InputMap::new())
            .unwrap_err()
            .to_string();
        assert!(e.contains("exceeded"), "{e}");
    }

    #[test]
    fn scripts_cannot_touch_the_filesystem() {
        let host = ScriptHost::new();
        let ast = host
            .compile("", r#"fn trace(input) { open_file("secrets"); }"#)
            .unwrap();
        assert!(host.run_trace(&ast, &InputMap::new()).is_err());
    }

    #[test]
    fn infinite_loops_terminate_via_the_operation_budget() {
        let host = ScriptHost::new();
        let ast = host
            .compile("", "fn trace(input) { let x = 0; loop { x += 1; } }")
            .unwrap();
        assert!(host.run_trace(&ast, &InputMap::new()).is_err());
    }

    #[test]
    fn prelude_functions_are_visible_to_problem_scripts() {
        let host = ScriptHost::new();
        let ast = host
            .compile(
                "fn helper(x) { x * 2 }",
                r#"fn trace(input) { enter("f"); step("a", `${helper(21)}`, []); }"#,
            )
            .unwrap();
        let t = host.run_trace(&ast, &InputMap::new()).unwrap();
        assert_eq!(t.steps[0].note, "42");
    }

    #[test]
    fn validate_returns_none_when_the_script_omits_it() {
        let host = ScriptHost::new();
        let ast = host.compile("", "fn trace(input) {}").unwrap();
        assert_eq!(host.run_validate(&ast, &InputMap::new()), None);
    }

    #[test]
    fn validate_reports_its_message() {
        let host = ScriptHost::new();
        let ast = host
            .compile("", r#"fn validate(input) { "too big" } fn trace(input) {}"#)
            .unwrap();
        assert_eq!(
            host.run_validate(&ast, &InputMap::new()),
            Some("too big".into())
        );
    }

    #[test]
    fn ordered_maps_keep_insertion_order_and_feed_kv_views() {
        let t = run(r#"
            fn trace(input) {
                enter("f");
                let seen = map_new();
                map_set(seen, 7, 0);
                map_set(seen, 2, 1);
                map_set(seen, 7, 9);
                step("a", `${map_len(seen)} ${map_get(seen, 7)} ${map_has(seen, 3)}`, [ kv("seen", seen) ]);
            }
        "#);
        assert_eq!(t.steps[0].note, "2 9 false");
        match &t.steps[0].views[0] {
            VizView::Kv(k) => {
                // 7 was overwritten in place, not moved to the end.
                assert_eq!(k.entries[0].0, "7");
                assert_eq!(k.entries[0].1.to_string(), "9");
                assert_eq!(k.entries[1].0, "2");
            }
            _ => panic!("kv"),
        }
    }

    #[test]
    fn map_inc_counts_and_sets_deduplicate() {
        let t = run(r#"
            fn trace(input) {
                enter("f");
                let c = map_new();
                map_inc(c, "a", 1);
                map_inc(c, "a", 1);
                let s = set_new();
                set_add(s, 4); set_add(s, 4); set_add(s, 5); set_del(s, 4);
                step("a", `${map_get(c, "a")} ${set_len(s)} ${set_has(s, 5)}`, []);
            }
        "#);
        assert_eq!(t.steps[0].note, "2 1 true");
    }

    #[test]
    fn declared_tags_are_scanned_statically() {
        let tags = ScriptHost::declared_tags(
            r#"step("init", "x", []); step( "loop" , "y", []); step("init", "z", []);"#,
        );
        assert_eq!(tags, vec!["init".to_string(), "loop".to_string()]);
    }
}
