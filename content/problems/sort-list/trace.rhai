// Sort List — merge sort, because a linked list has no random access.
//
// Two details worth watching. The split starts fast one node *ahead* of slow;
// start them together and a two-element list splits into two and zero, and the
// recursion never terminates. And the merge allocates one sentinel node and
// then only rewires — no values are copied, no array is built.

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

fn chain_str(nodes, head) {
    let vals = order_of(nodes, head);
    if vals.is_empty() { "nil" } else { join(vals, "→") }
}

fn merge_lists(nodes, a, b) {
    enter(`merge(${chain_str(nodes, a)}, ${chain_str(nodes, b)})`);
    step("dummy", "A sentinel node to hang the result off, so the first append needs no special case.",
        [ list(global::L, nodes).pointers(#{ a: a, b: b }) ]);

    let head = ();
    let tail = ();

    while type_of(a) != "()" && type_of(b) != "()" {
        step("both", `Compare ${nodes[a].val} and ${nodes[b].val} — the two smallest not yet taken.`,
            [ list(global::L, nodes).pointers(#{ a: a, b: b, tail: tail }) ]);

        if nodes[a].val <= nodes[b].val {
            step("pick", `${nodes[a].val} ≤ ${nodes[b].val}, so the left list wins. (≤, not <, is what makes the sort stable.)`,
                [ list(global::L, nodes).pointers(#{ a: a, b: b }) ]);
            let taken = a;
            a = nodes[a].next;
            if type_of(tail) == "()" { head = taken; } else { nodes[tail].next = taken; }
            nodes[taken].next = ();
            tail = taken;
            step("takea", `Append ${nodes[taken].val}. Nothing is copied — the node itself is relinked.`,
                [ list(global::L, nodes).pointers(#{ a: a, b: b, tail: tail }) ]);
        } else {
            step("pick", `${nodes[b].val} < ${nodes[a].val}, so the right list wins.`,
                [ list(global::L, nodes).pointers(#{ a: a, b: b }) ]);
            let taken = b;
            b = nodes[b].next;
            if type_of(tail) == "()" { head = taken; } else { nodes[tail].next = taken; }
            nodes[taken].next = ();
            tail = taken;
            step("takeb", `Append ${nodes[taken].val}.`,
                [ list(global::L, nodes).pointers(#{ a: a, b: b, tail: tail }) ]);
        }

        step("advtail", `Result so far: ${chain_str(nodes, head)}.`,
            [ list(global::L, nodes).pointers(#{ tail: tail }) ]);
    }

    if type_of(a) != "()" {
        step("rest", "The left list still has nodes.", [ list(global::L, nodes).pointers(#{ a: a }) ]);
        if type_of(tail) == "()" { head = a; } else { nodes[tail].next = a; }
        step("resta", `Attach the whole remaining tail ${chain_str(nodes, a)} in one write — it is already sorted.`,
            [ list(global::L, nodes).pointers(#{ head: head }) ]);
    } else {
        step("rest", "The left list is exhausted.", [ list(global::L, nodes).pointers(#{ b: b }) ]);
        if type_of(tail) == "()" { head = b; } else { nodes[tail].next = b; }
        step("restb", `Attach the remaining right tail ${chain_str(nodes, b)} in one write.`,
            [ list(global::L, nodes).pointers(#{ head: head }) ]);
    }

    step("done", `Merged: ${chain_str(nodes, head)}.`,
        [ list(global::L, nodes).pointers(#{ head: head }) ], "return");
    leave(chain_str(nodes, head));
    [nodes, head]
}

fn sort_chain(nodes, head) {
    enter(`sortList(${chain_str(nodes, head)})`);

    if type_of(head) == "()" || type_of(nodes[head].next) == "()" {
        step("base", `${chain_str(nodes, head)} has at most one node.`,
            [ list(global::L, nodes).pointers(#{ head: head }) ]);
        step("trivial", "A list of one is already sorted — return it unchanged.",
            [ list(global::L, nodes).pointers(#{ head: head }) ], "return");
        leave(chain_str(nodes, head));
        return [nodes, head];
    }

    step("base", `${chain_str(nodes, head)} has more than one node, so it must be split.`,
        [ list(global::L, nodes).pointers(#{ head: head }) ]);

    let slow = head;
    let fast = nodes[head].next;
    step("find", "slow and fast start one apart — fast one node ahead. Start them together and a two-node list splits into two and nothing.",
        [ list(global::L, nodes).pointers(#{ slow: slow, fast: fast }) ]);

    while type_of(fast) != "()" && type_of(nodes[fast].next) != "()" {
        step("advance", "fast can still take two steps, so the middle has not been reached.",
            [ list(global::L, nodes).pointers(#{ slow: slow, fast: fast }) ]);
        slow = nodes[slow].next;
        fast = nodes[nodes[fast].next].next;
        step("hop", `slow is at ${val_or_nil(nodes, slow)}, fast at ${val_or_nil(nodes, fast)} — one moves half as fast, so slow lands on the middle.`,
            [ list(global::L, nodes).pointers(#{ slow: slow, fast: fast }) ]);
    }

    let mid = nodes[slow].next;
    step("mid", `The second half starts at ${val_or_nil(nodes, mid)}.`,
        [ list(global::L, nodes).pointers(#{ slow: slow, mid: mid }) ]);

    nodes[slow].next = ();
    step("cut", "Cut the link. Two independent lists now — a linked list can be split in O(1), which an array cannot.",
        [ list(global::L, nodes).pointers(#{ head: head, mid: mid }) ]);

    step("left", `Sort the left half, ${chain_str(nodes, head)}.`,
        [ list(global::L, nodes).pointers(#{ head: head }) ], "call");
    let lr = sort_chain(nodes, head);
    nodes = lr[0];
    let lh = lr[1];

    step("right", `Sort the right half, ${chain_str(nodes, mid)}.`,
        [ list(global::L, nodes).pointers(#{ mid: mid }) ], "call");
    let rr = sort_chain(nodes, mid);
    nodes = rr[0];
    let rh = rr[1];

    step("merge", `Merge ${chain_str(nodes, lh)} with ${chain_str(nodes, rh)}.`,
        [ list(global::L, nodes).pointers(#{ left: lh, right: rh }) ], "call");
    let m = merge_lists(nodes, lh, rh);
    nodes = m[0];

    leave(chain_str(nodes, m[1]));
    [nodes, m[1]]
}

fn trace(input) {
    let nodes = chain(input.list);
    let head = if input.list.len() > 0 { 0 } else { () };

    let r = sort_chain(nodes, head);
    let out = join(order_of(r[0], r[1]), " ");
    log(out, "result");
    result(out);
}
