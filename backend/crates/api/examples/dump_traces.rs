//! Record traces with the real Rhai engine and write them as JSON fixtures.
//!
//! The web renderer is a port of `dsa-viz`; the fastest way to be sure it draws
//! what the desktop draws is to feed it exactly what the engine produces.
//!
//! ```text
//! cargo run -p dsa-api --example dump_traces -- <out-dir> [slug ...]
//! cargo run -p dsa-api --example dump_traces -- <out-dir> --kinds   # one per view kind
//! ```

use dsa_content::Library;
use dsa_core::VizView;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(
        args.next()
            .expect("usage: dump_traces <out-dir> [slug ...|--kinds]"),
    );
    let rest: Vec<String> = args.collect();

    let root = std::env::var("DSA_CONTENT_DIR").unwrap_or_else(|_| "../content".into());
    let lib = Library::load(&root);
    anyhow::ensure!(lib.errors.is_empty(), "content errors: {:?}", lib.errors);
    std::fs::create_dir_all(&out)?;

    let slugs: Vec<String> = if rest.iter().any(|a| a == "--kinds") {
        coverage(&lib)
    } else {
        rest
    };

    for slug in &slugs {
        let pack = lib
            .pack(slug)
            .ok_or_else(|| anyhow::anyhow!("no pack {slug}"))?;
        let trace = lib
            .trace(slug, &pack.meta.default_input)
            .map_err(|e| anyhow::anyhow!("{slug}: {e}"))?;
        let path = out.join(format!("{slug}.json"));
        std::fs::write(&path, serde_json::to_string(&trace)?)?;
        println!(
            "{slug:<48} {:>4} steps  {}",
            trace.len(),
            kinds(&trace).join(",")
        );
    }
    Ok(())
}

fn kind(v: &VizView) -> &'static str {
    match v {
        VizView::Array(a) if a.bars => "bars",
        VizView::Array(_) => "array",
        VizView::Kv(_) => "kv",
        VizView::Stack(s) => match s.kind {
            dsa_core::StackKind::Stack => "stack",
            dsa_core::StackKind::Queue => "queue",
            dsa_core::StackKind::Deque => "deque",
            dsa_core::StackKind::Heap => "heap",
        },
        VizView::List(_) => "list",
        VizView::Tree(_) => "tree",
        VizView::Grid(_) => "grid",
        VizView::Graph(_) => "graph",
        VizView::Bits(_) => "bits",
        VizView::Text(_) => "text",
    }
}

fn kinds(trace: &dsa_core::Trace) -> Vec<&'static str> {
    let mut k: Vec<&'static str> = trace
        .steps
        .iter()
        .flat_map(|s| s.views.iter().map(kind))
        .collect();
    k.sort_unstable();
    k.dedup();
    k
}

/// The shortest trace that shows each view kind, plus the decorations that
/// animate specially (tree swaps, reversed list arrows, weighted graph edges).
fn coverage(lib: &Library) -> Vec<String> {
    let mut best: BTreeMap<&'static str, (usize, String)> = BTreeMap::new();
    for (slug, pack) in lib.packs() {
        let Ok(trace) = lib.trace(slug, &pack.meta.default_input) else {
            continue;
        };
        let mut feats: Vec<&'static str> = kinds(&trace);
        for s in &trace.steps {
            for v in &s.views {
                match v {
                    VizView::Tree(t) if t.swap.is_some() => feats.push("tree-swap"),
                    VizView::List(l) if !l.reversed.is_empty() => feats.push("list-reversed"),
                    VizView::Graph(g) if g.edges.iter().any(|e| e.weight.is_some()) => {
                        feats.push("graph-weighted")
                    }
                    VizView::Graph(g) if !g.active.is_empty() => feats.push("graph-active"),
                    VizView::Array(a) if a.window.is_some() => feats.push("array-window"),
                    VizView::Stack(s) if s.popped.is_some() => feats.push("stack-popped"),
                    _ => {}
                }
            }
        }
        feats.sort_unstable();
        feats.dedup();
        for f in feats {
            let entry = best.entry(f).or_insert((usize::MAX, String::new()));
            if trace.len() < entry.0 {
                *entry = (trace.len(), slug.clone());
            }
        }
    }
    let mut slugs: Vec<String> = best.into_values().map(|(_, s)| s).collect();
    // Always include the showcase problems the desktop screenshots use.
    for s in ["two-sum", "invert-binary-tree", "course-schedule"] {
        slugs.push(s.to_string());
    }
    slugs.sort();
    slugs.dedup();
    slugs
}
