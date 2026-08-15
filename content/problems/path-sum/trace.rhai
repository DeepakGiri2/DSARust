// Path Sum — carry what is left, not what has been spent.
//
// Passing the *remaining* target down means a leaf needs no context at all: if
// the remainder is zero the path worked. And because the test happens only at
// a leaf, a path that hits the target halfway down is correctly rejected — the
// question was root-to-leaf, not root-to-anywhere.

fn hasp(nodes, id, rem, path) {
    if type_of(id) == "()" {
        enter(`hasPathSum(nil, ${rem})`, #{ rem: N(rem) });
        step("base", "There is no node here, so no path ends here either.",
            [ tree("tree", nodes).path(path) ]);
        leave("false");
        return false;
    }

    let v = nodes[id].val;
    path.push(id);
    enter(`hasPathSum(${v}, ${rem})`, #{ rem: N(rem) });
    step("base", `At node ${v}, still needing ${rem}.`,
        [ tree("tree", nodes).cur(id).path(path) ]);

    let left = rem - v;
    set(#{ rem: N(left) });
    step("sub", `Spend ${v} here: ${rem} - ${v} = ${left} still to find below.`,
        [ tree("tree", nodes).cur(id).path(path),
          text("path so far", [`nodes: ${join(vals_on(nodes, path), " -> ")}`, `remaining: ${left}`]) ]);

    if type_of(nodes[id].left) == "()" && type_of(nodes[id].right) == "()" {
        step("leaf", `${v} is a leaf, so this path is finished — it is now a yes/no question.`,
            [ tree("tree", nodes).cur(id).path(path) ]);
        let ok = left == 0;
        if ok {
            log(`path ${join(vals_on(nodes, path), " + ")} hits the target`, "result");
        }
        step("check",
            if ok {
                `${left} left over, which is exactly nothing — the path adds up.`
            } else {
                `${left} left over, so this path misses.`
            },
            [ tree("tree", nodes).cur(id).path(if ok { path } else { [] }) ], "return");
        leave(`${ok}`);
        return ok;
    }

    step("leaf", `${v} still has a child, so the path is not over.`,
        [ tree("tree", nodes).cur(id).path(path) ]);

    step("recL", `Try the left side of ${v} with ${left} remaining.`,
        [ tree("tree", nodes).cur(id).path(path) ], "call");
    if hasp(nodes, nodes[id].left, left, path) {
        step("foundL", "The left side already found a path, so nothing on the right needs looking at.",
            [ tree("tree", nodes).path(path) ], "return");
        leave("true");
        return true;
    }

    step("recR", `Left came back empty. Try the right side of ${v} with ${left} remaining.`,
        [ tree("tree", nodes).cur(id).path(path) ], "call");
    let ok = hasp(nodes, nodes[id].right, left, path);
    leave(`${ok}`);
    ok
}

fn vals_on(nodes, ids) {
    let out = [];
    for id in ids { out.push(`${nodes[id].val}`); }
    out
}

fn trace(input) {
    let nodes = tree_nodes(input.root);
    let target = input.targetSum;

    enter("hasPathSum", #{ targetSum: N(target) });
    let ok = false;
    if nodes.len() > 0 {
        ok = hasp(nodes, 0, target, []);
    } else {
        step("base", "An empty tree has no root-to-leaf path at all.", [ tree("tree", nodes) ]);
    }

    log(`answer ${ok}`, "result");
    result(`${ok}`);
    leave(`${ok}`);
}
