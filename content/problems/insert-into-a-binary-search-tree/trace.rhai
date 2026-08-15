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

// Insert Into a BST — the tree already knows where the value belongs.
//
// Every comparison eliminates a whole subtree, so the walk down is not a
// search for a good spot; it is the only spot. When the side you want is
// empty, hanging the node there cannot break the ordering, because every
// ancestor comparison that led here still holds.

fn trace(input) {
    let nodes = tree_nodes(input.root);
    let v = input.val;

    enter("insertIntoBST", #{ val: N(v) });

    if nodes.len() == 0 {
        nodes.push(#{ id: 0, val: v, left: (), right: () });
        step("empty", `The tree was empty, so ${v} becomes the root.`, [ tree("tree", nodes) ]);
        let answer = level_from(nodes, 0);
        result(answer);
        step("ret", `Done: ${answer}.`, [ tree("tree", nodes) ], "return");
        leave(answer);
        return;
    }

    step("empty", `The tree is not empty, so ${v} has to find its slot.`,
        [ tree("tree", nodes) ]);

    let cur = 0;
    let path = [0];
    set(#{ cur: PTR(`node(${nodes[0].val})`) });
    step("cur", "Start at the root.", [ tree("tree", nodes).cur(cur).path(path) ]);

    loop {
        step("loop", `At ${nodes[cur].val}. One comparison rules out half of what is below.`,
            [ tree("tree", nodes).cur(cur).path(path) ]);

        if v < nodes[cur].val {
            step("less", `${v} < ${nodes[cur].val}, so it belongs somewhere in the left subtree — the right subtree is out of the question forever.`,
                [ tree("tree", nodes).cur(cur).path(path) ]);

            if type_of(nodes[cur].left) == "()" {
                step("leftFree", `${nodes[cur].val} has no left child, and that empty slot is the answer.`,
                    [ tree("tree", nodes).cur(cur).path(path) ]);
                let id = nodes.len();
                nodes.push(#{ id: id, val: v, left: (), right: () });
                nodes[cur].left = id;
                path.push(id);
                log(`attach ${v} as the left child of ${nodes[cur].val}`, "result");
                step("attachL", `Hang ${v} there. Every comparison on the way down still holds, so the tree is still a BST.`,
                    [ tree("tree", nodes).cur(id).path(path) ]);
                break;
            }

            cur = nodes[cur].left;
            path.push(cur);
            set(#{ cur: PTR(`node(${nodes[cur].val})`) });
            step("goL", `Move down to ${nodes[cur].val}.`,
                [ tree("tree", nodes).cur(cur).path(path) ]);
        } else {
            step("more", `${v} > ${nodes[cur].val}, so it belongs in the right subtree.`,
                [ tree("tree", nodes).cur(cur).path(path) ]);

            if type_of(nodes[cur].right) == "()" {
                step("rightFree", `${nodes[cur].val} has no right child — that slot is free.`,
                    [ tree("tree", nodes).cur(cur).path(path) ]);
                let id = nodes.len();
                nodes.push(#{ id: id, val: v, left: (), right: () });
                nodes[cur].right = id;
                path.push(id);
                log(`attach ${v} as the right child of ${nodes[cur].val}`, "result");
                step("attachR", `Hang ${v} there and the walk is over.`,
                    [ tree("tree", nodes).cur(id).path(path) ]);
                break;
            }

            cur = nodes[cur].right;
            path.push(cur);
            set(#{ cur: PTR(`node(${nodes[cur].val})`) });
            step("goR", `Move down to ${nodes[cur].val}.`,
                [ tree("tree", nodes).cur(cur).path(path) ]);
        }
    }

    let answer = level_from(nodes, 0);
    result(answer);
    step("ret", `The root never changed — only one link was added. Result: ${answer}.`,
        [ tree("tree", nodes) ], "return");
    leave(answer);
}
