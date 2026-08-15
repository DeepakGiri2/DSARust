fn level_from(nodes, root) {
    if type_of(root) == "()" { return "nil"; }
    let out = [];
    let q = [root];
    let qi = 0;
    while qi < q.len() {
        let id = q[qi];
        qi += 1;
        if type_of(id) == "()" {
            out.push("n");
        } else {
            out.push(`${nodes[id].val}`);
            q.push(nodes[id].left);
            q.push(nodes[id].right);
        }
    }
    while out.len() > 0 && out[out.len() - 1] == "n" { out.truncate(out.len() - 1); }
    join(out, " ")
}

// Delete Node in a BST — the two-child case is turned into a one-child case.
//
// Nothing can simply be lifted into the hole left by a node with two children,
// so the value is replaced rather than the node. The in-order successor (the
// leftmost node of the right subtree) is the smallest value still larger than
// everything on the left, which is exactly what the slot requires. And that
// successor has no left child by definition, so deleting *it* is an easy case.

fn del(nodes, id, key, top) {
    if type_of(id) == "()" {
        enter(`deleteNode(nil, ${key})`, #{ key: N(key) });
        step("base", `${key} is not in this subtree — there is nothing here at all.`,
            [ tree("tree", nodes).root(top) ]);
        leave("nil");
        return [nodes, (), top];
    }

    let v = nodes[id].val;
    enter(`deleteNode(${v}, ${key})`, #{ key: N(key) });
    step("base", `At node ${v}.`, [ tree("tree", nodes).root(top).cur(id) ]);

    if key < v {
        step("less", `${key} < ${v}, so it can only be in the left subtree.`,
            [ tree("tree", nodes).root(top).cur(id) ], "call");
        let out = del(nodes, nodes[id].left, key, top);
        nodes = out[0];
        top = out[2];
        nodes[id].left = out[1];
        step("goL", `Re-attach whatever the left subtree became — deletion can change a subtree's root, so the parent must always take the returned link.`,
            [ tree("tree", nodes).root(top).cur(id) ]);
    } else if key > v {
        step("greater", `${key} > ${v}, so look right.`,
            [ tree("tree", nodes).root(top).cur(id) ], "call");
        let out = del(nodes, nodes[id].right, key, top);
        nodes = out[0];
        top = out[2];
        nodes[id].right = out[1];
        step("goR", "Re-attach the returned right subtree.",
            [ tree("tree", nodes).root(top).cur(id) ]);
    } else {
        log(`found ${key}`);
        step("found", `${v} is the node to remove.`,
            [ tree("tree", nodes).root(top).cur(id) ]);

        if type_of(nodes[id].left) == "()" {
            step("noLeft", `It has no left child, so there is no ordering to preserve on that side.`,
                [ tree("tree", nodes).root(top).cur(id) ]);
            let r = nodes[id].right;
            if type_of(top) != "()" && id == top { top = r; }
            step("liftR", "Lift the right child straight into its place.",
                [ tree("tree", nodes).root(top).cur(r) ], "return");
            leave("lifted right child");
            return [nodes, r, top];
        }
        step("noLeft", `${v} has a left child, so nothing can simply be dropped in.`,
            [ tree("tree", nodes).root(top).cur(id) ]);

        if type_of(nodes[id].right) == "()" {
            step("noRight", "But it has no right child, which makes it the mirror of the easy case.",
                [ tree("tree", nodes).root(top).cur(id) ]);
            let l = nodes[id].left;
            if type_of(top) != "()" && id == top { top = l; }
            step("liftL", "Lift the left child into its place.",
                [ tree("tree", nodes).root(top).cur(l) ], "return");
            leave("lifted left child");
            return [nodes, l, top];
        }
        step("noRight", `${v} has both children, so the hole cannot be filled by lifting anything.`,
            [ tree("tree", nodes).root(top).cur(id) ]);

        let s = nodes[id].right;
        step("succ", `Instead, find the in-order successor: start at the right child, ${nodes[s].val}.`,
            [ tree("tree", nodes).root(top).cur(s).path([id, s]) ]);

        while type_of(nodes[s].left) != "()" {
            step("descend", `${nodes[s].val} still has a left child, so something smaller is below it.`,
                [ tree("tree", nodes).root(top).cur(s) ]);
            s = nodes[s].left;
            step("stepL", `Down to ${nodes[s].val}.`,
                [ tree("tree", nodes).root(top).cur(s) ]);
        }
        step("descend", `${nodes[s].val} has no left child, so it is the smallest value larger than ${key} — exactly what belongs in the hole.`,
            [ tree("tree", nodes).root(top).cur(s) ]);

        let sv = nodes[s].val;
        nodes[id].val = sv;
        log(`copy successor ${sv} into the deleted slot`);
        step("copy", `Overwrite ${v} with ${sv}. Every value on the left is still smaller and every value on the right still larger, so the tree is valid again — except that ${sv} now appears twice.`,
            [ tree("tree", nodes).root(top).cur(id).path([id, s]) ]);

        step("remove", `Delete the original ${sv} from the right subtree. It has no left child, so that call lands in an easy case.`,
            [ tree("tree", nodes).root(top).cur(s) ], "call");
        let out = del(nodes, nodes[id].right, sv, top);
        nodes = out[0];
        top = out[2];
        nodes[id].right = out[1];
    }

    step("ret", `The subtree rooted here is a valid BST again.`,
        [ tree("tree", nodes).root(top).cur(id) ], "return");
    leave(`${nodes[id].val}`);
    [nodes, id, top]
}

fn trace(input) {
    let nodes = tree_nodes(input.root);
    let key = input.key;

    enter("deleteNode", #{ key: N(key) });
    let root = if nodes.len() > 0 { 0 } else { () };
    let out = del(nodes, root, key, root);
    nodes = out[0];

    let answer = level_from(nodes, out[1]);
    log(`answer ${answer}`, "result");
    result(answer);
    leave(answer);
}
