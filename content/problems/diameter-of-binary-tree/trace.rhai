// Diameter of Binary Tree — one post-order pass that answers two questions.
//
// The recursion *returns* height, but along the way it *records* the best
// left+right it has seen. That is the trick: the longest path through a node
// never needs to be returned upward, because a path can bend at only one node.

fn height(nodes, id, state) {
    if type_of(id) == "()" {
        enter("height(nil)");
        step("base", "An empty subtree.", [ tree("tree", nodes).done(state.done) ]);
        step("ret0", "Height 0.", [ tree("tree", nodes).done(state.done) ], "return");
        leave("0");
        return [0, state];
    }

    let val = nodes[id].val;
    enter(`height(${val})`, #{ node: PTR(`node(${val})`) });
    step("base", `node(${val}) is real, so its height depends on its children.`,
        [ tree("tree", nodes).cur(id).done(state.done) ]);

    step("recL", `Measure the left subtree of ${val}.`,
        [ tree("tree", nodes).cur(id).done(state.done) ], "call");
    let l = height(nodes, nodes[id].left, state);
    state = l[1];

    step("recR", `Left height is ${l[0]}. Now the right subtree.`,
        [ tree("tree", nodes).cur(id).done(state.done) ], "call");
    let r = height(nodes, nodes[id].right, state);
    state = r[1];

    let through = l[0] + r[0];
    if through > state.best {
        state.best = through;
        state.at = val;
        log(`new best diameter ${through} bending at ${val}`, "result");
    }
    set(#{ best: N(state.best) });
    step("update",
        `A path bending at ${val} is ${l[0]} + ${r[0]} = ${through} edges long. Best so far: ${state.best}.`,
        [ tree("tree", nodes).cur(id).done(state.done) ]);

    let h = 1 + if l[0] > r[0] { l[0] } else { r[0] };
    state.done.push(id);
    step("ret", `Return the height, 1 + max(${l[0]}, ${r[0]}) = ${h}. The bend is recorded, not returned.`,
        [ tree("tree", nodes).cur(id).done(state.done) ], "return");
    leave(`${h}`);
    [h, state]
}

fn trace(input) {
    let nodes = tree_nodes(input.root);
    let state = #{ best: 0, at: (), done: [] };

    enter("diameterOfBinaryTree", #{});
    set(#{ best: N(0) });
    step("init", "The diameter is the longest path in edges; start at 0.",
        [ tree("tree", nodes) ]);

    if nodes.len() > 0 {
        let out = height(nodes, 0, state);
        state = out[1];
    }

    log(`answer ${state.best}`, "result");
    result(`${state.best}`);
    step("result", `The longest path anywhere in the tree is ${state.best} edge(s).`,
        [ tree("tree", nodes).done(state.done) ], "return");
    leave(`${state.best}`);
}
