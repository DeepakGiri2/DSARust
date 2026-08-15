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

// Remove Linked List Elements — the sentinel exists to kill a special case.
//
// A node is deleted by rewriting the link that points *at* it, so the head is
// awkward: nothing points at it. Putting a sentinel in front means every real
// node has a predecessor, the loop body never has to ask "is this the head?",
// and the new head falls out of the sentinel's link at the end — even if every
// node was removed.

fn trace(input) {
    let values = input.list;
    let target = input.val;
    let nodes = chain_with_dummy(values);
    let head = if values.len() > 0 { 1 } else { () };
    let removed = [];

    enter("removeElements", #{ val: N(target) });
    step("dummy", "A sentinel in front of the head. It holds no value; it exists only so the real head has something pointing at it.",
        [ list("list", nodes).pointers(#{ dummy: 0, head: head }) ]);

    let prev = 0;
    set(#{ prev: PTR("dummy") });
    step("prev", "prev always trails one behind, holding the link that would have to change.",
        [ list("list", nodes).pointers(#{ prev: prev }) ]);

    let cur = head;
    while type_of(cur) != "()" {
        step("loop", `At node(${nodes[cur].val}); prev is at ${if prev == 0 { "the sentinel" } else { `node(${nodes[prev].val})` }}.`,
            [ list("list", nodes).pointers(#{ prev: prev, cur: cur }).reversed(removed) ]);

        if nodes[cur].val == target {
            step("match", `node(${nodes[cur].val}) matches ${target}, so it has to go.`,
                [ list("list", nodes).pointers(#{ prev: prev, cur: cur }).reversed(removed) ]);
            nodes[prev].next = nodes[cur].next;
            removed.push(cur);
            log(`remove node(${nodes[cur].val})`);
            step("unlink", `Point prev past it. Notice that prev does *not* move — the next node now sits directly after it.`,
                [ list("list", nodes).pointers(#{ prev: prev }).reversed(removed) ]);
        } else {
            step("keep", `node(${nodes[cur].val}) is not ${target}, so it stays.`,
                [ list("list", nodes).pointers(#{ prev: prev, cur: cur }).reversed(removed) ]);
            prev = cur;
            set(#{ prev: PTR(`node(${nodes[prev].val})`) });
            step("advance", `prev catches up to node(${nodes[prev].val}).`,
                [ list("list", nodes).pointers(#{ prev: prev }).reversed(removed) ]);
        }

        cur = nodes[cur].next;
    }

    let new_head = nodes[0].next;
    let answer = join(order_of(nodes, new_head), " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `Return whatever the sentinel points at — ${if type_of(new_head) == "()" { "nothing, because every node matched" } else { `node(${nodes[new_head].val})` }}. That works even when the original head was removed.`,
        [ list("list", nodes).pointers(#{ dummy: 0, head: new_head }).reversed(removed) ], "return");
    leave(answer);
}
