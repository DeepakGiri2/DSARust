// Subtree of Another Tree — try an exact match at every node.
//
// Two nested recursions: the outer one picks a candidate root, the inner one
// checks whether the two trees are identical from there. The important detail
// is that "subtree" means the *whole* remaining shape must match, not just the
// values along a path.

fn same(an, bn, a, b) {
    if type_of(a) == "()" && type_of(b) == "()" {
        enter("isSame(nil, nil)");
        step("bothNil", "Both sides ended together.", [ tree("root", an), tree("subRoot", bn) ]);
        step("sameTrue", "Return true.", [ tree("root", an), tree("subRoot", bn) ], "return");
        leave("true");
        return true;
    }
    if type_of(a) == "()" || type_of(b) == "()" {
        enter("isSame");
        step("oneNil", "One side ran out before the other — the shapes differ.",
            [ tree("root", an).cur(a), tree("subRoot", bn).cur(b) ]);
        step("sameFalse", "Return false.", [ tree("root", an), tree("subRoot", bn) ], "return");
        leave("false");
        return false;
    }

    enter(`isSame(${an[a].val}, ${bn[b].val})`);
    if an[a].val != bn[b].val {
        step("valDiff", `${an[a].val} and ${bn[b].val} differ.`,
            [ tree("root", an).cur(a), tree("subRoot", bn).cur(b) ]);
        step("sameDiff", "Return false.", [ tree("root", an), tree("subRoot", bn) ], "return");
        leave("false");
        return false;
    }
    step("valDiff", `Both hold ${an[a].val}.`,
        [ tree("root", an).cur(a).done([a]), tree("subRoot", bn).cur(b).done([b]) ]);

    step("sameRec", "Check the children on both sides.",
        [ tree("root", an).cur(a), tree("subRoot", bn).cur(b) ], "call");
    let l = same(an, bn, an[a].left, bn[b].left);
    let r = if l { same(an, bn, an[a].right, bn[b].right) } else { false };
    let both = l && r;
    leave(`${both}`);
    both
}

fn check(an, bn, id) {
    if type_of(id) == "()" {
        enter("isSubtree(nil)");
        step("rootNil", "There is no node here to match against.", [ tree("root", an) ]);
        step("retFalse", "Return false.", [ tree("root", an) ], "return");
        leave("false");
        return false;
    }

    enter(`isSubtree(${an[id].val})`);
    step("check", `Try matching the whole of subRoot against node(${an[id].val}).`,
        [ tree("root", an).cur(id), tree("subRoot", bn) ], "call");

    if same(an, bn, id, if bn.len() > 0 { 0 } else { () }) {
        log(`match rooted at ${an[id].val}`, "result");
        step("retTrue", `The trees are identical from node(${an[id].val}) down.`,
            [ tree("root", an).cur(id).done([id]), tree("subRoot", bn) ], "return");
        leave("true");
        return true;
    }

    step("recurse", `node(${an[id].val}) is not the root of a match — try its children.`,
        [ tree("root", an).cur(id) ], "call");
    let found = check(an, bn, an[id].left) || check(an, bn, an[id].right);
    leave(`${found}`);
    found
}

fn trace(input) {
    let an = tree_nodes(input.root);
    let bn = tree_nodes(input.subRoot);
    let found = check(an, bn, if an.len() > 0 { 0 } else { () });
    result(`${found}`);
}
