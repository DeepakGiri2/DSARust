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

// Search in a BST — binary search over links instead of indices.
//
// Each comparison throws away a whole subtree, which is the same halving an
// array binary search gets from `lo`/`hi`. The only difference is that the
// structure, rather than arithmetic, decides how big each half is.

fn subtree_ids(nodes, id) {
    if type_of(id) == "()" { return []; }
    let out = [];
    let q = [id];
    let qi = 0;
    while qi < q.len() {
        let x = q[qi];
        qi += 1;
        out.push(x);
        if type_of(nodes[x].left) != "()" { q.push(nodes[x].left); }
        if type_of(nodes[x].right) != "()" { q.push(nodes[x].right); }
    }
    out
}

fn trace(input) {
    let nodes = tree_nodes(input.root);
    let v = input.val;

    enter("searchBST", #{ val: N(v) });

    let cur = if nodes.len() > 0 { 0 } else { () };
    let path = [];
    set(#{ cur: PTR(if nodes.len() > 0 { `node(${nodes[0].val})` } else { "nil" }) });
    step("cur", "Start at the root.", [ tree("tree", nodes).cur(cur) ]);

    let found = ();
    while type_of(cur) != "()" {
        path.push(cur);
        step("loop", `Still somewhere to look: ${nodes[cur].val}.`,
            [ tree("tree", nodes).cur(cur).path(path) ]);

        if v == nodes[cur].val {
            found = cur;
            log(`found ${v}`, "result");
            step("hit", `${nodes[cur].val} is the value being searched for.`,
                [ tree("tree", nodes).cur(cur).path(path) ]);
            step("ret", `Return this node and everything under it.`,
                [ tree("tree", nodes).cur(cur).done(subtree_ids(nodes, cur)) ], "return");
            break;
        }

        if v < nodes[cur].val {
            step("less", `${v} < ${nodes[cur].val}. Every value to the right of ${nodes[cur].val} is larger still, so that entire subtree is ruled out.`,
                [ tree("tree", nodes).cur(cur).path(path) ]);
            cur = nodes[cur].left;
            set(#{ cur: PTR(if type_of(cur) != "()" { `node(${nodes[cur].val})` } else { "nil" }) });
            step("goL",
                if type_of(cur) != "()" { `Move left to ${nodes[cur].val}.` }
                else { "Move left — and there is nothing there.".to_string() },
                [ tree("tree", nodes).cur(cur).path(path) ]);
        } else {
            step("greater", `${v} > ${nodes[cur].val}, so the whole left subtree is ruled out.`,
                [ tree("tree", nodes).cur(cur).path(path) ]);
            cur = nodes[cur].right;
            set(#{ cur: PTR(if type_of(cur) != "()" { `node(${nodes[cur].val})` } else { "nil" }) });
            step("goR",
                if type_of(cur) != "()" { `Move right to ${nodes[cur].val}.` }
                else { "Move right — and there is nothing there.".to_string() },
                [ tree("tree", nodes).cur(cur).path(path) ]);
        }
    }

    if type_of(found) == "()" {
        log(`${v} is not in the tree`, "result");
        step("miss", `The walk fell off the bottom of the tree, so ${v} is not present. Only ${path.len()} comparison(s) were needed to be sure.`,
            [ tree("tree", nodes).path(path) ], "return");
    }

    let answer = level_from(nodes, found);
    result(answer);
    leave(answer);
}
