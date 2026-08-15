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

// Swap Nodes in Pairs — three links, rewritten in a safe order.
//
// A swap is not symmetric work: the node before the pair also has to be
// redirected, and for the very first pair that node is the head itself. A
// sentinel supplies it. Then the three rewrites must run in an order where
// each one still has the pointer it needs — reach past the pair first, close
// the pair second, and only then redirect the outside link.

fn trace(input) {
    let values = input.list;
    let nodes = chain_with_dummy(values);
    let head = if values.len() > 0 { 1 } else { () };
    let swapped = [];

    enter("swapPairs", #{});
    step("dummy", "A sentinel in front, so the first pair has an outside link to redirect just like every other pair.",
        [ list("list", nodes).pointers(#{ dummy: 0, head: head }) ]);

    let prev = 0;
    step("prev", "prev always sits immediately before the pair being swapped.",
        [ list("list", nodes).pointers(#{ prev: prev }) ]);

    while type_of(nodes[prev].next) != "()" && type_of(nodes[nodes[prev].next].next) != "()" {
        let first = nodes[prev].next;
        let second = nodes[first].next;
        step("loop", `There are at least two nodes left: node(${nodes[first].val}) and node(${nodes[second].val}).`,
            [ list("list", nodes).pointers(#{ prev: prev, first: first, second: second }).reversed(swapped) ]);

        step("first", `first = node(${nodes[first].val}).`,
            [ list("list", nodes).pointers(#{ prev: prev, first: first }).reversed(swapped) ]);
        step("second", `second = node(${nodes[second].val}). After the swap these two change places.`,
            [ list("list", nodes).pointers(#{ prev: prev, first: first, second: second }).reversed(swapped) ]);

        nodes[first].next = nodes[second].next;
        step("relinkFirst", `First, reach past the pair: node(${nodes[first].val}) points at ${val_or_nil(nodes, nodes[first].next)}. This must happen before second's link is overwritten, because that is where the rest of the list is recorded.`,
            [ list("list", nodes).pointers(#{ prev: prev, first: first, second: second }).reversed(swapped) ]);

        nodes[second].next = first;
        step("relinkSecond", `Now close the pair: node(${nodes[second].val}) points at node(${nodes[first].val}).`,
            [ list("list", nodes).pointers(#{ prev: prev, first: first, second: second }).reversed(swapped) ]);

        nodes[prev].next = second;
        swapped.push(first);
        swapped.push(second);
        log(`swap ${nodes[first].val} and ${nodes[second].val}`);
        step("relinkPrev", `Finally redirect the outside link, so what came before now reaches node(${nodes[second].val}) first.`,
            [ list("list", nodes).pointers(#{ prev: prev, head: nodes[0].next }).reversed(swapped) ]);

        prev = first;
        step("advance", `prev moves to node(${nodes[prev].val}), which is now the second of the swapped pair and therefore sits just before the next pair.`,
            [ list("list", nodes).pointers(#{ prev: prev }).reversed(swapped) ]);
    }

    let new_head = nodes[0].next;
    let answer = join(order_of(nodes, new_head), " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `Fewer than two nodes remain, so nothing else can be paired. Result: ${answer}.`,
        [ list("list", nodes).pointers(#{ head: new_head }).reversed(swapped) ], "return");
    leave(answer);
}
