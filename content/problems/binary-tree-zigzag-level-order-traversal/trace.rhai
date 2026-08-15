// Zigzag Level Order — reverse the writing, not the queue.
//
// The queue must keep taking children left to right, because that is what
// makes the *next* level come out in order. So the alternation is applied
// where the values land in the row, not to the traversal itself. Watch the
// queue: it never changes direction.

fn labels_of(nodes, ids) {
    let out = [];
    for id in ids { out.push(`${nodes[id].val}`); }
    out
}

fn shown(level) {
    let out = [];
    for v in level { out.push(if type_of(v) == "()" { "·" } else { `${v}` }); }
    out
}

fn trace(input) {
    let nodes = tree_nodes(input.tree);

    enter("zigzagLevelOrder", #{});

    if nodes.len() == 0 {
        step("empty", "The tree is empty.", [ tree("tree", nodes) ]);
        result("");
        step("noroot", "Return nothing.", [ tree("tree", nodes) ], "return");
        leave("[]");
        return;
    }
    step("empty", "There is a root, so there is at least one level.", [ tree("tree", nodes).cur(0) ]);

    let rows = [];
    step("init", "The answer is one row per level.", [ tree("tree", nodes) ]);

    let queue = [0];
    let done = [];
    step("seed", "Seed the queue with the root.",
        [ tree("tree", nodes).cur(0), queue("queue", labels_of(nodes, queue)).pushed(true) ]);

    let ltr = true;
    set(#{ leftToRight: B(ltr) });
    step("flag", "The first level reads left to right; the flag flips after each one.",
        [ tree("tree", nodes).cur(0) ]);

    while queue.len() > 0 {
        step("while", `${plural(queue.len(), "node")} waiting.`,
            [ tree("tree", nodes).done(done).path(queue),
              queue("queue", labels_of(nodes, queue)) ]);

        let size = queue.len();
        set(#{ size: N(size) });
        step("size", `Snapshot the queue length: ${size}. Exactly these nodes form this level.`,
            [ queue("queue", labels_of(nodes, queue)) ]);

        let level = [];
        for i in 0..size { level.push(()); }
        step("level", `A row of ${size} empty slot(s), to be filled ${if ltr { "left to right" } else { "right to left" }}.`,
            [ array("level being built", shown(level)) ]);

        for i in 0..size {
            let id = queue[0];
            queue.remove(0);
            step("pop", `Take ${nodes[id].val} off the front (the ${i + 1} of ${size} on this level).`,
                [ tree("tree", nodes).cur(id).done(done),
                  queue("queue", labels_of(nodes, queue)).popped(true) ]);

            let pos = i;
            step("pos", `By arrival order this is position ${i}.`,
                [ array("level being built", shown(level)).ptr("i", i) ]);

            if !ltr {
                pos = size - 1 - i;
                step("flip", `But this level runs right to left, so it goes at ${size} − 1 − ${i} = ${pos} instead.`,
                    [ array("level being built", shown(level)).ptr("pos", pos) ]);
            } else {
                step("flip", "This level runs left to right, so the position stands.",
                    [ array("level being built", shown(level)).ptr("pos", pos) ]);
            }

            level[pos] = nodes[id].val;
            step("place", `level[${pos}] = ${nodes[id].val}.`,
                [ array("level being built", shown(level)).hl([pos]) ]);

            if type_of(nodes[id].left) != "()" {
                queue.push(nodes[id].left);
                step("left", `Enqueue the left child ${nodes[nodes[id].left].val} — always left first, whatever direction this level reads in.`,
                    [ tree("tree", nodes).cur(id).path(queue),
                      queue("queue", labels_of(nodes, queue)).pushed(true) ]);
            } else {
                step("left", "No left child.", [ tree("tree", nodes).cur(id) ]);
            }

            if type_of(nodes[id].right) != "()" {
                queue.push(nodes[id].right);
                step("right", `Enqueue the right child ${nodes[nodes[id].right].val}.`,
                    [ tree("tree", nodes).cur(id).path(queue),
                      queue("queue", labels_of(nodes, queue)).pushed(true) ]);
            } else {
                step("right", "No right child.", [ tree("tree", nodes).cur(id) ]);
            }

            done.push(id);
        }

        rows.push(join(level, " "));
        step("row", `Level complete: ${brackets(level)}.`,
            [ tree("tree", nodes).done(done), text("rows so far", rows) ]);

        ltr = !ltr;
        set(#{ leftToRight: B(ltr) });
        step("toggle", `Flip the direction: the next level reads ${if ltr { "left to right" } else { "right to left" }}.`,
            [ text("rows so far", rows) ]);
    }

    let out = join(rows, " | ");
    log(out, "result");
    result(out);
    step("ret", `${plural(rows.len(), "level")}, zigzagged.`,
        [ tree("tree", nodes).done(done), text("rows so far", rows) ], "return");
    leave(out);
}
