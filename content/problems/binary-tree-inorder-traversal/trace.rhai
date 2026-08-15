fn vals_of(nodes, ids) {
    let out = [];
    for id in ids { out.push(nodes[id].val); }
    out
}

// Inorder Traversal — the stack *is* the set of unfinished ancestors.
//
// Inorder says "everything left of me, then me, then everything right of me".
// Walking left as far as possible and stacking what you pass turns that rule
// into a mechanism: a node is only recorded when the walk can go no further
// left, which is precisely the moment its left subtree is exhausted.

fn trace(input) {
    let nodes = tree_nodes(input.root);
    let out = [];
    let stack = [];
    let done = [];

    enter("inorderTraversal", #{});
    step("init", "The values, in the order they are settled.",
        [ tree("tree", nodes), array("output", out) ]);
    step("stack", "The stack holds ancestors whose left side is still unfinished.",
        [ tree("tree", nodes), stack("stack", vals_of(nodes, stack)) ]);

    let cur = if nodes.len() > 0 { 0 } else { () };
    set(#{ cur: PTR(if nodes.len() > 0 { `node(${nodes[0].val})` } else { "nil" }) });
    step("cur", "Start at the root.",
        [ tree("tree", nodes).cur(cur), stack("stack", vals_of(nodes, stack)) ]);

    while type_of(cur) != "()" || stack.len() > 0 {
        step("loop",
            if type_of(cur) != "()" {
                `There is still a subtree to descend into.`.to_string()
            } else {
                `Nowhere left to descend, but ${stack.len()} ancestor(s) are still owed a visit.`
            },
            [ tree("tree", nodes).cur(cur).done(done),
              stack("stack", vals_of(nodes, stack)) ]);

        while type_of(cur) != "()" {
            step("dive", `node(${nodes[cur].val}) may have something to its left, so it cannot be recorded yet.`,
                [ tree("tree", nodes).cur(cur).done(done),
                  stack("stack", vals_of(nodes, stack)) ]);

            stack.push(cur);
            step("push", `Stack ${nodes[cur].val} and come back to it later.`,
                [ tree("tree", nodes).cur(cur).done(done),
                  stack("stack", vals_of(nodes, stack)).pushed(true) ]);

            cur = nodes[cur].left;
            set(#{ cur: PTR(if type_of(cur) != "()" { `node(${nodes[cur].val})` } else { "nil" }) });
            step("left",
                if type_of(cur) != "()" {
                    `Go left to ${nodes[cur].val}.`
                } else {
                    `No left child — the descent stops here.`.to_string()
                },
                [ tree("tree", nodes).cur(cur).done(done),
                  stack("stack", vals_of(nodes, stack)) ]);
        }

        cur = stack[stack.len() - 1];
        stack.truncate(stack.len() - 1);
        step("pop", `The deepest unfinished ancestor is ${nodes[cur].val}; its left side is now empty.`,
            [ tree("tree", nodes).cur(cur).done(done),
              stack("stack", vals_of(nodes, stack)).popped(nodes[cur].val) ]);

        out.push(nodes[cur].val);
        done.push(cur);
        log(`visit ${nodes[cur].val}`);
        set(#{ out: A(out) });
        step("visit", `Record ${nodes[cur].val} — position ${out.len()} in the traversal.`,
            [ tree("tree", nodes).cur(cur).done(done), array("output", out) ]);

        cur = nodes[cur].right;
        set(#{ cur: PTR(if type_of(cur) != "()" { `node(${nodes[cur].val})` } else { "nil" }) });
        step("right",
            if type_of(cur) != "()" {
                `Everything to its right is still owed; restart the descent at ${nodes[cur].val}.`
            } else {
                `No right child, so this node is completely finished.`.to_string()
            },
            [ tree("tree", nodes).cur(cur).done(done),
              stack("stack", vals_of(nodes, stack)) ]);
    }

    let answer = join(out, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `Every node was stacked once and popped once: ${answer}.`,
        [ tree("tree", nodes).done(done), array("output", out) ], "return");
    leave(answer);
}
