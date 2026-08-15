// BST Iterator — a paused inorder traversal.
//
// The recursive inorder is three lines and cannot be paused. Turning it into a
// stack machine makes the pause point explicit: the stack holds exactly the
// ancestors whose own value has not been emitted yet, so the top is always the
// next smallest. That is the whole invariant, and next() restores it in one
// move — pop, then push the left spine of the right subtree.
//
// It is O(h) memory, not O(n): the follow-up an interviewer asks about.

fn labels_of(nodes, ids) {
    let out = [];
    for id in ids { out.push(`${nodes[id].val}`); }
    out
}

fn parse_ops(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if !w.is_empty() { out.push(w[0]); }
    }
    out
}

const ST = "stack — ancestors not yet returned";

fn push_left(nodes, stack, start, visited) {
    let node = start;
    enter(`pushLeft(${if type_of(node) == "()" { "nil" } else { `${nodes[node].val}` }})`);
    step("push", "Walk down the left spine, pushing everything on the way.",
        [ tree("tree", nodes).cur(node).done(visited),
          stack(global::ST, labels_of(nodes, stack)) ]);

    while type_of(node) != "()" {
        step("descend", `${nodes[node].val} is not nil, so it goes on the stack.`,
            [ tree("tree", nodes).cur(node).done(visited),
              stack(global::ST, labels_of(nodes, stack)) ]);
        stack.push(node);
        step("stack", `Push ${nodes[node].val}. Everything smaller than it lies to its left, so it cannot be emitted yet.`,
            [ tree("tree", nodes).cur(node).done(visited),
              stack(global::ST, labels_of(nodes, stack)).pushed(true) ]);
        node = nodes[node].left;
        step("goleft", `Move left to ${if type_of(node) == "()" { "nil" } else { `${nodes[node].val}` }}.`,
            [ tree("tree", nodes).cur(node).done(visited),
              stack(global::ST, labels_of(nodes, stack)) ]);
    }

    step("descend", "nil — the smallest unvisited value is now on top of the stack.",
        [ stack(global::ST, labels_of(nodes, stack)) ], "return");
    leave("");
    stack
}

fn trace(input) {
    let nodes = tree_nodes(input.tree);
    let ops = parse_ops(input.ops);

    enter("BSTIterator", #{});
    let stack = [];
    let visited = [];
    step("init", "The iterator stores a stack, never a flattened list — that is the point of the O(h) requirement.",
        [ tree("tree", nodes), stack(global::ST, labels_of(nodes, stack)) ]);

    step("seed", "Seed it with the left spine from the root.",
        [ tree("tree", nodes).cur(0) ], "call");
    stack = push_left(nodes, stack, if nodes.len() > 0 { 0 } else { () }, visited);
    leave("ready");

    let out = [];
    for op in ops {
        if op == "hasNext" {
            enter("hasNext");
            let any = stack.len() > 0;
            out.push(if any { "true" } else { "false" });
            step("hasnext", "hasNext().", [ stack(global::ST, labels_of(nodes, stack)) ]);
            step("any", `The stack ${if any { `holds ${plural(stack.len(), "node")}` } else { "is empty" }}, so hasNext() is ${any}.`,
                [ stack(global::ST, labels_of(nodes, stack)), tree("tree", nodes).done(visited) ],
                "return");
            leave(out[out.len() - 1]);
            continue;
        }

        enter("next");
        step("next", "next().",
            [ tree("tree", nodes).done(visited), stack(global::ST, labels_of(nodes, stack)) ]);

        let id = stack[stack.len() - 1];
        step("top", `The top of the stack is ${nodes[id].val} — by the invariant, the smallest value not yet returned.`,
            [ tree("tree", nodes).cur(id).done(visited),
              stack(global::ST, labels_of(nodes, stack)).hl([stack.len() - 1]) ]);

        stack.pop();
        visited.push(id);
        step("pop", `Pop it.`,
            [ tree("tree", nodes).done(visited),
              stack(global::ST, labels_of(nodes, stack)).popped(true) ]);

        step("right", `Everything between ${nodes[id].val} and its parent lives in its right subtree, so push that subtree's left spine.`,
            [ tree("tree", nodes).cur(nodes[id].right).done(visited) ], "call");
        stack = push_left(nodes, stack, nodes[id].right, visited);

        out.push(`${nodes[id].val}`);
        log(`next() = ${nodes[id].val}`, "result");
        step("val", `Return ${nodes[id].val}.`,
            [ tree("tree", nodes).done(visited),
              stack(global::ST, labels_of(nodes, stack)) ], "return");
        leave(`${nodes[id].val}`);
    }

    let answer = join(out, " ");
    log(answer, "result");
    result(answer);
}
