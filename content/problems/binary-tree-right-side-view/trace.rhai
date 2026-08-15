// Binary Tree Right Side View — the last node of every BFS level.
//
// Standing to the right, the node you see on each level is the last one the
// queue hands out for that level. The level boundary comes from snapshotting
// the queue length, exactly as in level-order traversal.

fn labels_of(nodes, ids) {
    let out = [];
    for id in ids { out.push(`${nodes[id].val}`); }
    out
}

fn trace(input) {
    let nodes = tree_nodes(input.root);
    let res = [];
    let seen = [];

    enter("rightSideView", #{});
    step("init", "One visible node per level.", [ tree("tree", nodes) ]);

    if nodes.len() == 0 {
        step("empty", "The tree is empty.", [ tree("tree", nodes) ]);
        result("[]");
        step("retempty", "Nothing to see.", [ tree("tree", nodes) ], "return");
        leave("[]");
        return;
    }
    step("empty", "The tree has a root, so at least one node is visible.",
        [ tree("tree", nodes).cur(0) ]);

    let queue = [0];
    set(#{ queue: A(labels_of(nodes, queue)) });
    step("enq", "Seed the queue with the root.",
        [ tree("tree", nodes).cur(0), queue("queue", labels_of(nodes, queue)).pushed(true) ]);

    while queue.len() > 0 {
        let size = queue.len();
        step("while", `${size} node(s) on this level.`,
            [ tree("tree", nodes).path(queue).done(seen),
              queue("queue", labels_of(nodes, queue)) ]);
        step("size", `Snapshot the level width: ${size}. The last of these ${size} is the one visible from the right.`,
            [ tree("tree", nodes).path(queue).done(seen),
              queue("queue", labels_of(nodes, queue)) ]);

        for i in 0..size {
            step("front", `Node ${i + 1} of ${size}.`,
                [ tree("tree", nodes).path(queue).done(seen),
                  queue("queue", labels_of(nodes, queue)) ]);

            let id = queue[0];
            queue.remove(0);
            step("pop", `Take node(${nodes[id].val}).`,
                [ tree("tree", nodes).cur(id).done(seen),
                  queue("queue", labels_of(nodes, queue)).popped(`${nodes[id].val}`) ]);

            if i == size - 1 {
                res.push(nodes[id].val);
                seen.push(id);
                set(#{ res: A(res) });
                log(`level ${res.len()} shows ${nodes[id].val}`, "result");
                step("islast", `It is the last on its level, so it is what you see from the right.`,
                    [ tree("tree", nodes).cur(id).done(seen) ]);
                step("take", `Record ${nodes[id].val}.`,
                    [ tree("tree", nodes).done(seen), array("visible", res).hl([res.len() - 1]) ]);
            } else {
                step("islast", `Not the last on its level — hidden behind a node further right.`,
                    [ tree("tree", nodes).cur(id).done(seen) ]);
            }

            if type_of(nodes[id].left) != "()" {
                queue.push(nodes[id].left);
                step("left", `Queue its left child ${nodes[nodes[id].left].val}.`,
                    [ tree("tree", nodes).cur(nodes[id].left).done(seen),
                      queue("queue", labels_of(nodes, queue)).pushed(true) ]);
            } else {
                step("left", "No left child.",
                    [ tree("tree", nodes).cur(id).done(seen),
                      queue("queue", labels_of(nodes, queue)) ]);
            }
            if type_of(nodes[id].right) != "()" {
                queue.push(nodes[id].right);
                step("right", `Queue its right child ${nodes[nodes[id].right].val}.`,
                    [ tree("tree", nodes).cur(nodes[id].right).done(seen),
                      queue("queue", labels_of(nodes, queue)).pushed(true) ]);
            } else {
                step("right", "No right child.",
                    [ tree("tree", nodes).cur(id).done(seen),
                      queue("queue", labels_of(nodes, queue)) ]);
            }
        }
    }

    log(`answer ${brackets(res)}`, "result");
    result(brackets(res));
    step("ret", `From the right you see ${brackets(res)} — one node per level.`,
        [ tree("tree", nodes).done(seen), array("visible", res) ], "return");
    leave(brackets(res));
}
