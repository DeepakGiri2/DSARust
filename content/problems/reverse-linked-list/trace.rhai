// Reverse Linked List — three pointers, one arrow at a time.
//
// The animation exists to answer the question everyone asks here: why do we
// need `next`? Because the moment `curr.next` is re-pointed at `prev`, the
// rest of the list is unreachable unless it was saved first.

fn nodes_from(values) {
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

fn trace(input) {
    let values = input.list;
    let n = values.len();
    let nodes = nodes_from(values);
    let flipped = [];

    let prev = ();
    let curr = if n > 0 { 0 } else { () };

    enter("reverseList", #{ head: PTR(if n > 0 { `node(${values[0]})` } else { "nil" }) });
    set(#{ prev: NIL() });
    step("init", "prev starts at nil — after the reversal the old head must point at nothing.",
        [ list("list", nodes).pointers(#{ prev: prev, curr: curr }) ]);

    set(#{ curr: PTR(if n > 0 { `node(${values[0]})` } else { "nil" }) });
    step("init2", "curr starts at the head. It walks the list once, flipping one arrow per stop.",
        [ list("list", nodes).pointers(#{ prev: prev, curr: curr }) ]);

    while type_of(curr) != "()" {
        step("loop", `curr is at node(${nodes[curr].val}), so there is still an arrow to flip.`,
            [ list("list", nodes).pointers(#{ prev: prev, curr: curr }).reversed(flipped) ]);

        let next = nodes[curr].next;
        set(#{ next: PTR(if type_of(next) == "()" { "nil" } else { `node(${nodes[next].val})` }) });
        log(`save next = ${if type_of(next) == "()" { "nil" } else { `node(${nodes[next].val})` }}`);
        step("save",
            "Save the next node first. The very next line destroys this link, and without the copy the rest of the list would be lost.",
            [ list("list", nodes).pointers(#{ prev: prev, curr: curr, next: next }).reversed(flipped) ]);

        nodes[curr].next = prev;
        flipped.push(curr);
        log(`node(${nodes[curr].val}).next → ${if type_of(prev) == "()" { "nil" } else { `node(${nodes[prev].val})` }}`);
        step("flip", `Point node(${nodes[curr].val}) backwards. That one assignment is the entire reversal.`,
            [ list("list", nodes).pointers(#{ prev: prev, curr: curr, next: next }).reversed(flipped) ]);

        prev = curr;
        set(#{ prev: PTR(`node(${nodes[prev].val})`) });
        step("advP", `prev moves up to node(${nodes[prev].val}) — the head of the reversed part so far.`,
            [ list("list", nodes).pointers(#{ prev: prev, curr: curr, next: next }).reversed(flipped) ]);

        curr = next;
        set(#{ curr: PTR(if type_of(curr) == "()" { "nil" } else { `node(${nodes[curr].val})` }) });
        step("advC",
            if type_of(curr) == "()" {
                "curr steps off the end of the list — every arrow has been flipped."
            } else {
                `curr steps to node(${nodes[curr].val}).`
            },
            [ list("list", nodes).pointers(#{ prev: prev, curr: curr }).reversed(flipped) ]);
    }

    let head = if type_of(prev) == "()" { "nil" } else { `node(${nodes[prev].val})` };
    log(`return ${head}`, "result");
    result(head);
    step("ret", `curr is nil, so prev is sitting on the last node — that is the new head. Return ${head}.`,
        [ list("list", nodes).pointers(#{ prev: prev }).reversed(flipped) ], "return");
    leave(head);
}
