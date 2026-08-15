// Invert Binary Tree — recursion, seen as a call stack.
//
// The interesting part is the *order*: nothing is swapped until the recursion
// has already bottomed out, so the tree flips from the leaves upward. Watch
// the call stack panel grow to the leaves and unwind while the swaps happen.

// The tree is threaded through the recursion by value and returned, because a
// script function that takes an array gets a copy of it.
fn invert(nodes, id, depth) {
    if type_of(id) == "()" {
        enter("invertTree(nil)");
        step("base", "This child is nil — an empty subtree is already its own mirror.",
            [ tree("tree", nodes).cur(()) ]);
        step("retnil", "Return nil and unwind one level.", [ tree("tree", nodes) ], "return");
        leave("nil");
        return nodes;
    }

    let val = nodes[id].val;
    enter(`invertTree(${val})`, #{ node: PTR(`node(${val})`) });
    step("base", `node(${val}) is not nil, so this subtree has real work to do.`,
        [ tree("tree", nodes).cur(id) ]);

    step("recL", `Descend into the left child of ${val} first. Nothing is swapped until the bottom is reached.`,
        [ tree("tree", nodes).cur(id) ], "call");
    nodes = invert(nodes, nodes[id].left, depth + 1);

    step("recR", `Left subtree of ${val} is done. Now the right child.`,
        [ tree("tree", nodes).cur(id) ], "call");
    nodes = invert(nodes, nodes[id].right, depth + 1);

    let l = nodes[id].left;
    let r = nodes[id].right;
    nodes[id].left = r;
    step("setL",
        `Both children are already inverted, so ${val} only has to swap them: left now points at ${if type_of(r) == "()" { "nil" } else { `${nodes[r].val}` }}.`,
        [ tree("tree", nodes).cur(id).swap(if type_of(l) == "()" { id } else { l },
                                          if type_of(r) == "()" { id } else { r }) ]);

    nodes[id].right = l;
    log(`swap children of ${val}`);
    step("setR", `And right now points at ${if type_of(l) == "()" { "nil" } else { `${nodes[l].val}` }}. node(${val}) is mirrored.`,
        [ tree("tree", nodes).cur(id) ]);

    step("ret", `Return node(${val}) to the caller.`, [ tree("tree", nodes).done([id]) ], "return");
    leave(`node(${val})`);
    nodes
}

fn trace(input) {
    let nodes = tree_nodes(input.tree);
    if nodes.len() == 0 {
        enter("invertTree(nil)");
        step("base", "The tree is empty.", [ tree("tree", nodes) ]);
        step("retnil", "Return nil.", [ tree("tree", nodes) ], "return");
        leave("nil");
        return;
    }

    nodes = invert(nodes, 0, 0);
    result(`root(${nodes[0].val})`);
}
