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

// Reorder List — three familiar passes, composed.
//
// Nothing here is new: find the middle, reverse a list, merge two lists. What
// makes it work in O(1) space is doing all three by relinking rather than
// copying into an array.

fn trace(input) {
    let values = input.list;
    let n = values.len();
    let nodes = chain(values);
    let head = if n > 0 { 0 } else { () };

    enter("reorderList", #{});
    let slow = head;
    let fast = nodes[head].next;
    step("mid", "Find the middle with a slow and a fast pointer.",
        [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);

    while type_of(fast) != "()" && type_of(nodes[fast].next) != "()" {
        slow = nodes[slow].next;
        fast = nodes[nodes[fast].next].next;
        step("midLoop", `slow at node(${nodes[slow].val}), fast twice as far.`,
            [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);
    }

    let second = nodes[slow].next;
    step("split", `The second half starts at node(${nodes[second].val}).`,
        [ list("list", nodes).pointers(#{ slow: slow, second: second }) ]);

    nodes[slow].next = ();
    step("cut", "Cut the list in two.",
        [ list("list", nodes).pointers(#{ first: head, second: second }) ]);

    let prev = ();
    let flipped = [];
    step("revInit", "Now reverse the second half.",
        [ list("list", nodes).pointers(#{ second: second }) ]);

    while type_of(second) != "()" {
        let next = nodes[second].next;
        nodes[second].next = prev;
        flipped.push(second);
        step("revLoop", `Point node(${nodes[second].val}) backwards.`,
            [ list("list", nodes).pointers(#{ second: second, prev: prev }).reversed(flipped) ]);
        step("flip", "One arrow flipped.",
            [ list("list", nodes).pointers(#{ second: next }).reversed(flipped) ]);
        prev = second;
        second = next;
    }

    let first = head;
    let sec = prev;
    step("weaveInit", "Two halves: the first in order, the second reversed.",
        [ list("list", nodes).pointers(#{ first: first, sec: sec }).reversed(flipped) ]);

    while type_of(sec) != "()" {
        step("weave", `Weave node(${nodes[first].val}) with node(${nodes[sec].val}).`,
            [ list("list", nodes).pointers(#{ first: first, sec: sec }) ]);
        let f = nodes[first].next;
        let s = nodes[sec].next;
        step("save", "Save both next pointers before relinking destroys them.",
            [ list("list", nodes).pointers(#{ f: f, s: s }) ]);
        nodes[first].next = sec;
        step("linkA", `node(${nodes[first].val}) now points at node(${nodes[sec].val}).`,
            [ list("list", nodes).pointers(#{ first: first, sec: sec }) ]);
        if type_of(f) != "()" {
            nodes[sec].next = f;
            step("linkB", `node(${nodes[sec].val}) points back into the first half.`,
                [ list("list", nodes).pointers(#{ first: first, sec: sec }) ]);
        } else {
            nodes[sec].next = ();
            step("linkB", "The first half is exhausted, so this is the tail.",
                [ list("list", nodes).pointers(#{ sec: sec }) ]);
        }
        first = f;
        sec = s;
        step("advance", "Move both pointers on.",
            [ list("list", nodes).pointers(#{ first: first, sec: sec }) ]);
    }

    let answer = join(order_of(nodes, head), " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("weave", `Reordered in place: ${answer}.`,
        [ list("list", nodes).pointers(#{ head: head }) ], "return");
    leave(answer);
}
