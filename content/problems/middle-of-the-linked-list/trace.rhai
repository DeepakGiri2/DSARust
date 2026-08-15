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

// Middle of the Linked List — one pass, because one pointer moves twice.
//
// The length is unknown until the walk is over, so counting first would mean
// walking twice. Instead let a second pointer move at double speed: when it
// falls off the end it has covered the whole list, so the slow one has covered
// half of it. The even-length case is settled entirely by the loop condition —
// stopping on `fast.next == nil` lands on the *second* middle node.

fn trace(input) {
    let values = input.list;
    let n = values.len();
    let nodes = chain(values);
    let head = if n > 0 { 0 } else { () };

    enter("middleNode", #{ n: N(n) });

    let slow = head;
    let fast = head;
    set(#{ slow: PTR(`node(${val_or_nil(nodes, slow)})`), fast: PTR(`node(${val_or_nil(nodes, fast)})`) });
    step("init", "Both pointers start at the head.",
        [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);

    while type_of(fast) != "()" && type_of(nodes[fast].next) != "()" {
        step("loop", `fast is at node(${nodes[fast].val}) and has a node after it, so there is room for another double step.`,
            [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);

        slow = nodes[slow].next;
        set(#{ slow: PTR(`node(${nodes[slow].val})`) });
        step("slow", `slow moves one, to node(${nodes[slow].val}).`,
            [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);

        fast = nodes[nodes[fast].next].next;
        set(#{ fast: PTR(`node(${val_or_nil(nodes, fast)})`) });
        step("fast", `fast moves two, to ${if type_of(fast) == "()" { "the end" } else { `node(${nodes[fast].val})` }} — always exactly twice as far from the head.`,
            [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);
    }

    step("loop", `fast can go no further, so it has covered the whole list — and slow, at half the speed, has covered half of it.`,
        [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);

    let answer = join(order_of(nodes, slow), " ");
    log(`middle is node(${nodes[slow].val})`, "result");
    result(answer);
    step("ret", `The middle is node(${nodes[slow].val}); the tail from there is ${answer}.`,
        [ list("list", nodes).pointers(#{ middle: slow }) ], "return");
    leave(answer);
}
