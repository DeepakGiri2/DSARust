// Construct Binary Tree from Preorder and Inorder — each traversal supplies
// half the answer.
//
// Preorder tells you *who* the root is (it comes first). Inorder tells you
// *where* the split is (everything left of the root belongs to the left
// subtree). Neither alone is enough; together they pin the tree down exactly.

fn build(pre, ino, state, lo, hi, depth) {
    if lo > hi {
        enter("build(empty)");
        step("base", "This side of the split is empty.",
            [ array("inorder", ino).done(state.placed) ]);
        step("retNil", "Return nil.", [ array("inorder", ino) ], "return");
        leave("nil");
        return state;
    }

    let val = pre[state.pi];
    enter(`build(${val})`, #{ preIndex: N(state.pi) });
    step("pickRoot", `The next value in preorder is ${val}, so it is the root of this subtree.`,
        [ array("preorder", pre).ptr("pi", state.pi).hl([state.pi]),
          array("inorder", ino).window(lo, hi).done(state.placed) ]);

    state.pi += 1;
    step("advance", "Consume it from preorder.",
        [ array("preorder", pre).ptr("pi", state.pi).done(before(state.pi)),
          array("inorder", ino).window(lo, hi) ]);

    let mid = lo;
    while ino[mid] != val { mid += 1; }
    state.nodes.push(#{ id: state.nodes.len(), val: val, left: (), right: () });
    let me = state.nodes.len() - 1;
    step("makeNode", `Create node(${val}).`,
        [ array("inorder", ino).window(lo, hi).hl([mid]),
          tree("tree so far", state.nodes).cur(me) ]);
    step("findMid",
        `${val} sits at index ${mid} of inorder, so [${lo}..${mid - 1}] is its left subtree and [${mid + 1}..${hi}] its right.`,
        [ array("inorder", ino).hl([mid]).window(lo, hi),
          tree("tree so far", state.nodes).cur(me) ]);

    state.placed.push(mid);
    step("recL", `Build the left subtree of ${val}.`,
        [ array("inorder", ino).window(lo, mid - 1).done(state.placed) ], "call");
    let before_left = state.nodes.len();
    state = build(pre, ino, state, lo, mid - 1, depth + 1);
    if state.nodes.len() > before_left {
        state.nodes[me].left = before_left;
    }

    step("recR", `Now the right subtree of ${val}.`,
        [ array("inorder", ino).window(mid + 1, hi).done(state.placed),
          tree("tree so far", state.nodes).cur(me) ], "call");
    let before_right = state.nodes.len();
    state = build(pre, ino, state, mid + 1, hi, depth + 1);
    if state.nodes.len() > before_right {
        state.nodes[me].right = before_right;
    }

    step("ret", `node(${val}) is complete.`,
        [ tree("tree so far", state.nodes).done([me]) ], "return");
    leave(`node(${val})`);
    state
}

fn trace(input) {
    let pre = input.preorder;
    let ino = input.inorder;
    let state = #{ pi: 0, nodes: [], placed: [] };

    enter("buildTree", #{ preorder: A(pre), inorder: A(ino) });
    step("base", "Preorder names the roots in order; inorder shows where each one splits its range.",
        [ array("preorder", pre), array("inorder", ino) ]);

    state = build(pre, ino, state, 0, ino.len() - 1, 0);

    result(`root(${state.nodes[0].val})`);
    step("ret", "Both traversals are fully consumed and the tree is unique.",
        [ tree("tree", state.nodes) ], "return");
    leave(`root(${state.nodes[0].val})`);
}
