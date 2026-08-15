// Course Schedule II — Kahn's algorithm, driven by in-degrees.
//
// A course is takeable exactly when nothing is still owed to it. Taking one
// releases its dependents, which is a decrement rather than a re-scan. The
// cycle check falls out for free: if some course never reaches zero, it never
// enters the order, and the order comes up short.

fn nodes_of(n) {
    let out = [];
    for i in 0..n { out.push(#{ id: i, label: `${i}` }); }
    out
}

fn zero_nodes(indeg) {
    let out = [];
    for i in 0..indeg.len() { if indeg[i] == 0 { out.push(i); } }
    out
}

fn trace(input) {
    let n = input.numCourses;
    let edges = [];
    for pair in words(input.prereqs) {
        let parts = pair.split(",");
        if parts.len() == 2 {
            let a = num(parts[0]);
            let b = num(parts[1]);
            if a >= 0 && a < n && b >= 0 && b < n { edges.push([b, a, true]); }
        }
    }
    let nodes = nodes_of(n);

    let adj = [];
    for i in 0..n { adj.push([]); }
    let indeg = [];
    for i in 0..n { indeg.push(0); }

    enter("findOrder", #{ numCourses: N(n) });
    step("adj", `${n} courses.`, [ graph("courses", nodes, []) ]);
    set(#{ indeg: A(indeg) });
    step("indeg", "indeg[c] counts how many prerequisites course c is still waiting on.",
        [ graph("courses", nodes, []), array("in-degree", indeg) ]);

    for e in edges {
        adj[e[0]].push(e[1]);
        indeg[e[1]] += 1;
        set(#{ indeg: A(indeg) });
        step("build", `Course ${e[1]} needs course ${e[0]} first.`,
            [ graph("courses", nodes, edges).active([[e[0], e[1]]]) ]);
        step("edge", `Add the edge ${e[0]} → ${e[1]}.`,
            [ graph("courses", nodes, edges).active([[e[0], e[1]]]) ]);
        step("count", `in-degree of ${e[1]} is now ${indeg[e[1]]}.`,
            [ array("in-degree", indeg).hl([e[1]]) ]);
    }

    let queue = [];
    step("queue", "Anything with in-degree 0 can be taken immediately.",
        [ graph("courses", nodes, edges), array("in-degree", indeg) ]);

    for c in 0..n {
        step("seed", `Course ${c} has in-degree ${indeg[c]}.`,
            [ array("in-degree", indeg).ptr("c", c) ]);
        if indeg[c] == 0 {
            queue.push(c);
            step("free", `Nothing is owed to course ${c}.`,
                [ graph("courses", nodes, edges).frontier(queue) ]);
            step("enqueue", `Queue it.`,
                [ graph("courses", nodes, edges).frontier(queue),
                  queue("ready", queue).pushed(true) ]);
        }
    }

    let order = [];
    set(#{ order: A(order) });
    step("order", "Take courses as they become free.",
        [ graph("courses", nodes, edges).frontier(queue), queue("ready", queue) ]);

    while queue.len() > 0 {
        step("loop", `${queue.len()} course(s) ready.`,
            [ graph("courses", nodes, edges).frontier(queue).done(order),
              queue("ready", queue) ]);

        let c = queue[0];
        queue.remove(0);
        step("take", `Take course ${c}.`,
            [ graph("courses", nodes, edges).cur(c).done(order),
              queue("ready", queue).popped(`${c}`) ]);

        order.push(c);
        set(#{ order: A(order) });
        log(`take ${c}`);
        step("append", `Order so far: ${brackets(order)}.`,
            [ graph("courses", nodes, edges).done(order), array("order", order) ]);

        for next in adj[c] {
            step("release", `Course ${next} was waiting on ${c}.`,
                [ graph("courses", nodes, edges).cur(next).active([[c, next]]).done(order) ]);
            indeg[next] -= 1;
            set(#{ indeg: A(indeg) });
            step("dec", `Its in-degree drops to ${indeg[next]}.`,
                [ array("in-degree", indeg).hl([next]) ]);
            if indeg[next] == 0 {
                queue.push(next);
                step("nowFree", `Nothing is owed to ${next} any more.`,
                    [ graph("courses", nodes, edges).frontier(queue).done(order) ]);
                step("enqueue2", "Queue it.",
                    [ queue("ready", queue).pushed(true) ]);
            }
        }
    }

    if order.len() != n {
        step("check", `Only ${order.len()} of ${n} courses were taken.`,
            [ graph("courses", nodes, edges).done(order).bad(zero_nodes(indeg)) ]);
        log("cycle detected", "result");
        result("");
        step("cycle",
            "The rest never reached in-degree 0, which can only happen if they depend on each other in a loop. Return nothing.",
            [ graph("courses", nodes, edges).bad(seq(0, n - 1)) ], "return");
        leave("");
        return;
    }

    let answer = join(order, " ");
    step("check", `All ${n} courses were taken.`,
        [ graph("courses", nodes, edges).done(order) ]);
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `A valid order: ${answer}.`,
        [ array("order", order).hl(seq(0, order.len() - 1)) ], "return");
    leave(answer);
}
