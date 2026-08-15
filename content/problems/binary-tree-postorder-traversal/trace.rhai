fn vals_of(nodes, ids) {
    let out = [];
    for id in ids { out.push(nodes[id].val); }
    out
}

// Postorder Traversal — a mirrored preorder, reversed.
//
// Postorder is left-right-node. Reverse that and you get node-right-left,
// which is just preorder with the children swapped — and preorder is the easy
// one, because a node is recorded on arrival and never revisited. So: run the
// mirrored preorder, then reverse. No visited flags, no second stack.

fn trace(input) {
    let nodes = tree_nodes(input.root);
    let out = [];
    let stack = [];
    let done = [];

    enter("postorderTraversal", #{});
    step("init", "Collect node-right-left, then reverse it — that is left-right-node.",
        [ tree("tree", nodes), array("collected (node, right, left)", out) ]);
    step("stack", "One stack of subtrees not yet started.",
        [ tree("tree", nodes), stack("stack", vals_of(nodes, stack)) ]);

    if nodes.len() > 0 { stack.push(0); }
    step("seed", "Seed with the root.",
        [ tree("tree", nodes), stack("stack", vals_of(nodes, stack)).pushed(true) ]);

    while stack.len() > 0 {
        step("loop", `${stack.len()} subtree(s) waiting.`,
            [ tree("tree", nodes).done(done), stack("stack", vals_of(nodes, stack)) ]);

        let id = stack[stack.len() - 1];
        stack.truncate(stack.len() - 1);
        step("pop", `Take ${nodes[id].val}.`,
            [ tree("tree", nodes).cur(id).done(done),
              stack("stack", vals_of(nodes, stack)).popped(nodes[id].val) ]);

        out.push(nodes[id].val);
        done.push(id);
        set(#{ collected: A(out) });
        step("visit", `Record ${nodes[id].val} now; the reversal at the end will push it back to where postorder wants it.`,
            [ tree("tree", nodes).cur(id).done(done),
              array("collected (node, right, left)", out) ]);

        if type_of(nodes[id].left) != "()" {
            step("hasL", `Left child ${nodes[nodes[id].left].val}.`,
                [ tree("tree", nodes).cur(id).done(done),
                  stack("stack", vals_of(nodes, stack)) ]);
            stack.push(nodes[id].left);
            step("pushL", "Left goes in first — the mirror of ordinary preorder.",
                [ tree("tree", nodes).cur(id).done(done),
                  stack("stack", vals_of(nodes, stack)).pushed(true) ]);
        } else {
            step("hasL", `${nodes[id].val} has no left child.`,
                [ tree("tree", nodes).cur(id).done(done),
                  stack("stack", vals_of(nodes, stack)) ]);
        }

        if type_of(nodes[id].right) != "()" {
            step("hasR", `Right child ${nodes[nodes[id].right].val}.`,
                [ tree("tree", nodes).cur(id).done(done),
                  stack("stack", vals_of(nodes, stack)) ]);
            stack.push(nodes[id].right);
            step("pushR", "Right goes in second, so it pops before the left — node, right, left.",
                [ tree("tree", nodes).cur(id).done(done),
                  stack("stack", vals_of(nodes, stack)).pushed(true) ]);
        } else {
            step("hasR", `${nodes[id].val} has no right child.`,
                [ tree("tree", nodes).cur(id).done(done),
                  stack("stack", vals_of(nodes, stack)) ]);
        }
    }

    let collected = join(out, " ");
    let rev = [];
    let k = out.len() - 1;
    while k >= 0 { rev.push(out[k]); k -= 1; }
    let answer = join(rev, " ");
    log(`node-right-left was ${collected}`);
    step("reverse", `Collected node-right-left as ${collected}. Reverse it.`,
        [ array("collected (node, right, left)", out), array("output", rev) ]);

    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `Reversing node-right-left gives left-right-node: ${answer}.`,
        [ tree("tree", nodes).done(done), array("output", rev) ], "return");
    leave(answer);
}
