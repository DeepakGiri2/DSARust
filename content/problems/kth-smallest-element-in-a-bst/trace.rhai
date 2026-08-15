// Kth Smallest in a BST — iterative in-order traversal.
//
// In-order visits a BST in sorted order, so the kth node popped is the answer.
// The explicit stack replaces recursion and, more usefully, lets the walk stop
// the instant k reaches zero instead of traversing the whole tree.

fn labels_of(nodes, ids) {
    let out = [];
    for id in ids { out.push(`${nodes[id].val}`); }
    out
}

fn trace(input) {
    let nodes = tree_nodes(input.root);
    let k = input.k;
    let stack = [];
    let visited = [];
    let cur = if nodes.len() > 0 { 0 } else { () };

    enter("kthSmallest", #{ k: N(k) });
    set(#{ stack: A([]) });
    step("init", "The stack remembers the ancestors we still owe a visit to.",
        [ tree("tree", nodes), stack("stack", []) ]);
    set(#{ cur: PTR(if type_of(cur) == "()" { "nil" } else { `node(${nodes[cur].val})` }) });
    step("setcur", "Start at the root.",
        [ tree("tree", nodes).cur(cur), stack("stack", labels_of(nodes, stack)) ]);

    while type_of(cur) != "()" || stack.len() > 0 {
        step("loop", "There is still something to visit.",
            [ tree("tree", nodes).cur(cur).done(visited),
              stack("stack", labels_of(nodes, stack)) ]);

        while type_of(cur) != "()" {
            step("dive", `Smaller values live to the left of ${nodes[cur].val}, so go there first.`,
                [ tree("tree", nodes).cur(cur).done(visited),
                  stack("stack", labels_of(nodes, stack)) ]);
            stack.push(cur);
            set(#{ stack: A(labels_of(nodes, stack)) });
            step("push", `Remember node(${nodes[cur].val}) for later.`,
                [ tree("tree", nodes).cur(cur).done(visited),
                  stack("stack", labels_of(nodes, stack)).pushed(true) ]);
            cur = nodes[cur].left;
            step("left", `Move to ${if type_of(cur) == "()" { "nil" } else { `node(${nodes[cur].val})` }}.`,
                [ tree("tree", nodes).cur(cur).done(visited),
                  stack("stack", labels_of(nodes, stack)) ]);
        }

        cur = stack[stack.len() - 1];
        stack.truncate(stack.len() - 1);
        visited.push(cur);
        set(#{ stack: A(labels_of(nodes, stack)) });
        step("pop", `No smaller value left — visit node(${nodes[cur].val}). This is the ${visited.len()}th smallest.`,
            [ tree("tree", nodes).cur(cur).done(visited),
              stack("stack", labels_of(nodes, stack)).popped(`${nodes[cur].val}`) ]);

        k -= 1;
        set(#{ k: N(k) });
        step("dec", `${k} to go.`,
            [ tree("tree", nodes).cur(cur).done(visited),
              stack("stack", labels_of(nodes, stack)) ]);

        if k == 0 {
            log(`answer ${nodes[cur].val}`, "result");
            step("check", "k has reached zero.", [ tree("tree", nodes).cur(cur).done(visited) ]);
            result(`${nodes[cur].val}`);
            step("found", `node(${nodes[cur].val}) is the answer — the rest of the tree is never touched.`,
                [ tree("tree", nodes).done(visited).cur(cur) ], "return");
            leave(`${nodes[cur].val}`);
            return;
        }

        cur = nodes[cur].right;
        step("right", `Continue into the right subtree.`,
            [ tree("tree", nodes).cur(cur).done(visited),
              stack("stack", labels_of(nodes, stack)) ]);
    }

    log("k is larger than the tree", "result");
    result("-1");
    step("notfound", "The traversal finished before reaching k.",
        [ tree("tree", nodes).done(visited) ], "return");
    leave("-1");
}
