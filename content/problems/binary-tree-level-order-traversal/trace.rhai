// Binary Tree Level Order Traversal — BFS with a level boundary.
//
// The trick is the single line `size := len(queue)`: taking a snapshot of the
// queue length before draining it is what separates one level from the next.
// Watch the queue hold exactly one level at the top of each round.

fn labels_of(nodes, ids) {
    let out = [];
    for id in ids { out.push(`${nodes[id].val}`); }
    out
}

fn trace(input) {
    let nodes = tree_nodes(input.tree);
    let res = [];

    enter("levelOrder", #{});
    step("init", "The result will be one row per level.", [ tree("tree", nodes) ]);

    if nodes.len() == 0 {
        step("empty", "The tree is empty.", [ tree("tree", nodes) ]);
        log("empty tree", "result");
        result("[]");
        step("retempty", "Return an empty list.", [ tree("tree", nodes) ], "return");
        leave("[]");
        return;
    }
    step("empty", "The tree has a root, so there is at least one level.",
        [ tree("tree", nodes).cur(0) ]);

    let queue = [0];
    let done = [];
    set(#{ queue: A(labels_of(nodes, queue)) });
    step("enq", "Seed the queue with the root.",
        [ tree("tree", nodes).cur(0),
          queue("queue", labels_of(nodes, queue)).pushed(true) ]);

    while queue.len() > 0 {
        step("while", `${queue.len()} node(s) waiting.`,
            [ tree("tree", nodes).done(done).path(queue),
              queue("queue", labels_of(nodes, queue)) ]);

        let size = queue.len();
        set(#{ size: N(size) });
        step("size",
            `Snapshot the queue length: ${size}. Exactly these nodes form the current level — anything enqueued from now on belongs to the next one.`,
            [ tree("tree", nodes).done(done).path(queue),
              queue("queue", labels_of(nodes, queue)) ]);

        let level = [];
        set(#{ level: A(level) });
        step("newlevel", "Start an empty row for this level.",
            [ tree("tree", nodes).done(done).path(queue),
              queue("queue", labels_of(nodes, queue)),
              array("level", level) ]);

        for i in 0..size {
            step("front", `Node ${i + 1} of ${size} on this level.`,
                [ tree("tree", nodes).done(done).path(queue),
                  queue("queue", labels_of(nodes, queue)),
                  array("level", level) ]);

            let id = queue[0];
            queue.remove(0);
            step("pop", `Take node(${nodes[id].val}) off the front.`,
                [ tree("tree", nodes).cur(id).done(done).path(queue),
                  queue("queue", labels_of(nodes, queue)).popped(`${nodes[id].val}`),
                  array("level", level) ]);

            level.push(nodes[id].val);
            done.push(id);
            set(#{ level: A(level) });
            log(`visit ${nodes[id].val}`);
            step("visit", `Record ${nodes[id].val} in this level's row.`,
                [ tree("tree", nodes).cur(id).done(done).path(queue),
                  queue("queue", labels_of(nodes, queue)),
                  array("level", level).hl([level.len() - 1]) ]);

            if type_of(nodes[id].left) != "()" {
                queue.push(nodes[id].left);
                step("left", `Its left child ${nodes[nodes[id].left].val} joins the queue for the next level.`,
                    [ tree("tree", nodes).cur(nodes[id].left).done(done).path(queue),
                      queue("queue", labels_of(nodes, queue)).pushed(true),
                      array("level", level) ]);
            } else {
                step("left", `node(${nodes[id].val}) has no left child.`,
                    [ tree("tree", nodes).cur(id).done(done).path(queue),
                      queue("queue", labels_of(nodes, queue)), array("level", level) ]);
            }

            if type_of(nodes[id].right) != "()" {
                queue.push(nodes[id].right);
                step("right", `Its right child ${nodes[nodes[id].right].val} joins too.`,
                    [ tree("tree", nodes).cur(nodes[id].right).done(done).path(queue),
                      queue("queue", labels_of(nodes, queue)).pushed(true),
                      array("level", level) ]);
            } else {
                step("right", `node(${nodes[id].val}) has no right child.`,
                    [ tree("tree", nodes).cur(id).done(done).path(queue),
                      queue("queue", labels_of(nodes, queue)), array("level", level) ]);
            }
        }

        res.push(brackets(level));
        log(`level ${res.len()}: ${brackets(level)}`, "result");
        step("pushlevel", `Level ${res.len()} is complete: ${brackets(level)}.`,
            [ tree("tree", nodes).done(done).path(queue),
              queue("queue", labels_of(nodes, queue)),
              text("levels so far", res) ]);
    }

    result(`[${join(res, ", ")}]`);
    step("ret", `The queue is empty, so every level has been read: [${join(res, ", ")}].`,
        [ tree("tree", nodes).done(done), text("levels", res) ], "return");
    leave(`[${join(res, ", ")}]`);
}
