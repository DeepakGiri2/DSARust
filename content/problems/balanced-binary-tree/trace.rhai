// Balanced Binary Tree — height and the verdict in one return value.
//
// -1 is used as "already unbalanced". Because it propagates straight up, the
// recursion stops doing real work the moment an imbalance is found, and the
// whole tree is still visited only once.

fn dfs(nodes, id, done) {
    if type_of(id) == "()" {
        enter("dfs(nil)");
        step("base", "An empty subtree is balanced and has height 0.",
            [ tree("tree", nodes).done(done) ]);
        step("ret0", "Return 0.", [ tree("tree", nodes).done(done) ], "return");
        leave("0");
        return [0, done];
    }

    let val = nodes[id].val;
    enter(`dfs(${val})`, #{ node: PTR(`node(${val})`) });
    step("base", `node(${val}).`, [ tree("tree", nodes).cur(id).done(done) ]);

    step("recL", `Measure the left subtree of ${val}.`,
        [ tree("tree", nodes).cur(id).done(done) ], "call");
    let l = dfs(nodes, nodes[id].left, done);
    done = l[1];

    step("recR", `Left came back ${l[0]}. Now the right.`,
        [ tree("tree", nodes).cur(id).done(done) ], "call");
    let r = dfs(nodes, nodes[id].right, done);
    done = r[1];

    if l[0] == -1 || r[0] == -1 {
        step("propagate", "One of the subtrees already reported an imbalance, so this one is doomed too.",
            [ tree("tree", nodes).cur(id).done(done) ]);
        step("retProp", "Return -1 without any further checking.",
            [ tree("tree", nodes).cur(id) ], "return");
        leave("-1");
        return [-1, done];
    }

    let diff = if l[0] > r[0] { l[0] - r[0] } else { r[0] - l[0] };
    if diff > 1 {
        log(`node ${val}: heights ${l[0]} vs ${r[0]}`, "result");
        step("check", `node(${val}) has subtree heights ${l[0]} and ${r[0]} — a difference of ${diff}, more than 1.`,
            [ tree("tree", nodes).cur(id).done(done) ]);
        step("retBad", "Return -1 to mark the whole tree unbalanced.",
            [ tree("tree", nodes).cur(id) ], "return");
        leave("-1");
        return [-1, done];
    }
    step("check", `Heights ${l[0]} and ${r[0]} differ by ${diff}, which is fine.`,
        [ tree("tree", nodes).cur(id).done(done) ]);

    let h = 1 + if l[0] > r[0] { l[0] } else { r[0] };
    done.push(id);
    step("ret", `Return the real height ${h}.`, [ tree("tree", nodes).cur(id).done(done) ], "return");
    leave(`${h}`);
    [h, done]
}

fn trace(input) {
    let nodes = tree_nodes(input.root);

    enter("isBalanced", #{});
    step("top", "Ask for the height, using -1 as a flag that means \"already unbalanced\".",
        [ tree("tree", nodes) ], "call");

    let out = if nodes.len() > 0 { dfs(nodes, 0, []) } else { [0, []] };
    let ok = out[0] != -1;

    log(`balanced: ${ok}`, "result");
    result(`${ok}`);
    step("top",
        if ok {
            `Every node's subtrees differ in height by at most 1 — balanced.`.to_string()
        } else {
            `An imbalance was found and propagated to the root — not balanced.`
        },
        [ tree("tree", nodes).done(out[1]) ], "return");
    leave(`${ok}`);
}
