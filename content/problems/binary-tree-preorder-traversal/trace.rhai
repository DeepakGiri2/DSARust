fn vals_of(nodes, ids) {
    let out = [];
    for id in ids { out.push(nodes[id].val); }
    out
}

// Preorder Traversal — record on arrival, so nothing is ever revisited.
//
// Inorder has to stack ancestors because a node cannot be settled until its
// left side is done. Preorder has no such debt: the node is recorded the
// instant it is reached, so the stack only ever holds subtrees that have not
// been *started*. The one thing to get right is the push order — a stack
// returns what went in last, so the right child goes in first.

fn trace(input) {
    let nodes = tree_nodes(input.root);
    let out = [];
    let stack = [];
    let done = [];

    enter("preorderTraversal", #{});
    step("init", "Preorder: a node is recorded the moment it is reached.",
        [ tree("tree", nodes), array("output", out) ]);
    step("stack", "The stack holds subtrees that have not been started yet.",
        [ tree("tree", nodes), stack("stack", vals_of(nodes, stack)) ]);

    if nodes.len() > 0 { stack.push(0); }
    step("seed", "Seed it with the root — the one subtree that exists to begin with.",
        [ tree("tree", nodes), stack("stack", vals_of(nodes, stack)).pushed(true) ]);

    while stack.len() > 0 {
        step("loop", `${stack.len()} subtree(s) still waiting.`,
            [ tree("tree", nodes).done(done), stack("stack", vals_of(nodes, stack)) ]);

        let id = stack[stack.len() - 1];
        stack.truncate(stack.len() - 1);
        step("pop", `Take ${nodes[id].val} — the most recently pushed subtree.`,
            [ tree("tree", nodes).cur(id).done(done),
              stack("stack", vals_of(nodes, stack)).popped(nodes[id].val) ]);

        out.push(nodes[id].val);
        done.push(id);
        log(`visit ${nodes[id].val}`);
        set(#{ out: A(out) });
        step("visit", `Record ${nodes[id].val} straight away — preorder owes it nothing first.`,
            [ tree("tree", nodes).cur(id).done(done), array("output", out) ]);

        if type_of(nodes[id].right) != "()" {
            step("hasR", `${nodes[id].val} has a right child, ${nodes[nodes[id].right].val}.`,
                [ tree("tree", nodes).cur(id).done(done),
                  stack("stack", vals_of(nodes, stack)) ]);
            stack.push(nodes[id].right);
            step("pushR", "Push it *first*, so it comes back out last.",
                [ tree("tree", nodes).cur(id).done(done),
                  stack("stack", vals_of(nodes, stack)).pushed(true) ]);
        } else {
            step("hasR", `${nodes[id].val} has no right child.`,
                [ tree("tree", nodes).cur(id).done(done),
                  stack("stack", vals_of(nodes, stack)) ]);
        }

        if type_of(nodes[id].left) != "()" {
            step("hasL", `${nodes[id].val} has a left child, ${nodes[nodes[id].left].val}.`,
                [ tree("tree", nodes).cur(id).done(done),
                  stack("stack", vals_of(nodes, stack)) ]);
            stack.push(nodes[id].left);
            step("pushL", "Push it second, so it is the next thing popped — left before right.",
                [ tree("tree", nodes).cur(id).done(done),
                  stack("stack", vals_of(nodes, stack)).pushed(true) ]);
        } else {
            step("hasL", `${nodes[id].val} has no left child.`,
                [ tree("tree", nodes).cur(id).done(done),
                  stack("stack", vals_of(nodes, stack)) ]);
        }
    }

    let answer = join(out, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `The stack emptied, so every subtree was started and finished: ${answer}.`,
        [ tree("tree", nodes).done(done), array("output", out) ], "return");
    leave(answer);
}
