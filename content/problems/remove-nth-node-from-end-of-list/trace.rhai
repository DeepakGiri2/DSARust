// Remove Nth Node From End — two pointers a fixed gap apart.
//
// A singly linked list cannot be walked backwards, so "nth from the end" is
// turned into "nth from where the leader stops": open a gap of n between two
// pointers, then move both until the leader falls off. The trailer is then
// sitting exactly one before the node to unlink.

fn trace(input) {
    let values = input.list;
    let n = input.n;
    let nodes = [#{ id: 0, val: "dummy", next: if values.len() > 0 { 1 } else { () } }];
    for i in 0..values.len() {
        nodes.push(#{
            id: i + 1,
            val: values[i],
            next: if i + 1 < values.len() { i + 2 } else { () },
        });
    }

    enter("removeNthFromEnd", #{ n: N(n) });
    step("dummy", "A dummy in front means removing the real head needs no special case.",
        [ list("list", nodes).pointers(#{ dummy: 0 }) ]);
    step("link", "It points at the head.", [ list("list", nodes).pointers(#{ dummy: 0 }) ]);

    let slow = 0;
    let fast = 0;
    set(#{ slow: PTR("dummy"), fast: PTR("dummy") });
    step("slow0", "Both pointers start at the dummy.",
        [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);
    step("fast0", "The leader will run ahead first.",
        [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);

    for i in 0..n {
        fast = nodes[fast].next;
        set(#{ fast: PTR(`node(${nodes[fast].val})`) });
        step("gap", `Advance the leader ${i + 1} of ${n}.`,
            [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);
    }
    step("gapAdv", `The leader is now ${n} node(s) ahead. That gap never changes again.`,
        [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);

    while type_of(nodes[fast].next) != "()" {
        step("loop", "The leader has not reached the last node yet.",
            [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);
        slow = nodes[slow].next;
        set(#{ slow: PTR(`node(${nodes[slow].val})`) });
        step("advS", `Trailer moves to node(${nodes[slow].val}).`,
            [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);
        fast = nodes[fast].next;
        set(#{ fast: PTR(`node(${nodes[fast].val})`) });
        step("advF", `Leader moves to node(${nodes[fast].val}).`,
            [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);
    }

    let doomed = nodes[slow].next;
    log(`unlink node(${nodes[doomed].val})`, "result");
    nodes[slow].next = nodes[doomed].next;
    step("unlink",
        `The leader is on the last node, so the trailer is exactly one before the ${n}th from the end. Skip node(${nodes[doomed].val}).`,
        [ list("list", nodes).pointers(#{ slow: slow }).reversed([slow]) ]);

    let order = [];
    let cur = nodes[0].next;
    while type_of(cur) != "()" {
        order.push(nodes[cur].val);
        cur = nodes[cur].next;
    }
    let answer = join(order, " ");
    result(answer);
    step("ret", `Return the dummy's next: ${answer}.`,
        [ list("list", nodes).pointers(#{ head: nodes[0].next }) ], "return");
    leave(answer);
}
