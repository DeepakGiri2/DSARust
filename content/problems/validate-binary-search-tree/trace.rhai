// Validate Binary Search Tree — carry an allowed range down the tree.
//
// Checking `left < node < right` at each node is the classic wrong answer: it
// misses a grandchild that violates an ancestor. Passing a (low, high) window
// down fixes that, because every node inherits the constraints of everything
// above it.

fn valid(nodes, id, low, high, done) {
    let range = `(${if low == () { "-inf" } else { `${low}` }}, ${if high == () { "+inf" } else { `${high}` }})`;

    if type_of(id) == "()" {
        enter(`valid(nil, ${range})`);
        step("base", "An empty subtree cannot break anything.",
            [ tree("tree", nodes).done(done) ]);
        step("rettrue", "Return true.", [ tree("tree", nodes).done(done) ], "return");
        leave("true");
        return [true, done];
    }

    let val = nodes[id].val;
    enter(`valid(${val}, ${range})`, #{ low: S(`${if low == () { "-inf" } else { `${low}` }}`),
                                        high: S(`${if high == () { "+inf" } else { `${high}` }}`) });
    step("base", `node(${val}) must lie strictly inside ${range}.`,
        [ tree("tree", nodes).cur(id).done(done),
          text("allowed range", [`${range}`, `node value = ${val}`]) ]);

    step("read", `Read the value: ${val}.`, [ tree("tree", nodes).cur(id).done(done) ]);

    let too_low = low != () && val <= low;
    let too_high = high != () && val >= high;
    if too_low || too_high {
        let why = if too_low { `${val} <= ${low}` } else { `${val} >= ${high}` };
        log(`violation at ${val}: ${why}`, "result");
        step("check", `${why} — an ancestor already forbids this value here.`,
            [ tree("tree", nodes).cur(id).done(done) ]);
        step("retfalse", "Return false.", [ tree("tree", nodes).cur(id) ], "return");
        leave("false");
        return [false, done];
    }
    step("check", `${val} is inside ${range}.`, [ tree("tree", nodes).cur(id).done(done) ]);

    step("recL", `Everything left of ${val} must be below ${val} as well as inside ${range}.`,
        [ tree("tree", nodes).cur(id).done(done) ], "call");
    let l = valid(nodes, nodes[id].left, low, val, done);
    done = l[1];
    if !l[0] {
        step("badL", "The left subtree failed, so this one does too.",
            [ tree("tree", nodes).cur(id) ], "return");
        leave("false");
        return [false, done];
    }

    step("recR", `Everything right of ${val} must be above ${val}.`,
        [ tree("tree", nodes).cur(id).done(done) ], "call");
    let r = valid(nodes, nodes[id].right, val, high, done);
    done = r[1];
    if !r[0] {
        step("badR", "The right subtree failed.", [ tree("tree", nodes).cur(id) ], "return");
        leave("false");
        return [false, done];
    }

    done.push(id);
    step("ret", `The subtree rooted at ${val} is a valid BST.`,
        [ tree("tree", nodes).cur(id).done(done) ], "return");
    leave("true");
    [true, done]
}

fn trace(input) {
    let nodes = tree_nodes(input.tree);

    enter("isValidBST", #{});
    step("call", "The root may hold any value, so it starts with an unbounded range.",
        [ tree("tree", nodes) ], "call");

    let out = if nodes.len() > 0 { valid(nodes, 0, (), (), []) } else { [true, []] };

    log(`valid: ${out[0]}`, "result");
    result(`${out[0]}`);
    step("call",
        if out[0] {
            "Every node fell inside the range its ancestors allowed.".to_string()
        } else {
            "A node violated a bound set by one of its ancestors."
        },
        [ tree("tree", nodes).done(out[1]) ], "return");
    leave(`${out[0]}`);
}
