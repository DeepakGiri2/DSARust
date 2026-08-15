// Lowest Common Ancestor of a BST — walk down until the values split.
//
// In a BST the answer needs no recursion: if both targets are smaller than the
// current node they are both in its left subtree, and the same on the right.
// The first node that sits *between* them is the lowest ancestor of both.

fn trace(input) {
    let nodes = tree_nodes(input.root);
    let p = input.p;
    let q = input.q;
    let path = [];

    enter("lowestCommonAncestor", #{ p: N(p), q: N(q) });
    let cur = if nodes.len() > 0 { 0 } else { () };
    step("start", `Looking for the lowest node that has both ${p} and ${q} beneath it.`,
        [ tree("tree", nodes).cur(cur) ]);

    while type_of(cur) != "()" {
        path.push(cur);
        let val = nodes[cur].val;
        set(#{ cur: N(val) });
        step("loop", `At node(${val}).`,
            [ tree("tree", nodes).cur(cur).path(path) ]);

        if p < val && q < val {
            step("bothLeft", `${p} and ${q} are both below ${val}, so both live in its left subtree.`,
                [ tree("tree", nodes).cur(cur).path(path) ]);
            cur = nodes[cur].left;
            step("goLeft", `Move left to node(${nodes[cur].val}).`,
                [ tree("tree", nodes).cur(cur).path(path) ]);
        } else if p > val && q > val {
            step("bothRight", `${p} and ${q} are both above ${val}, so both live in its right subtree.`,
                [ tree("tree", nodes).cur(cur).path(path) ]);
            cur = nodes[cur].right;
            step("goRight", `Move right to node(${nodes[cur].val}).`,
                [ tree("tree", nodes).cur(cur).path(path) ]);
        } else {
            log(`split at ${val}`, "result");
            step("split",
                `${p} and ${q} fall on opposite sides of ${val} (or one of them is ${val}), so the paths to them part here.`,
                [ tree("tree", nodes).cur(cur).path(path).done([cur]) ]);
            result(`${val}`);
            step("ret", `node(${val}) is the lowest common ancestor.`,
                [ tree("tree", nodes).done([cur]).path(path) ], "return");
            leave(`${val}`);
            return;
        }
    }

    log("not found", "result");
    result("-1");
    step("none", "Walked off the tree without finding a split.", [ tree("tree", nodes) ], "return");
    leave("-1");
}
