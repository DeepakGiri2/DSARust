// Count Good Nodes — carry the maximum *down* the path.
//
// "Good" is a property of the path from the root, not of the subtree, so the
// information travels downwards as an argument while the counts come back up
// as a return value.

fn dfs(nodes, id, max_so_far, path, good) {
    if type_of(id) == "()" {
        enter("dfs(nil)");
        step("base", "Nothing here.", [ tree("tree", nodes).done(good) ]);
        step("ret0", "Return 0.", [ tree("tree", nodes).done(good) ], "return");
        leave("0");
        return [0, good];
    }

    let val = nodes[id].val;
    path.push(id);
    enter(`dfs(${val}, max=${max_so_far})`, #{ node: PTR(`node(${val})`), maxSoFar: N(max_so_far) });
    step("base", `At node(${val}). The largest value on the path from the root is ${max_so_far}.`,
        [ tree("tree", nodes).cur(id).path(path).done(good) ]);

    let count = 0;
    set(#{ count: N(count) });
    step("init", "Count the good nodes in this subtree.",
        [ tree("tree", nodes).cur(id).path(path).done(good) ]);

    if val >= max_so_far {
        step("check", `${val} is at least ${max_so_far}, so nothing on the way here blocks the view.`,
            [ tree("tree", nodes).cur(id).path(path).done(good) ]);
        count = 1;
        good.push(id);
        log(`good node ${val}`, "result");
        step("good", `node(${val}) is good.`,
            [ tree("tree", nodes).cur(id).path(path).done(good) ]);
    } else {
        step("check", `${val} is smaller than ${max_so_far} further up the path, so it is not good.`,
            [ tree("tree", nodes).cur(id).path(path).done(good) ]);
    }

    let next_max = if val > max_so_far { val } else { max_so_far };
    step("upd", `Children of ${val} must beat ${next_max}.`,
        [ tree("tree", nodes).cur(id).path(path).done(good) ]);

    step("recL", `Descend left from ${val}.`,
        [ tree("tree", nodes).cur(id).path(path).done(good) ], "call");
    let l = dfs(nodes, nodes[id].left, next_max, path, good);
    good = l[1];
    count += l[0];

    step("recR", `Left contributed ${l[0]}. Now the right.`,
        [ tree("tree", nodes).cur(id).path(path).done(good) ], "call");
    let r = dfs(nodes, nodes[id].right, next_max, path, good);
    good = r[1];
    count += r[0];

    set(#{ count: N(count) });
    step("ret", `Subtree at ${val} holds ${count} good node(s).`,
        [ tree("tree", nodes).cur(id).done(good) ], "return");
    leave(`${count}`);
    [count, good]
}

fn trace(input) {
    let nodes = tree_nodes(input.root);

    enter("goodNodes", #{});
    step("call", "The root is always good — nothing stands above it.",
        [ tree("tree", nodes) ], "call");

    let out = if nodes.len() > 0 { dfs(nodes, 0, -1000000, [], []) } else { [0, []] };

    log(`answer ${out[0]}`, "result");
    result(`${out[0]}`);
    step("call", `${out[0]} node(s) are at least as large as everything above them.`,
        [ tree("tree", nodes).done(out[1]) ], "return");
    leave(`${out[0]}`);
}
