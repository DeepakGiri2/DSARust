// Shared helpers, prepended to every problem script.
//
// This file is content, not code: edit it, save, and every animation in the
// app picks up the change on the next trace. Keep helpers here small and about
// *presentation* — the algorithm itself belongs in each problem's trace.rhai,
// where a reader can follow it.

// ── index sets for array decorations ────────────────────────────────────────

/// Indices of `n` cells that fall outside the live range [lo, hi].
/// Binary search dims them so the halving is visible at a glance.
fn dead(n, lo, hi) {
    let out = [];
    for i in 0..n {
        if i < lo || i > hi { out.push(i); }
    }
    out
}

/// Indices strictly before `i` — "already processed".
fn before(i) {
    let out = [];
    for k in 0..i { out.push(k); }
    out
}

/// Indices from `i` to the end of an `n`-element array.
fn after(i, n) {
    let out = [];
    for k in (i + 1)..n { out.push(k); }
    out
}

/// Inclusive index band, clamped to nothing when the range is empty.
fn span(lo, hi) {
    let out = [];
    let k = lo;
    while k <= hi { out.push(k); k += 1; }
    out
}

// ── formatting ──────────────────────────────────────────────────────────────

/// "1 island" / "2 islands"
fn plural(n, word) {
    if n == 1 { `${n} ${word}` } else { `${n} ${word}s` }
}

/// Renders a value the way the explanation text should read it.
fn show(v) {
    if type_of(v) == "()" { "nil" } else { `${v}` }
}

/// "[1, 2, 3]"
fn brackets(arr) {
    `[${join(arr, ", ")}]`
}

// ── structure helpers ───────────────────────────────────────────────────────

/// Ids of every node on the path from the root to `id`, using the parent map
/// built by `parents_of`.
fn path_to(par, id) {
    let out = [];
    let cur = id;
    while type_of(cur) != "()" {
        out.push(cur);
        cur = map_get(par, cur);
    }
    out
}

/// child id -> parent id for a level-order tree spec.
fn parents_of(nodes) {
    let par = map_new();
    for n in nodes {
        if type_of(n.left) != "()" { map_set(par, n.left, n.id); }
        if type_of(n.right) != "()" { map_set(par, n.right, n.id); }
    }
    par
}

/// Node ids in in-order, which is the order a BST animation should visit them.
fn inorder_ids(nodes, root) {
    let out = [];
    let stack = [];
    let cur = root;
    while type_of(cur) != "()" || stack.len() > 0 {
        while type_of(cur) != "()" {
            stack.push(cur);
            cur = nodes[cur].left;
        }
        cur = stack.pop();
        out.push(cur);
        cur = nodes[cur].right;
    }
    out
}
