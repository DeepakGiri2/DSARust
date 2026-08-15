fn parse_pairs(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 2 { out.push([num(w[0]), num(w[1])]); }
    }
    out
}

// Graph Valid Tree — count the edges, then rule out cycles.
//
// "Connected and acyclic" looks like two searches. But an n-node graph with
// exactly n-1 edges is connected if and only if it is acyclic — one condition
// implies the other once the count is fixed. So the edge count is checked
// first, and union-find then only has to reject a cycle. Connectivity is never
// examined directly, and it does not need to be.

fn find_root(parent, x) {
    enter(`find(${x})`, #{ x: N(x) });
    let cur = x;
    while parent[cur] != cur {
        step("walk", `${cur} points at ${parent[cur]}; keep climbing.`,
            [ array("parent of each node", parent).ptr("x", cur).hl([cur]) ]);
        parent[cur] = parent[parent[cur]];
        cur = parent[cur];
    }
    step("root", `${cur} is the root of ${x}'s component.`,
        [ array("parent of each node", parent).ptr("root", cur).hl([cur]) ], "return");
    leave(`${cur}`);
    [parent, cur]
}

fn node_labels(n) {
    let out = [];
    for i in 0..n { out.push(`${i}`); }
    out
}

fn edges_so_far(pairs, k) {
    let out = [];
    for i in 0..k { out.push([pairs[i][0], pairs[i][1]]); }
    out
}

fn trace(input) {
    let n = input.n;
    let pairs = parse_pairs(input.edges);
    let m = pairs.len();

    enter("validTree", #{ n: N(n), edges: N(m) });

    if m != n - 1 {
        log(`${m} edges, needs ${n - 1}`, "result");
        step("count", `${plural(m, "edge")} for ${n} nodes. A tree needs exactly ${n - 1}: fewer cannot connect everything, more must create a cycle.`,
            [ graph("graph", node_labels(n), edges_so_far(pairs, m)),
              text("counts", [`nodes: ${n}`, `edges: ${m}`, `a tree needs: ${n - 1}`]) ], "return");
        result("false");
        leave("false");
        return;
    }
    step("count", `${plural(m, "edge")} for ${n} nodes is exactly right for a tree. From here, acyclic and connected are the same condition.`,
        [ graph("graph", node_labels(n), edges_so_far(pairs, m)),
          text("counts", [`nodes: ${n}`, `edges: ${m}`]) ]);

    let parent = [];
    for i in 0..n { parent.push(i); }
    step("init", "Union-find over the nodes.",
        [ array("parent of each node", parent) ]);
    step("seed", "Each node starts alone.",
        [ array("parent of each node", parent) ]);

    let ok = true;
    for k in 0..m {
        let e = pairs[k];
        step("loop", `Edge ${k}: ${e[0]}–${e[1]}.`,
            [ graph("graph", node_labels(n), edges_so_far(pairs, k)).frontier([e[0], e[1]]),
              array("parent of each node", parent) ]);

        step("find", `Roots of ${e[0]} and ${e[1]}.`,
            [ array("parent of each node", parent) ], "call");
        let ra = find_root(parent, e[0]);
        parent = ra[0];
        let rb = find_root(parent, e[1]);
        parent = rb[0];

        if ra[1] == rb[1] {
            ok = false;
            log(`cycle at edge ${e[0]}–${e[1]}`, "result");
            step("cycle", `Both are already in component ${ra[1]}, so this edge closes a cycle.`,
                [ graph("graph", node_labels(n), edges_so_far(pairs, k + 1)).bad([e[0], e[1]]) ]);
            step("no", "A cycle with exactly n-1 edges also means something is disconnected — either way, not a tree.",
                [ graph("graph", node_labels(n), edges_so_far(pairs, m)) ], "return");
            break;
        }

        step("cycle", `Roots ${ra[1]} and ${rb[1]} differ — this edge joins two separate pieces.`,
            [ graph("graph", node_labels(n), edges_so_far(pairs, k + 1)).done([e[0], e[1]]) ]);
        parent[ra[1]] = rb[1];
        step("union", `Merge the components.`,
            [ array("parent of each node", parent).hl([ra[1], rb[1]]) ]);
    }

    if ok {
        log("valid tree", "result");
        step("yes", `Every edge joined two distinct pieces, so ${n - 1} merges reduced ${n} components to one — connected and acyclic.`,
            [ graph("graph", node_labels(n), edges_so_far(pairs, m)) ], "return");
    }

    result(`${ok}`);
    leave(`${ok}`);
}
