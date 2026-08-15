fn parse_pairs(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 2 { out.push([num(w[0]), num(w[1])]); }
    }
    out
}

// Redundant Connection — union-find turns "is there a cycle?" into a lookup.
//
// Adding edges in the order given, an edge is redundant exactly when its two
// endpoints are already connected: joining them again must close a loop. Since
// only one extra edge exists, the first edge that fails this test is the
// answer — and because the scan follows the input order, "first found" is also
// "last in the input", which is what the problem asks for.

fn find_root(parent, x) {
    enter(`find(${x})`, #{ x: N(x) });
    let cur = x;
    while parent[cur] != cur {
        step("walk", `${cur} is not its own root; its parent is ${parent[cur]}.`,
            [ array("parent of each node", parent).ptr("x", cur).hl([cur]) ]);
        parent[cur] = parent[parent[cur]];
        step("compress", `Point ${cur} straight at ${parent[cur]}, halving the path for next time.`,
            [ array("parent of each node", parent).ptr("x", cur).hl([cur]) ]);
        cur = parent[cur];
        step("up", `Move up to ${cur}.`,
            [ array("parent of each node", parent).ptr("x", cur) ]);
    }
    step("root", `${cur} is its own parent, so it is the root of this component.`,
        [ array("parent of each node", parent).ptr("root", cur).hl([cur]) ], "return");
    leave(`${cur}`);
    [parent, cur]
}

fn node_labels(n) {
    let out = [];
    for i in 0..(n + 1) { out.push(`${i}`); }
    out
}

fn edges_so_far(pairs, k) {
    let out = [];
    for i in 0..k { out.push([pairs[i][0], pairs[i][1]]); }
    out
}

fn trace(input) {
    let pairs = parse_pairs(input.edges);
    let n = pairs.len();

    enter("findRedundantConnection", #{ edges: N(n) });

    let parent = [];
    for i in 0..(n + 1) { parent.push(i); }
    step("init", `${n} nodes, each in a component of its own to begin with.`,
        [ graph("graph so far", node_labels(n), []),
          array("parent of each node", parent) ]);
    step("seed", "Every node is its own root — that is what 'not yet connected to anything' looks like.",
        [ array("parent of each node", parent) ]);

    let answer = "";
    for k in 0..n {
        let e = pairs[k];
        step("loop", `Edge ${k}: ${e[0]}–${e[1]}.`,
            [ graph("graph so far", node_labels(n), edges_so_far(pairs, k)).frontier([e[0], e[1]]),
              array("parent of each node", parent) ]);

        step("find", `Find the roots of ${e[0]} and ${e[1]}.`,
            [ array("parent of each node", parent) ], "call");
        let ra = find_root(parent, e[0]);
        parent = ra[0];
        let rb = find_root(parent, e[1]);
        parent = rb[0];

        if ra[1] == rb[1] {
            answer = `${e[0]} ${e[1]}`;
            log(`redundant edge ${answer}`, "result");
            step("cycle", `Both already have root ${ra[1]}, so they were connected before this edge existed — adding it closes a cycle.`,
                [ graph("graph so far", node_labels(n), edges_so_far(pairs, k + 1)).bad([e[0], e[1]]),
                  array("parent of each node", parent) ]);
            step("ret", `Edge ${e[0]}–${e[1]} is the redundant one.`,
                [ graph("graph so far", node_labels(n), edges_so_far(pairs, k + 1)).bad([e[0], e[1]]) ],
                "return");
            break;
        }

        step("cycle", `Roots ${ra[1]} and ${rb[1]} differ, so this edge joins two separate components — no cycle.`,
            [ graph("graph so far", node_labels(n), edges_so_far(pairs, k + 1)).done([e[0], e[1]]) ]);
        parent[ra[1]] = rb[1];
        log(`merge component ${ra[1]} into ${rb[1]}`);
        step("union", `Merge them: ${ra[1]}'s root becomes ${rb[1]}.`,
            [ array("parent of each node", parent).hl([ra[1], rb[1]]) ]);
    }

    if answer == "" {
        log("no redundant edge", "result");
        step("none", "Every edge joined two distinct components, so nothing is redundant.",
            [ graph("graph so far", node_labels(n), edges_so_far(pairs, n)) ], "return");
        answer = "none";
    }

    result(answer);
    leave(answer);
}
