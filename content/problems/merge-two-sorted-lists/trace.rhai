// Merge Two Sorted Lists — a dummy head removes every special case.
//
// Without the dummy node, the first append needs its own branch ("is the
// result empty yet?"). With it, every append is identical, and the answer is
// simply whatever the dummy ends up pointing at.

fn build(values, base, label_prefix) {
    let out = [];
    for i in 0..values.len() {
        out.push(#{
            id: base + i,
            val: values[i],
            next: if i + 1 < values.len() { base + i + 1 } else { () },
        });
    }
    out
}

fn trace(input) {
    let a = input.list1;
    let b = input.list2;
    // Node 0 is the dummy; list1 ids start at 1, list2 ids after that.
    let nodes = [#{ id: 0, val: "dummy", next: () }];
    for nd in build(a, 1, "a") { nodes.push(nd); }
    for nd in build(b, 1 + a.len(), "b") { nodes.push(nd); }

    let l1 = if a.len() > 0 { 1 } else { () };
    let l2 = if b.len() > 0 { 1 + a.len() } else { () };
    let tail = 0;

    enter("mergeTwoLists", #{});
    step("dummy", "A dummy node in front of the answer, so appending never needs a special first case.",
        [ list("merged", nodes).pointers(#{ dummy: 0 }) ]);
    set(#{ tail: PTR("dummy") });
    step("tail0", "tail is where the next node gets attached.",
        [ list("merged", nodes).pointers(#{ tail: tail, l1: l1, l2: l2 }) ]);

    while type_of(l1) != "()" && type_of(l2) != "()" {
        step("loop", "Both lists still have nodes.",
            [ list("merged", nodes).pointers(#{ tail: tail, l1: l1, l2: l2 }) ]);

        if nodes[l1].val <= nodes[l2].val {
            step("cmp", `${nodes[l1].val} <= ${nodes[l2].val}, so list1's node goes next.`,
                [ list("merged", nodes).pointers(#{ tail: tail, l1: l1, l2: l2 }).reversed([l1]) ]);
            nodes[tail].next = l1;
            log(`append ${nodes[l1].val}`);
            step("take1", `Attach node(${nodes[l1].val}).`,
                [ list("merged", nodes).pointers(#{ tail: tail, l1: l1, l2: l2 }) ]);
            l1 = nodes[l1].next;
            step("adv1", "Advance list1.",
                [ list("merged", nodes).pointers(#{ tail: tail, l1: l1, l2: l2 }) ]);
        } else {
            step("cmp", `${nodes[l2].val} < ${nodes[l1].val}, so list2's node goes next.`,
                [ list("merged", nodes).pointers(#{ tail: tail, l1: l1, l2: l2 }).reversed([l2]) ]);
            nodes[tail].next = l2;
            log(`append ${nodes[l2].val}`);
            step("take2", `Attach node(${nodes[l2].val}).`,
                [ list("merged", nodes).pointers(#{ tail: tail, l1: l1, l2: l2 }) ]);
            l2 = nodes[l2].next;
            step("adv2", "Advance list2.",
                [ list("merged", nodes).pointers(#{ tail: tail, l1: l1, l2: l2 }) ]);
        }

        tail = nodes[tail].next;
        set(#{ tail: PTR(`node(${nodes[tail].val})`) });
        step("advtail", `tail is now node(${nodes[tail].val}).`,
            [ list("merged", nodes).pointers(#{ tail: tail, l1: l1, l2: l2 }) ]);
    }

    step("rest", "One list is empty. Whatever remains in the other is already sorted and already linked.",
        [ list("merged", nodes).pointers(#{ tail: tail, l1: l1, l2: l2 }) ]);

    if type_of(l1) != "()" {
        nodes[tail].next = l1;
        step("attach1", `Attach the rest of list1 in one move — no copying needed.`,
            [ list("merged", nodes).pointers(#{ tail: tail }) ]);
    } else {
        nodes[tail].next = l2;
        step("attach2", `Attach the rest of list2 in one move.`,
            [ list("merged", nodes).pointers(#{ tail: tail }) ]);
    }

    // Read the merged order off the chain for the result banner.
    let order = [];
    let cur = nodes[0].next;
    while type_of(cur) != "()" {
        order.push(nodes[cur].val);
        cur = nodes[cur].next;
    }
    let answer = join(order, " ");
    log(`merged ${answer}`, "result");
    result(answer);
    step("ret", `The answer is whatever the dummy points at: ${answer}.`,
        [ list("merged", nodes).pointers(#{ head: nodes[0].next }) ], "return");
    leave(answer);
}
