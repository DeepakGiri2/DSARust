// Linked List Cycle — Floyd's tortoise and hare.
//
// Inside a loop the fast pointer gains exactly one position on the slow one
// every step, so it can never jump over it — it must land on it. Watch the gap
// between the two markers shrink by one each time.

fn trace(input) {
    let values = input.list;
    let n = values.len();
    let pos = input.pos;

    // Build the list, then close the cycle if `pos` points back into it.
    let nodes = [];
    for i in 0..n {
        nodes.push(#{
            id: i,
            val: values[i],
            next: if i + 1 < n { i + 1 } else if pos >= 0 && pos < n { pos } else { () },
        });
    }

    let slow = if n > 0 { 0 } else { () };
    let fast = slow;

    enter("hasCycle", #{ head: PTR(if n > 0 { `node(${values[0]})` } else { "nil" }) });
    set(#{ slow: PTR(if n > 0 { `node(${values[0]})` } else { "nil" }) });
    step("init", "The slow pointer starts at the head and moves one node per step.",
        [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);

    set(#{ fast: PTR(if n > 0 { `node(${values[0]})` } else { "nil" }) });
    step("init2", "The fast pointer starts there too, but moves two.",
        [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);

    let guard = 0;
    while type_of(fast) != "()" && type_of(nodes[fast].next) != "()" && guard < 200 {
        guard += 1;
        step("loop", "fast still has two nodes ahead of it, so another double step is possible.",
            [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);

        slow = nodes[slow].next;
        set(#{ slow: PTR(`node(${nodes[slow].val})`) });
        step("slow", `slow advances one, to node(${nodes[slow].val}).`,
            [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);

        fast = nodes[nodes[fast].next].next;
        set(#{ fast: PTR(if type_of(fast) == "()" { "nil" } else { `node(${nodes[fast].val})` }) });
        step("fast",
            if type_of(fast) == "()" {
                "fast advances two and runs off the end."
            } else {
                `fast advances two, to node(${nodes[fast].val}). It gains exactly one position on slow each round.`
            },
            [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);

        if type_of(fast) != "()" && slow == fast {
            log(`slow and fast met at node(${nodes[slow].val})`, "result");
            result("true");
            step("check", `Both pointers are on node(${nodes[slow].val}). Only a cycle can make that happen.`,
                [ list("list", nodes).pointers(#{ slow: slow, fast: fast }).reversed([slow]) ]);
            step("true", "Return true.",
                [ list("list", nodes).pointers(#{ slow: slow }).reversed([slow]) ], "return");
            leave("true");
            return;
        }
        if type_of(fast) != "()" {
            step("check", "The pointers are still on different nodes.",
                [ list("list", nodes).pointers(#{ slow: slow, fast: fast }) ]);
        }
    }

    log("fast reached the end", "result");
    result("false");
    step("false",
        "fast walked off the end of the list. A pointer can only do that if the list terminates, so there is no cycle.",
        [ list("list", nodes).pointers(#{ slow: slow }) ],
        "return");
    leave("false");
}
