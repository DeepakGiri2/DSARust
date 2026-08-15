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

// Palindrome Linked List — make the back half readable backwards.
//
// The obstacle is one-directional links: a palindrome check wants to read from
// both ends, and only one end is reachable. Reversing the second half turns its
// tail into a head, so two forward walks meet in the middle. The comparison
// loop stops when the *reversed* side runs out, which is what lets an odd
// middle node be ignored without a special case.

fn trace(input) {
    let values = input.list;
    let n = values.len();
    let nodes = chain(values);
    let head = if n > 0 { 0 } else { () };

    enter("isPalindrome", #{ n: N(n) });

    let slow = head;
    let fast = head;
    step("mid", "First find the middle, so the back half can be reversed on its own.",
        [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);

    while type_of(fast) != "()" && type_of(nodes[fast].next) != "()" {
        slow = nodes[slow].next;
        fast = nodes[nodes[fast].next].next;
        step("midLoop", `slow at node(${nodes[slow].val}), fast twice as far.`,
            [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);
    }

    let prev = ();
    let flipped = [];
    step("revInit", `The back half starts at node(${nodes[slow].val}). Reverse it in place — no extra memory, and the original front half is untouched.`,
        [ list("list", nodes).pointers(#{ slow: slow }) ]);

    while type_of(slow) != "()" {
        let next = nodes[slow].next;
        step("revLoop", `Turning node(${nodes[slow].val}) around.`,
            [ list("list", nodes).pointers(#{ slow: slow, prev: prev }).reversed(flipped) ]);
        nodes[slow].next = prev;
        flipped.push(slow);
        step("flip", `node(${nodes[slow].val}) now points backwards.`,
            [ list("list", nodes).pointers(#{ slow: slow, prev: prev }).reversed(flipped) ]);
        prev = slow;
        slow = next;
    }

    let left = head;
    let right = prev;
    let ok = true;
    step("cmpInit", `Two forward walks now: one from the head, one from node(${nodes[right].val}) — which is the original tail.`,
        [ list("list", nodes).pointers(#{ left: left, right: right }).reversed(flipped) ]);

    while type_of(right) != "()" {
        step("cmpLoop", `Comparing the ${ordinal_from_ends(left, right, nodes)}.`,
            [ list("list", nodes).pointers(#{ left: left, right: right }).reversed(flipped) ]);

        if nodes[left].val != nodes[right].val {
            ok = false;
            log(`${nodes[left].val} != ${nodes[right].val}`, "result");
            step("cmp", `${nodes[left].val} against ${nodes[right].val} — they differ.`,
                [ list("list", nodes).pointers(#{ left: left, right: right }).reversed(flipped) ]);
            step("no", "One mismatch is enough.",
                [ list("list", nodes).pointers(#{ left: left, right: right }) ], "return");
            break;
        }

        step("cmp", `${nodes[left].val} matches ${nodes[right].val}.`,
            [ list("list", nodes).pointers(#{ left: left, right: right }).reversed(flipped) ]);
        left = nodes[left].next;
        right = nodes[right].next;
        step("advance", "Step both inwards.",
            [ list("list", nodes).pointers(#{ left: left, right: right }).reversed(flipped) ]);
    }

    if ok {
        log("palindrome", "result");
        step("yes", "The reversed half ran out with every pair matching, so the list is a palindrome. An odd middle node never needed comparing — it is its own mirror.",
            [ list("list", nodes).reversed(flipped) ], "return");
    }

    result(`${ok}`);
    leave(`${ok}`);
}

fn ordinal_from_ends(left, right, nodes) {
    `front node(${nodes[left].val}) against back node(${nodes[right].val})`
}
