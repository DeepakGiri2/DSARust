// Symmetric Tree — recurse on pairs, not on nodes.
//
// "Is this tree a mirror of itself" is not a question about any single node,
// so the recursion carries two positions at once and walks them in opposite
// directions: left-of-one against right-of-the-other. Three base rules do all
// the work — two nils agree, one nil never does, and unequal values fail on
// the spot.

fn name_of(nodes, id) {
    if type_of(id) == "()" { "nil" } else { `${nodes[id].val}` }
}

fn pair_ids(a, b) {
    let out = [];
    if type_of(a) != "()" { out.push(a); }
    if type_of(b) != "()" { out.push(b); }
    out
}

fn mirror(nodes, a, b) {
    enter(`mirror(${name_of(nodes, a)}, ${name_of(nodes, b)})`);

    if type_of(a) == "()" && type_of(b) == "()" {
        step("bothNil", "Both sides ran out at the same time, which is what symmetry looks like at the fringe.",
            [ tree("tree", nodes) ]);
        leave("true");
        return true;
    }

    if type_of(a) == "()" || type_of(b) == "()" {
        log(`${name_of(nodes, a)} vs ${name_of(nodes, b)}: shapes differ`, "result");
        step("oneNil", `One side is ${name_of(nodes, a)} and the other ${name_of(nodes, b)} — the shapes already disagree.`,
            [ tree("tree", nodes).path(pair_ids(a, b)) ]);
        leave("false");
        return false;
    }

    step("bothNil", `Comparing ${nodes[a].val} on the left against ${nodes[b].val} on the right.`,
        [ tree("tree", nodes).path(pair_ids(a, b)) ]);
    step("oneNil", "Both nodes exist, so the shapes agree here.",
        [ tree("tree", nodes).path(pair_ids(a, b)) ]);

    if nodes[a].val != nodes[b].val {
        log(`${nodes[a].val} != ${nodes[b].val}`, "result");
        step("valCmp", `${nodes[a].val} != ${nodes[b].val} — no mirror can survive that.`,
            [ tree("tree", nodes).path(pair_ids(a, b)) ]);
        leave("false");
        return false;
    }
    step("valCmp", `${nodes[a].val} == ${nodes[b].val}, so keep going.`,
        [ tree("tree", nodes).path(pair_ids(a, b)) ]);

    step("outer", `Now the outer pair: the left child of ${nodes[a].val} against the right child of ${nodes[b].val}.`,
        [ tree("tree", nodes).path(pair_ids(nodes[a].left, nodes[b].right)) ], "call");
    if !mirror(nodes, nodes[a].left, nodes[b].right) {
        step("failOuter", "The outer pair disagreed, so this pair fails too.",
            [ tree("tree", nodes).path(pair_ids(a, b)) ], "return");
        leave("false");
        return false;
    }

    step("inner", `Outer pair matched. Now the inner pair: right child of ${nodes[a].val} against left child of ${nodes[b].val}.`,
        [ tree("tree", nodes).path(pair_ids(nodes[a].right, nodes[b].left)) ], "call");
    let ok = mirror(nodes, nodes[a].right, nodes[b].left);
    leave(`${ok}`);
    ok
}

fn trace(input) {
    let nodes = tree_nodes(input.root);

    enter("isSymmetric", #{});
    step("empty",
        if nodes.len() == 0 {
            "An empty tree is trivially its own mirror.".to_string()
        } else {
            `The root is ${nodes[0].val}; symmetry is about what hangs either side of it.`
        },
        [ tree("tree", nodes).cur(if nodes.len() > 0 { 0 } else { () }) ]);

    let ok = true;
    if nodes.len() > 0 {
        step("call", "Compare the two subtrees of the root against each other, walking them in opposite directions.",
            [ tree("tree", nodes).path(pair_ids(nodes[0].left, nodes[0].right)) ], "call");
        ok = mirror(nodes, nodes[0].left, nodes[0].right);
    }

    log(`symmetric: ${ok}`, "result");
    result(`${ok}`);
    step("call",
        if ok {
            "Every mirrored pair agreed, all the way down — the tree is symmetric.".to_string()
        } else {
            "A mirrored pair disagreed, and one disagreement is enough."
        },
        [ tree("tree", nodes) ], "return");
    leave(`${ok}`);
}
