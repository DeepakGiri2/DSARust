// Flatten Binary Tree — thread it, do not traverse it.
//
// Preorder visits a node, then all of its left subtree, then its right
// subtree. So the node that immediately precedes the right subtree is the
// *rightmost* node of the left subtree. Hang the right subtree there, slide the
// left subtree across, and the order is preserved — with no stack, because the
// tree itself is being used to remember where to go next.

fn chain_from(nodes, id) {
    let out = [];
    let cur = id;
    let guard = 0;
    while type_of(cur) != "()" && guard < 200 {
        out.push(`${nodes[cur].val}`);
        cur = nodes[cur].right;
        guard += 1;
    }
    out
}

fn trace(input) {
    let nodes = tree_nodes(input.tree);

    enter("flatten", #{});

    let curr = if nodes.len() > 0 { 0 } else { () };
    step("init", "curr walks down the right spine, which is where the flattened list is being built.",
        [ tree("tree", nodes).cur(curr) ]);

    while type_of(curr) != "()" {
        set(#{ curr: PTR(`${nodes[curr].val}`) });
        step("loop", `At ${nodes[curr].val}.`,
            [ tree("tree", nodes).cur(curr) ]);

        if type_of(nodes[curr].left) != "()" {
            step("hasleft", `${nodes[curr].val} has a left subtree, so it must be moved to the right — but the current right subtree has to go somewhere first.`,
                [ tree("tree", nodes).cur(curr).hl([nodes[curr].left]) ]);

            let pred = nodes[curr].left;
            set(#{ pred: PTR(`${nodes[pred].val}`) });
            step("pred", `Start at the left child, ${nodes[pred].val}.`,
                [ tree("tree", nodes).cur(curr).hl([pred]) ]);

            while type_of(nodes[pred].right) != "()" {
                step("rightmost", `${nodes[pred].val} still has a right child, so it is not the last node preorder will visit in this subtree.`,
                    [ tree("tree", nodes).cur(curr).hl([pred]) ]);
                pred = nodes[pred].right;
                set(#{ pred: PTR(`${nodes[pred].val}`) });
                step("down", `Move to ${nodes[pred].val}.`,
                    [ tree("tree", nodes).cur(curr).hl([pred]) ]);
            }
            step("rightmost", `${nodes[pred].val} is the rightmost node of the left subtree — the last node preorder visits before it would return here.`,
                [ tree("tree", nodes).cur(curr).hl([pred]) ]);

            nodes[pred].right = nodes[curr].right;
            step("graft", `So graft ${nodes[curr].val}'s right subtree onto it. Preorder still reads the same.`,
                [ tree("tree", nodes).cur(curr).hl([pred]) ]);

            nodes[curr].right = nodes[curr].left;
            step("promote", "Now the left subtree can move across to the right.",
                [ tree("tree", nodes).cur(curr) ]);

            nodes[curr].left = ();
            step("clear", "And the left pointer is cleared — the problem demands every left be null.",
                [ tree("tree", nodes).cur(curr) ]);
        } else {
            step("hasleft", `${nodes[curr].val} has no left subtree — nothing to move.`,
                [ tree("tree", nodes).cur(curr) ]);
        }

        curr = nodes[curr].right;
        step("next", `Step right to ${if type_of(curr) == "()" { "nil" } else { `${nodes[curr].val}` }}.`,
            [ tree("tree", nodes).cur(curr) ]);
    }

    let order = chain_from(nodes, 0);
    let out = join(order, " ");
    log(out, "result");
    result(out);
    step("loop", `Flattened in preorder: ${out}. Every node was touched a constant number of times, with no stack.`,
        [ tree("tree", nodes), array("right spine", order) ], "return");
    leave(out);
}
