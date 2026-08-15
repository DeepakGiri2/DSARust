// Same Tree — one recursion walking two trees in lockstep.
//
// Three cheap checks at every node — both nil, one nil, values differ — and
// only then descend. The two tree views move together, so a mismatch shows up
// as the two cursors sitting on visibly different nodes.

fn same(pn, qn, pid, qid) {
    let pl = if type_of(pid) == "()" { "nil" } else { `${pn[pid].val}` };
    let ql = if type_of(qid) == "()" { "nil" } else { `${qn[qid].val}` };
    enter(`isSameTree(${pl}, ${ql})`);

    if type_of(pid) == "()" && type_of(qid) == "()" {
        step("bothNil", "Both sides are nil — two absent subtrees are trivially the same.",
            [ tree("p", pn), tree("q", qn) ]);
        step("rettrue", "Return true.", [ tree("p", pn), tree("q", qn) ], "return");
        leave("true");
        return true;
    }
    step("bothNil", "At least one side is a real node.",
        [ tree("p", pn).cur(pid), tree("q", qn).cur(qid) ]);

    if type_of(pid) == "()" || type_of(qid) == "()" {
        step("oneNil", `One side is nil and the other is ${if pl == "nil" { ql } else { pl }} — the shapes already differ.`,
            [ tree("p", pn).cur(pid), tree("q", qn).cur(qid) ]);
        log("shape mismatch", "result");
        step("retNil", "Return false.", [ tree("p", pn), tree("q", qn) ], "return");
        leave("false");
        return false;
    }
    step("oneNil", "Both sides are real nodes, so the values are worth comparing.",
        [ tree("p", pn).cur(pid), tree("q", qn).cur(qid) ]);

    if pn[pid].val != qn[qid].val {
        step("valDiff", `${pl} and ${ql} are different values.`,
            [ tree("p", pn).cur(pid), tree("q", qn).cur(qid) ]);
        log(`${pl} != ${ql}`, "result");
        step("retDiff", "Return false.", [ tree("p", pn), tree("q", qn) ], "return");
        leave("false");
        return false;
    }
    step("valDiff", `Both nodes hold ${pl}. So far so good.`,
        [ tree("p", pn).cur(pid).done([pid]), tree("q", qn).cur(qid).done([qid]) ]);

    step("recL", "Compare the left subtrees.",
        [ tree("p", pn).cur(pid), tree("q", qn).cur(qid) ], "call");
    let l = same(pn, qn, pn[pid].left, qn[qid].left);

    step("recR", `Left came back ${l}. Now the right subtrees.`,
        [ tree("p", pn).cur(pid), tree("q", qn).cur(qid) ], "call");
    let r = same(pn, qn, pn[pid].right, qn[qid].right);

    let both = l && r;
    log(`node ${pl}: left=${l}, right=${r}`);
    step("ret", `${pl} matches and both subtrees came back ${l} and ${r}, so this subtree is ${both}.`,
        [ tree("p", pn).cur(pid), tree("q", qn).cur(qid) ], "return");
    leave(`${both}`);
    both
}

fn trace(input) {
    let pn = tree_nodes(input.p);
    let qn = tree_nodes(input.q);
    let proot = if pn.len() > 0 { 0 } else { () };
    let qroot = if qn.len() > 0 { 0 } else { () };

    let answer = same(pn, qn, proot, qroot);
    result(`${answer}`);
}
