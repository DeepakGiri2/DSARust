// Maximum Depth of Binary Tree — the shape of a post-order recursion.
//
// Nothing is computed on the way *down*; every answer is assembled on the way
// back up. The call stack panel is the point of this one: watch it grow to a
// leaf, return a number, and grow again down the sibling.

fn depth(nodes, id, done) {
    if type_of(id) == "()" {
        enter("maxDepth(nil)");
        step("base", "A nil child. An empty tree has depth 0.", [ tree("tree", nodes).done(done) ]);
        log("nil -> 0");
        step("ret0", "Return 0.", [ tree("tree", nodes).done(done) ], "return");
        leave("0");
        return [0, done];
    }

    let val = nodes[id].val;
    enter(`maxDepth(${val})`, #{ node: PTR(`node(${val})`) });
    step("base", `node(${val}) exists, so the answer here is 1 plus the deeper of its two subtrees.`,
        [ tree("tree", nodes).cur(id).done(done) ]);

    step("recL", `Go left from ${val}.`, [ tree("tree", nodes).cur(id).done(done) ], "call");
    let l = depth(nodes, nodes[id].left, done);
    done = l[1];

    step("recR", `Left came back ${l[0]}. Now go right from ${val}.`,
        [ tree("tree", nodes).cur(id).done(done) ], "call");
    let r = depth(nodes, nodes[id].right, done);
    done = r[1];

    let best = if l[0] > r[0] { l[0] } else { r[0] };
    done.push(id);
    log(`depth(${val}) = 1 + max(${l[0]}, ${r[0]}) = ${best + 1}`);
    step("ret", `1 + max(${l[0]}, ${r[0]}) = ${best + 1}. Return that for node(${val}).`,
        [ tree("tree", nodes).cur(id).done(done) ], "return");
    leave(`${best + 1}`);
    [best + 1, done]
}

fn trace(input) {
    let nodes = tree_nodes(input.tree);
    if nodes.len() == 0 {
        enter("maxDepth(nil)");
        step("base", "The tree is empty.", [ tree("tree", nodes) ]);
        step("ret0", "Return 0.", [ tree("tree", nodes) ], "return");
        leave("0");
        result("0");
        return;
    }

    let out = depth(nodes, 0, []);
    result(`${out[0]}`);
}
