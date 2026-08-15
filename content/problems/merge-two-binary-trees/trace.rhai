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

// Merge Two Binary Trees — a missing side ends the recursion early.
//
// The interesting line is the one that looks like a base case but is really a
// shortcut: when one tree runs out, the other subtree is *already* the merged
// answer, so it is adopted whole and never walked. That is why the work is
// bounded by the smaller tree, not the larger.

fn name_of(nodes, id) {
    if type_of(id) == "()" { "nil" } else { `${nodes[id].val}` }
}

// Deep-copy the subtree rooted at `id` of `src` into `built`; returns [built, new_id].
fn graft(src, id, built) {
    if type_of(id) == "()" { return [built, ()]; }
    let me = built.len();
    built.push(#{ id: me, val: src[id].val, left: (), right: () });
    let l = graft(src, src[id].left, built);
    built = l[0];
    built[me].left = l[1];
    let r = graft(src, src[id].right, built);
    built = r[0];
    built[me].right = r[1];
    [built, me]
}

fn merge_at(na, nb, ai, bi, built) {
    enter(`mergeTrees(${name_of(na, ai)}, ${name_of(nb, bi)})`);

    if type_of(ai) == "()" {
        let g = graft(nb, bi, built);
        built = g[0];
        step("aNil", `Tree A has nothing here, so the whole subtree at ${name_of(nb, bi)} in tree B is already the answer — adopt it and stop descending.`,
            [ tree("tree A", na), tree("tree B", nb).cur(bi), tree("merged", built) ], "return");
        leave(name_of(nb, bi));
        return [built, g[1]];
    }

    if type_of(bi) == "()" {
        let g = graft(na, ai, built);
        built = g[0];
        step("bNil", `Tree B has nothing here, so tree A's subtree at ${name_of(na, ai)} carries over untouched.`,
            [ tree("tree A", na).cur(ai), tree("tree B", nb), tree("merged", built) ], "return");
        leave(name_of(na, ai));
        return [built, g[1]];
    }

    let me = built.len();
    let total = na[ai].val + nb[bi].val;
    built.push(#{ id: me, val: total, left: (), right: () });
    log(`${na[ai].val} + ${nb[bi].val} = ${total}`);
    step("sum", `Both trees have a node here: ${na[ai].val} + ${nb[bi].val} = ${total}.`,
        [ tree("tree A", na).cur(ai), tree("tree B", nb).cur(bi), tree("merged", built).cur(me) ]);

    step("recL", `Merge the two left children of ${total}.`,
        [ tree("tree A", na).cur(ai), tree("tree B", nb).cur(bi), tree("merged", built).cur(me) ], "call");
    let l = merge_at(na, nb, na[ai].left, nb[bi].left, built);
    built = l[0];
    built[me].left = l[1];

    step("recR", `Left side done. Merge the two right children of ${total}.`,
        [ tree("tree A", na).cur(ai), tree("tree B", nb).cur(bi), tree("merged", built).cur(me) ], "call");
    let r = merge_at(na, nb, na[ai].right, nb[bi].right, built);
    built = r[0];
    built[me].right = r[1];

    step("ret", `The subtree under ${total} is complete.`,
        [ tree("tree A", na), tree("tree B", nb), tree("merged", built).cur(me) ], "return");
    leave(`${total}`);
    [built, me]
}

fn trace(input) {
    let na = tree_nodes(input.root1);
    let nb = tree_nodes(input.root2);

    enter("mergeTrees", #{});
    let out = merge_at(na, nb, if na.len() > 0 { 0 } else { () },
                       if nb.len() > 0 { 0 } else { () }, []);
    let built = out[0];

    let answer = level_from(built, out[1]);
    log(`answer ${answer}`, "result");
    result(answer);
    leave(answer);
}
