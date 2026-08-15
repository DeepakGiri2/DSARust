// Reverse Linked List II — head insertion, not a second reversal pass.
//
// Notice that curr never moves. It starts at the first node of the sublist and
// stays there — which is correct, because after the reversal that node is the
// *last* one. Everything else is lifted out from behind it and dropped in at
// the front, one node per iteration. Three writes, in an order where no link
// is destroyed before it has been read.

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

const L = "list";

fn trace(input) {
    let values = input.list;
    let left = input.left;
    let right = input.right;

    // id 0 is the sentinel; real values are ids 1 … n.
    let nodes = [#{ id: 0, val: "dummy", next: if values.len() > 0 { 1 } else { () } }];
    for i in 0..values.len() {
        nodes.push(#{ id: i + 1, val: values[i],
                      next: if i + 2 <= values.len() { i + 2 } else { () } });
    }

    enter("reverseBetween", #{ left: N(left), right: N(right) });
    step("dummy", "A sentinel in front of the head, so reversing from position 1 needs no special case — there is always a node before the sublist.",
        [ list(global::L, nodes).pointers(#{ dummy: 0 }) ]);

    let prev = 0;
    set(#{ prev: PTR("dummy") });
    step("prev", `prev will sit just before position ${left} and never move again.`,
        [ list(global::L, nodes).pointers(#{ prev: prev }) ]);

    for i in 0..(left - 1) {
        step("walk", `Walking to position ${left}: step ${i + 1} of ${left - 1}.`,
            [ list(global::L, nodes).pointers(#{ prev: prev }) ]);
        prev = nodes[prev].next;
        set(#{ prev: PTR(val_or_nil(nodes, prev)) });
        step("hop", `prev is now at ${val_or_nil(nodes, prev)}.`,
            [ list(global::L, nodes).pointers(#{ prev: prev }) ]);
    }

    let curr = nodes[prev].next;
    set(#{ curr: PTR(val_or_nil(nodes, curr)) });
    step("curr", `curr is the first node of the sublist, ${val_or_nil(nodes, curr)}. It stays here for the whole reversal — it ends up last.`,
        [ list(global::L, nodes).pointers(#{ prev: prev, curr: curr }) ]);

    for i in 0..(right - left) {
        step("loop", `Move ${i + 1} of ${right - left}.`,
            [ list(global::L, nodes).pointers(#{ prev: prev, curr: curr }) ]);

        let next = nodes[curr].next;
        set(#{ next: PTR(val_or_nil(nodes, next)) });
        step("save", `Save ${val_or_nil(nodes, next)} — the node about to be lifted out.`,
            [ list(global::L, nodes).pointers(#{ prev: prev, curr: curr, next: next }) ]);

        nodes[curr].next = nodes[next].next;
        step("unlink", `Splice it out: curr now points past it, to ${val_or_nil(nodes, nodes[curr].next)}.`,
            [ list(global::L, nodes).pointers(#{ prev: prev, curr: curr, next: next }) ]);

        nodes[next].next = nodes[prev].next;
        step("front", `Point it at the current front of the sublist.`,
            [ list(global::L, nodes).pointers(#{ prev: prev, curr: curr, next: next }) ]);

        nodes[prev].next = next;
        step("attach", `And hang it off prev — ${val_or_nil(nodes, next)} is now the first node of the sublist.`,
            [ list(global::L, nodes).pointers(#{ prev: prev, curr: curr }) ]);
    }

    let out = join(order_of(nodes, nodes[0].next), " ");
    log(out, "result");
    result(out);
    step("ret", `Positions ${left} … ${right} are reversed, in one pass: ${out}.`,
        [ list(global::L, nodes).pointers(#{ head: nodes[0].next }) ], "return");
    leave(out);
}
