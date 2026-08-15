// Build a chain of #{id, val, next} nodes from plain values.
fn chain(values) {
    let out = [];
    for i in 0..values.len() {
        out.push(#{
            id: i,
            val: values[i],
            next: if i + 1 < values.len() { i + 1 } else { () },
        });
    }
    out
}

// Same chain, but with a sentinel node in front so that removing the real head
// needs no special case. The sentinel is id 0 and the first real value is id 1.
fn chain_with_dummy(values) {
    let out = [#{ id: 0, val: "dummy", next: if values.len() > 0 { 1 } else { () } }];
    for i in 0..values.len() {
        out.push(#{
            id: i + 1,
            val: values[i],
            next: if i + 2 <= values.len() { i + 2 } else { () },
        });
    }
    out
}

fn order_of(nodes, head) {
    let out = [];
    let cur = head;
    let guard = 0;
    while type_of(cur) != "()" && guard < 200 {
        out.push(nodes[cur].val);
        cur = nodes[cur].next;
        guard += 1;
    }
    out
}

fn val_or_nil(nodes, id) {
    if type_of(id) == "()" { "nil" } else { `${nodes[id].val}` }
}

// Reverse Nodes in K-Group — look before you leap, then reverse to a boundary.
//
// Two ideas carry this. First, the k-th node is located *before* anything is
// touched: a short final group must be left alone, and a half-finished
// reversal is expensive to take back. Second, the inner loop stops at the node
// *after* the group rather than at nil, and seeds `prev` with that same node —
// so the group comes out reversed and already attached to the tail, with no
// separate stitching step.

fn kth_node(nodes, cur, k) {
    enter(`kthNode(${k})`, #{ k: N(k) });
    let left = k;
    while type_of(cur) != "()" && left > 0 {
        step("walk", `${left} more step(s) to go; at node(${nodes[cur].val}).`,
            [ list("list", nodes).pointers(#{ cur: cur }) ]);
        cur = nodes[cur].next;
        left -= 1;
    }
    step("walk",
        if type_of(cur) == "()" {
            `The list ran out with ${left} step(s) still to take.`
        } else {
            `Landed on node(${nodes[cur].val}) — the k-th node of this group.`
        },
        [ list("list", nodes).pointers(#{ kth: cur }) ]);
    step("retKth", if type_of(cur) == "()" { "Return nil.".to_string() } else { `Return node(${nodes[cur].val}).` },
        [ list("list", nodes).pointers(#{ kth: cur }) ], "return");
    leave(val_or_nil(nodes, cur));
    cur
}

fn trace(input) {
    let values = input.list;
    let k = input.k;
    let nodes = chain_with_dummy(values);
    let head = if values.len() > 0 { 1 } else { () };
    let flipped = [];

    enter("reverseKGroup", #{ k: N(k) });
    step("dummy", "A sentinel, so the first group's predecessor exists like every other group's.",
        [ list("list", nodes).pointers(#{ dummy: 0, head: head }) ]);

    let group_prev = 0;
    step("groupPrev", "groupPrev sits just before the group about to be reversed.",
        [ list("list", nodes).pointers(#{ groupPrev: group_prev }) ]);

    loop {
        step("loop", "Try to take another full group.",
            [ list("list", nodes).pointers(#{ groupPrev: group_prev }).reversed(flipped) ]);

        step("findKth", `Walk ${k} node(s) forward from groupPrev — before touching anything, because a partial group must be left alone.`,
            [ list("list", nodes).pointers(#{ groupPrev: group_prev }).reversed(flipped) ], "call");
        let kth = kth_node(nodes, group_prev, k);

        if type_of(kth) == "()" {
            step("short", `Fewer than ${k} nodes remain, so this tail keeps its original order. Checking first is what makes that possible.`,
                [ list("list", nodes).pointers(#{ groupPrev: group_prev }).reversed(flipped) ]);
            break;
        }

        let group_next = nodes[kth].next;
        step("groupNext", `The group ends at node(${nodes[kth].val}); everything from ${val_or_nil(nodes, group_next)} onwards is outside it.`,
            [ list("list", nodes).pointers(#{ groupPrev: group_prev, kth: kth, groupNext: group_next }).reversed(flipped) ]);

        let prev = group_next;
        let cur = nodes[group_prev].next;
        step("revInit", `Seed prev with ${val_or_nil(nodes, group_next)} rather than nil. That single choice is what leaves the reversed group already joined to the tail.`,
            [ list("list", nodes).pointers(#{ prev: prev, cur: cur }).reversed(flipped) ]);

        while cur != group_next {
            step("revLoop", `Reversing inside the group: at node(${nodes[cur].val}).`,
                [ list("list", nodes).pointers(#{ prev: prev, cur: cur }).reversed(flipped) ]);
            let next = nodes[cur].next;
            step("save", `Remember ${val_or_nil(nodes, next)} — the next link is about to be overwritten.`,
                [ list("list", nodes).pointers(#{ cur: cur, next: next }).reversed(flipped) ]);
            nodes[cur].next = prev;
            flipped.push(cur);
            step("flip", `node(${nodes[cur].val}) now points at ${val_or_nil(nodes, prev)}.`,
                [ list("list", nodes).pointers(#{ prev: prev, cur: cur }).reversed(flipped) ]);
            prev = cur;
            step("stepPrev", `prev = node(${nodes[prev].val}).`,
                [ list("list", nodes).pointers(#{ prev: prev }).reversed(flipped) ]);
            cur = next;
            step("stepCur", `cur = ${val_or_nil(nodes, cur)}.`,
                [ list("list", nodes).pointers(#{ prev: prev, cur: cur }).reversed(flipped) ]);
        }

        let tmp = nodes[group_prev].next;
        step("tmp", `The old first node, node(${nodes[tmp].val}), is now the group's *last* node — which makes it the predecessor of the next group.`,
            [ list("list", nodes).pointers(#{ groupPrev: group_prev, tmp: tmp }).reversed(flipped) ]);

        nodes[group_prev].next = kth;
        log(`group reversed, now starting at ${nodes[kth].val}`);
        step("relink", `Point groupPrev at node(${nodes[kth].val}), the group's new front.`,
            [ list("list", nodes).pointers(#{ groupPrev: group_prev, head: nodes[0].next }).reversed(flipped) ]);

        group_prev = tmp;
        step("advance", `Move groupPrev to node(${nodes[group_prev].val}) and look for another group.`,
            [ list("list", nodes).pointers(#{ groupPrev: group_prev }).reversed(flipped) ]);
    }

    let new_head = nodes[0].next;
    let answer = join(order_of(nodes, new_head), " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `Result: ${answer}.`,
        [ list("list", nodes).pointers(#{ head: new_head }).reversed(flipped) ], "return");
    leave(answer);
}
