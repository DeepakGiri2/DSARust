// Binary Tree Maximum Path Sum — the return value and the answer differ.
//
// A path may bend at one node, using both of its children. But a *parent* can
// only use one side, so the recursion returns the best straight branch while
// separately recording the best bent path it has seen. Negative branches are
// clamped to 0: taking nothing beats taking a loss.

fn dfs(nodes, id, state) {
    if type_of(id) == "()" {
        enter("gain(nil)");
        step("base", "Nothing here.", [ tree("tree", nodes).done(state.done) ]);
        step("ret0", "An empty branch contributes 0.",
            [ tree("tree", nodes).done(state.done) ], "return");
        leave("0");
        return [0, state];
    }

    let val = nodes[id].val;
    enter(`gain(${val})`, #{ node: PTR(`node(${val})`) });
    step("base", `At node(${val}).`, [ tree("tree", nodes).cur(id).done(state.done) ]);

    step("recL", `Best downward branch on the left of ${val}?`,
        [ tree("tree", nodes).cur(id).done(state.done) ], "call");
    let l = dfs(nodes, nodes[id].left, state);
    state = l[1];
    let left = if l[0] > 0 { l[0] } else { 0 };

    step("recR", `Left gives ${l[0]}${if l[0] < 0 { ", clamped to 0 — a negative branch is worse than nothing" } else { `` }}. Now the right.`,
        [ tree("tree", nodes).cur(id).done(state.done) ], "call");
    let r = dfs(nodes, nodes[id].right, state);
    state = r[1];
    let right = if r[0] > 0 { r[0] } else { 0 };

    let bent = val + left + right;
    if bent > state.best {
        state.best = bent;
        log(`new best path ${bent} bending at ${val}`, "result");
    }
    set(#{ best: N(state.best) });
    step("best",
        `A path bending at ${val} scores ${val} + ${left} + ${right} = ${bent}. Best anywhere so far: ${state.best}.`,
        [ tree("tree", nodes).cur(id).done(state.done) ]);

    let straight = val + if left > right { left } else { right };
    state.done.push(id);
    step("ret",
        `Return ${straight} — a parent can only continue through one side, so only the better branch is offered upward.`,
        [ tree("tree", nodes).cur(id).done(state.done) ], "return");
    leave(`${straight}`);
    [straight, state]
}

fn trace(input) {
    let nodes = tree_nodes(input.root);
    let state = #{ best: -1000000, done: [] };

    enter("maxPathSum", #{});
    set(#{ best: S("-inf") });
    step("init", "No path measured yet.", [ tree("tree", nodes) ]);
    step("call", "Walk the tree once, bottom up.", [ tree("tree", nodes) ], "call");

    if nodes.len() > 0 {
        let out = dfs(nodes, 0, state);
        state = out[1];
    }

    log(`answer ${state.best}`, "result");
    result(`${state.best}`);
    step("result", `The highest-scoring path in the tree sums to ${state.best}.`,
        [ tree("tree", nodes).done(state.done) ], "return");
    leave(`${state.best}`);
}
