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

// Remove Duplicates From Sorted List — sortedness makes duplicates neighbours.
//
// Nothing has to be remembered. If the list is sorted, two equal values can
// only ever be adjacent, so comparing a node with the one after it is a
// complete test. The one thing to get right is *not* advancing after a
// deletion: three copies in a row would leave one behind if the pointer moved.

fn trace(input) {
    let values = input.list;
    let nodes = chain(values);
    let head = if values.len() > 0 { 0 } else { () };
    let dropped = [];

    enter("deleteDuplicates", #{});
    let cur = head;
    set(#{ cur: PTR(`node(${val_or_nil(nodes, cur)})`) });
    step("init", "Walk the list with a single pointer.",
        [ list("list", nodes).pointers(#{ cur: cur }) ]);

    while type_of(cur) != "()" && type_of(nodes[cur].next) != "()" {
        let nxt = nodes[cur].next;
        step("loop", `node(${nodes[cur].val}) has a neighbour, node(${nodes[nxt].val}).`,
            [ list("list", nodes).pointers(#{ cur: cur, next: nxt }).reversed(dropped) ]);

        if nodes[cur].val == nodes[nxt].val {
            step("same", `Both are ${nodes[cur].val}. Because the list is sorted, adjacency is the *only* way a duplicate can appear.`,
                [ list("list", nodes).pointers(#{ cur: cur, next: nxt }).reversed(dropped) ]);
            nodes[cur].next = nodes[nxt].next;
            dropped.push(nxt);
            log(`drop a duplicate ${nodes[nxt].val}`);
            step("skip", `Link past it — and stay here, because the next node might be another ${nodes[cur].val}.`,
                [ list("list", nodes).pointers(#{ cur: cur }).reversed(dropped) ]);
        } else {
            step("diff", `${nodes[cur].val} and ${nodes[nxt].val} differ, so this value is finished.`,
                [ list("list", nodes).pointers(#{ cur: cur, next: nxt }).reversed(dropped) ]);
            cur = nxt;
            set(#{ cur: PTR(`node(${nodes[cur].val})`) });
            step("advance", `Move on to node(${nodes[cur].val}).`,
                [ list("list", nodes).pointers(#{ cur: cur }).reversed(dropped) ]);
        }
    }

    let answer = join(order_of(nodes, head), " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `Each value now appears once: ${answer}.`,
        [ list("list", nodes).pointers(#{ head: head }).reversed(dropped) ], "return");
    leave(answer);
}
